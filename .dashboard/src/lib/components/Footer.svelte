<script lang="ts">
	import { ENGINE_URL, DATA_DIR, BUILD_DATE } from '$lib/meta.js';
</script>

<footer>
	<p>{BUILD_DATE} · {ENGINE_URL} · localhost-only · {DATA_DIR}</p>
</footer>

<style>
	footer {
		border-top: 1px solid var(--color-rule);
		padding: var(--space-sm) var(--space-md);
	}

	footer p {
		margin: 0;
		font-family: var(--font-mono);
		font-size: var(--text-2xs);
		color: var(--color-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
