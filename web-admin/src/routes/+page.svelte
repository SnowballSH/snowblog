<script lang="ts">
	import { enhance } from '$app/forms';
	import { base } from '$app/paths';
	import { Badge, Button, Input, Link, Panel, Select } from 'foundationui/svelte';
	import OutcomeBanner from '$lib/components/OutcomeBanner.svelte';
	import type { ActionData, PageData } from './$types';

	let { data, form }: { data: PageData; form: ActionData } = $props();

	function formatUpdated(timestamp: string): string {
		return timestamp.slice(0, 16).replace('T', ' ');
	}

	function languages(post: PageData['posts'][number]): string {
		const codes = post.translations.map((translation) => translation.language);
		return codes.length > 0 ? codes.join(', ') : 'none';
	}
</script>

<svelte:head>
	<title>snowblog admin</title>
</svelte:head>

<div class="flex flex-col gap-6">
	<OutcomeBanner result={form} />

	<Panel tier="raised" class="flex flex-col gap-3">
		<h2 class="font-display text-lg font-semibold text-ink">New post</h2>
		<form method="POST" action="?/create" use:enhance class="flex flex-wrap items-end gap-3">
			<label class="flex flex-col gap-1 text-sm text-ink-secondary">
				slug
				<Input name="slug" required minlength={1} />
			</label>
			<label class="flex flex-col gap-1 text-sm text-ink-secondary">
				default language
				<Select name="default_language" required>
					<option value="en">en</option>
					<option value="zh">zh</option>
				</Select>
			</label>
			<Button type="submit">Create</Button>
		</form>
	</Panel>

	<div class="flex flex-col gap-3">
		{#each data.posts as post (post.id)}
			<Panel class="flex flex-col gap-2">
				<div class="flex flex-wrap items-center justify-between gap-2">
					<Link href={`${base}/posts/${post.slug}`}>{post.slug}</Link>
					<Badge tone={post.status === 'published' ? 'aurora' : 'neutral'}>{post.status}</Badge>
				</div>
				<div class="flex flex-wrap gap-3 text-sm text-ink-muted">
					<span>revision {post.revision}</span>
					<span>{languages(post)}</span>
					<time datetime={post.updated_at}>{formatUpdated(post.updated_at)}</time>
				</div>
			</Panel>
		{:else}
			<p class="text-ink-secondary">No posts yet.</p>
		{/each}
	</div>
</div>
