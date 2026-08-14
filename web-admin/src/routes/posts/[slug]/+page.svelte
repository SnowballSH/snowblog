<script lang="ts">
	import { enhance } from '$app/forms';
	import { Badge, Button, Callout, Divider, Input, Link, Panel, Select } from 'foundationui/svelte';
	import OutcomeBanner from '$lib/components/OutcomeBanner.svelte';
	import ConflictNotice from '$lib/components/ConflictNotice.svelte';
	import type { ActionData, PageData } from './$types';

	let { data, form }: { data: PageData; form: ActionData } = $props();

	const post = $derived(data.post);

	function freshnessTone(
		freshness: 'fresh' | 'stale' | 'missing'
	): 'aurora' | 'accent' | 'neutral' {
		if (freshness === 'fresh') return 'aurora';
		if (freshness === 'stale') return 'accent';
		return 'neutral';
	}

	function freshnessFor(language: string): 'fresh' | 'stale' | 'missing' {
		return post.freshness.find((entry) => entry.language === language)?.freshness ?? 'missing';
	}

	const warnings = $derived(
		post.renders.flatMap((render) =>
			render.warnings.map((warning) => ({ language: render.language, ...warning }))
		)
	);

	function confirmSubmit(message: string) {
		return ({ cancel }: { cancel: () => void }) => {
			if (!confirm(message)) cancel();
		};
	}

	function shortHash(hash: string): string {
		return hash.length > 12 ? `${hash.slice(0, 12)}…` : hash;
	}
</script>

<svelte:head>
	<title>{post.slug} · snowblog admin</title>
</svelte:head>

<div class="flex flex-col gap-6">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div class="flex items-center gap-3">
			<h1 class="font-display text-xl font-semibold text-ink">{post.slug}</h1>
			<Badge tone={post.status === 'published' ? 'aurora' : 'neutral'}>{post.status}</Badge>
			<span class="text-sm text-ink-muted">revision {post.revision}</span>
		</div>
		<Link href="/">Back to posts</Link>
	</div>

	<OutcomeBanner result={form} />
	{#if form?.conflict}
		<ConflictNotice slug={post.slug} message={form.message} />
	{/if}

	{#if warnings.length > 0}
		<Callout tone="warn" class="flex flex-col gap-1">
			<p class="font-semibold text-ink">Render warnings</p>
			{#each warnings as warning, index (index)}
				<p class="text-sm">{warning.language}: {warning.message}</p>
			{/each}
		</Callout>
	{/if}

	<Panel tier="raised" class="flex flex-col gap-3">
		<h2 class="font-display text-lg font-semibold text-ink">Metadata</h2>
		<form method="POST" action="?/saveMeta" use:enhance class="flex flex-col gap-3">
			<input type="hidden" name="revision" value={post.revision} />
			<label class="flex flex-col gap-1 text-sm text-ink-secondary">
				slug
				<Input name="slug" value={post.slug} required minlength={1} />
			</label>
			<label class="flex flex-col gap-1 text-sm text-ink-secondary">
				default language
				<Select name="default_language" value={post.default_language} required>
					<option value="en">en</option>
					<option value="zh">zh</option>
				</Select>
			</label>
			<label class="flex flex-col gap-1 text-sm text-ink-secondary">
				tags (comma separated)
				<Input name="tags" value={post.tags.join(', ')} />
			</label>
			<Button type="submit">Save metadata</Button>
		</form>
	</Panel>

	<Panel tier="raised" class="flex flex-col gap-3">
		<h2 class="font-display text-lg font-semibold text-ink">Actions</h2>
		<div class="flex flex-wrap gap-3">
			<form method="POST" action="?/publish" use:enhance>
				<input type="hidden" name="revision" value={post.revision} />
				<Button type="submit" disabled={post.status === 'published'}>Publish</Button>
			</form>
			<form method="POST" action="?/unpublish" use:enhance>
				<input type="hidden" name="revision" value={post.revision} />
				<Button type="submit" variant="secondary" disabled={post.status !== 'published'}>
					Unpublish
				</Button>
			</form>
			<form method="POST" action="?/archive" use:enhance={confirmSubmit('Archive this post?')}>
				<input type="hidden" name="revision" value={post.revision} />
				<Button type="submit" variant="secondary" disabled={post.status === 'archived'}>
					Archive
				</Button>
			</form>
			<form
				method="POST"
				action="?/deletePost"
				use:enhance={confirmSubmit('Delete this post? This cannot be undone.')}
			>
				<input type="hidden" name="revision" value={post.revision} />
				<Button type="submit" variant="ghost">Delete</Button>
			</form>
		</div>
	</Panel>

	<Divider />

	<div class="flex flex-col gap-4">
		<h2 class="font-display text-lg font-semibold text-ink">Translations</h2>
		{#each post.translations as translation (translation.language)}
			<Panel class="flex flex-col gap-3">
				<div class="flex items-center justify-between gap-3">
					<h3 class="font-semibold text-ink">{translation.language}</h3>
					<Badge tone={freshnessTone(freshnessFor(translation.language))}>
						{freshnessFor(translation.language)}
					</Badge>
				</div>
				<form method="POST" action="?/saveTranslation" use:enhance class="flex flex-col gap-3">
					<input type="hidden" name="revision" value={post.revision} />
					<input type="hidden" name="language" value={translation.language} />
					<label class="flex flex-col gap-1 text-sm text-ink-secondary">
						title
						<Input name="title" value={translation.title} required minlength={1} />
					</label>
					<label class="flex flex-col gap-1 text-sm text-ink-secondary">
						description
						<Input name="description" value={translation.description} />
					</label>
					<label class="flex flex-col gap-1 text-sm text-ink-secondary">
						source
						<textarea
							name="source"
							class="w-full rounded border border-line bg-surface p-3 font-mono text-sm"
							rows="20">{translation.source}</textarea
						>
					</label>
					<Button type="submit">Save {translation.language}</Button>
				</form>
				<form
					method="POST"
					action="?/deleteTranslation"
					use:enhance={confirmSubmit(`Delete the ${translation.language} translation?`)}
				>
					<input type="hidden" name="revision" value={post.revision} />
					<input type="hidden" name="language" value={translation.language} />
					<Button type="submit" variant="ghost">Delete {translation.language}</Button>
				</form>
			</Panel>
		{/each}

		<Panel tier="flat" class="flex flex-col gap-3">
			<h3 class="font-semibold text-ink">Add translation</h3>
			<form method="POST" action="?/saveTranslation" use:enhance class="flex flex-col gap-3">
				<input type="hidden" name="revision" value={post.revision} />
				<label class="flex flex-col gap-1 text-sm text-ink-secondary">
					language
					<Input name="language" required minlength={2} maxlength={8} placeholder="en" />
				</label>
				<label class="flex flex-col gap-1 text-sm text-ink-secondary">
					title
					<Input name="title" required minlength={1} />
				</label>
				<label class="flex flex-col gap-1 text-sm text-ink-secondary">
					description
					<Input name="description" />
				</label>
				<label class="flex flex-col gap-1 text-sm text-ink-secondary">
					source
					<textarea
						name="source"
						class="w-full rounded border border-line bg-surface p-3 font-mono text-sm"
						rows="20"></textarea>
				</label>
				<Button type="submit">Add translation</Button>
			</form>
		</Panel>
	</div>

	<Divider />

	<div class="flex flex-col gap-4">
		<h2 class="font-display text-lg font-semibold text-ink">Assets</h2>
		{#if post.assets.length > 0}
			<div class="overflow-x-auto">
				<table class="w-full text-left text-sm">
					<thead>
						<tr class="text-ink-secondary">
							<th class="pr-4 pb-2">path</th>
							<th class="pr-4 pb-2">content type</th>
							<th class="pr-4 pb-2">hash</th>
							<th class="pb-2"></th>
						</tr>
					</thead>
					<tbody>
						{#each post.assets as asset (asset.path)}
							<tr class="border-t border-line">
								<td class="py-2 pr-4">{asset.path}</td>
								<td class="py-2 pr-4">{asset.content_type}</td>
								<td class="py-2 pr-4 font-mono text-xs">{shortHash(asset.content_hash)}</td>
								<td class="py-2">
									<form method="POST" action="?/deleteAsset" use:enhance>
										<input type="hidden" name="revision" value={post.revision} />
										<input type="hidden" name="path" value={asset.path} />
										<Button type="submit" variant="ghost" size="sm">Delete</Button>
									</form>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{:else}
			<p class="text-ink-secondary">No assets yet.</p>
		{/if}

		<Panel tier="flat" class="flex flex-col gap-3">
			<h3 class="font-semibold text-ink">Upload asset</h3>
			<form
				method="POST"
				action="?/uploadAsset"
				enctype="multipart/form-data"
				use:enhance
				class="flex flex-wrap items-end gap-3"
			>
				<input type="hidden" name="revision" value={post.revision} />
				<label class="flex flex-col gap-1 text-sm text-ink-secondary">
					path (optional, defaults to file name)
					<Input name="path" />
				</label>
				<label class="flex flex-col gap-1 text-sm text-ink-secondary">
					file
					<input type="file" name="file" required />
				</label>
				<Button type="submit">Upload</Button>
			</form>
		</Panel>
	</div>
</div>
