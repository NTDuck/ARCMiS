<script lang="ts">
	import type { CampaignState } from '$lib/types.js';

	let { campaign }: { campaign: CampaignState | null } = $props();

	function verdictClass(v: string): string {
		if (v === 'SOLVED') return 'verdict solved';
		if (v === 'NOT CLEARED') return 'verdict not-cleared';
		if (v === 'ABORTED') return 'verdict aborted';
		if (v === 'IN FLIGHT') return 'verdict in-flight';
		return 'verdict';
	}

	function wallLabel(s: number): string {
		if (s < 90) return `${s}s`;
		const m = Math.round(s / 60);
		if (m < 90) return `${m}m`;
		return `${(m / 60).toFixed(1)}h`;
	}

	// Log-scale wall-time bars (log10; walls span 60s to 15h).
	const histMax = $derived(
		campaign?.history.length ? Math.log10(Math.max(...campaign.history.map((h) => h.wallS))) : 1
	);
</script>

<article class="cell span-2" aria-label="Campaign state">
	<h2>Campaign state</h2>

	<div class="topline">
		<span class="big">{campaign?.cleared ?? '—'}<span class="of">/{campaign?.clearTotal ?? '—'}</span></span>
		<span class="qualifier">cleared · brief total (disk: {campaign?.onDiskTotal ?? '—'} dirs)</span>
	</div>

	<div class="families">
		<p class="micro">CLEARED BY FAMILY</p>
		<ul class="rows">
			{#each campaign?.families ?? [] as f (f.family)}
				<li>
					<span class="name">{f.family}</span>
					<span class="val">{f.cleared}/{f.total}</span>
				</li>
			{/each}
		</ul>
	</div>

	<div class="pairs">
		<p class="micro">PAIRS POSITION · {campaign?.pairsFile.name ?? '—'}</p>
		<p class="val">
			{#if campaign?.pairsFile.line !== null && campaign?.pairsFile.line !== undefined}
				line {campaign.pairsFile.line}/{campaign.pairsFile.count} · {campaign.pairsFile.current}
			{:else}
				{campaign?.pairsFile.count ?? '—'} pairs · no live round matched
			{/if}
		</p>
	</div>

	<div class="census">
		<p class="micro">DEATH CENSUS · LEDGER TOTALS</p>
		<ul class="rows census-rows">
			<li><span class="name">max-turns deaths</span><span class="val">{campaign?.deathCensus.maxTurns ?? '—'}</span></li>
			<li><span class="name">output-cap deaths</span><span class="val">{campaign?.deathCensus.outputCap ?? '—'}</span></li>
			<li><span class="name">stalled rounds</span><span class="val">{campaign?.deathCensus.stalled ?? '—'}</span></li>
			<li><span class="name">admission-503 (last 10 closed)</span><span class="val">{campaign?.admission503.total ?? '—'}</span></li>
		</ul>
	</div>

	<div class="verdicts">
		<p class="micro">VERDICT TABLE · LATEST PER CANDIDATE</p>
		<table>
			<thead>
				<tr>
					<th scope="col">Problem</th>
					<th scope="col">Family</th>
					<th scope="col">Verdict</th>
					<th scope="col">Tests</th>
					<th scope="col">Wall s</th>
				</tr>
			</thead>
			<tbody>
				{#each campaign?.verdicts ?? [] as v (v.candidateId)}
					<tr>
						<td data-label="Problem">{v.problem}</td>
						<td data-label="Family">{v.family ?? '—'}</td>
						<td data-label="Verdict" class={verdictClass(v.verdict)}>{v.verdict}</td>
						<td data-label="Tests">{v.tests}</td>
						<td data-label="Wall s">{v.wallS}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>

	{#if campaign?.errors.length}
		<p class="empty" role="alert">{campaign.errors.join(' · ')}</p>
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

	.topline {
		display: flex;
		align-items: baseline;
		gap: var(--space-sm);
		margin-top: var(--space-sm);
	}

	.big {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: var(--text-2xl);
		line-height: 1;
		font-variant-numeric: tabular-nums;
	}

	.of {
		color: var(--color-muted);
		font-size: var(--text-lg);
	}

	.qualifier {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-muted);
	}

	.families,
	.pairs,
	.census {
		margin-top: var(--space-sm);
	}

	.rows {
		list-style: none;
		margin: var(--space-2xs) 0 0;
		padding: 0;
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2xs) var(--space-md);
	}

	.rows li {
		display: flex;
		gap: var(--space-2xs);
		align-items: baseline;
	}

	.name {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-ink-2);
	}

	.val {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		font-weight: 500;
		font-variant-numeric: tabular-nums;
	}

	.census-rows {
		flex-direction: column;
	}

	.verdicts {
		margin-top: var(--space-sm);
		padding-top: var(--space-xs);
		border-top: 1px solid var(--color-rule);
		overflow-x: auto;
	}

	table {
		width: 100%;
		margin-top: var(--space-2xs);
		border-collapse: collapse;
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
	}

	th {
		text-align: left;
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-muted);
		padding: 0 var(--space-xs) var(--space-3xs) 0;
		border-bottom: 1px solid var(--color-rule-2);
	}

	td {
		padding: var(--space-3xs) var(--space-xs) var(--space-3xs) 0;
		border-bottom: 1px solid var(--color-rule);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}

	.verdict.solved {
		color: var(--color-ok);
		font-weight: 600;
	}

	.verdict.not-cleared {
		color: var(--color-error);
	}

	.verdict.aborted {
		color: var(--color-muted);
	}

	.verdict.in-flight {
		color: var(--color-accent);
		font-weight: 600;
	}

	.empty {
		margin: var(--space-2xs) 0 0;
		font-size: var(--text-2xs);
		color: var(--color-muted);
	}
</style>
