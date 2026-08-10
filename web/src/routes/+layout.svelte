<script lang="ts">
	import '../app.css';
	import { page } from '$app/state';
	import { resolve } from '$app/paths';
	import { Footer, Header, Link, ThemeToggle } from 'foundationui/svelte';

	let { data, children } = $props();

	const atHome = $derived(page.url.pathname === '/');
</script>

<svelte:head>
	<link rel="icon" href="/favicon.ico" sizes="48x48" />
	<link rel="icon" type="image/png" sizes="32x32" href="/favicon-32.png" />
	<link rel="icon" type="image/png" sizes="192x192" href="/favicon-192.png" />
	<link rel="apple-touch-icon" href="/apple-touch-icon.png" />
	<meta name="description" content={data.site.description} />
</svelte:head>

<div
	class="pointer-events-none fixed inset-0 -z-10 bg-[radial-gradient(60rem_40rem_at_85%_-10%,color-mix(in_oklab,var(--fui-accent)_20%,transparent),transparent),radial-gradient(50rem_36rem_at_-10%_110%,color-mix(in_oklab,var(--fui-aurora)_16%,transparent),transparent)]"
></div>
<Header>
	<a
		href={resolve('/')}
		class="flex items-center gap-2 font-display text-sm font-semibold tracking-wide whitespace-nowrap text-ink"
	>
		<img src="/favicon-192.png" alt="" width="28" height="28" decoding="async" class="h-7 w-7" />
		{#if data.site.author}
			{data.site.author}
			<span class="text-ink-secondary max-[420px]:hidden">{data.site.name}</span>
		{:else}
			{data.site.name}
		{/if}
	</a>
	<nav aria-label="Site" class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm sm:gap-x-5">
		<a
			href={resolve('/')}
			aria-current={atHome ? 'page' : undefined}
			class={atHome ? 'font-medium text-ink' : 'text-ink-secondary hover:text-ink'}
		>
			Posts
		</a>
		<ThemeToggle />
	</nav>
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
