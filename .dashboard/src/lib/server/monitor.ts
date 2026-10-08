// Server-side monitoring probes: read-only over .artifacts and /proc.
// Types mirror src/lib/types.ts (Campaign* interfaces).

import { readFile, readdir, stat } from 'node:fs/promises';
import { load as loadYaml } from 'js-yaml';
import { run } from './exec.js';

const REPO = '/home/ayin/projs/ARCMiS';
const EXP = `${REPO}/.artifacts/experiments`;

// Rounds whose ledger verdict is SOLVED but whose campaign class is not a
// clear: chtrie (run stopped at round cap before Done) and commons-csv s27
// (rescore banked before the dir was reused by a retry attempt). Cleared-set
// semantics: rounds that completed through Done with the final gate passed.
// Source of record: .artifacts/experiments/SUMMARY.md (2026-10-06 entries).
const LEDGER_SOLVED_NOT_CLEAR = new Set([
	'20261006T055729Zv3s33-chtrie-sweep',
	'20261002T064559Zv3s27-commons-csv-sweep',
	'20261006T153706Zv3s34-rect_pack_h-sweep',
	'20261006T153306Zv3s34-strsim-sweep'
]);


export interface PhaseSpan {
	phase: string;
	startS: number | null; // seconds since round start
	durS: number | null; // null for the current (open) phase
}

export interface ThroughputPoint {
	t: number; // seconds since round start
	tokS: number | null; // output tokens per second of the call
	latencyS: number | null; // model_call → model_response
}

export interface DecisionRow {
	at: string; // display timestamp
	phase: string;
	action: string;
	detail: string;
}

export interface TaskRow {
	id: string;
	status: string;
	title: string; // first 90 chars of description
	dependsOn: string[];
}

export interface GenericInfo {
	problem: string | null;
	family: string | null;
	sourceLanguage: string | null;
	targetLanguage: string | null;
	problemSet: string | null;
	createdAtLocal: string | null; // Asia/Bangkok display (+07:00)
	createdAtUtc: string | null;
	provider: string | null;
	model: string | null;
	gitRevision: string | null;
	hypothesis: string | null;
	pairsLine: number | null;
	pairsCount: number | null;
	wallS: number | null;
	phase: string | null;
}

export interface MetricsExtra {
	inputTokens: number | null;
	outputTokens: number | null;
	cachedTokens: number | null;
	calls: number | null;
	callsPerMin: number | null;
	lastLatencyS: number | null;
	numCtx: number | null;
}

export interface RoleNode {
	role: string;
	tier: 'orchestrator' | 'lead' | 'member' | 'advisory';
	lastActionAt: string | null; // display ts
	lastAction: string | null;
	lastTaskId: string | null;
	recent: boolean; // activity in the last 5 minutes
}

export interface MachineEdge {
	from: string;
	to: string;
	regression: boolean;
	count: number;
}

export interface BlackboardState {
	phase: string | null;
	delegations: number | null;
	delegationWatermark: number | null;
	currentModel: string | null;
	updatedAt: string | null;
	lastTransition: string | null;
}

export interface BlackboardEvent {
	at: string; // display ts
	event: string;
	detail: string;
}

export interface ManifestDelta {
	key: string;
	old: string | null;
	neu: string | null;
}

export interface ProposerDecision {
	predecessor: string | null; // candidate id of previous attempt, same problem
	predecessorVerdict: string | null;
	predecessorHypothesis: string | null;
	relationship: string; // 'retry of', 'first attempt', ...
	manifestDeltas: ManifestDelta[];
	configDeltas: ManifestDelta[];
	note: string; // honest placeholder when nothing comparable
}

export interface RoundDetail {
	dir: string;
	problem: string;
	createdAt: string | null;
	wallS: number | null;
	phases: PhaseSpan[];
	currentPhase: string | null;
	throughput: ThroughputPoint[];
	throughputBasis: 'tokens'; // traces carry usage; kept for honest labeling
	tokSAvg: number | null;
	calls: number;
	lastLatencyS: number | null;
	decisions: DecisionRow[];
	tasks: TaskRow[];
	taskCounts: Record<string, number>;
	fanout: { openTasks: number; inProgress: number; done: number; utilization: number | null };
	generic: GenericInfo;
	metrics: MetricsExtra;
	hierarchy: RoleNode[];
	// Observed phase transitions in launch order; backward steps are the
	// regression arcs actually taken.
	machine: { seq: string[]; edges: MachineEdge[] };
	blackboard: BlackboardState;
	blackboardEvents: BlackboardEvent[];
	proposer: ProposerDecision;
	errors: string[];
}

// Canonical phase order, from lib/blackboard/src/state.rs (Phase enum).
// Index comparison decides forward vs regression transitions.
const PHASE_ORDER = [
	'Preflight',
	'Discovery',
	'Contract',
	'Planning',
	'Pilot',
	'Migration',
	'Integration',
	'Hardening',
	'FinalValidation',
	'Done'
];

// Agent hierarchy layout: orchestrator on top, three leads, members under
// their lead (teams from config.yml mas.teams), fleet-analyst dashed
// advisory. Verified against decisions.jsonl role vocabulary across rounds.
const HIERARCHY: { role: string; tier: 'orchestrator' | 'lead' | 'member' | 'advisory'; lead?: string }[] = [
	{ role: 'orchestrator', tier: 'orchestrator' },
	{ role: 'migration-lead', tier: 'lead' },
	{ role: 'discovery-lead', tier: 'lead' },
	{ role: 'integration-lead', tier: 'lead' },
	{ role: 'translator', tier: 'member', lead: 'migration-lead' },
	{ role: 'validator', tier: 'member', lead: 'migration-lead' },
	{ role: 'tester', tier: 'member', lead: 'migration-lead' },
	{ role: 'analyst', tier: 'member', lead: 'discovery-lead' },
	{ role: 'architect', tier: 'member', lead: 'discovery-lead' },
	{ role: 'planner', tier: 'member', lead: 'integration-lead' },
	{ role: 'failure-analyst', tier: 'member', lead: 'integration-lead' },
	{ role: 'critic', tier: 'member', lead: 'integration-lead' },
	{ role: 'repairer', tier: 'member', lead: 'integration-lead' },
	{ role: 'fleet-analyst', tier: 'advisory' }
];

function problemOf(dir: string): string {
	return dir.replace(/^\d{8}T\d{6}Zv3s\d+-/, '').replace(/-sweep$/, '');
}

function startOf(dir: string): Date | null {
	const m = dir.match(/^(\d{4})(\d{2})(\d{2})T(\d{2})(\d{2})(\d{2})Z/);
	if (!m) return null;
	return new Date(
		Date.UTC(+m[1], +m[2] - 1, +m[3], +m[4], +m[5], +m[6])
	);
}

function tsToMs(at: unknown): number | null {
	// traces/turns.jsonl writes plain epoch seconds as JSON numbers;
	// events.jsonl and ledgers write ISO strings or 'unix:NNN' strings.
	if (typeof at === 'number') return Number.isFinite(at) ? at * 1000 : null;
	if (typeof at !== 'string') return null;
	if (at.startsWith('unix:')) {
		const n = parseInt(at.slice(5), 10);
		return Number.isNaN(n) ? null : n * 1000;
	}
	const iso = at.match(/^(\d{4}-\d{2}-\d{2}T[\d:.]+)/);
	if (iso) {
		const ms = Date.parse(iso[1] + 'Z');
		return Number.isNaN(ms) ? null : ms;
	}
	const n = parseInt(at, 10);
	// Raw epoch seconds: traces/turns.jsonl writes plain unix timestamps.
	return Number.isNaN(n) ? null : n * 1000;
}

function shortTs(at: string): string {
	// '2026-10-06T15:33:06.273Z.273Z' | 'unix:1791300800' → '15:33:06Z' | HH:MM:SSZ
	if (at.startsWith('unix:')) {
		const ms = tsToMs(at);
		if (ms !== null) return new Date(ms).toISOString().slice(11, 19) + 'Z';
		return at;
	}
	const iso = at.match(/^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2}:\d{2})/);
	return iso ? `${iso[2]}Z` : at;
}

export async function activeRoundDirs(): Promise<string[]> {
	try {
		const dirs = (await readdir(EXP, { withFileTypes: true }))
			.filter((d) => d.isDirectory() && /^\d{8}T\d{6}Zv3s\d+-.+-sweep$/.test(d.name))
			.map((d) => d.name)
			.sort();
		const active: string[] = [];
		for (const dir of dirs) {
			try {
				await stat(`${EXP}/${dir}/result/aggregate.yml`);
				continue; // closed
			} catch {
				// no aggregate → live candidate
			}
			try {
				// Spec: manifest.json present + no result aggregate = active. A
				// recency gate on events.jsonl keeps dead-but-unpurged orphan
				// rounds (no aggregate, silent for days) out of the live panel;
				// they remain visible to the purge action, which lists by the
				// no-aggregate rule alone.
				await stat(`${EXP}/${dir}/manifest.json`);
				const ev = await stat(`${EXP}/${dir}/events.jsonl`);
				if (Date.now() - ev.mtimeMs < 15 * 60 * 1000) active.push(dir);
			} catch {
				// no manifest or no events file
			}
		}
		return active;
	} catch {
		return [];
	}
}

export async function roundDetail(dir: string): Promise<RoundDetail> {
	const full = `${EXP}/${dir}`;
	const errors: string[] = [];
	const d: RoundDetail = {
		dir,
		problem: problemOf(dir),
		createdAt: null,
		wallS: null,
		phases: [],
		currentPhase: null,
		throughput: [],
		throughputBasis: 'tokens',
		tokSAvg: null,
		calls: 0,
		lastLatencyS: null,
		decisions: [],
		tasks: [],
		taskCounts: {},
		fanout: { openTasks: 0, inProgress: 0, done: 0, utilization: null },
		generic: {
			problem: problemOf(dir),
			family: null,
			sourceLanguage: null,
			targetLanguage: null,
			problemSet: null,
			createdAtLocal: null,
			createdAtUtc: null,
			provider: null,
			model: null,
			gitRevision: null,
			hypothesis: null,
			pairsLine: null,
			pairsCount: null,
			wallS: null,
			phase: null
		},
		metrics: {
			inputTokens: null,
			outputTokens: null,
			cachedTokens: null,
			calls: null,
			callsPerMin: null,
			lastLatencyS: null,
			numCtx: null
		},
		hierarchy: [],
		machine: { seq: [], edges: [] },
		blackboard: {
			phase: null,
			delegations: null,
			delegationWatermark: null,
			currentModel: null,
			updatedAt: null,
			lastTransition: null
		},
		blackboardEvents: [],
		proposer: {
			predecessor: null,
			predecessorVerdict: null,
			predecessorHypothesis: null,
			relationship: 'first attempt',
			manifestDeltas: [],
			configDeltas: [],
			note: 'no earlier attempt for this problem on disk'
		},
		errors
	};

	// manifest: created_at → wall; problem identity + engine + hypothesis.
	const start = startOf(dir);
	try {
		const man = JSON.parse(await readFile(`${full}/manifest.json`, 'utf8')) as Record<string, unknown>;
		if (typeof man.created_at === 'string') {
			d.createdAt = man.created_at;
			// Local display: Asia/Bangkok (+07:00, no DST). Format with the
			// offset baked in so the primary text is unambiguous; UTC stays in
			// a secondary line.
			const ms = Date.parse(man.created_at);
			if (!Number.isNaN(ms)) {
				d.generic.createdAtLocal = new Date(ms + 7 * 3600 * 1000)
					.toISOString()
					.replace('T', ' ')
					.slice(0, 19) + ' +07:00';
				d.generic.createdAtUtc = new Date(ms).toISOString().replace('T', ' ').slice(0, 19) + ' UTC';
			}
		}
		if (typeof man.problem_set === 'string') {
			d.generic.problemSet = man.problem_set;
			const fam = man.problem_set.match(/tool_projects\/([\w-]+)/);
			if (fam) d.generic.family = fam[1];
		}
		if (typeof man.source_language === 'string') d.generic.sourceLanguage = man.source_language;
		if (typeof man.target_language === 'string') d.generic.targetLanguage = man.target_language;
		if (typeof man.model === 'string') d.generic.model = man.model;
		if (typeof man.git_revision === 'string' && man.git_revision) d.generic.gitRevision = man.git_revision;
		if (typeof man.hypothesis === 'string' && man.hypothesis) d.generic.hypothesis = man.hypothesis;
	} catch (e) {
		errors.push(`manifest: ${(e as Error).message}`);
	}
	// config.yml: provider + num_ctx. Shallow scan, no YAML lib needed here.
	try {
		const cfg = await readFile(`${full}/config.yml`, 'utf8');
		d.generic.provider = cfg.match(/^  provider: (\S+)/m)?.[1] ?? null;
		const numCtx = cfg.match(/^  num_ctx: (\d+)/m);
		d.metrics.numCtx = numCtx ? parseInt(numCtx[1], 10) : null;
	} catch {
		// config.yml optional for the panel; generic info stays partial
	}
	const startMs = start?.getTime() ?? (d.createdAt ? Date.parse(d.createdAt) : null);
	if (startMs !== null && !Number.isNaN(startMs)) {
		d.wallS = Math.round((Date.now() - startMs) / 1000);
		d.generic.wallS = d.wallS;
	}

	// events.jsonl: phase timeline + delegations + transition sequence.
	try {
		const raw = await readFile(`${full}/events.jsonl`, 'utf8');
		const events: { atMs: number | null; phase: string | null }[] = [];
		const bbEvents: BlackboardEvent[] = [];
		for (const line of raw.trimEnd().split('\n')) {
			if (!line.trim()) continue;
			try {
				const ev = JSON.parse(line);
				if (ev?.event === 'phase' && typeof ev?.fields?.to === 'string') {
					events.push({ atMs: tsToMs(ev.at), phase: ev.fields.to });
					if (bbEvents.length < 10) {
						bbEvents.push({
							at: typeof ev.at === 'string' ? shortTs(ev.at) : '—',
							event: 'phase',
							detail: `→ ${ev.fields.to}`
						});
					}
				} else if (ev?.event === 'delegation' && bbEvents.length < 10) {
					const task = typeof ev.fields?.task === 'string' ? ev.fields.task : '';
					const role = typeof ev.fields?.role === 'string' ? ev.fields.role : '';
					bbEvents.push({
						at: typeof ev.at === 'string' ? shortTs(ev.at) : '—',
						event: 'delegation',
						detail: `${role}${task ? ` · ${task.slice(0, 40)}` : ''}`
					});
				}
			} catch {
				// tolerate torn tail line
			}
		}
		if (startMs !== null && events.length > 0) {
			d.currentPhase = events[events.length - 1].phase;
			d.generic.phase = events[events.length - 1].phase;
			for (let i = 0; i < events.length; i++) {
				const startS = events[i].atMs !== null ? Math.round((events[i].atMs! - startMs) / 1000) : null;
				const nextMs = i + 1 < events.length ? events[i + 1].atMs : Date.now();
				const durS =
					events[i].atMs !== null && nextMs !== null
						? Math.round((nextMs - events[i].atMs!) / 1000)
						: null;
				d.phases.push({ phase: events[i].phase ?? '—', startS, durS });
			}
			// State-machine edges: collapse consecutive (from, to) transitions.
			// A backward step (target earlier in the canonical order) is a
			// regression arc actually taken. Self-loops (re-entry into the same
			// phase, e.g. a replan) count as regressions too: the harness
			// records them as re-transitions.
			const ORDER = PHASE_ORDER;
			d.machine.seq = events.map((e) => e.phase ?? '—');
			const edgeMap = new Map<string, MachineEdge>();
			for (let i = 0; i + 1 < events.length; i++) {
				const from = events[i].phase ?? '—';
				const to = events[i + 1].phase ?? '—';
				const fi = ORDER.indexOf(from);
				const ti = ORDER.indexOf(to);
				const regression = fi >= 0 && ti >= 0 && ti <= fi;
				const key = `${from}→${to}`;
				const cur = edgeMap.get(key) ?? { from, to, regression, count: 0 };
				cur.count += 1;
				edgeMap.set(key, cur);
			}
			d.machine.edges = [...edgeMap.values()];
		}
		d.blackboardEvents = bbEvents;
	} catch (e) {
		errors.push(`events: ${(e as Error).message}`);
	}

	// traces/turns.jsonl: model_call/model_response → latency + tok/s.
	// Schema verified: model_response fields.usage {input_tokens, output_tokens, total_tokens}.
	try {
		const raw = await readFile(`${full}/traces/turns.jsonl`, 'utf8');
		const callAt = new Map<string, number>();
		let tokSum = 0;
		let tokN = 0;
		let inTok = 0;
		let outTok = 0;
		let cachedTok = 0;
		let lastRespMs: number | null = null;
		for (const line of raw.trimEnd().split('\n')) {
			if (!line.trim()) continue;
			let ev: { at: unknown; event: string; fields?: Record<string, unknown> };
			try {
				ev = JSON.parse(line);
			} catch {
				continue;
			}
			const atMs = tsToMs(ev.at);
			const agent = typeof ev.fields?.agent === 'string' ? ev.fields.agent : '—';
			if (ev.event === 'model_call') {
				d.calls += 1;
				if (atMs !== null) callAt.set(agent, atMs);
			} else if (ev.event === 'model_response') {
				let latencyS: number | null = null;
				const started = callAt.get(agent);
				if (atMs !== null && started !== undefined) {
					latencyS = Math.round(((atMs - started) / 1000) * 10) / 10;
					d.lastLatencyS = latencyS;
				}
				const usage = ev.fields?.usage as Record<string, unknown> | undefined;
				if (usage) {
					if (typeof usage.input_tokens === 'number') inTok += usage.input_tokens;
					if (typeof usage.output_tokens === 'number') outTok += usage.output_tokens;
					if (typeof usage.cached_input_tokens === 'number') cachedTok += usage.cached_input_tokens;
				}
				const outTokOne = usage && typeof usage.output_tokens === 'number' ? usage.output_tokens : null;
				let tokS: number | null = null;
				if (outTokOne !== null && latencyS !== null && latencyS > 0) {
					tokS = Math.round((outTokOne / latencyS) * 10) / 10;
					tokSum += tokS;
					tokN += 1;
				}
				if (atMs !== null && startMs !== null) {
					d.throughput.push({
						t: Math.round((atMs - startMs) / 1000),
						tokS,
						latencyS
					});
					lastRespMs = atMs;
				}
			}
		}
		// Rolling window: keep the last 120 response points for the chart.
		if (d.throughput.length > 120) d.throughput = d.throughput.slice(-120);
		d.tokSAvg = tokN > 0 ? Math.round((tokSum / tokN) * 10) / 10 : null;
		d.metrics.inputTokens = inTok > 0 ? inTok : null;
		d.metrics.outputTokens = outTok > 0 ? outTok : null;
		d.metrics.cachedTokens = cachedTok > 0 ? cachedTok : null;
		d.metrics.calls = d.calls > 0 ? d.calls : null;
		d.metrics.lastLatencyS = d.lastLatencyS;
		// Calls per minute over the observed trace span.
		if (d.calls > 0 && lastRespMs !== null && startMs !== null) {
			const spanMin = Math.max((lastRespMs - startMs) / 60000, 1 / 60);
			d.metrics.callsPerMin = Math.round((d.calls / spanMin) * 100) / 100;
		}
	} catch (e) {
		errors.push(`traces: ${(e as Error).message}`);
	}

	// run/ledgers/decisions.jsonl: orchestrator decisions, newest first.
	// Also builds the hierarchy overlay: last action per role + 5 min ring.
	try {
		const raw = await readFile(`${full}/run/ledgers/decisions.jsonl`, 'utf8');
		const rows: DecisionRow[] = [];
		const lastByRole = new Map<string, { atMs: number | null; at: string; action: string; taskId: string | null }>();
		for (const line of raw.trimEnd().split('\n')) {
			if (!line.trim()) continue;
			try {
				const r = JSON.parse(line);
				const detail = r.detail ?? {};
				const what =
					typeof detail.task === 'string'
						? detail.task
						: typeof detail.lead === 'string'
							? `lead ${detail.lead}`
							: typeof detail.role === 'string'
								? `role ${detail.role}`
								: '';
				rows.push({
					at: typeof r.at === 'string' ? shortTs(r.at) : '—',
					phase: typeof r.phase === 'string' ? r.phase : '—',
					action: typeof r.action === 'string' ? r.action : '—',
					detail: what.slice(0, 140)
				});
				// Role attribution: detail.lead for delegate-lead entries,
				// detail.role for member dispatches. Ambiguous rows (neither
				// field) do not contribute to the overlay.
				const role =
					typeof detail.lead === 'string'
						? detail.lead
						: typeof detail.role === 'string'
							? detail.role
							: null;
				if (role && typeof r.action === 'string') {
					const atMs = tsToMs(r.at);
					lastByRole.set(role, {
						atMs,
						at: typeof r.at === 'string' ? shortTs(r.at) : '—',
						action: r.action,
						taskId: typeof detail.id === 'string' ? detail.id : null
					});
				}
			} catch {
				// tolerate torn tail
			}
		}
		d.decisions = rows.reverse().slice(0, 12);
		const now = Date.now();
		d.hierarchy = HIERARCHY.map((h) => {
			const last = lastByRole.get(h.role);
			return {
				role: h.role,
				tier: h.tier,
				lastActionAt: last?.at ?? null,
				lastAction: last?.action ?? null,
				lastTaskId: last?.taskId ?? null,
				recent: last?.atMs != null && now - last.atMs < 5 * 60 * 1000
			};
		});
	} catch (e) {
		errors.push(`decisions: ${(e as Error).message}`);
	}

	// run/tasks.json + run/state.json: blackboard snapshot.
	try {
		const tasks: unknown = JSON.parse(await readFile(`${full}/run/tasks.json`, 'utf8'));
		if (Array.isArray(tasks)) {
			const counts: Record<string, number> = {};
			for (const t of tasks) {
				if (!t || typeof t !== 'object' || !('id' in t)) continue;
				const status = typeof t.status === 'string' ? t.status : 'unknown';
				counts[status] = (counts[status] ?? 0) + 1;
				d.tasks.push({
					id: typeof t.id === 'string' ? t.id : '—',
					status,
					title:
						typeof t.description === 'string'
							? t.description.replace(/\s+/g, ' ').slice(0, 90)
							: '—',
					dependsOn: Array.isArray(t.depends_on)
						? (t.depends_on as unknown[]).filter((x): x is string => typeof x === 'string')
						: []
				});
			}
			d.taskCounts = counts;
			const open = tasks.length;
			const inProg = counts['in_progress'] ?? 0;
			const done = counts['done'] ?? 0;
			// Utilization: share of the task graph that is actively dispatched.
			// Fanout cap is not recorded on disk; report open/in-progress honestly.
			d.fanout = {
				openTasks: open,
				inProgress: inProg,
				done,
				utilization: open > 0 ? Math.round((inProg / open) * 100) : null
			};
			d.tasks = d.tasks.slice(-6).reverse(); // newest writes land last
		}
	} catch (e) {
		errors.push(`tasks: ${(e as Error).message}`);
	}
	try {
		const st = JSON.parse(await readFile(`${full}/run/state.json`, 'utf8')) as Record<string, unknown>;
		d.blackboard = {
			phase: typeof st.phase === 'string' ? st.phase : null,
			delegations: typeof st.phase_delegations === 'number' ? st.phase_delegations : null,
			delegationWatermark:
				typeof st.phase_delegation_watermark === 'number' ? st.phase_delegation_watermark : null,
			currentModel: typeof st.current_model === 'string' ? st.current_model : null,
			updatedAt: typeof st.updated_at === 'string' ? shortTs(st.updated_at) : null,
			lastTransition: typeof st.last_transition === 'string' ? st.last_transition : null
		};
	} catch (e) {
		errors.push(`state: ${(e as Error).message}`);
	}

	// PAIRS position for this problem (same source as the campaign panel).
	try {
		const raw = await readFile(`${EXP}/PAIRS-ninfer-w2.txt`, 'utf8');
		const lines = raw.split('\n').filter((l) => l.trim() && !l.trim().startsWith('#'));
		d.generic.pairsCount = lines.length;
		const idx = lines.findIndex((l) => l.split('|').pop()?.trim() === d.problem);
		if (idx >= 0) d.generic.pairsLine = idx + 1;
	} catch {
		// pairs file optional for the panel
	}

	// Proposer decisions: diff vs the previous attempt of the same problem.
	d.proposer = await proposerDiff(dir, errors);

	return d;
}

function shallowDeltaMap(
	a: Record<string, unknown> | null,
	b: Record<string, unknown> | null,
	skip: (k: string) => boolean
): ManifestDelta[] {
	if (!a || !b) return [];
	// Scalars render without JSON quoting; composites keep it so structure is
	// visible but stay truncated.
	const fmt = (v: unknown): string | null =>
		v === undefined
			? null
			: typeof v === 'string'
				? v.slice(0, 80)
				: JSON.stringify(v)?.slice(0, 80) ?? null;
	const out: ManifestDelta[] = [];
	const keys = [...new Set([...Object.keys(a), ...Object.keys(b)])].sort();
	for (const k of keys) {
		if (skip(k)) continue;
		const va = fmt(a[k]);
		const vb = fmt(b[k]);
		if (va !== vb) out.push({ key: k, old: va, neu: vb });
	}
	return out;
}

// Previous attempt of the same problem: latest dir with the same problem
// suffix and an earlier timestamp. Cross-checks the ROUNDS.yaml hypothesis
// of the predecessor for the verdict line. Cheap: two manifest reads plus a
// bounded yaml text scan.
async function proposerDiff(
	dir: string,
	errors: string[]
): Promise<ProposerDecision> {
	const out: ProposerDecision = {
		predecessor: null,
		predecessorVerdict: null,
		predecessorHypothesis: null,
		relationship: 'first attempt',
		manifestDeltas: [],
		configDeltas: [],
		note: 'no earlier attempt for this problem on disk'
	};
	const problem = problemOf(dir);
	const prefix = dir.match(/^(\d{8}T\d{6}Z)v3s(\d+)-/);
	if (!prefix) return out;
	try {
		const dirs = (await readdir(EXP, { withFileTypes: true }))
			.filter((e) => e.isDirectory() && e.name.endsWith(`-${problem}-sweep`) && e.name < dir)
			.map((e) => e.name)
			.sort();
		const pred = dirs[dirs.length - 1];
		if (!pred) return out;
		out.predecessor = pred;
		out.relationship = 'relaunch of an earlier attempt';
		let curMan: Record<string, unknown> | null = null;
		let predMan: Record<string, unknown> | null = null;
		try {
			curMan = JSON.parse(await readFile(`${EXP}/${dir}/manifest.json`, 'utf8'));
			predMan = JSON.parse(await readFile(`${EXP}/${pred}/manifest.json`, 'utf8'));
		} catch (e) {
			errors.push(`proposer manifest: ${(e as Error).message}`);
		}
		if (predMan && typeof predMan.hypothesis === 'string') out.predecessorHypothesis = predMan.hypothesis;
		// Skip volatile keys: per-round paths, timestamps, ids.
		out.manifestDeltas = shallowDeltaMap(predMan, curMan, (k) =>
			['config_path', 'created_at', 'harness_id', 'parents'].includes(k)
		);
		// config.yml: two-level shallow diff (section.key), skip dir noise.
		const cfgShallow = async (d: string): Promise<Record<string, unknown> | null> => {
			try {
				const lines = (await readFile(`${EXP}/${d}/config.yml`, 'utf8')).split('\n');
				const obj: Record<string, unknown> = {};
				let section = '';
				for (const l of lines) {
					const top = l.match(/^(\w+):\s*(.*)$/);
					if (top) {
						section = top[1];
						if (top[2]) obj[top[1]] = top[2];
						continue;
					}
					const sub = l.match(/^  (\w+):\s*(.*)$/);
					if (sub && section && sub[1] !== 'dir') obj[`${section}.${sub[1]}`] = sub[2];
				}
				return obj;
			} catch {
				return null;
			}
		};
		const [curCfg, predCfg] = await Promise.all([cfgShallow(dir), cfgShallow(pred)]);
		out.configDeltas = shallowDeltaMap(predCfg, curCfg, (k) => k.endsWith('.dir'));
		// Verdict + hypothesis of the predecessor from ROUNDS.yaml (text scan;
		// the yaml lib chokes on the file's duplicate-round keys).
		try {
			const txt = await readFile(`${EXP}/ROUNDS.yaml`, 'utf8');
			const blocks = txt.split('candidate_id:');
			for (const b of blocks) {
				// The block after the split starts with the candidate id of its
				// own row; parents lists live in earlier blocks and must not match.
				if (!b.trimStart().startsWith(pred)) continue;
				out.predecessorVerdict = b.match(/verdict: "?([\w -]+)"?/)?.[1]?.trim() ?? null;
				const hyp = b.match(/hypothesis: "(.*)"/)?.[1];
				// Prefer the predecessor manifest hypothesis (structured); the
				// ROUNDS text row is the fallback for early rounds that recorded
				// 'unknown' placeholders.
				if (hyp && hyp !== 'unknown' && !out.predecessorHypothesis) out.predecessorHypothesis = hyp;
				break;
			}
		} catch {
			// ROUNDS.yaml optional for the panel
		}
		out.note =
			out.manifestDeltas.length + out.configDeltas.length > 0
				? ''
				: 'no config delta between attempts';
	} catch (e) {
		errors.push(`proposer: ${(e as Error).message}`);
	}
	return out;
}

// ---- Fleet health (manual refresh) ----

export interface GpuInfo {
	index: number | null;
	name: string;
	uuid: string | null;
	util: number | null;
	memUsed: number | null;
	memTotal: number | null;
}

export interface LauncherTail {
	file: string;
	mtimeMs: number | null;
	tail: string[];
}

export interface FleetState {
	gpus: GpuInfo[];
	units: { unit: string; active: string; error: string | null }[];
	heartbeats: { sweep: { ageS: number | null; stale: boolean; error: string | null }; watchdog: { ageS: number | null; stale: boolean; error: string | null } };
	launcherLogs: LauncherTail[];
	errors: string[];
}

export async function fleet(): Promise<FleetState> {
	const errors: string[] = [];
	const state: FleetState = {
		gpus: [],
		units: [],
		heartbeats: {
			sweep: { ageS: null, stale: false, error: 'not read' },
			watchdog: { ageS: null, stale: false, error: 'not read' }
		},
		launcherLogs: [],
		errors
	};

	// GPU rows — every GPU, not just the selected one.
	const r = await run(
		'nvidia-smi',
		['--query-gpu=index,name,uuid,utilization.gpu,memory.used,memory.total', '--format=csv,noheader,nounits'],
		3000
	);
	if (r.ok) {
		for (const line of r.out.trimEnd().split('\n').filter(Boolean)) {
			const [index, name, uuid, util, memUsed, memTotal] = line.split(',').map((s) => s.trim());
			const n = (v: string) => (Number.isNaN(parseInt(v, 10)) ? null : parseInt(v, 10));
			state.gpus.push({
				index: n(index),
				name,
				uuid,
				util: n(util),
				memUsed: n(memUsed),
				memTotal: n(memTotal)
			});
		}
	} else {
		errors.push(`nvidia-smi: ${r.err.trim() || 'failed'}`);
	}

	// Unit states.
	const unitNames = ['q27sweep-driver', 'q27sweep-watchdog', 'ninfer-serve'];
	state.units = await Promise.all(
		unitNames.map(async (unit) => {
			const u = await run('systemctl', ['--user', 'is-active', unit]);
			return { unit, active: u.out.trim().split('\n')[0] || 'unknown', error: u.ok ? null : u.err.trim() || null };
		})
	);

	// Heartbeat ages. Stale threshold: 600s per the ops checklist.
	const hbOne = async (name: string) => {
		try {
			const s = await stat(`${EXP}/.sweep-heartbeat`.replace('.sweep', `.${name}`));
			const ageS = Math.round((Date.now() - s.mtimeMs) / 1000);
			return { ageS, stale: ageS > 600, error: null };
		} catch {
			return { ageS: null, stale: false, error: 'missing' };
		}
	};
	const [sweep, watchdog] = await Promise.all([hbOne('sweep'), hbOne('watchdog')]);
	state.heartbeats = { sweep, watchdog };

	// Launcher logs: the 8 newest /tmp/sweep-*.launcher.log with tails.
	try {
		const logs = (await readdir('/tmp'))
			.filter((f) => /^sweep-.*\.launcher\.log$/.test(f))
			.map(async (f) => {
				const p = `/tmp/${f}`;
				try {
					const s = await stat(p);
					return { file: f, mtimeMs: s.mtimeMs };
				} catch {
					return { file: f, mtimeMs: null };
				}
			});
		const sorted = (await Promise.all(logs))
			.sort((a, b) => (b.mtimeMs ?? 0) - (a.mtimeMs ?? 0))
			.slice(0, 8);
		state.launcherLogs = await Promise.all(
			sorted.map(async ({ file, mtimeMs }) => {
				try {
					const raw = await readFile(`/tmp/${file}`, 'utf8');
					return { file, mtimeMs, tail: raw.trimEnd().split('\n').filter(Boolean).slice(-4) };
				} catch {
					return { file, mtimeMs, tail: [] };
				}
			})
		);
	} catch (e) {
		errors.push(`launcher logs: ${(e as Error).message}`);
	}

	return state;
}

// ---- Campaign state (cheap, pollable) ----

export interface VerdictRow {
	candidateId: string;
	roundTag: string;
	problem: string;
	family: string | null;
	verdict: string;
	tests: string;
	wallS: string;
}

export interface FamilyRow {
	family: string;
	total: number; // on-disk problem count
	cleared: number;
}

export interface HistoryPoint {
	problem: string;
	verdict: string;
	wallS: number; // full-window wall from dir timestamps
}

export interface CampaignState {
	verdicts: VerdictRow[];
	families: FamilyRow[];
	cleared: number;
	clearTotal: number; // brief number, 114
	onDiskTotal: number; // counted dirs
	activeDirs: string[]; // manifest present, no result aggregate, newest last
	pairsFile: { name: string; line: number | null; count: number | null; current: string | null };
	deathCensus: { maxTurns: number; outputCap: number; stalled: number };
	admission503: { perRound: { dir: string; count: number }[]; total: number };
	history: HistoryPoint[];
	errors: string[];
}

// On-disk family counts for the denominator. The campaign brief says 114;
// disk holds 237 problem dirs (paper reconciles: brief-time bookkeeping).
const FAMILY_DIRS: { family: string; total: number }[] = [
	{ family: 'crust', total: 200 },
	{ family: 'skel', total: 16 },
	{ family: 'oxidizer', total: 13 },
	{ family: 'alphatrans', total: 8 }
];

async function familyOf(cid: string): Promise<string | null> {
	try {
		const man: unknown = JSON.parse(await readFile(`${EXP}/${cid}/manifest.json`, 'utf8'));
		if (man && typeof man === 'object' && 'problem_set' in man && typeof man.problem_set === 'string') {
			const m = man.problem_set.match(/tool_projects\/([\w-]+)/);
			return m ? m[1] : null;
		}
	} catch {
		// manifest missing
	}
	return null;
}

export async function campaign(): Promise<CampaignState> {
	const errors: string[] = [];
	const state: CampaignState = {
		verdicts: [],
		families: [],
		cleared: 0,
		clearTotal: 114,
		onDiskTotal: 237,
		activeDirs: [],
		pairsFile: { name: 'PAIRS-ninfer-w2.txt', line: null, count: null, current: null },
		deathCensus: { maxTurns: 0, outputCap: 0, stalled: 0 },
		admission503: { perRound: [], total: 0 },
		history: [],
		errors
	};

	// Ledger: verdict table (latest per candidate), cleared set, death census.
	let rounds: Record<string, unknown>[] = [];
	try {
		const raw = await readFile(`${EXP}/ROUNDS.yaml`, 'utf8');
		const doc: unknown = loadYaml(raw);
		rounds = doc && typeof doc === 'object' && 'rounds' in doc && Array.isArray(doc.rounds)
			? (doc.rounds as Record<string, unknown>[])
			: [];
	} catch (e) {
		errors.push(`ledger: ${(e as Error).message}`);
	}

	const latest = new Map<string, { verdict: string; roundTag: string }>();
	for (const r of rounds) {
		if (!r || typeof r !== 'object') continue;
		const cid = r.candidate_id;
		const verdict = r.verdict;
		if (typeof cid === 'string' && typeof verdict === 'string') {
			latest.set(cid, { verdict, roundTag: typeof r.round === 'string' ? r.round : '—' });
		}
	}

	// Death census: sum over ALL ledger rows with numeric metrics.
	for (const r of rounds) {
		if (!r || typeof r !== 'object') continue;
		const m = r.metrics;
		if (!m || typeof m !== 'object') continue;
		const metrics = m as Record<string, unknown>;
		const num = (v: unknown) => (typeof v === 'number' ? v : 0);
		state.deathCensus.maxTurns += num(metrics.max_turns_deaths);
		state.deathCensus.outputCap += num(metrics.output_cap_deaths);
		state.deathCensus.stalled += num(metrics.stalled_rounds);
	}

	// Verdict table: latest per candidate, sweep rounds, newest first (16 rows).
	const sweepIds = [...latest.entries()].filter(([id]) => /^\d{8}T\d{6}Zv3s\d+/.test(id));
	const clearedIds: string[] = [];
	const verdictRows: VerdictRow[] = [];
	for (const [cid, { verdict, roundTag }] of sweepIds) {
		const problem = cid.replace(/^\d{8}T\d{6}Zv3s?\d*-/, '').replace(/-sweep$/, '');
		if (verdict === 'SOLVED' || verdict === 'CLEARED' || verdict === 'VERIFIED-WORKSPACE') {
			if (!LEDGER_SOLVED_NOT_CLEAR.has(cid)) clearedIds.push(cid);
		}
		// per-round metrics for the table rows
		const row = rounds.find((r) => r && typeof r === 'object' && r.candidate_id === cid);
		const metrics =
			row && typeof row === 'object' && row.metrics && typeof row.metrics === 'object'
				? (row.metrics as Record<string, unknown>)
				: {};
		const tp = metrics.tests_passed;
		const tf = metrics.tests_failed;
		const tests =
			typeof tp === 'number' && typeof tf === 'number' ? `${tp}/${tf}` : '—';
		const wall = metrics.wall_seconds;
		verdictRows.push({
			candidateId: cid,
			roundTag,
			problem,
			family: null, // filled below
			verdict,
			tests,
			wallS: typeof wall === 'number' ? String(wall) : '—'
		});
	}
	// Newest first by launch timestamp embedded in the id; keep 16.
	verdictRows.sort((a, b) => (a.candidateId < b.candidateId ? 1 : -1));
	const table = verdictRows.slice(0, 16);
	for (const row of table) {
		row.family = await familyOf(row.candidateId);
	}
	state.verdicts = table;
	state.cleared = clearedIds.length;

	// Per-family cleared from manifest problem_set of cleared candidate ids.
	const famCleared = new Map<string, number>();
	for (const cid of clearedIds) {
		const fam = await familyOf(cid);
		if (fam) famCleared.set(fam, (famCleared.get(fam) ?? 0) + 1);
	}
	state.families = FAMILY_DIRS.map(({ family, total }) => ({
		family,
		total,
		cleared: famCleared.get(family) ?? 0
	}));

	// PAIRS position: which pair line the live round corresponds to.
	state.activeDirs = await activeRoundDirs();
	try {
		const raw = await readFile(`${EXP}/PAIRS-ninfer-w2.txt`, 'utf8');
		const lines = raw.split('\n').filter((l) => l.trim() && !l.trim().startsWith('#'));
		state.pairsFile.count = lines.length;
		for (const dir of state.activeDirs) {
			const problem = problemOf(dir);
			const idx = lines.findIndex((l) => l.split('|').pop()?.trim() === problem);
			if (idx >= 0) {
				state.pairsFile.line = idx + 1;
				state.pairsFile.current = problem;
				break;
			}
		}
	} catch (e) {
		errors.push(`pairs: ${(e as Error).message}`);
	}

	// Admission-503 per round from run/ledgers/failures.jsonl (greppable, cheap:
	// last 10 closed rounds only).
	const closedIds = sweepIds
		.filter(([, v]) => v.verdict !== 'IN FLIGHT')
		.map(([id]) => id)
		.slice(-10);
	for (const cid of closedIds) {
		try {
			const raw = await readFile(`${EXP}/${cid}/run/ledgers/failures.jsonl`, 'utf8');
			const count = raw.split('\n').filter((l) => l.includes('503')).length;
			state.admission503.perRound.push({ dir: cid, count });
			state.admission503.total += count;
		} catch {
			// no failures ledger
		}
	}

	// Wall-time history: full-window wall from dir name start to last events.jsonl ts.
	try {
		const dirs = (await readdir(EXP, { withFileTypes: true }))
			.filter((d) => d.isDirectory() && /^\d{8}T\d{6}Zv3s\d+-.+-sweep$/.test(d.name))
			.map((d) => d.name);
		for (const dir of dirs) {
			const verdict = latest.get(dir)?.verdict;
			if (!verdict || verdict === 'IN FLIGHT') continue;
			const start = startOf(dir);
			if (!start) continue;
			try {
				const raw = await readFile(`${EXP}/${dir}/events.jsonl`, 'utf8');
				const lines = raw.trimEnd().split('\n');
				let endMs: number | null = null;
				for (let i = lines.length - 1; i >= 0; i--) {
					try {
						const ev = JSON.parse(lines[i]);
						endMs = tsToMs(ev.at);
						if (endMs !== null) break;
					} catch {
						continue;
					}
				}
				if (endMs === null) continue;
				state.history.push({
					problem: problemOf(dir),
					verdict,
					wallS: Math.round((endMs - start.getTime()) / 1000)
				});
			} catch {
				continue;
			}
		}
		state.history.sort((a, b) => a.wallS - b.wallS);
	} catch (e) {
		errors.push(`history: ${(e as Error).message}`);
	}

	return state;
}
