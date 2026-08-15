<script lang="ts">
	import '../app.css';
	import { Header, ThemeToggle } from 'foundationui/svelte';
	import { page } from '$app/state';
	let { children, data } = $props();

	const fullBleed = $derived(page.route.id === '/posts/[slug]/write/[language]');

	$effect(() => {
		document.documentElement.classList.toggle('write-view-lock', fullBleed);
		return () => document.documentElement.classList.remove('write-view-lock');
	});
</script>

<Header>
	<span class="font-display text-sm font-semibold tracking-wide text-ink">snowblog admin</span>
	<div class="flex items-center gap-3">
		<span class="text-sm text-ink-muted">{data.user}</span>
		<ThemeToggle />
	</div>
</Header>
<main
	class={fullBleed ? 'flex min-h-0 flex-1 flex-col' : 'mx-auto w-full max-w-5xl flex-1 px-4 py-8'}
>
	{@render children()}
</main>
