<script lang="ts">
	import { enhance } from '$app/forms';
	import { base } from '$app/paths';
	import {
		Badge,
		Button,
		Callout,
		Divider,
		Input,
		Link,
		Panel,
		Prose,
		Select,
		Spinner
	} from 'foundationui/svelte';
	import OutcomeBanner from '$lib/components/OutcomeBanner.svelte';
	import ConflictNotice from '$lib/components/ConflictNotice.svelte';
	import type { PreviewResult } from '$lib/api/types.js';
	import type { ActionData, PageData } from './$types';

	let { data, form }: { data: PageData; form: ActionData } = $props();

	const post = $derived(data.post);

	interface PreviewState {
		loading: boolean;
		result: PreviewResult | null;
		error: string | null;
	}

	const idlePreview: PreviewState = { loading: false, result: null, error: null };
	let previewState = $state<Record<string, PreviewState>>({});
	let sourceFields: Record<string, HTMLTextAreaElement> = {};

	function previewFor(language: string): PreviewState {
		return previewState[language] ?? idlePreview;
	}

	async function runPreview(language: string) {
		const textarea = sourceFields[language];
		if (!textarea) return;

		previewState[language] = { loading: true, result: null, error: null };
		try {
			const response = await fetch(`${base}/posts/${post.slug}/preview`, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ source: textarea.value })
			});
			if (!response.ok) {
				const body = (await response.json().catch(() => null)) as { message?: string } | null;
				previewState[language] = {
					loading: false,
					result: null,
					error: body?.message ?? `preview failed with status ${response.status}`
				};
				return;
			}
			const result = (await response.json()) as PreviewResult;
			previewState[language] = { loading: false, result, error: null };
		} catch {
			previewState[language] = { loading: false, result: null, error: 'preview request failed' };
		}
	}

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
		<Link href={base || '/'}>Back to posts</Link>
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
			{@const preview = previewFor(translation.language)}
			<Panel class="flex flex-col gap-3">
				<div class="flex items-center justify-between gap-3">
					<h3 class="font-semibold text-ink">{translation.language}</h3>
					<div class="flex items-center gap-3">
						<Badge tone={freshnessTone(freshnessFor(translation.language))}>
							{freshnessFor(translation.language)}
						</Badge>
						<Link href={`${base}/posts/${post.slug}/write/${translation.language}`}>Write</Link>
					</div>
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
							bind:this={sourceFields[translation.language]}
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

				<div class="flex flex-col gap-3">
					<Button
						type="button"
						variant="secondary"
						disabled={preview.loading}
						onclick={() => runPreview(translation.language)}
					>
						{#if preview.loading}
							<Spinner size="sm" /> Previewing…
						{:else}
							Preview {translation.language}
						{/if}
					</Button>

					{#if preview.error}
						<Callout tone="warn">
							<p class="text-sm">{preview.error}</p>
						</Callout>
					{:else if preview.result?.status === 'ok'}
						{@const result = preview.result}
						<details class="rounded border border-line">
							<summary class="cursor-pointer p-3 text-sm font-semibold text-ink">
								Preview output
							</summary>
							<div class="flex flex-col gap-3 border-t border-line p-3">
								<Prose html={result.html} />
								{#if result.warnings.length > 0}
									<Callout tone="warn" class="flex flex-col gap-1">
										{#each result.warnings as warning, index (index)}
											<p class="text-sm">{warning.severity}: {warning.message}</p>
										{/each}
									</Callout>
								{/if}
							</div>
						</details>
					{:else if preview.result?.status === 'failed'}
						{@const result = preview.result}
						<Callout tone="warn" class="flex flex-col gap-2">
							<p class="font-semibold text-ink">Preview failed</p>
							{#each result.diagnostics as diagnostic, index (index)}
								<div class="text-sm">
									<p>{diagnostic.severity}: {diagnostic.message}</p>
									{#if diagnostic.hints && diagnostic.hints.length > 0}
										<ul class="list-disc pl-5 text-ink-secondary">
											{#each diagnostic.hints as hint, hintIndex (hintIndex)}
												<li>{hint}</li>
											{/each}
										</ul>
									{/if}
								</div>
							{/each}
						</Callout>
					{/if}
				</div>
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
