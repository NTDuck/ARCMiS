<script lang="ts">
	import type { StatusPayload } from '$lib/types.js';

	let { status }: { status: StatusPayload | null } = $props();

	let prevCleared = $state<number | null>(null);
	let tick = $state(false);

	$effect(() => {
		const c = status?.scoreboard.cleared ?? null;
		if (c !== null && prevCleared !== null && c !== prevCleared) {
			tick = true;
			const t = setTimeout(() => (tick = false), 400);
			return () => clearTimeout(t);
		}
		prevCleared = c;
	});

	const sb = $derived(status?.scoreboard);
</script>

<article class="cell" aria-live="polite" aria-label="Scoreboard">
	<h2>Scoreboard</h2>
	{#if sb}
		<p class="big" class:tick>
			{sb.cleared ?? '—'}<span class="of">/{sb.total}</span>
		</p>
		<dl>
			<div>
				<dt>WAVE</dt>
				<dd>{sb.waveId ?? '—'}</dd>
			</div>
			<div>
				<dt>LAUNCHED</dt>
				<dd>{sb.launched ?? '—'}/{sb.wavePairs ?? '—'}</dd>
			</div>
			<div>
				<dt>DONE</dt>
				<dd>{sb.done ?? '—'}/{sb.wavePairs ?? '—'}</dd>
			</div>
			{#if sb.clearsPerDay !== null}
				<div>
					<dt>CLEARS/DAY</dt>
					<dd>{sb.clearsPerDay}</dd>
				</div>
			{/if}
		</dl>
	{:else}
		<p class="big">—</p>
	{/if}
</article>

<style>
	.cell {
		background: var(--color-paper);
	}

	h2 {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		line-height: 1;
		color: var(--color-muted);
	}

	.big {
		margin: var(--space-sm) 0 0;
		font-family: var(--font-display);
		font-weight: 600;
		font-size: var(--text-4xl);
		letter-spacing: -0.02em;
		line-height: 1;
		font-variant-numeric: tabular-nums;
	}

	.big.tick {
		animation: fade-tick 400ms var(--ease-out);
	}

	@keyframes fade-tick {
		from {
			opacity: 0.3;
		}
		to {
			opacity: 1;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.big.tick {
			animation: none;
		}
	}

	.of {
		color: var(--color-muted);
		font-size: var(--text-2xl);
	}

	dl {
		margin: var(--space-md) 0 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-xs);
	}

	dl > div {
		display: flex;
		flex-direction: column;
	}

	dt {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		line-height: 1;
		color: var(--color-muted);
	}

	dd {
		margin: var(--space-3xs) 0 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 500;
		font-variant-numeric: tabular-nums;
	}
</style>
