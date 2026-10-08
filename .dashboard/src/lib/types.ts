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

// ---- Monitoring payloads (server: src/lib/server/monitor.ts) ----

export interface PhaseSpan {
	phase: string;
	startS: number | null;
	durS: number | null;
}

export interface ThroughputPoint {
	t: number;
	tokS: number | null;
	latencyS: number | null;
}

export interface DecisionRow {
	at: string;
	phase: string;
	action: string;
	detail: string;
}

export interface TaskRow {
	id: string;
	status: string;
	title: string;
	dependsOn: string[];
}

export interface GenericInfo {
	problem: string | null;
	family: string | null;
	sourceLanguage: string | null;
	targetLanguage: string | null;
	problemSet: string | null;
	createdAtLocal: string | null;
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
	lastActionAt: string | null;
	lastAction: string | null;
	lastTaskId: string | null;
	recent: boolean;
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
	at: string;
	event: string;
	detail: string;
}

export interface ManifestDelta {
	key: string;
	old: string | null;
	neu: string | null;
}

export interface ProposerDecision {
	predecessor: string | null;
	predecessorVerdict: string | null;
	predecessorHypothesis: string | null;
	relationship: string;
	manifestDeltas: ManifestDelta[];
	configDeltas: ManifestDelta[];
	note: string;
}

export interface RoundDetail {
	dir: string;
	problem: string;
	createdAt: string | null;
	wallS: number | null;
	phases: PhaseSpan[];
	currentPhase: string | null;
	throughput: ThroughputPoint[];
	throughputBasis: 'tokens';
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
	machine: { seq: string[]; edges: MachineEdge[] };
	blackboard: BlackboardState;
	blackboardEvents: BlackboardEvent[];
	proposer: ProposerDecision;
	errors: string[];
}

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
	heartbeats: {
		sweep: { ageS: number | null; stale: boolean; error: string | null };
		watchdog: { ageS: number | null; stale: boolean; error: string | null };
	};
	launcherLogs: LauncherTail[];
	errors: string[];
}

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
	total: number;
	cleared: number;
}

export interface HistoryPoint {
	problem: string;
	verdict: string;
	wallS: number;
}

export interface CampaignState {
	verdicts: VerdictRow[];
	families: FamilyRow[];
	cleared: number;
	clearTotal: number;
	onDiskTotal: number;
	activeDirs: string[];
	pairsFile: { name: string; line: number | null; count: number | null; current: string | null };
	deathCensus: { maxTurns: number; outputCap: number; stalled: number };
	admission503: { perRound: { dir: string; count: number }[]; total: number };
	history: HistoryPoint[];
	errors: string[];
}
