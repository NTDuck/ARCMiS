import { readFile, readdir, stat } from 'node:fs/promises';
import { join } from "node:path";
import { load as loadYaml } from 'js-yaml';
import { run } from './exec.js';
import { HEARTBEAT_STALE_MS, LIVE_ROUND_WINDOW_MS } from '$lib/config.js';
import type {
	EngineState,
	InFlightRound,
	LedgerRow,
	ScoreboardState,
	StatusPayload,
	UnitState
} from '$lib/types.js';

const REPO = '/home/ayin/projs/ARCMiS';
const EXP = `${REPO}/.artifacts/experiments`;
const CLEARED_VERDICTS = new Set(['SOLVED', 'CLEARED', 'VERIFIED-WORKSPACE']);

export async function scoreboard(): Promise<ScoreboardState> {
	const state: ScoreboardState = {
		cleared: null,
		total: 114,
		waveId: null,
		wavePairs: null,
		launched: null,
		done: null,
		clearsPerDay: null,
		error: null
	};
	try {
		const raw = await readFile(`${EXP}/ROUNDS.yaml`, 'utf8');
		const doc: unknown = loadYaml(raw);
		const rounds =
			Array.isArray(doc)
				? doc
				: doc && typeof doc === 'object' && 'rounds' in doc && Array.isArray(doc.rounds)
					? doc.rounds
					: [];
		// Ledger is append-ordered; the LAST entry per candidate_id is the current verdict.
		// Cleared counts sweep rounds only (v0.3.sweep.*); one-shot v3rN probes don't bank coverage.
		const latest = new Map<string, string>();
		for (const r of rounds) {
			if (
				r &&
				typeof r === 'object' &&
				'candidate_id' in r &&
				typeof r.candidate_id === 'string' &&
				'verdict' in r &&
				typeof r.verdict === 'string'
			) {
				latest.set(r.candidate_id, r.verdict);
			}
		}
		const sweepLatest = [...latest.entries()].filter(([id]) => /v3s\d+/.test(id));
		state.cleared = sweepLatest.filter(([, v]) => CLEARED_VERDICTS.has(v)).length;
		// Rate: cleared rounds / distinct days on which any sweep round was launched.
		const daySet = new Set<string>();
		for (const [id] of sweepLatest) {
			const m = id.match(/^(\d{8})T/);
			if (m) daySet.add(m[1]);
		}
		if (daySet.size > 0) {
			state.clearsPerDay = Math.round((state.cleared / daySet.size) * 100) / 100;
		}
	} catch (e) {
		state.error = `ledger read failed: ${(e as Error).message}`;
	}

	// Wave: pair list line count vs round dirs launched for that wave.
	try {
		const pairs = await readFile(`${EXP}/PAIRS-ninfer-w2.txt`, 'utf8');
		const lines = pairs.split('\n').filter((l) => l.trim() && !l.trim().startsWith('#'));
		state.waveId = 'ninfer-w2';
		state.wavePairs = lines.length;
		const tags = lines.map((l) => l.split('|').pop()?.trim() ?? l.trim());
		const dirs = await readdir(EXP, { withFileTypes: true });
		let launched = 0;
		let closed = 0;
		for (const d of dirs) {
			if (!d.isDirectory() || !/^\d{8}T\d{6}Zv3s\d+-.+-sweep$/.test(d.name)) continue;
			const problem = d.name.split('-sweep')[0].replace(/^\d{8}T\d{6}Zv3s\d+-/, '');
			if (!tags.includes(problem)) continue;
			launched += 1;
			try {
				await stat(`${EXP}/${d.name}/result/aggregate.yml`);
				closed += 1;
			} catch {
				// in flight
			}
		}
		state.launched = launched;
		state.done = closed;
	} catch (e) {
		state.error = state.error ?? `wave read failed: ${(e as Error).message}`;
	}
	return state;
}

export async function units(): Promise<UnitState[]> {
	const probes: Promise<UnitState>[] = [
		['--user', 'q27sweep-driver'],
		['--user', 'q27sweep-watchdog'],
		['', 'ninfer-serve']
	].map(async ([scope, unit]) => {
		const args = scope ? [scope, 'is-active', unit] : ['is-active', unit];
		const r = await run('systemctl', args);
		return { unit, active: r.out.trim().split('\n')[0] || 'unknown', error: r.ok ? null : r.err.trim() || null };
	});
	return Promise.all(probes);
}

export async function heartbeats() {
	const one = async (name: string) => {
		try {
			const s = await stat(`${EXP}/.sweep-heartbeat`.replace('.sweep', `.${name}`));
			const ageS = Math.round((Date.now() - s.mtimeMs) / 1000);
			return { ageS, stale: ageS > HEARTBEAT_STALE_MS / 1000, error: null };
		} catch {
			return { ageS: null, stale: false, error: 'missing' };
		}
	};
	const [sweep, watchdog] = await Promise.all([one('sweep'), one('watchdog')]);
	return { sweep, watchdog };
}

export async function inFlight(): Promise<InFlightRound[]> {
	const out: InFlightRound[] = [];
	let dirs: string[];
	try {
		dirs = (await readdir(EXP, { withFileTypes: true }))
			.filter((d) => d.isDirectory() && /^\d{8}T\d{6}Zv3s\d+-.+-sweep$/.test(d.name))
			.map((d) => d.name)
			.sort();
	} catch {
		return out;
	}
	for (const dir of dirs) {
		const full = join(EXP, dir);
		try {
			await stat(`${full}/result/aggregate.yml`);
			continue; // closed
		} catch {
			// still live-candidate
		}
		try {
			const evStat = await stat(`${full}/events.jsonl`);
			if (Date.now() - evStat.mtimeMs >= LIVE_ROUND_WINDOW_MS) continue; // quiet
			const raw = await readFile(`${full}/events.jsonl`, 'utf8');
			let phase = '—';
			for (const line of raw.trimEnd().split('\n').slice(-40)) {
				try {
					const ev = JSON.parse(line);
					if (ev?.event === 'phase' && ev?.fields?.to) phase = ev.fields.to;
				} catch {
					// tolerate torn tail line
				}
			}
			let wallS: number | null = null;
			try {
				const cfg: unknown = JSON.parse(await readFile(`${full}/manifest.json`, 'utf8'));
				const started =
					cfg && typeof cfg === 'object' && 'created_at' in cfg && typeof cfg.created_at === 'string'
						? cfg.created_at
						: null;
				wallS = started
					? Math.round((Date.now() - Date.parse(started)) / 1000)
					: Math.round((Date.now() - evStat.mtimeMs) / 1000);
			} catch {
				wallS = Math.round((Date.now() - evStat.mtimeMs) / 1000);
			}
			const problem = dir.replace(/^\d{8}T\d{6}Zv3s\d+-/, '').replace(/-sweep$/, '');
			out.push({
				dir,
				problem,
				phase,
				lastEventAgeS: Math.round((Date.now() - evStat.mtimeMs) / 1000),
				wallS
			});
		} catch {
			continue;
		}
	}
	return out;
}

export async function recentCloses(): Promise<LedgerRow[] | { error: string }> {
	try {
		const raw = await readFile(`${EXP}/ROUNDS.yaml`, 'utf8');
		const doc: unknown = loadYaml(raw);
		const rounds =
			doc && typeof doc === 'object' && 'rounds' in doc && Array.isArray(doc.rounds) ? doc.rounds : [];
		const rows: LedgerRow[] = [];
		for (const r of rounds) {
			if (!r || typeof r !== 'object' || !('candidate_id' in r)) continue;
			const id = typeof r.candidate_id === 'string' ? r.candidate_id : '';
			if (!/^\d{8}T\d{6}Z/.test(id)) continue;
			const m = id.match(/^(\d{8})T(\d{6})Z/);
			// Rows sort by candidate_id launch timestamp: the ledger has no close-time
			// field, and under near-serial slot launch this approximates close order.
			const closeTime = m
				? Date.parse(
						`${m[1].slice(0, 4)}-${m[1].slice(4, 6)}-${m[1].slice(6, 8)}T${m[2].slice(0, 2)}:${m[2].slice(2, 4)}:${m[2].slice(4, 6)}Z`
					)
				: null;
			const metrics =
				'metrics' in r && r.metrics && typeof r.metrics === 'object'
					? (r.metrics as Record<string, unknown>)
					: {};
			const verdict = 'verdict' in r && typeof r.verdict === 'string' ? r.verdict : '—';
			// A closes panel must not show open rounds (gate 46 family): drop IN FLIGHT rows.
			if (verdict === 'IN FLIGHT') continue;
			// Unknown metrics render as '—', never the word "unknown".
			const testsPassed = metrics.tests_passed;
			const testsFailed = metrics.tests_failed;
			const tests =
				typeof testsPassed === 'number' && typeof testsFailed === 'number'
					? `${testsPassed}/${testsFailed}`
					: '—';
			const wallSeconds = metrics.wall_seconds;
			const wallS = typeof wallSeconds === 'number' ? String(wallSeconds) : '—';
			rows.push({
				roundTag: 'round' in r && typeof r.round === 'string' ? r.round : '—',
				problem: id.replace(/^\d{8}T\d{6}Zv3s?\d*-/, '').replace(/-sweep$/, ''),
				verdict,
				tests,
				wallS,
				closeTime
			});
		}
		rows.sort((a, b) => (b.closeTime ?? 0) - (a.closeTime ?? 0));
		return rows.slice(0, 8);
	} catch (e) {
		return { error: `ledger read failed: ${(e as Error).message}` };
	}
}

export async function engine(): Promise<EngineState> {
	const errors: string[] = [];
	const state: EngineState = {
		modelId: null,
		ownedBy: null,
		match: false,
		vmrssKb: null,
		gpuName: null,
		psi: { cpu: null, mem: null, io: null },
		gpu: { util: null, memUsed: null, memTotal: null },
		errors
	};

	// Model id
	try {
		const ctl = AbortSignal.timeout(3000);
		const res = await fetch('http://localhost:8081/v1/models', { signal: ctl });
		const doc: unknown = await res.json();
		const first =
			doc && typeof doc === 'object' && 'data' in doc && Array.isArray(doc.data) && doc.data.length > 0
				? doc.data[0]
				: null;
		if (first && typeof first === 'object' && 'id' in first && typeof first.id === 'string') {
			state.modelId = first.id;
			state.ownedBy =
				'owned_by' in first && typeof first.owned_by === 'string' ? first.owned_by : null;
			state.match = state.modelId === 'qwen3.8-27b' && state.ownedBy === 'ninfer';
		}
	} catch (e) {
		errors.push(`models: ${(e as Error).message}`);
	}

	// MainPID + VmRSS + PSI in parallel
	const [pid, psi] = await Promise.all([mainPid(), readPsi()]);
	if (pid.error) {
		errors.push(pid.error);
	} else {
		try {
			const status = await readFile(`/proc/${pid.value}/status`, 'utf8');
			const m = status.match(/^VmRSS:\s+(\d+)\s+kB/m);
			state.vmrssKb = m ? parseInt(m[1], 10) : null;
		} catch (e) {
			errors.push(`vmrss: ${(e as Error).message}`);
		}
	}
	state.psi = psi;

	// GPU: the unit pins CUDA_VISIBLE_DEVICES to a UUID; read it and match the row.
	// Fallback: pick the row with the largest memory.total (the datacenter card).
	const [uuid, uuidErr] = await unitCudaUuid();
	if (uuidErr) errors.push(uuidErr);
	const gpuRows = await listGpus();
	if (gpuRows.length > 0) {
		const byUuid = uuid ? gpuRows.find((g) => g.uuid === uuid) : undefined;
		const selected =
			byUuid ?? gpuRows.reduce((a, b) => ((b.memTotal ?? 0) > (a.memTotal ?? 0) ? b : a));
		state.gpuName = selected.name;
		state.gpu = { util: selected.util, memUsed: selected.memUsed, memTotal: selected.memTotal };
	} else {
		errors.push('nvidia-smi failed or returned no rows');
	}
	return state;
}

interface GpuRow {
	uuid: string | null;
	name: string | null;
	util: number | null;
	memUsed: number | null;
	memTotal: number | null;
}

async function unitCudaUuid(): Promise<[string | null, string | null]> {
	const r = await run('systemctl', ['show', 'ninfer-serve', '-p', 'Environment', '--value']);
	const m = r.out.match(/CUDA_VISIBLE_DEVICES=(\S+)/);
	if (!r.ok || !m) return [null, r.ok ? null : 'cuda uuid: unit env unreadable'];
	return [m[1], null];
}

async function listGpus(): Promise<GpuRow[]> {
	const r = await run(
		'nvidia-smi',
		[
			'--query-gpu=name,uuid,utilization.gpu,memory.used,memory.total',
			'--format=csv,noheader,nounits'
		],
		3000
	);
	if (!r.ok) return [];
	return r.out
		.trimEnd()
		.split('\n')
		.filter(Boolean)
		.map((line) => {
			const [name, uuid, util, memUsed, memTotal] = line.split(',').map((s) => s.trim());
			const n = (v: string) => (Number.isNaN(parseInt(v, 10)) ? null : parseInt(v, 10));
			return { name, uuid, util: n(util), memUsed: n(memUsed), memTotal: n(memTotal) };
		});
}

async function mainPid(): Promise<{ value: number | null; error: string | null }> {
	const r = await run('systemctl', ['show', 'ninfer-serve', '-p', 'MainPID', '--value']);
	const v = parseInt(r.out.trim(), 10);
	if (!r.ok || Number.isNaN(v) || v <= 0) return { value: null, error: 'mainpid: unit not running' };
	return { value: v, error: null };
}

async function readPsi(): Promise<EngineState['psi']> {
	const one = async (name: string) => {
		try {
			const raw = await readFile(`/proc/pressure/${name}`, 'utf8');
			const some = raw.match(/^some avg10=([\d.]+)/m);
			return some ? parseFloat(some[1]) : null;
		} catch {
			return null;
		}
	};
	const [cpu, mem, io] = await Promise.all([one('cpu'), one('memory'), one('io')]);
	return { cpu, mem, io };
}

export async function journal(): Promise<string[] | { error: string }> {
	const r = await run(
		'journalctl',
		['--user', '-u', 'q27sweep-driver', '-n', '25', '--no-pager', '--output=short-iso'],
		3000
	);
	if (!r.ok && !r.out) return { error: r.err.trim() || 'journalctl failed' };
	const lines = r.out.trimEnd().split('\n').filter(Boolean);
	return lines.slice(-25);
}
