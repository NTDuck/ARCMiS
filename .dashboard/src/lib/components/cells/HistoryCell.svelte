<script lang="ts">
	import type { CampaignState } from '$lib/types.js';

	let { campaign }: { campaign: CampaignState | null } = $props();

	function verdictClass(v: string): string {
		if (v === 'SOLVED') return 'solved';
		if (v === 'NOT CLEARED') return 'not-cleared';
		if (v === 'ABORTED') return 'aborted';
		return '';
	}

	function wallLabel(s: number): string {
		if (s < 90) return `${s}s`;
		const m = Math.round(s / 60);
		if (m < 90) return `${m}m`;
		return `${(m / 60).toFixed(1)}h`;
	}

	// Log-scale bars: walls span 60s to ~15h; linear hides everything.
	const histMax = $derived(
		campaign?.history.length ? Math.log10(Math.max(...campaign.history.map((h) => h.wallS))) : 1
	);

	function barPct(s: number): number {
		return Math.max(2, Math.round((Math.log10(Math.max(s, 1)) / histMax) * 100));
	}
</script>

<article class="cell span-2" aria-label="Wall-time history">
	<h2>Wall time per round · log scale</h2>
	{#if !campaign?.history.length}
		<p class="empty">no closed rounds with events on disk</p>
	{:else}
		<div class="chart" role="img" aria-label="Wall time per closed round, log scale">
			{#each campaign.history as h (h.problem + h.wallS)}
				<span
					class="bar {verdictClass(h.verdict)}"
					title="{h.problem} · {h.verdict} · {wallLabel(h.wallS)}"
					style="height: {barPct(h.wallS)}%"
				></span>
			{/each}
		</div>
		<p class="legend">
			{campaign.history.length} closed rounds · {wallLabel(campaign.history[0].wallS)} fastest
			({campaign.history[0].problem}) · {wallLabel(campaign.history[campaign.history.length - 1].wallS)} slowest
			({campaign.history[campaign.history.length - 1].problem})
		</p>
		<p class="legend legend-2">
			<span><i class="swatch solved"></i>SOLVED</span>
			<span><i class="swatch not-cleared"></i>NOT CLEARED</span>
			<span><i class="swatch aborted"></i>ABORTED</span>
		</p>
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

	.chart {
		display: flex;
		align-items: flex-end;
		gap: 2px;
		height: 72px;
		margin-top: var(--space-sm);
	}

	.bar {
		flex: 1;
		min-width: 2px;
		border-radius: 1px 1px 0 0;
		background: var(--color-rule-2);
	}

	.bar.solved {
		background: var(--color-ok);
	}

	.bar.not-cleared {
		background: var(--color-error);
		opacity: 0.65;
	}

	.bar.aborted {
		background: var(--color-rule-2);
	}

	.legend {
		margin: var(--space-xs) 0 0;
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-muted);
	}

	.legend-2 {
		display: flex;
		gap: var(--space-md);
	}

	.legend-2 span {
		display: flex;
		align-items: center;
		gap: var(--space-3xs);
	}

	.swatch {
		display: inline-block;
		width: 8px;
		height: 8px;
		border-radius: 1px;
	}

	.swatch.solved {
		background: var(--color-ok);
	}

	.swatch.not-cleared {
		background: var(--color-error);
		opacity: 0.65;
	}

	.swatch.aborted {
		background: var(--color-rule-2);
	}
</style>
