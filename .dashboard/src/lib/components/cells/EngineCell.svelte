<script lang="ts">
	import type { StatusPayload } from '$lib/types.js';

	let { status }: { status: StatusPayload | null } = $props();

	const eng = $derived(status?.engine);
	const mismatch = $derived(eng && eng.modelId !== null && !eng.match);
	const modelError = $derived(eng?.errors.some((e) => e.startsWith('models:')) ?? false);

	function kbToGb(kb: number | null): string {
		if (kb === null) return '—';
		return (kb / 1024 / 1024).toFixed(1) + ' GiB';
	}
</script>

<article class="cell dark" aria-label="Engine">
	<h2>Engine · ninfer-serve</h2>
	{#if mismatch}
		<p class="mismatch" role="alert">MODEL MISMATCH — expected qwen3.8-27b/ninfer, got {eng?.modelId}/{eng?.ownedBy}</p>
	{:else if modelError}
		<p class="mismatch" role="alert">MODEL UNREACHABLE — {eng?.errors.find((e) => e.startsWith('models:'))?.replace('models: ', '')}</p>
	{:else}
		<p class="model">
			{eng?.modelId ?? '—'} <span class="owned">· {eng?.ownedBy ?? '—'}</span>
		</p>
	{/if}
	<dl>
		<div>
			<dt>VMRSS</dt>
			<dd>{kbToGb(eng?.vmrssKb ?? null)}</dd>
		</div>
		<div>
			<dt>PSI CPU</dt>
			<dd>{eng?.psi.cpu !== null && eng?.psi.cpu !== undefined ? `${eng.psi.cpu.toFixed(1)}%` : '—'}</dd>
		</div>
		<div>
			<dt>PSI MEM</dt>
			<dd>{eng?.psi.mem !== null && eng?.psi.mem !== undefined ? `${eng.psi.mem.toFixed(1)}%` : '—'}</dd>
		</div>
		<div>
			<dt>PSI IO</dt>
			<dd>{eng?.psi.io !== null && eng?.psi.io !== undefined ? `${eng.psi.io.toFixed(1)}%` : '—'}</dd>
		</div>
		<div>
			<dt>GPU UTIL</dt>
			<dd>{eng?.gpu.util !== null && eng?.gpu.util !== undefined ? `${eng.gpu.util}%` : '—'}</dd>
		</div>
		<div>
			<dt>GPU VRAM</dt>
			<dd>
				{eng?.gpu.memUsed !== null && eng?.gpu.memUsed !== undefined && eng?.gpu.memTotal
					? `${eng.gpu.memUsed} / ${eng.gpu.memTotal} MiB`
					: '—'}
			</dd>
		</div>
	</dl>
</article>

<style>
	.dark {
		background: var(--color-graphite-2);
		border-color: var(--color-graphite);
		color: var(--color-paper);
	}

	h2 {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		line-height: 1;
		color: var(--color-paper);
		opacity: 0.7;
	}

	.model {
		margin: var(--space-sm) 0 0;
		font-family: var(--font-mono);
		font-size: var(--text-base);
		font-weight: 600;
	}

	.owned {
		opacity: 0.7;
		font-weight: 400;
	}

	.mismatch {
		margin: var(--space-sm) 0 0;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--color-warn);
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
		opacity: 0.7;
	}

	dd {
		margin: var(--space-3xs) 0 0;
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 500;
		font-variant-numeric: tabular-nums;
	}
</style>
