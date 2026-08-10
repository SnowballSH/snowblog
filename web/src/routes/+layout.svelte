<script lang="ts">
	import '../app.css';
	import { resolve } from '$app/paths';
	import { Footer, Header, Link, ThemeToggle } from 'foundationui/svelte';

	let { data, children } = $props();
</script>

<div
	class="pointer-events-none fixed inset-0 -z-10 bg-[radial-gradient(60rem_40rem_at_85%_-10%,color-mix(in_oklab,var(--fui-accent)_20%,transparent),transparent),radial-gradient(50rem_36rem_at_-10%_110%,color-mix(in_oklab,var(--fui-aurora)_16%,transparent),transparent)]"
></div>
<Header>
	<a
		href={resolve('/')}
		class="flex items-center gap-2 font-display text-sm font-semibold tracking-wide whitespace-nowrap text-ink"
	>
		{data.site.name}
	</a>
	<div class="flex items-center gap-4">
		<ThemeToggle />
	</div>
</Header>
<main class="mx-auto w-full max-w-6xl flex-1 px-4 py-10 sm:px-6">
	{@render children()}
</main>
<Footer>
	<div class="flex flex-wrap items-center justify-between gap-4">
		<span>{data.site.footerText}</span>
		<Link href="https://github.com/SnowballSH/snowblog" subtle external>Built with snowblog</Link>
	</div>
</Footer>
