<script lang="ts">
	import type { StatusPayload } from '$lib/types.js';

	let { status }: { status: StatusPayload | null } = $props();

	const lines = $derived(
		status?.journal && !('error' in status.journal) ? status.journal : []
	);
	const error = $derived(
		status?.journal && 'error' in status.journal ? status.journal.error : null
	);
</script>

<section class="tail" aria-label="Driver journal tail">
	<p class="source">JOURNAL · q27sweep-driver</p>
	{#if error}
		<p class="jerr">{error}</p>
	{:else if lines.length === 0}
		<p class="jerr">no journal lines</p>
	{:else}
		<pre>{lines.join('\n')}</pre>
	{/if}
</section>

<style>
	.tail {
		margin-top: var(--space-md);
		border-top: 1px solid var(--color-rule);
		background: var(--color-graphite-2);
		border-radius: 0 0 var(--radius-panel) var(--radius-panel);
		padding: var(--space-sm) var(--space-md);
		overflow-x: auto;
	}

	.source {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		line-height: 1;
		color: var(--color-paper);
		opacity: 0.7;
	}

	pre {
		margin: var(--space-xs) 0 0;
		font-family: var(--font-mono);
		font-size: 12px;
		line-height: 1.5;
		color: var(--color-paper);
		white-space: pre;
		overflow-x: auto;
	}

	.jerr {
		margin: var(--space-xs) 0 0;
		font-family: var(--font-mono);
		font-size: 12px;
		color: var(--color-paper);
		opacity: 0.7;
	}
</style>
