<script lang="ts">
	import '../app.css';
	import { Header, ThemeToggle } from 'foundationui/svelte';
	let { children, data } = $props();
</script>

<Header>
	<span class="font-display text-sm font-semibold tracking-wide text-ink">snowblog admin</span>
	<div class="flex items-center gap-3">
		<span class="text-sm text-ink-muted">{data.user}</span>
		<ThemeToggle />
	</div>
</Header>
<main class="mx-auto w-full max-w-5xl flex-1 px-4 py-8">
	{@render children()}
</main>
