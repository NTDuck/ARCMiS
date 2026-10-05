<script lang="ts">
	import type { StatusPayload } from '$lib/types.js';

	let { status }: { status: StatusPayload | null } = $props();

	const rounds = $derived(status?.inFlight ?? []);

	function ageLabel(ageS: number | null): string {
		if (ageS === null) return '—';
		if (ageS < 90) return `${ageS}s`;
		return `${Math.round(ageS / 60)}m`;
	}

	function wallLabel(wallS: number | null): string {
		if (wallS === null) return '—';
		const m = Math.floor(wallS / 60);
		return `${m}m`;
	}
</script>

<article class="cell span-2" aria-label="In-flight rounds">
	<h2>In-flight slots</h2>
	{#if rounds.length === 0}
		<p class="empty">
			No live rounds — every sweep dir has either a result/aggregate.yml or a quiet events.jsonl
			(&gt;15 min since last event).
		</p>
	{:else}
		<ul>
			{#each rounds as r}
				<li>
					<span class="dir">{r.dir}</span>
					<span class="micro">PHASE</span>
					<span class="val">{r.phase}</span>
					<span class="micro">LAST EVENT</span>
					<span class="val">{ageLabel(r.lastEventAgeS)}</span>
					<span class="micro">WALL</span>
					<span class="val">{wallLabel(r.wallS)}</span>
				</li>
			{/each}
		</ul>
	{/if}
</article>

<style>
	h2 {
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

	ul {
		list-style: none;
		margin: var(--space-sm) 0 0;
		padding: 0;
		display: flex;
		flex-direction: column;
	}

	li {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: var(--space-2xs) var(--space-sm);
		padding: var(--space-xs) 0;
		border-bottom: 1px solid var(--color-rule);
		min-width: 0;
	}

	li:last-child {
		border-bottom: none;
	}

	.dir {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		font-weight: 500;
		flex: 1 1 100%;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	@media (min-width: 40rem) {
		.dir {
			flex: 1;
			flex-basis: auto;
		}
	}

	.micro {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		line-height: 1;
		color: var(--color-muted);
		flex-shrink: 0;
	}

	.val {
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		font-weight: 500;
		font-variant-numeric: tabular-nums;
		flex-shrink: 0;
	}
</style>
