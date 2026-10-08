<script lang="ts">
	import { tick } from 'svelte';

	interface Item {
		id: string;
		label: string;
		hint: string;
		kind: 'action' | 'cell';
		dir?: string;
		// Exact aria-label of the target cell for jump items.
		target?: string;
	}

	let { onClose, onRun }: { onClose: () => void; onRun: (name: string, dir?: string) => void } =
		$props();

	const ITEMS: Item[] = [
		{ id: 'start', label: 'Start sweep', hint: 'action', kind: 'action' },
		{ id: 'stop', label: 'Stop sweep', hint: 'action', kind: 'action' },
		{ id: 'restart-engine', label: 'Restart engine', hint: 'action', kind: 'action' },
		{ id: 'rescore', label: 'Rescore dir', hint: 'action', kind: 'action' },
		{ id: 'ledger-sync', label: 'Ledger sync', hint: 'action', kind: 'action' },
		{ id: 'purge-orphans', label: 'Purge orphans', hint: 'action', kind: 'action' },
		{ id: 'cell-scoreboard', label: 'Go to scoreboard', hint: 'cell', kind: 'cell', target: 'Scoreboard' },
		{ id: 'cell-driver', label: 'Go to driver · watchdog', hint: 'cell', kind: 'cell', target: 'Driver and watchdog' },
		{ id: 'cell-inflight', label: 'Go to in-flight slots', hint: 'cell', kind: 'cell', target: 'In-flight rounds' },
		{ id: 'cell-closes', label: 'Go to recent closes', hint: 'cell', kind: 'cell', target: 'Recent closes' },
		{ id: 'cell-engine', label: 'Go to engine', hint: 'cell', kind: 'cell', target: 'Engine' },
		{ id: 'cell-actions', label: 'Go to actions', hint: 'cell', kind: 'cell', target: 'Actions' },
		{ id: 'cell-live', label: 'Go to live rounds', hint: 'cell', kind: 'cell', target: 'Live rounds' },
		{ id: 'cell-campaign', label: 'Go to campaign state', hint: 'cell', kind: 'cell', target: 'Campaign state' },
		{ id: 'cell-fleet', label: 'Go to fleet health', hint: 'cell', kind: 'cell', target: 'Fleet health' },
		{ id: 'cell-history', label: 'Go to wall-time history', hint: 'cell', kind: 'cell', target: 'Wall-time history' }
	];

	let query = $state('');
	let selected = $state(0);
	let dialogEl = $state<HTMLDivElement | null>(null);
	let inputEl = $state<HTMLInputElement | null>(null);
	let restoreFocusTo: HTMLElement | null = null;

	const filtered = $derived(
		ITEMS.filter((it) => it.label.toLowerCase().includes(query.toLowerCase()))
	);

	$effect(() => {
		if (selected >= filtered.length) selected = Math.max(0, filtered.length - 1);
	});

	$effect(() => {
		restoreFocusTo = document.activeElement as HTMLElement | null;
		inputEl?.focus();
		return () => {
			restoreFocusTo?.focus();
		};
	});

	async function trapFocus(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			onClose();
			return;
		}
		if (e.key === 'ArrowDown') {
			e.preventDefault();
			selected = (selected + 1) % Math.max(1, filtered.length);
			await tick();
			scrollToSelected();
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			selected = (selected - 1 + Math.max(1, filtered.length)) % Math.max(1, filtered.length);
			await tick();
			scrollToSelected();
		} else if (e.key === 'Enter') {
			e.preventDefault();
			const item = filtered[selected];
			if (item) pick(item);
		}
	}

	function scrollToSelected() {
		const el = dialogEl?.querySelector('[data-selected="true"]');
		el?.scrollIntoView({ block: 'nearest' });
	}

	function pick(item: Item) {
		if (item.kind === 'action') {
			onRun(item.id, item.dir);
		} else if (item.target) {
			const cell = document.querySelector(`[aria-label="${item.target}"]`);
			cell?.scrollIntoView({ block: 'start' });
			onClose();
		}
	}

	function backdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) onClose();
	}
</script>

<svelte:window onkeydown={trapFocus} />

<div
	class="backdrop"
	role="presentation"
	onclick={backdropClick}
>
	<div
		class="dialog"
		role="dialog"
		aria-modal="true"
		aria-label="Command palette"
		bind:this={dialogEl}
	>
		<input
			type="text"
			placeholder="Search actions…"
			aria-label="Filter actions"
			bind:this={inputEl}
			bind:value={query}
			oninput={() => (selected = 0)}
		/>
		<ul role="listbox" aria-label="Commands">
			{#each filtered as item, i (item.id)}
				<li role="option" aria-selected={i === selected} data-selected={i === selected}>
					<button
						type="button"
						onclick={() => pick(item)}
						onfocus={() => (selected = i)}
						onmouseover={() => (selected = i)}
					>
						<span class="label">{item.label}</span>
						<span class="hint">{item.hint}</span>
					</button>
				</li>
			{:else}
				<li class="no-match" role="presentation">no matching command</li>
			{/each}
		</ul>
		<p class="palette-footer">↑↓ select · ↵ run · esc close</p>
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: color-mix(in oklab, var(--color-ink) 35%, transparent);
		display: flex;
		align-items: flex-start;
		justify-content: center;
		padding: var(--space-2xl) var(--space-md) 0;
		z-index: 100;
	}

	.dialog {
		width: 100%;
		max-width: 32rem;
		background: var(--color-paper);
		border: 1px solid var(--color-rule-2);
		border-radius: var(--radius-panel);
		overflow: hidden;
	}

	input {
		width: 100%;
		height: 44px;
		padding: 0 var(--space-md);
		border: none;
		border-bottom: 1px solid var(--color-rule);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		color: var(--color-ink);
		background: var(--color-paper);
	}

	input:focus-visible {
		outline: none;
	}

	ul {
		list-style: none;
		margin: 0;
		padding: var(--space-2xs);
		max-height: 16rem;
		overflow-y: auto;
	}

	li button {
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-md);
		padding: var(--space-xs) var(--space-sm);
		border: none;
		border-radius: var(--radius-btn);
		background: transparent;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-ink);
		cursor: pointer;
		text-align: left;
		min-height: 44px;
	}

	li[data-selected='true'] button {
		background: var(--color-rule);
	}

	li[data-selected='true'] button:focus-visible {
		outline: 2px solid var(--color-focus);
		outline-offset: 2px;
	}

	.hint {
		font-size: var(--text-2xs);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-muted);
		flex-shrink: 0;
	}

	.no-match {
		padding: var(--space-sm);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-muted);
	}

	.palette-footer {
		margin: 0;
		padding: var(--space-2xs) var(--space-md);
		border-top: 1px solid var(--color-rule);
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-muted);
		white-space: nowrap;
	}
</style>
