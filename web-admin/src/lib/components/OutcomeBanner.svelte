<script lang="ts">
	import { Callout } from 'foundationui/svelte';

	interface Outcome {
		ok: boolean;
		auditId: string;
		message: string;
		conflict?: boolean;
	}

	let { result }: { result: Outcome | null | undefined } = $props();
</script>

{#if result}
	<Callout tone={result.ok ? 'aurora' : 'warn'}>
		<p>{result.message}</p>
		<p class="text-sm text-ink-muted">audit {result.auditId}</p>
	</Callout>
{/if}
