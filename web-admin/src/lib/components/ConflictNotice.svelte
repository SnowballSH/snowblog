<script lang="ts">
	import { base } from '$app/paths';
	import { Callout, Link } from 'foundationui/svelte';

	let { slug, message }: { slug: string; message: string } = $props();
</script>

<Callout tone="warn" class="flex flex-col gap-2">
	<p class="font-semibold text-ink">This post changed underneath you</p>
	<p>{message}</p>
	<p class="text-sm text-ink-secondary">
		Someone else's edit landed first. Copy any unsaved text out of the form now, then reload to see
		the current revision before trying again.
	</p>
	<Link href={`${base}/posts/${slug}`} data-sveltekit-reload>Reload this post</Link>
</Callout>
