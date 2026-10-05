<script lang="ts">
	import type { StatusPayload, ActionResult } from '$lib/types.js';
	import { POLL_MS } from '$lib/config.js';

	let {
		status,
		actionState,
		actionsRef = $bindable(null)
	}: {
		status: StatusPayload | null;
		actionState: { inFlight: string | null; result: { name: string; ok: boolean; detail: string } | null };
		actionsRef: { run: (name: string, dir?: string) => Promise<void> } | null;
	} = $props();

	interface ActionDef {
		id: string;
		label: string;
		primary?: boolean;
		destructive?: boolean;
	}

	const ACTIONS: ActionDef[] = [
		{ id: 'start', label: 'Start sweep', primary: true },
		{ id: 'stop', label: 'Stop sweep', destructive: true },
		{ id: 'restart-engine', label: 'Restart engine' },
		{ id: 'rescore', label: 'Rescore dir' },
		{ id: 'ledger-sync', label: 'Ledger sync' },
		{ id: 'purge-orphans', label: 'Purge orphans', destructive: true }
	];

	let armed = $state<string | null>(null);
	let armTimer: ReturnType<typeof setTimeout> | null = null;
	let rescorePrompting = $state(false);
	let rescoreDir = $state('');

	// Expose run() so the command palette and layout can trigger actions.
	actionsRef = {
		run: async (name: string, dir?: string) => {
			await act(name, dir);
		}
	};

	async function act(id: string, dir?: string) {
		if (actionState.inFlight) return;
		disarm();
		await actionsRef?.run(id, dir);
	}

	function click(def: ActionDef) {
		if (actionState.inFlight) return;
		if (def.destructive && armed !== def.id) {
			armed = def.id;
			if (armTimer) clearTimeout(armTimer);
			armTimer = setTimeout(disarm, 5000);
			return;
		}
		disarm();
		act(def.id);
	}

	function disarm() {
		armed = null;
		if (armTimer) {
			clearTimeout(armTimer);
			armTimer = null;
		}
	}

	async function rescoreSubmit() {
		if (!rescoreDir.trim()) return;
		rescorePrompting = false;
		await act('rescore', rescoreDir.trim());
		rescoreDir = '';
	}
</script>

<article class="cell" aria-label="Actions">
	<h2>Actions</h2>
	<div class="buttons">
		{#each ACTIONS as def (def.id)}
			<button
				type="button"
				class:primary={def.primary}
				class:danger={def.destructive && armed === def.id}
				disabled={actionState.inFlight !== null}
				aria-disabled={actionState.inFlight !== null}
				onclick={() => (def.id === 'rescore' ? (rescorePrompting = true) : click(def))}
			>
				{#if actionState.inFlight === def.id}
					<span class="spinner" aria-hidden="true"></span>
				{:else if def.destructive && armed === def.id}
					{def.id === 'stop' ? 'Confirm stop?' : 'Confirm purge?'}
				{:else}
					{def.label}
				{/if}
			</button>
		{/each}
	</div>

	{#if rescorePrompting}
		<form class="rescore-row" onsubmit={(e) => { e.preventDefault(); rescoreSubmit(); }}>
			<label class="micro" for="rescore-dir">ROUND DIR</label>
			<input
				id="rescore-dir"
				type="text"
				bind:value={rescoreDir}
				placeholder="20261005T072628Zv3s31-bst-sweep"
				spellcheck="false"
			/>
			<button type="submit" class="primary" disabled={actionState.inFlight !== null}>Run</button>
			<button type="button" onclick={() => (rescorePrompting = false)}>Cancel</button>
		</form>
	{/if}

	<p class="status-line" aria-live="polite">
		{#if actionState.inFlight}
			running {actionState.inFlight}…
		{:else if actionState.result}
			<span class={actionState.result.ok ? 'ok-text' : 'err-text'}>
				{actionState.result.ok ? 'ok — ' : ''}{actionState.result.detail}
			</span>
		{/if}
	</p>
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

	.buttons {
		margin-top: var(--space-sm);
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-xs);
	}

	button {
		height: 44px;
		padding: 0 var(--space-sm);
		background: var(--color-paper);
		border: 1px solid var(--color-rule-2);
		border-radius: var(--radius-btn);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		font-weight: 500;
		color: var(--color-ink);
		cursor: pointer;
		white-space: nowrap;
		transition: border-color 150ms var(--ease-out), background-color 150ms var(--ease-out);
	}

	button:hover:not(:disabled) {
		border-color: var(--color-accent);
	}

	button:active:not(:disabled) {
		transform: translateY(1px);
	}

	button:disabled {
		opacity: 0.55;
		cursor: not-allowed;
	}

	button.primary {
		background: var(--color-accent);
		border-color: var(--color-accent);
		color: var(--color-accent-ink);
	}

	button.danger {
		border-color: var(--color-error);
		color: var(--color-error);
	}

	.spinner {
		display: inline-block;
		width: 12px;
		height: 12px;
		border: 2px solid var(--color-rule-2);
		border-top-color: var(--color-accent);
		border-radius: 50%;
		animation: spin 800ms linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.spinner {
			animation: none;
			border-top-color: var(--color-rule-2);
		}

		button:active:not(:disabled) {
			transform: none;
		}
	}

	.rescore-row {
		margin-top: var(--space-sm);
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-xs);
	}

	.micro {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		line-height: 1;
		color: var(--color-muted);
	}

	input {
		flex: 1;
		min-width: 0;
		height: 44px;
		padding: 0 var(--space-xs);
		border: 1px solid var(--color-rule-2);
		border-radius: var(--radius-btn);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-ink);
		background: var(--color-paper);
	}

	.status-line {
		margin: var(--space-sm) 0 0;
		min-height: 1.2em;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.ok-text {
		color: var(--color-ink-2);
	}

	.err-text {
		color: var(--color-error);
	}
</style>
