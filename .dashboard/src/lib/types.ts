// Type contract for GET /api/status. Server and client import this.

export type Verdict = string;

export interface LedgerRow {
	roundTag: string;
	problem: string;
	verdict: Verdict;
	tests: string;
	wallS: string;
	closeTime: number | null; // epoch ms
}

export interface InFlightRound {
	dir: string;
	problem: string;
	phase: string;
	lastEventAgeS: number | null;
	wallS: number | null;
}

export interface HeartbeatInfo {
	ageS: number | null;
	stale: boolean;
	error: string | null;
}

export interface UnitState {
	unit: string;
	active: string; // "active" | "inactive" | "failed" | ...
	error: string | null;
}

export interface EngineState {
	modelId: string | null;
	ownedBy: string | null;
	match: boolean;
	vmrssKb: number | null;
	gpuName: string | null;
	psi: { cpu: number | null; mem: number | null; io: number | null };
	gpu: { util: number | null; memUsed: number | null; memTotal: number | null };
	errors: string[];
}

export interface ScoreboardState {
	cleared: number | null;
	total: number;
	waveId: string | null;
	wavePairs: number | null;
	launched: number | null;
	done: number | null;
	clearsPerDay: number | null;
	error: string | null;
}

export interface StatusPayload {
	now: string; // ISO
	scoreboard: ScoreboardState;
	units: UnitState[];
	heartbeats: { sweep: HeartbeatInfo; watchdog: HeartbeatInfo };
	inFlight: InFlightRound[];
	recentCloses: LedgerRow[] | { error: string };
	engine: EngineState;
	journal: string[] | { error: string };
}

export interface ActionResult {
	ok: boolean;
	detail: string;
}
