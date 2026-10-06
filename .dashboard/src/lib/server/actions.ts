import { readdir, rm, stat } from 'node:fs/promises';
import { join } from 'node:path';
import { run, runWithStdin } from './exec.js';
import { LIVE_ROUND_WINDOW_MS } from '$lib/config.js';
import type { ActionResult } from '$lib/types.js';

const REPO = '/home/ayin/projs/ARCMiS';
const EXP = `${REPO}/.artifacts/experiments`;
const DRIVER_UNIT = 'q27sweep-driver';
const WATCHDOG_UNIT = 'q27sweep-watchdog';

/** Only plain dir names may reach a filesystem path or CLI arg. */
function sanitizeDir(input: string): string | null {
	if (!/^[A-Za-z0-9_.-]+$/.test(input) || input.startsWith('.')) return null;
	return input;
}

const DRIVER_RELAUNCH_ARGS = [
	'--user',
	'--slice=q27sweep.slice',
	`--unit=${DRIVER_UNIT}`,
	'-p',
	'WorkingDirectory=/home/ayin/projs/ARCMiS',
	'--setenv=SLOTS=2',
	'--setenv=STAGGER=240',
	'--setenv=ENGINE_URL=http://localhost:8081/v1',
	'--setenv=HYPOTHESIS=dashboard relaunch',
	'--',
	'bash',
	`${REPO}/scripts/sweep.sh`,
	'.artifacts/experiments/PAIRS-ninfer-w2.txt',
	'3'
];

export type ActionName =
	| 'start'
	| 'stop'
	| 'restart-engine'
	| 'rescore'
	| 'ledger-sync'
	| 'purge-orphans';

export async function runAction(
	name: ActionName,
	body: { dir?: string; confirm?: boolean }
): Promise<ActionResult> {
	switch (name) {
		case 'start':
			return startSweep();
		case 'stop':
			return stopSweep();
		case 'restart-engine':
			return restartEngine();
		case 'rescore':
			return rescore(body.dir ?? '');
		case 'ledger-sync':
			return ledgerSync();
		case 'purge-orphans':
			return body.dir ? purgeSpecific(body.dir, body.confirm === true) : purgeOrphansList();
	}
}

async function startSweep(): Promise<ActionResult> {
	// Watchdog twin: mirror the live transient unit's ExecStart verbatim.
	const wdCat = await run('systemctl', ['--user', 'cat', WATCHDOG_UNIT]);
	const wdExecLine = wdCat.out
		.split('\n')
		.find((l) => l.startsWith('ExecStart=') && l !== 'ExecStart=');
	const wdArgs = wdExecLine
		? wdExecLine
				.slice('ExecStart='.length)
				.split('\t')
				.slice(1)
				.map((a) => a.replace(/^"|"$/g, ''))
		: ['env', 'bash', 'scripts/watchdog.sh', '.artifacts/experiments/PAIRS-ninfer-w2.txt'];
	const watchdog = await run('systemd-run', [
		'--user',
		'--slice=q27sweep.slice',
		`--unit=${WATCHDOG_UNIT}`,
		...wdArgs
	]);
	const driver = await run('systemd-run', DRIVER_RELAUNCH_ARGS);
	const detail =
		driver.ok && watchdog.ok
			? 'driver + watchdog launched'
			: `driver ${driver.ok ? 'ok' : 'failed'}; watchdog ${watchdog.ok ? 'ok' : 'failed: ' + (watchdog.err.trim() || 'error')}`;
	return { ok: driver.ok && watchdog.ok, detail };
}

async function stopSweep(): Promise<ActionResult> {
	const r = await run('systemctl', [
		'--user',
		'stop',
		DRIVER_UNIT,
		WATCHDOG_UNIT
	]);
	return {
		ok: r.ok,
		detail: r.ok ? 'driver + watchdog stopped' : r.err.trim() || 'systemctl stop failed'
	};
}

async function restartEngine(): Promise<ActionResult> {
	const pass = process.env.DASHBOARD_SUDO_PASSWORD ?? '';
	if (!pass) {
		return {
			ok: false,
			detail: 'DASHBOARD_SUDO_PASSWORD not set — export it in .dashboard/.env before restarting the engine'
		};
	}
	const r = await runWithStdin(
		'sudo',
		['-S', '-p', '', 'systemctl', 'restart', 'ninfer-serve'],
		pass + '\n',
		15_000
	);
	return {
		ok: r.ok,
		detail: r.ok ? 'ninfer-serve restarted' : r.err.trim() || 'sudo systemctl restart failed'
	};
}

async function rescore(dirInput: string): Promise<ActionResult> {
	const dir = sanitizeDir(dirInput);
	if (!dir) return { ok: false, detail: 'invalid dir name — use the round directory name only' };
	const r = await run('python3', ['scripts/rescore.py', join(EXP, dir)], 600_000, REPO);
	const tail = (r.out + '\n' + r.err).trim().split('\n').slice(-8).join('\n');
	return { ok: r.ok, detail: r.ok ? `rescore ok — ${tail || dir}` : `${tail || 'rescore failed'}` };
}

async function ledgerSync(): Promise<ActionResult> {
	const r = await run('python3', ['scripts/rounds_ledger.py'], 120_000, REPO);
	const tail = (r.out + '\n' + r.err).trim().split('\n').slice(-4).join(' · ');
	return { ok: r.ok, detail: r.ok ? `ok — ${tail || 'ledger regenerated'}` : tail || 'ledger sync failed' };
}

/** Orphans: no result/aggregate.yml AND no event activity inside the live window. */
async function orphanDirs(): Promise<string[]> {
	const dirs = (await readdir(EXP, { withFileTypes: true }))
		.filter((d) => d.isDirectory() && /^\d{8}T\d{6}Zv3s\d+-.+-sweep$/.test(d.name))
		.map((d) => d.name)
		.sort();
	const out: string[] = [];
	for (const dir of dirs) {
		const full = join(EXP, dir);
		try {
			await stat(`${full}/result/aggregate.yml`);
			continue;
		} catch {
			// candidate
		}
		try {
			const ev = await stat(`${full}/events.jsonl`);
			if (Date.now() - ev.mtimeMs < LIVE_ROUND_WINDOW_MS) continue;
		} catch {
			// no events at all → orphan
		}
		out.push(dir);
	}
	return out;
}

async function purgeOrphansList(): Promise<ActionResult> {
	const orphans = await orphanDirs();
	if (orphans.length === 0) return { ok: true, detail: 'no orphan rounds' };
	return { ok: true, detail: orphans.join('\n') };
}

async function purgeSpecific(dirInput: string, confirm: boolean): Promise<ActionResult> {
	const dir = sanitizeDir(dirInput);
	if (!dir) return { ok: false, detail: 'invalid dir name' };
	if (!confirm) return { ok: false, detail: 'purge requires confirm: true' };
	await rm(join(EXP, dir), { recursive: true, force: true });
	return { ok: true, detail: `ok — removed ${dir}` };
}
