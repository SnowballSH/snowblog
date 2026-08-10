<script lang="ts">
	import { resolve } from '$app/paths';
	import { Badge, Link, Panel } from 'foundationui/svelte';
	import { formatDate } from '$lib/format.js';

	let { data } = $props();

	const postHref = (slug: string) => resolve('/posts/[slug]', { slug });
</script>

<svelte:head>
	<title>{data.site?.brand ?? 'Blogs'}</title>
</svelte:head>

<div class="mx-auto flex max-w-[74ch] flex-col gap-4">
	{#if data.page === 1}
		<section class="flex flex-col items-center gap-4 py-8 text-center sm:py-12">
			<img
				src="/mascot.webp"
				alt=""
				width="140"
				height="140"
				decoding="async"
				class="h-28 w-28 sm:h-35 sm:w-35"
			/>
			<h1 class="font-display text-4xl font-semibold tracking-tight text-balance sm:text-5xl">
				{data.site?.name ?? 'Blogs'}
			</h1>
			<p class="max-w-[46ch] text-lg text-pretty text-ink-secondary">
				{data.site?.description}
			</p>
		</section>
	{/if}

	{#each data.posts as post (post.id)}
		<a href={postHref(post.slug)} class="block no-underline">
			<Panel interactive>
				<article class="flex flex-col gap-2">
					<h2 class="font-display text-xl font-semibold text-ink">{post.title}</h2>
					<div class="flex flex-wrap items-center gap-2 text-sm text-ink-muted">
						{#if post.published_at}
							<time datetime={post.published_at}>{formatDate(post.published_at)}</time>
						{/if}
						{#each post.tags as tag (tag)}
							<Badge>{tag}</Badge>
						{/each}
					</div>
					{#if post.description}
						<p class="text-ink-secondary">{post.description}</p>
					{/if}
				</article>
			</Panel>
		</a>
	{:else}
		<p class="text-ink-secondary">Nothing published yet.</p>
	{/each}

	{#if data.page > 1 || data.hasNext}
		<nav aria-label="Pagination" class="flex items-center justify-between pt-2">
			<span>
				{#if data.page > 1}
					<Link href={`${resolve('/')}?page=${data.page - 1}`}>← Newer</Link>
				{/if}
			</span>
			<span class="text-sm text-ink-muted">Page {data.page}</span>
			<span>
				{#if data.hasNext}
					<Link href={`${resolve('/')}?page=${data.page + 1}`}>Older →</Link>
				{/if}
			</span>
		</nav>
	{/if}
</div>
