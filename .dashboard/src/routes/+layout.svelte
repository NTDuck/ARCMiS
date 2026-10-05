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
	import JournalTail from '$lib/components/JournalTail.svelte';
	import Footer from '$lib/components/Footer.svelte';
	import CommandPalette from '$lib/components/CommandPalette.svelte';
	import { POLL_MS } from '$lib/config.js';
	import type { StatusPayload } from '$lib/types.js';

	let { children } = $props();

	let status = $state<StatusPayload | null>(null);
	let statusError = $state<string | null>(null);
	let paletteOpen = $state(false);
	let actionsRef: { run: (name: string, dir?: string) => Promise<void> } | null = $state(null);
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
	}

	$effect(() => {
		poll();
		const id = setInterval(poll, POLL_MS);
		return () => clearInterval(id);
	});

	function onKeydown(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
			e.preventDefault();
			paletteOpen = !paletteOpen;
		}
	}

	async function runAction(name: string, dir?: string) {
		if (actionState.inFlight) return;
		actionState = { inFlight: name, result: actionState.result };
		try {
			const res = await fetch(`/api/actions/${name}`, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify(dir ? { dir, confirm: true } : {})
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
		<Bento {status} {actionState} bind:actionsRef />
		<JournalTail {status} />
	</main>
	<Footer />
</div>

{@render children()}

{#if paletteOpen}
	<CommandPalette
		onClose={() => (paletteOpen = false)}
		onRun={(name, dir) => {
			paletteOpen = false;
			runAction(name, dir);
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
