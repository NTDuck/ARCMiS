<script lang="ts">
	import type { FleetState } from '$lib/types.js';

	let { fleet: fleetState, onRefresh }: { fleet: FleetState | null; onRefresh: () => void } = $props();

	function ageLabel(ageS: number | null): string {
		if (ageS === null) return '—';
		if (ageS < 90) return `${ageS}s ago`;
		const m = Math.round(ageS / 60);
		if (m < 90) return `${m}m ago`;
		return `${(m / 60).toFixed(1)}h ago`;
	}

	function logAge(mtimeMs: number | null): string {
		if (mtimeMs === null) return '—';
		return ageLabel(Math.round((Date.now() - mtimeMs) / 1000));
	}

	function unitClass(active: string): string {
		if (active === 'active') return 'dot ok';
		if (active === 'failed') return 'dot err';
		return 'dot idle';
	}
</script>

<article class="cell span-2" aria-label="Fleet health">
	<div class="head">
		<h2>Fleet health</h2>
		<button type="button" onclick={onRefresh}>Refresh probes</button>
	</div>

	<div class="grid">
		<div>
			<p class="micro">GPU</p>
			<ul class="rows">
				{#each fleetState?.gpus ?? [] as g (g.uuid ?? g.name)}
					<li>
						<span class="val">{g.index}</span>
						<span class="name">{g.name}</span>
						<span class="val">{g.util !== null ? `${g.util}%` : '—'} · {g.memUsed ?? '—'}/{g.memTotal ?? '—'} MiB</span>
					</li>
				{:else}
					<li><span class="empty">nvidia-smi unavailable</span></li>
				{/each}
			</ul>
		</div>
		<div>
			<p class="micro">UNITS</p>
			<ul class="rows">
				{#each fleetState?.units ?? [] as u (u.unit)}
					<li>
						<span class={unitClass(u.active)} aria-hidden="true"></span>
						<span class="name">{u.unit}</span>
						<span class="val">{u.active}</span>
					</li>
				{/each}
			</ul>
		</div>
		<div>
			<p class="micro">HEARTBEATS · STALE &gt; 600s</p>
			<ul class="rows">
				<li>
					<span class="name">sweep</span>
					<span class="val" class:stale={fleetState?.heartbeats.sweep.stale}>
						{fleetState?.heartbeats.sweep.error ? '— missing' : ageLabel(fleetState?.heartbeats.sweep.ageS ?? null)}
					</span>
				</li>
				<li>
					<span class="name">watchdog</span>
					<span class="val" class:stale={fleetState?.heartbeats.watchdog.stale}>
						{fleetState?.heartbeats.watchdog.error ? '— missing' : ageLabel(fleetState?.heartbeats.watchdog.ageS ?? null)}
					</span>
				</li>
			</ul>
		</div>
	</div>

	<div class="logs">
		<p class="micro">LAUNCHER LOGS · NEWEST 8</p>
		<ul class="rows">
			{#each fleetState?.launcherLogs ?? [] as l (l.file)}
				<li class="log-row">
					<span class="name">{l.file}</span>
					<span class="val at">{logAge(l.mtimeMs)}</span>
					<span class="tail">{l.tail.length ? l.tail[l.tail.length - 1] : '(empty)'}</span>
				</li>
			{:else}
				<li><span class="empty">no launcher logs in /tmp</span></li>
			{/each}
		</ul>
	</div>

	{#if fleetState?.errors.length}
		<p class="empty" role="alert">{fleetState.errors.join(' · ')}</p>
	{/if}
</article>

<style>
	h2,
	.micro {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		font-weight: 500;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		line-height: 1;
		color: var(--color-muted);
	}

	.head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: var(--space-sm);
	}

	.head button {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--color-ink);
		background: var(--color-paper);
		border: 1px solid var(--color-rule-2);
		border-radius: var(--radius-btn);
		padding: var(--space-3xs) var(--space-xs);
		cursor: pointer;
	}

	.head button:hover {
		border-color: var(--color-accent);
	}

	.head button:focus-visible {
		outline: 2px solid var(--color-focus);
		outline-offset: 1px;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(3, minmax(0, 1fr));
		gap: var(--space-md);
		margin-top: var(--space-sm);
	}

	@media (max-width: 60rem) {
		.grid {
			grid-template-columns: 1fr;
		}
	}

	.rows {
		list-style: none;
		margin: var(--space-2xs) 0 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-2xs);
	}

	.rows li {
		display: flex;
		gap: var(--space-2xs);
		align-items: baseline;
		min-width: 0;
	}

	.name {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-ink-2);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.val {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		font-weight: 500;
		font-variant-numeric: tabular-nums;
		flex-shrink: 0;
	}

	.at {
		color: var(--color-muted);
	}

	.stale {
		color: var(--color-error);
	}

	.empty {
		font-size: var(--text-2xs);
		color: var(--color-muted);
	}

	.logs {
		margin-top: var(--space-sm);
		padding-top: var(--space-xs);
		border-top: 1px solid var(--color-rule);
	}

	.log-row {
		align-items: baseline;
	}

	.tail {
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		min-width: 0;
		flex: 1;
	}

	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		flex-shrink: 0;
		align-self: center;
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
</style>
