<script lang="ts">
	import type { CampaignState, RoundDetail } from '$lib/types.js';
	import { POLL_MS } from '$lib/config.js';

	let { campaign }: { campaign: CampaignState | null } = $props();

	// The live round this panel watches: the newest active dir (manifest
	// present, no result aggregate). The campaign payload's activeDirs is
	// sorted ascending, so the last entry is the latest launch.
	let round = $state<RoundDetail | null>(null);
	let roundError = $state<string | null>(null);

	const watchDir = $derived(campaign?.activeDirs.length ? campaign.activeDirs[campaign.activeDirs.length - 1] : null);

	async function loadRound(dir: string) {
		try {
			const res = await fetch(`/api/round/${dir}`, { cache: 'no-store' });
			if (!res.ok) throw new Error(`round ${res.status}`);
			round = await res.json();
			roundError = null;
		} catch (e) {
			roundError = (e as Error).message;
		}
	}

	$effect(() => {
		// Watch the newest active dir. Refresh on the same cadence as the
		// status poll; a dir switch reloads immediately.
		const dir = watchDir;
		if (!dir) {
			round = null;
			return;
		}
		loadRound(dir);
		const id = setInterval(() => loadRound(dir), POLL_MS);
		return () => clearInterval(id);
	});

	function durLabel(s: number | null): string {
		if (s === null) return '—';
		if (s < 90) return `${s}s`;
		const m = Math.round(s / 60);
		if (m < 90) return `${m}m`;
		return `${(m / 60).toFixed(1)}h`;
	}

	function wallLabel(s: number | null): string {
		if (s === null) return '—';
		const m = Math.floor(s / 60);
		const h = Math.floor(m / 60);
		return h > 0 ? `${h}h ${m % 60}m` : `${m}m`;
	}

	// Rolling tok/s sparkline: bucket the last 120 points into 30 bars.
	const bars = $derived.by(() => {
		const pts = round?.throughput.filter((p) => p.tokS !== null) ?? [];
		if (pts.length < 2) return [];
		const BARS = 30;
		const per = Math.ceil(pts.length / BARS);
		const out: (number | null)[] = [];
		for (let i = 0; i < BARS; i++) {
			const slice = pts.slice(i * per, (i + 1) * per).map((p) => p.tokS ?? 0);
			out.push(slice.length ? Math.round((slice.reduce((a, b) => a + b, 0) / slice.length) * 10) / 10 : null);
		}
		const max = Math.max(...out.map((v) => v ?? 0), 1);
		return out.map((v) => (v === null ? null : Math.round((v / max) * 100)));
	});
</script>

<article class="cell span-2" aria-label="Live rounds">
	<h2>Live rounds</h2>
	{#if !round}
		<p class="empty">
			{roundError ? `round probe failed — ${roundError}` : 'No active rounds (manifest present, no result aggregate).'}
		</p>
	{:else}
		<div class="head">
			<span class="dir">{round.dir}</span>
			<span class="micro">PHASE</span>
			<span class="val">{round.currentPhase ?? '—'}</span>
			<span class="micro">WALL</span>
			<span class="val">{wallLabel(round.wallS)}</span>
		</div>

		<div class="section">
			<p class="micro">PHASE TIMELINE</p>
			<div class="timeline" role="img" aria-label="Phase timeline">
				{#each round.phases as p, i (i)}
					<span
						class="tl-block {i === round.phases.length - 1 ? 'tl-open' : ''}"
						title="{p.phase}: {durLabel(p.durS)}"
						style="flex: {Math.max(p.durS ?? 1, 1)}"
					></span>
				{/each}
			</div>
			<ul class="phase-list">
				{#each round.phases.slice(-4) as p, i (round.phases.length - 4 + i)}
					<li>
						<span class="phase-name">{p.phase}</span>
						<span class="val">{durLabel(p.durS)}</span>
					</li>
				{/each}
			</ul>
		</div>

		<div class="section">
			<p class="micro">THROUGHPUT · OUTPUT TOK/S PER MODEL RESPONSE</p>
			{#if bars.length === 0}
				<p class="empty">no completed model responses yet</p>
			{:else}
				<div class="spark" role="img" aria-label="Rolling output tokens per second">
					{#each bars as b, i (i)}
						<span class="bar" style="height: {b ?? 2}%"></span>
					{/each}
				</div>
				<p class="legend">
					avg {round.tokSAvg ?? '—'} tok/s · {round.calls} model calls · last latency {round.lastLatencyS ?? '—'}s
				</p>
			{/if}
		</div>

		<div class="section two">
			<div>
				<p class="micro">DECISIONS · NEWEST FIRST</p>
				<ul class="decisions">
					{#each round.decisions.slice(0, 5) as dec, i (i)}
						<li>
							<span class="val at">{dec.at}</span>
							<span class="action">{dec.action}</span>
							<span class="detail">{dec.detail || dec.phase}</span>
						</li>
					{:else}
						<li><span class="empty">no decisions recorded</span></li>
					{/each}
				</ul>
			</div>
			<div>
				<p class="micro">TASK GRAPH · {round.fanout.done}/{round.fanout.openTasks} DONE · {round.fanout.inProgress} IN PROGRESS</p>
				<ul class="tasks">
					{#each round.tasks as t (t.id)}
						<li>
							<span class="task-id">{t.id}</span>
							<span class="status {t.status}">{t.status}</span>
							<span class="detail">{t.title}</span>
						</li>
					{:else}
						<li><span class="empty">no tasks yet</span></li>
					{/each}
				</ul>
			</div>
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
		margin: var(--space-sm) 0 0;
		font-size: var(--text-sm);
		color: var(--color-muted);
	}

	.head {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: var(--space-2xs) var(--space-sm);
		margin-top: var(--space-sm);
	}

	.dir {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		font-weight: 500;
		margin-right: auto;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.micro {
		flex-shrink: 0;
	}

	.val {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		font-weight: 500;
		font-variant-numeric: tabular-nums;
		flex-shrink: 0;
	}

	.section {
		margin-top: var(--space-sm);
		padding-top: var(--space-xs);
		border-top: 1px solid var(--color-rule);
	}

	.timeline {
		display: flex;
		gap: 2px;
		height: 10px;
		margin-top: var(--space-2xs);
	}

	.tl-block {
		background: var(--color-rule-2);
		min-width: 4px;
		border-radius: 1px;
	}

	.tl-block.tl-open {
		background: var(--color-accent);
	}

	.phase-list {
		list-style: none;
		margin: var(--space-2xs) 0 0;
		padding: 0;
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2xs) var(--space-md);
	}

	.phase-list li {
		display: flex;
		gap: var(--space-2xs);
	}

	.phase-name {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-muted);
	}

	.spark {
		display: flex;
		align-items: flex-end;
		gap: 1px;
		height: 36px;
		margin-top: var(--space-2xs);
	}

	.bar {
		flex: 1;
		min-width: 2px;
		background: var(--color-accent);
		opacity: 0.75;
		border-radius: 1px 1px 0 0;
	}

	.legend {
		margin: var(--space-2xs) 0 0;
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-muted);
	}

	.two {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: var(--space-md);
	}

	@media (max-width: 60rem) {
		.two {
			grid-template-columns: 1fr;
		}
	}

	ul.decisions,
	ul.tasks {
		list-style: none;
		margin: var(--space-2xs) 0 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-2xs);
	}

	ul.decisions li,
	ul.tasks li {
		display: flex;
		gap: var(--space-2xs);
		align-items: baseline;
		min-width: 0;
		font-size: var(--text-2xs);
	}

	.at {
		color: var(--color-muted);
		flex-shrink: 0;
	}

	.action {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-accent);
		flex-shrink: 0;
	}

	.task-id {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		flex-shrink: 0;
	}

	.status {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		text-transform: uppercase;
		flex-shrink: 0;
	}

	.status.done {
		color: var(--color-ok);
	}

	.status.in_progress {
		color: var(--color-accent);
	}

	.status.failed {
		color: var(--color-error);
	}

	.detail {
		font-family: var(--font-body);
		color: var(--color-ink-2);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		min-width: 0;
	}
</style>
