<script lang="ts">
	import type { StatusPayload } from '$lib/types.js';

	let { status }: { status: StatusPayload | null } = $props();

	const units = $derived(status?.units ?? []);
	const hb = $derived(status?.heartbeats);

	function unitClass(active: string): string {
		if (active === 'active') return 'dot ok';
		if (active === 'failed') return 'dot err';
		return 'dot idle';
	}

	function ageLabel(ageS: number | null): string {
		if (ageS === null) return '—';
		if (ageS < 90) return `${ageS}s ago`;
		return `${Math.round(ageS / 60)}m ago`;
	}
</script>

<article class="cell" aria-label="Driver and watchdog">
	<h2>Driver · Watchdog</h2>
	<ul class="units">
		{#each units as u}
			<li>
				<span class={unitClass(u.active)} aria-hidden="true"></span>
				<span class="unit-name">{u.unit}</span>
				<span class="unit-state">{u.error ? '—' : u.active}</span>
			</li>
		{/each}
	</ul>
	<div class="heartbeats">
		<div>
			<span class="micro">SWEEP HB</span>
			<span class="val" class:warn={hb?.sweep.stale}>
				{hb?.sweep.error ? '— missing' : ageLabel(hb?.sweep.ageS ?? null)}
			</span>
		</div>
		<div>
			<span class="micro">WATCHDOG HB</span>
			<span class="val" class:warn={hb?.watchdog.stale}>
				{hb?.watchdog.error ? '— missing' : ageLabel(hb?.watchdog.ageS ?? null)}
			</span>
		</div>
	</div>
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

	.units {
		list-style: none;
		margin: var(--space-sm) 0 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-xs);
	}

	.units li {
		display: flex;
		align-items: center;
		gap: var(--space-xs);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
	}

	.unit-name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.unit-state {
		font-variant-numeric: tabular-nums;
	}

	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.dot.ok {
		background: var(--color-ok);
	}

	.dot.err {
		background: var(--color-error);
	}

	.dot.idle {
		background: var(--color-rule-2);
	}

	.heartbeats {
		margin-top: var(--space-md);
		padding-top: var(--space-sm);
		border-top: 1px solid var(--color-rule);
		display: flex;
		flex-direction: column;
		gap: var(--space-xs);
	}

	.heartbeats > div {
		display: flex;
		flex-direction: column;
	}

	.micro {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		line-height: 1;
		color: var(--color-muted);
	}

	.val {
		margin-top: var(--space-3xs);
		font-family: var(--font-mono);
		font-size: var(--text-sm);
		font-weight: 500;
		font-variant-numeric: tabular-nums;
	}

	.val.warn {
		color: var(--color-warn);
	}
</style>
