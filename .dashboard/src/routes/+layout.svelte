<script lang="ts">
	import '@fontsource/space-grotesk/500.css';
	import '@fontsource/space-grotesk/600.css';
	import '@fontsource/inter/400.css';
	import '@fontsource/inter/500.css';
	import '@fontsource/jetbrains-mono/400.css';
	import '@fontsource/jetbrains-mono/500.css';
	import '@fontsource/jetbrains-mono/600.css';
	import '$lib/../app.css';
	import Header from '$lib/components/Header.svelte';
	import Bento from '$lib/components/Bento.svelte';
	import LiveRounds from '$lib/components/cells/LiveRounds.svelte';
	import RoundDetailCell from '$lib/components/cells/RoundDetailCell.svelte';
	import CampaignCell from '$lib/components/cells/CampaignCell.svelte';
	import FleetCell from '$lib/components/cells/FleetCell.svelte';
	import HistoryCell from '$lib/components/cells/HistoryCell.svelte';
	import JournalTail from '$lib/components/JournalTail.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import CommandPalette from '$lib/components/CommandPalette.svelte';
	import { POLL_MS } from '$lib/config.js';
	import type { StatusPayload, CampaignState, FleetState } from '$lib/types.js';

	let { children } = $props();

	let status = $state<StatusPayload | null>(null);
	let campaign = $state<CampaignState | null>(null);
	let fleet = $state<FleetState | null>(null);
	let statusError = $state<string | null>(null);
	let paletteOpen = $state(false);
	let actionsRef: {
		run: (name: string, dir?: string, confirm?: boolean) => Promise<void>;
		arm: (id: string) => void;
	} | null = $state(null);
	let actionState = $state<{
		inFlight: string | null;
		result: { name: string; ok: boolean; detail: string } | null;
	}>({ inFlight: null, result: null });

	async function poll() {
		try {
			const res = await fetch('/api/status', { cache: 'no-store' });
			if (!res.ok) throw new Error(`status ${res.status}`);
			status = await res.json();
			statusError = null;
		} catch (e) {
			statusError = (e as Error).message;
		}
		// Campaign state: same cadence, cheap file reads; failure keeps the
		// last payload on screen rather than blanking the panel.
		try {
			const res = await fetch('/api/campaign', { cache: 'no-store' });
			if (res.ok) campaign = await res.json();
		} catch {
			// keep previous campaign payload
		}
	}

	async function refreshFleet() {
		try {
			const res = await fetch('/api/fleet', { cache: 'no-store' });
			if (res.ok) fleet = await res.json();
		} catch {
			// keep previous fleet payload; button can be pressed again
		}
	}

	$effect(() => {
		poll();
		refreshFleet();
		const id = setInterval(poll, POLL_MS);
		return () => clearInterval(id);
	});

	function onKeydown(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
			e.preventDefault();
			paletteOpen = !paletteOpen;
		}
	}

	async function runAction(name: string, dir?: string, confirm?: boolean) {
		if (actionState.inFlight) return;
		actionState = { inFlight: name, result: actionState.result };
		try {
			// Conditional literal, not property mutation: rollup treeshake drops
			// property assignments on locals even when the object is later read.
			const payload: Record<string, unknown> = dir
				? // confirm rides only alongside a dir (purge-specific deletes).
					{ dir, confirm: confirm === true }
				: {};
			const res = await fetch(`/api/actions/${name}`, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify(payload)
			});
			const data = (await res.json()) as { ok: boolean; detail: string };
			actionState = { inFlight: null, result: { name, ok: data.ok, detail: data.detail } };
		} catch (e) {
			actionState = { inFlight: null, result: { name, ok: false, detail: (e as Error).message } };
		}
		poll();
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="shell">
	<Header {status} onOpenPalette={() => (paletteOpen = true)} />
	<main>
		{#if statusError}
			<p class="status-error" role="alert">status probe failed — {statusError} · retrying every 10s</p>
		{/if}
		<Bento {status} {actionState} onAction={runAction} bind:actionsRef />
		<LiveRounds {campaign} />
		<RoundDetailCell {campaign} />
		<CampaignCell {campaign} />
		<FleetCell {fleet} onRefresh={refreshFleet} />
		<HistoryCell {campaign} />
		<JournalTail {status} />
	</main>
	<Footer />
</div>

{@render children()}

{#if paletteOpen}
	<CommandPalette
		onClose={() => (paletteOpen = false)}
		onRun={(name) => {
			paletteOpen = false;
			// Destructive palette items never run directly: arm the button so the
			// two-step confirm happens in the actions cell, same as a click.
			if (name === 'stop' || name === 'purge-orphans') {
				actionsRef?.arm(name);
			} else {
				runAction(name);
			}
		}}
	/>
{/if}

<style>
	.shell {
		min-height: 100dvh;
		display: flex;
		flex-direction: column;
	}

	main {
		flex: 1;
		width: 100%;
		max-width: 80rem;
		margin: 0 auto;
		padding: 0 var(--space-md) var(--space-lg);
		min-width: 0;
	}

	.status-error {
		margin: 0 0 var(--space-sm);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		color: var(--color-error);
	}
</style>
