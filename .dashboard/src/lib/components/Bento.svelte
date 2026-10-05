<script lang="ts">
	import type { StatusPayload, ActionResult } from '$lib/types.js';
	import Scoreboard from './cells/Scoreboard.svelte';
	import DriverCell from './cells/DriverCell.svelte';
	import InFlightCell from './cells/InFlightCell.svelte';
	import RecentCloses from './cells/RecentCloses.svelte';
	import EngineCell from './cells/EngineCell.svelte';
	import ActionsCell from './cells/ActionsCell.svelte';

	let {
		status,
		actionState,
		onAction,
		actionsRef = $bindable(null)
	}: {
		status: StatusPayload | null;
		actionState: { inFlight: string | null; result: { name: string; ok: boolean; detail: string } | null };
		onAction: (name: string, dir?: string, confirm?: boolean) => Promise<void>;
		actionsRef: {
			run: (name: string, dir?: string, confirm?: boolean) => Promise<void>;
			arm: (id: string) => void;
		} | null;
	} = $props();
</script>

<section class="bento" aria-label="Campaign status grid">
	<Scoreboard status={status} />
	<DriverCell status={status} />
	<InFlightCell status={status} />
	<RecentCloses status={status} />
	<EngineCell status={status} />
	<ActionsCell {status} {actionState} {onAction} bind:actionsRef />
</section>

<style>
	.bento {
		display: grid;
		grid-template-columns: repeat(4, minmax(0, 1fr));
		gap: var(--space-md);
		margin: var(--space-md) 0 0;
	}

	.bento > :global(*) {
		border: 1px solid var(--color-rule);
		border-radius: var(--radius-panel);
		padding: var(--space-md);
		min-width: 0;
		transition: border-color 150ms var(--ease-out);
	}

	.bento > :global(*:hover) {
		border-color: var(--color-accent);
	}

	/* Irregular bento: 1x1, 2x1, 1x1 | 2x1, 1x1, 1x1 */
	.bento > :global(.span-2) {
		grid-column: span 2;
	}

	@media (max-width: 60rem) {
		.bento {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}
	}

	@media (max-width: 40rem) {
		.bento {
			grid-template-columns: minmax(0, 1fr);
		}

		.bento > :global(.span-2) {
			grid-column: span 1;
		}
	}
</style>
