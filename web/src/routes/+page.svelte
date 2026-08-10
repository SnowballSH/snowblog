<script lang="ts">
	import { resolve } from '$app/paths';
	import { Badge, Link, Panel } from 'foundationui/svelte';
	import { formatDate } from '$lib/format.js';

	let { data } = $props();

	const postHref = (slug: string) => `/posts/${encodeURIComponent(slug)}`;
</script>

<svelte:head>
	<title>{data.site?.name ?? 'snowblog'}</title>
</svelte:head>

<div class="mx-auto flex max-w-[74ch] flex-col gap-4">
	{#each data.posts as post (post.id)}
		<Panel>
			<article class="flex flex-col gap-2">
				<h2 class="font-display text-xl font-semibold">
					<Link href={postHref(post.slug)} subtle class="no-underline">{post.title}</Link>
				</h2>
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
