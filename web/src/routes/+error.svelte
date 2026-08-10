<script lang="ts">
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { Link, Panel } from 'foundationui/svelte';

	const message = $derived(
		page.status === 404 ? "This page doesn't exist." : 'Something went wrong upstream.'
	);
</script>

<svelte:head>
	<title>{page.status} — snowblog</title>
</svelte:head>

<div class="mx-auto max-w-[74ch]">
	<Panel padding="lg" class="flex flex-col items-start gap-3">
		<h1 class="font-display text-3xl font-semibold">{page.status}</h1>
		<p class="text-ink-secondary">{message}</p>
		<Link href={resolve('/')}>Back to all posts</Link>
	</Panel>
</div>
