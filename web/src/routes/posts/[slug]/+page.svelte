<script lang="ts">
	import { Badge, Divider, Link, Prose } from 'foundationui/svelte';
	import { formatDate } from '$lib/format.js';
	import { languageLabel } from '$lib/language.js';

	let { data } = $props();

	const post = $derived(data.post);
	const ogImage = $derived(new URL('/mascot.webp', data.canonical).href);
	const switcher = $derived(
		post.languages.length > 1
			? data.alternates.map((alt) => ({ ...alt, current: alt.lang === post.language }))
			: []
	);
</script>

<svelte:head>
	<title>{post.title} — {data.site?.brand ?? 'Blogs'}</title>
	{#if post.description}
		<meta name="description" content={post.description} />
		<meta property="og:description" content={post.description} />
	{/if}
	<meta property="og:title" content={post.title} />
	<meta property="og:type" content="article" />
	<meta property="og:url" content={data.canonical} />
	<meta property="og:image" content={ogImage} />
	<link rel="canonical" href={data.canonical} />
	{#each data.alternates as alt (alt.lang)}
		<link rel="alternate" hreflang={alt.lang} href={alt.href} />
	{/each}
</svelte:head>

<article lang={post.language} class="mx-auto max-w-[74ch]">
	<header class="flex flex-col gap-3">
		<h1 class="font-display text-3xl leading-tight font-semibold text-balance">{post.title}</h1>
		<div class="flex flex-wrap items-center gap-2 text-sm text-ink-muted">
			{#if post.published_at}
				<time datetime={post.published_at}>{formatDate(post.published_at)}</time>
			{/if}
			{#each post.tags as tag (tag)}
				<Badge>{tag}</Badge>
			{/each}
		</div>
		{#if switcher.length > 0}
			<nav aria-label="Translations" class="flex flex-wrap items-center gap-3 text-sm">
				{#each switcher as alt (alt.lang)}
					{#if alt.current}
						<span lang={alt.lang} class="font-medium text-ink">{languageLabel(alt.lang)}</span>
					{:else}
						<Link href={alt.href} subtle lang={alt.lang}>{languageLabel(alt.lang)}</Link>
					{/if}
				{/each}
			</nav>
		{/if}
	</header>
	<Divider class="my-6" />
	<Prose html={post.html} class="mx-auto" />
</article>
