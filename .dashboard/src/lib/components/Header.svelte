<script lang="ts">
	import type { StatusPayload } from '$lib/types.js';
	import { ENGINE_URL, DATA_DIR } from '$lib/meta.js';

	let { status, onOpenPalette }: { status: StatusPayload | null; onOpenPalette: () => void } =
		$props();

	let utc = $state('');
	$effect(() => {
		const tick = () => {
			utc = new Date().toISOString().replace('T', ' ').slice(0, 19) + 'Z';
		};
		tick();
		const id = setInterval(tick, 1000);
		return () => clearInterval(id);
	});

	const cleared = $derived(status?.scoreboard.cleared ?? null);
</script>

<header>
	<div class="left">
		<h1>ARCMiS · autooptimise console</h1>
		<p class="meta">
			{ENGINE_URL} · {DATA_DIR} · <span class="clock">{utc}</span>
		</p>
	</div>
	<button class="palette-pill" type="button" onclick={onOpenPalette}>
		Search actions… <kbd>⌘K</kbd>
	</button>
	<div class="score">
		{#if cleared !== null}
			<span class="numeral">{cleared}/114</span>
			<span class="qualifier">problems cleared · latest verdict per round</span>
		{:else}
			<span class="numeral numeral-unknown">—</span>
			<span class="qualifier">ledger unavailable</span>
		{/if}
	</div>
</header>

<style>
	header {
		position: sticky;
		top: 0;
		z-index: 10;
		display: flex;
		align-items: center;
		gap: var(--space-md);
		height: 5.5rem;
		padding: 0 var(--space-md);
		background: var(--color-paper);
		border-bottom: 1px solid var(--color-rule);
	}

	.left {
		min-width: 0;
		flex: 1;
	}

	h1 {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: var(--text-lg);
		letter-spacing: -0.02em;
		overflow-wrap: anywhere;
		min-width: 0;
	}

	.meta {
		margin: var(--space-3xs) 0 0;
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		min-width: 0;
	}

	.clock {
		font-variant-numeric: tabular-nums;
	}

	.palette-pill {
		flex-shrink: 0;
		display: none;
		align-items: center;
		gap: var(--space-xs);
		height: 44px;
		padding: 0 var(--space-sm);
		background: var(--color-paper);
		border: 1px solid var(--color-rule-2);
		border-radius: var(--radius-btn);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-muted);
		cursor: pointer;
		white-space: nowrap;
		transition: border-color 150ms var(--ease-out), background-color 150ms var(--ease-out);
	}

	@media (min-width: 60rem) {
		.palette-pill {
			display: inline-flex;
		}
	}

	.palette-pill:hover {
		border-color: var(--color-accent);
	}

	kbd {
		font-family: inherit;
		border: 1px solid var(--color-rule-2);
		border-radius: 4px;
		padding: 0 var(--space-2xs);
		font-size: var(--text-2xs);
	}

	.score {
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		min-width: 0;
	}

	.numeral {
		font-family: var(--font-display);
		font-weight: 600;
		font-size: var(--text-3xl);
		letter-spacing: -0.02em;
		line-height: 1;
		font-variant-numeric: tabular-nums;
	}

	.numeral-unknown {
		color: var(--color-muted);
	}

	.qualifier {
		margin-top: var(--space-3xs);
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-muted);
		white-space: nowrap;
	}

	@media (max-width: 60rem) {
		.palette-pill kbd {
			display: none;
		}

		.palette-pill {
			font-size: 0;
			gap: 0;
		}

		.palette-pill kbd {
			font-size: var(--text-xs);
		}

		.qualifier {
			display: none;
		}
	}
</style>
