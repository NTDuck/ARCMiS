<script lang="ts">
	import type { StatusPayload } from '$lib/types.js';

	let { status }: { status: StatusPayload | null } = $props();

	const rows = $derived(
		status?.recentCloses && !('error' in status.recentCloses) ? status.recentCloses : []
	);
	const error = $derived(status?.recentCloses && 'error' in status.recentCloses ? status.recentCloses.error : null);
</script>

<article class="cell span-2" aria-label="Recent closes">
	<h2>Recent closes</h2>
	{#if error}
		<p class="empty">{error}</p>
	{:else if rows.length === 0}
		<p class="empty">No closed rounds in the ledger yet.</p>
	{:else}
		<table>
			<thead>
				<tr>
					<th scope="col">Round</th>
					<th scope="col">Problem</th>
					<th scope="col">Verdict</th>
					<th scope="col">Tests</th>
					<th scope="col">Wall s</th>
				</tr>
			</thead>
			<tbody>
				{#each rows as r}
					<tr>
						<td data-label="Round">{r.roundTag}</td>
						<td data-label="Problem">{r.problem}</td>
						<td data-label="Verdict">{r.verdict}</td>
						<td data-label="Tests">{r.tests}</td>
						<td data-label="Wall s">{r.wallS}</td>
					</tr>
				{/each}
			</tbody>
		</table>
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

	table {
		width: 100%;
		margin-top: var(--space-sm);
		border-collapse: collapse;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
	}

	th {
		text-align: left;
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		line-height: 1.2;
		font-size: var(--text-2xs);
		color: var(--color-muted);
		padding: 0 var(--space-xs) var(--space-2xs) 0;
		border-bottom: 1px solid var(--color-rule-2);
	}

	td {
		padding: var(--space-2xs) var(--space-xs) var(--space-2xs) 0;
		border-bottom: 1px solid var(--color-rule);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}

	td[data-label='Problem'] {
		max-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	@media (max-width: 40rem) {
		thead {
			display: none;
		}

		tbody tr {
			display: flex;
			flex-wrap: wrap;
			gap: var(--space-xs) var(--space-sm);
			padding: var(--space-xs) 0;
			border-bottom: 1px solid var(--color-rule);
		}

		td {
			border: none;
			padding: 0;
		}

		td::before {
			content: attr(data-label) ' ';
			color: var(--color-muted);
			font-size: var(--text-2xs);
			text-transform: uppercase;
			letter-spacing: 0.06em;
		}
	}
</style>
