<script lang="ts">
	import type { CampaignState, RoundDetail } from '$lib/types.js';
	import { POLL_MS } from '$lib/config.js';

	let { campaign }: { campaign: CampaignState | null } = $props();

	// Watches the same round as Live rounds: the newest active dir.
	let round = $state<RoundDetail | null>(null);

	const watchDir = $derived(
		campaign?.activeDirs.length ? campaign.activeDirs[campaign.activeDirs.length - 1] : null
	);

	async function loadRound(dir: string) {
		try {
			const res = await fetch(`/api/round/${dir}`, { cache: 'no-store' });
			if (res.ok) round = await res.json();
		} catch {
			// keep previous payload
		}
	}

	$effect(() => {
		const dir = watchDir;
		if (!dir) {
			round = null;
			return;
		}
		loadRound(dir);
		const id = setInterval(() => loadRound(dir), POLL_MS);
		return () => clearInterval(id);
	});

	function wallLabel(s: number | null): string {
		if (s === null) return '—';
		const m = Math.floor(s / 60);
		const h = Math.floor(m / 60);
		return h > 0 ? `${h}h ${m % 60}m` : `${m}m`;
	}

	function tokLabel(n: number | null): string {
		if (n === null) return '—';
		if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(0)}k`;
		return String(n);
	}

	// Hypothesis clamp: full text in the title attribute. Visible text uses
	// ASCII dots; only title attributes keep the typographic ellipsis.
	function clamp(s: string | null, n: number): string {
		if (!s) return '—';
		return s.length > n ? s.slice(0, n) + '...' : s;
	}

	const PHASES = [
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

	// Per-phase durations keyed by phase name (last span wins for repeats).
	const phaseDur = $derived.by(() => {
		const m = new Map<string, number | null>();
		for (const p of round?.phases ?? []) m.set(p.phase, p.durS);
		return m;
	});

	const currentIdx = $derived(
		round?.currentPhase ? PHASES.indexOf(round.currentPhase) : -1
	);

	// State machine grid: two rows of five, left to right, then back.
	const MACHINE_POS: { x: number; y: number }[] = PHASES.map((_, i) =>
		i < 5 ? { x: 60 + i * 130, y: 36 } : { x: 60 + (9 - i) * 130, y: 96 }
	);

	// Hierarchy layout: orchestrator centered on top; three leads evenly
	// spaced; members in an ordered bottom row (grouped by lead but laid out
	// left to right to avoid overlaps); fleet-analyst dashed at the right of
	// the lead row.
	const HIER: {
		role: string;
		tier: 'orchestrator' | 'lead' | 'member' | 'advisory';
		lead?: string;
	}[] = [
		{ role: 'orchestrator', tier: 'orchestrator' },
		{ role: 'discovery-lead', tier: 'lead' },
		{ role: 'migration-lead', tier: 'lead' },
		{ role: 'integration-lead', tier: 'lead' },
		{ role: 'analyst', tier: 'member', lead: 'discovery-lead' },
		{ role: 'architect', tier: 'member', lead: 'discovery-lead' },
		{ role: 'translator', tier: 'member', lead: 'migration-lead' },
		{ role: 'validator', tier: 'member', lead: 'migration-lead' },
		{ role: 'tester', tier: 'member', lead: 'migration-lead' },
		{ role: 'planner', tier: 'member', lead: 'integration-lead' },
		{ role: 'failure-analyst', tier: 'member', lead: 'integration-lead' },
		{ role: 'critic', tier: 'member', lead: 'integration-lead' },
		{ role: 'repairer', tier: 'member', lead: 'integration-lead' },
		{ role: 'fleet-analyst', tier: 'advisory' }
	];

	const HIER_POS: Record<string, { x: number; y: number }> = {};
	{
		HIER_POS['orchestrator'] = { x: 250, y: 24 };
		HIER_POS['discovery-lead'] = { x: 90, y: 76 };
		HIER_POS['migration-lead'] = { x: 250, y: 76 };
		HIER_POS['integration-lead'] = { x: 390, y: 76 };
		HIER_POS['fleet-analyst'] = { x: 505, y: 24 };
		// 13 members is wider than the lead row; two member rows of 7 and 6.
		const members = HIER.filter((h) => h.tier === 'member');
		members.forEach((h, i) => {
			const row = i < 7 ? 0 : 1;
			const col = i < 7 ? i : i - 7;
			const rowW = row === 0 ? 7 : 6;
			HIER_POS[h.role] = {
				x: 40 + (col * 470) / (rowW - 1),
				y: row === 0 ? 128 : 176
			};
		});
	}

	const roleByRole = $derived.by(() => {
		const m = new Map<string, { recent: boolean; lastAction: string | null; lastActionAt: string | null }>();
		for (const n of round?.hierarchy ?? []) {
			m.set(n.role, { recent: n.recent, lastAction: n.lastAction, lastActionAt: n.lastActionAt });
		}
		return m;
	});

	function ringColor(role: string): string {
		const n = roleByRole.get(role);
		if (!n) return 'var(--color-rule-2)';
		return n.recent ? 'var(--color-accent)' : 'var(--color-rule-2)';
	}

	const statusColor: Record<string, string> = {
		done: 'var(--color-ok)',
		in_progress: 'var(--color-accent)',
		open: 'var(--color-rule-2)'
	};

	const hasHierarchyActivity = $derived((round?.hierarchy ?? []).some((n) => n.lastAction));
</script>

<article class="cell detail" aria-label="Round detail">
	<h2>Round detail</h2>
	{#if !round}
		<p class="empty">no active round to watch</p>
	{:else}
		<!-- A. Generic info -->
		<div class="section">
			<p class="micro">IDENTITY</p>
			<div class="id-grid">
				<span class="k">problem</span><span class="v mono">{round.generic.problem ?? '—'}</span>
				<span class="k">family</span><span class="v mono">{round.generic.family ?? '—'}</span>
				<span class="k">languages</span>
				<span class="v mono">{round.generic.sourceLanguage ?? '—'} → {round.generic.targetLanguage ?? '—'}</span>
				<span class="k">problem set</span><span class="v mono small">{clamp(round.generic.problemSet, 60)}</span>
				<span class="k">started</span>
				<span class="v mono" title={round.generic.createdAtUtc ?? ''}>
					{round.generic.createdAtLocal ?? '—'}</span>
				<span class="k">engine</span>
				<span class="v mono">{round.generic.provider ?? '—'} / {round.generic.model ?? '—'}</span>
				<span class="k">git</span><span class="v mono">{round.generic.gitRevision ?? '—'}</span>
				<span class="k">hypothesis</span><span class="v small" title={round.generic.hypothesis ?? ''}>
					{clamp(round.generic.hypothesis, 200)}</span>
				<span class="k">pairs</span>
				<span class="v mono">{round.generic.pairsLine ?? '—'} / {round.generic.pairsCount ?? '—'}</span>
				<span class="k">wall</span><span class="v mono">{wallLabel(round.generic.wallS ?? round.wallS)}</span>
				<span class="k">phase</span><span class="v mono badge">{round.generic.phase ?? '—'}</span>
			</div>
		</div>

		<!-- B. Metrics -->
		<div class="section">
			<p class="micro">METRICS · CUMULATIVE</p>
			<div class="metrics">
				<span><i class="micro">INPUT TOK</i><b class="mono">{tokLabel(round.metrics.inputTokens)}</b></span>
				<span><i class="micro">OUTPUT TOK</i><b class="mono">{tokLabel(round.metrics.outputTokens)}</b></span>
				<span><i class="micro">CACHED</i><b class="mono">{tokLabel(round.metrics.cachedTokens)}</b></span>
				<span><i class="micro">CALLS</i><b class="mono">{round.metrics.calls ?? '—'}</b></span>
				<span><i class="micro">CALLS/MIN</i><b class="mono">{round.metrics.callsPerMin ?? '—'}</b></span>
				<span><i class="micro">LAST LATENCY</i><b class="mono">{round.metrics.lastLatencyS ?? '—'}s</b></span>
				<span><i class="micro">AVG TOK/S</i><b class="mono">{round.tokSAvg ?? '—'}</b></span>
				<span><i class="micro">CTX WINDOW</i><b class="mono">{round.metrics.numCtx ?? '—'}</b></span>
			</div>
			<p class="qualifier">engine context from the engine probe shows model and GPU load in the engine cell; per-call context use is not in the trace schema, so it is not shown</p>
		</div>

		<!-- C. Hierarchy -->
		<div class="section">
			<p class="micro">AGENT HIERARCHY · RING = ACTIVITY IN LAST 5 MIN</p>
			{#if hasHierarchyActivity}
				<svg viewBox="0 0 560 210" class="hier" role="img" aria-label="Agent hierarchy graph">
					<!-- tier edges: orchestrator to each lead; dashed advisory to fleet-analyst -->
					{#each ['discovery-lead', 'migration-lead', 'integration-lead'] as lead (lead)}
						{@const o = HIER_POS['orchestrator']}
						{@const p = HIER_POS[lead]}
						<line x1={o.x} y1={o.y + 10} x2={p.x} y2={p.y - 10} class="edge" />
					{/each}
					<!-- advisory edge uses fixed positions; HIER_POS is static data -->
					<line x1="260" y1="24" x2="495" y2="24" class="edge advisory" />
					<!-- member edges: solid lead to member -->
					{#each HIER.filter((h) => h.lead) as h (h.role)}
						{@const p = HIER_POS[h.lead!]}
						{@const c = HIER_POS[h.role]}
						<line x1={p.x} y1={p.y + 10} x2={c.x} y2={c.y - 10} class="edge" />
					{/each}
					{#each HIER as h (h.role)}
						{@const c = HIER_POS[h.role]}
						{@const n = roleByRole.get(h.role)}
						<circle cx={c.x} cy={c.y} r={h.tier === 'member' ? 7 : 9} stroke={ringColor(h.role)} fill="none" stroke-width="1.5" />
						<circle cx={c.x} cy={c.y} r="3" fill={n?.recent ? 'var(--color-accent)' : 'var(--color-rule-2)'} />
						<text x={c.x} y={c.y + (h.tier === 'member' && h.lead ? 0 : 22)} text-anchor="middle" class="nlabel">
							{h.role.length > 14 ? h.role.slice(0, 13) + '...' : h.role}
						</text>
					{/each}
				</svg>
				<div class="rolelist">
					{#each round.hierarchy.filter((n) => n.lastAction) as n (n.role)}
						<span class="rolechip {n.recent ? 'recent' : ''}" title="{n.lastAction} · {n.lastActionAt}">
							{n.role}: {n.lastAction}{n.lastTaskId ? ` (${n.lastTaskId})` : ''}
						</span>
					{/each}
				</div>
			{:else}
				<p class="empty">no role attribution in the ledger yet; static layout only</p>
				<svg viewBox="0 0 560 210" class="hier dim" role="img" aria-label="Agent hierarchy layout">
					{#each ['discovery-lead', 'migration-lead', 'integration-lead'] as lead (lead)}
						{@const o = HIER_POS['orchestrator']}
						{@const p = HIER_POS[lead]}
						<line x1={o.x} y1={o.y + 10} x2={p.x} y2={p.y - 10} class="edge" />
					{/each}
					<!-- advisory edge uses fixed positions; HIER_POS is static data -->
					<line x1="260" y1="24" x2="495" y2="24" class="edge advisory" />
					{#each HIER.filter((h) => h.lead) as h (h.role)}
						{@const p = HIER_POS[h.lead!]}
						{@const c = HIER_POS[h.role]}
						<line x1={p.x} y1={p.y + 10} x2={c.x} y2={c.y - 10} class="edge" />
					{/each}
					{#each HIER as h (h.role)}
						{@const c = HIER_POS[h.role]}
						<circle cx={c.x} cy={c.y} r="3" fill="var(--color-rule-2)" />
						<text x={c.x} y={c.y + 22} text-anchor="middle" class="nlabel">
							{h.role.length > 14 ? h.role.slice(0, 13) + '...' : h.role}
						</text>
					{/each}
				</svg>
			{/if}
		</div>

		<!-- D. State machine -->
		<div class="section">
			<p class="micro">PHASE MACHINE · CURRENT HIGHLIGHTED · BACKWARD EDGES = REGRESSIONS TAKEN</p>
			<svg viewBox="0 0 600 150" class="machine" role="img" aria-label="Phase state machine">
				{#each PHASES as ph, i (ph)}
					{@const pos = MACHINE_POS[i]}
					{@const isCur = i === currentIdx}
					{@const dur = phaseDur.get(ph)}
					<rect
						x={pos.x - 42} y={pos.y - 14} width="84" height="26" rx="4"
						class={isCur ? 'node current' : dur != null ? 'node visited' : 'node'}
					/>
					<text x={pos.x} y={pos.y + 2} text-anchor="middle" class="plabel">{ph}</text>
					<text x={pos.x} y={pos.y + 22} text-anchor="middle" class="pdur">
						{dur != null ? wallLabel(dur) : ''}
					</text>
				{/each}
				<!-- Row-1 forward edges (Preflight→…→Pilot) and row-2 forward edges
					(Done←…←FinalValidation drawn right-to-left on screen); index
					pairs make the geometry explicit. -->
				{#each [0, 1, 2, 3] as i (i)}
					<line x1={MACHINE_POS[i].x + 44} y1={MACHINE_POS[i].y} x2={MACHINE_POS[i + 1].x - 44} y2={MACHINE_POS[i + 1].y} class="edge" />
				{/each}
				{#each [5, 6, 7, 8] as i (i)}
					<line x1={MACHINE_POS[i].x - 44} y1={MACHINE_POS[i].y} x2={MACHINE_POS[i + 1].x + 44} y2={MACHINE_POS[i + 1].y} class="edge" />
				{/each}
				<!-- Row-2 flow is right-to-left on screen: Migration(580) →
					Integration(450) → Hardening(320) → FinalValidation(190) →
					Done(60); every pair shrinks x. Lines pass through the box-free
					gap between the 44px half-width boxes, so no label collision. -->
				<!-- Pilot→Migration jump: row-gap midpoint (y=66) between the two
					rows, clear of both label bands (y-14..y+22). -->
				<line x1={MACHINE_POS[4].x} y1={MACHINE_POS[4].y + 14} x2={MACHINE_POS[5].x} y2={MACHINE_POS[5].y - 14} class="edge" />
				{#each round.machine.edges.filter((e) => e.regression) as e (e.from + e.to)}
					{@const fi = PHASES.indexOf(e.from)}
					{@const ti = PHASES.indexOf(e.to)}
					{#if fi >= 0 && ti >= 0}
						<path
							d="M {MACHINE_POS[fi].x} {MACHINE_POS[fi].y - 18} C {MACHINE_POS[fi].x} {MACHINE_POS[fi].y - 44}, {MACHINE_POS[ti].x} {MACHINE_POS[ti].y - 44}, {MACHINE_POS[ti].x} {MACHINE_POS[ti].y - 18}"
							class="regedge"
						/>
						<text
							x={(MACHINE_POS[fi].x + MACHINE_POS[ti].x) / 2}
							y={Math.min(MACHINE_POS[fi].y, MACHINE_POS[ti].y) - 30}
							text-anchor="middle" class="reglabel">x{e.count}</text>
					{/if}
				{/each}
			</svg>
			{#if round.machine.edges.some((e) => e.regression)}
				<p class="qualifier">
					regressions observed:
					{round.machine.edges.filter((e) => e.regression).map((e) => `${e.from} → ${e.to}`).join(', ')}
				</p>
			{:else}
				<p class="qualifier">no regression transitions in the event log</p>
			{/if}
		</div>

		<!-- E. Blackboard -->
		<div class="section">
			<p class="micro">BLACKBOARD · {round.blackboard.phase ?? '—'} · {round.blackboard.delegations ?? '—'} DELEGATIONS · MODEL {round.blackboard.currentModel ?? '—'}</p>
			<div class="bb">
				<div class="tasks">
					{#each round.tasks as t (t.id)}
						<div class="taskrow">
							<i class="dot {t.status}" title={t.status}></i>
							<span class="mono tid">{t.id}</span>
							<span class="tdeps mono">{t.dependsOn.length ? '← ' + t.dependsOn.join(', ') : ''}</span>
							<span class="ttitle">{t.title}</span>
						</div>
					{:else}
						<p class="empty">no tasks yet</p>
					{/each}
				</div>
				<div class="events">
					{#each round.blackboardEvents as ev (ev.at + ev.detail)}
						<div class="evrow">
							<span class="mono ts">{ev.at}</span>
							<span class="mono kind">{ev.event}</span>
							<span class="edetail">{ev.detail}</span>
						</div>
					{:else}
						<p class="empty">no blackboard events yet</p>
					{/each}
				</div>
			</div>
			{#if round.blackboard.lastTransition}
				<p class="qualifier">last transition: {round.blackboard.lastTransition}</p>
			{/if}
		</div>

		<!-- F. Proposer decisions -->
		<div class="section">
			<p class="micro">PROPOSER DECISIONS VS PREVIOUS ROUNDS</p>
			{#if round.proposer.predecessor}
				<p class="qualifier">
					{round.proposer.relationship}:
					<span class="mono">{round.proposer.predecessor}</span>
					({round.proposer.predecessorVerdict ?? 'verdict unknown'})
				</p>
				{#if round.proposer.manifestDeltas.length || round.proposer.configDeltas.length}
					<div class="deltas">
						{#each round.proposer.manifestDeltas as dl (dl.key)}
							<div class="deltarow">
								<span class="mono dkey">{dl.key}</span>
								<span class="dold mono">{clamp(dl.old, 46)}</span>
								<span class="darrow">→</span>
								<span class="dnew mono">{clamp(dl.neu, 46)}</span>
							</div>
						{/each}
						{#each round.proposer.configDeltas as dl (dl.key)}
							<div class="deltrow">
								<span class="mono dkey">cfg {dl.key}</span>
								<span class="dold mono">{clamp(dl.old, 46)}</span>
								<span class="darrow">→</span>
								<span class="dnew mono">{clamp(dl.neu, 46)}</span>
							</div>
						{/each}
					</div>
				{:else}
					<p class="empty">{round.proposer.note || 'no config delta'}</p>
				{/if}
				{#if round.proposer.predecessorHypothesis}
					<p class="qualifier">previous hypothesis: {clamp(round.proposer.predecessorHypothesis, 160)}</p>
				{/if}
			{:else}
				<p class="empty">{round.proposer.note}</p>
			{/if}
		</div>
	{/if}
</article>

<style>
	h2,
	.micro {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		line-height: 1;
		color: var(--color-muted);
	}

	.empty {
		margin: var(--space-2xs) 0 0;
		font-size: var(--text-2xs);
		color: var(--color-muted);
	}

	.detail {
		margin-top: var(--space-md);
		border: 1px solid var(--color-rule);
		border-radius: var(--radius-panel);
		padding: var(--space-md);
	}

	.section {
		margin-top: var(--space-md);
		padding-top: var(--space-sm);
		border-top: 1px solid var(--color-rule);
	}

	.id-grid {
		display: grid;
		grid-template-columns: max-content 1fr;
		gap: var(--space-2xs) var(--space-sm);
		margin-top: var(--space-2xs);
	}

	.k {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-muted);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		align-self: baseline;
	}

	.v {
		font-size: var(--text-xs);
		min-width: 0;
		overflow-wrap: anywhere;
	}

	.mono {
		font-family: var(--font-mono);
	}

	.small {
		font-size: var(--text-2xs);
		color: var(--color-ink-2);
	}

	.badge {
		color: var(--color-accent);
		font-weight: 600;
	}

	.metrics {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-sm) var(--space-md);
		margin-top: var(--space-2xs);
	}

	.metrics > span {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.metrics b {
		font-size: var(--text-sm);
		font-weight: 500;
		font-variant-numeric: tabular-nums;
	}

	.qualifier {
		margin: var(--space-2xs) 0 0;
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-muted);
	}

	.hier,
	.machine {
		width: 100%;
		max-width: 600px;
		height: auto;
		margin-top: var(--space-2xs);
	}

	.hier.dim {
		opacity: 0.5;
	}

	.edge {
		stroke: var(--color-rule-2);
		stroke-width: 1;
	}

	.edge.advisory {
		stroke-dasharray: 3 3;
	}

	.nlabel,
	.plabel {
		font-family: var(--font-mono);
		font-size: 7px;
		fill: var(--color-ink-2);
	}

	.pdur {
		font-family: var(--font-mono);
		font-size: 6.5px;
		fill: var(--color-muted);
	}

	.node {
		fill: var(--color-paper);
		stroke: var(--color-rule-2);
	}

	.node.visited {
		stroke: var(--color-ink-2);
	}

	.node.current {
		stroke: var(--color-accent);
		stroke-width: 2;
	}

	.regedge {
		fill: none;
		stroke: var(--color-warn);
		stroke-width: 1.2;
		stroke-dasharray: 4 2;
	}

	.reglabel {
		font-family: var(--font-mono);
		font-size: 7px;
		fill: var(--color-warn);
	}

	.rolelist {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2xs);
		margin-top: var(--space-2xs);
	}

	.rolechip {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-ink-2);
		border: 1px solid var(--color-rule-2);
		border-radius: 3px;
		padding: 1px 5px;
	}

	.rolechip.recent {
		border-color: var(--color-accent);
		color: var(--color-accent);
	}

	.bb {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-md);
	}

	.tasks,
	.events {
		margin-top: var(--space-2xs);
		min-width: 0;
	}

	.taskrow,
	.evrow {
		display: flex;
		align-items: baseline;
		gap: var(--space-2xs);
		margin-top: 3px;
		font-size: var(--text-2xs);
		min-width: 0;
	}

	.dot {
		display: inline-block;
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--color-rule-2);
		flex: none;
	}

	.dot.done {
		background: var(--color-ok);
	}

	.dot.in_progress {
		background: var(--color-accent);
	}

	.tid {
		color: var(--color-ink-2);
		flex: none;
	}

	.tdeps {
		color: var(--color-muted);
		flex: none;
		max-width: 30%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.ttitle {
		color: var(--color-ink-2);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.ts {
		color: var(--color-muted);
		flex: none;
	}

	.kind {
		color: var(--color-accent);
		flex: none;
	}

	.edetail {
		color: var(--color-ink-2);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.deltas {
		margin-top: var(--space-2xs);
	}

	.deltrow,
	.deltarow {
		display: flex;
		align-items: baseline;
		gap: var(--space-2xs);
		margin-top: 3px;
		font-size: var(--text-2xs);
		min-width: 0;
	}

	.dkey {
		color: var(--color-ink);
		font-weight: 600;
		flex: none;
		min-width: 16ch;
	}

	.dold {
		color: var(--color-muted);
		text-decoration: line-through;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		max-width: 34%;
	}

	.darrow {
		color: var(--color-muted);
		flex: none;
	}

	.dnew {
		color: var(--color-ok);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	@media (max-width: 60rem) {
		.bb {
			grid-template-columns: 1fr;
		}
	}
</style>
