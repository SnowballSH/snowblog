<script lang="ts">
	import { enhance } from '$app/forms';
	import { base } from '$app/paths';
	import type { SubmitFunction } from '@sveltejs/kit';
	import { Badge, Button, Callout, Input, Link, Prose, Spinner } from 'foundationui/svelte';
	import OutcomeBanner from '$lib/components/OutcomeBanner.svelte';
	import ConflictNotice from '$lib/components/ConflictNotice.svelte';
	import TypstEditor from '$lib/editor/TypstEditor.svelte';
	import { debounce } from '$lib/editor/debounce.js';
	import { createPreviewController } from '$lib/editor/preview-controller.js';
	import type { PreviewResult } from '$lib/api/types.js';
	import type { ActionData, PageData } from './$types';

	let { data, form }: { data: PageData; form: ActionData } = $props();

	const post = $derived(data.post);
	const language = $derived(data.translation.language);

	let revision = $derived(data.post.revision);
	let title = $derived(data.translation.title);
	let description = $derived(data.translation.description);
	let source = $derived(data.translation.source);
	let saving = $state(false);
	let formEl: HTMLFormElement | undefined;

	let previewLoading = $state(false);
	let previewResult = $state<PreviewResult | null>(null);
	let previewError = $state<string | null>(null);

	type PreviewOutcome = { kind: 'ok'; result: PreviewResult } | { kind: 'error'; message: string };

	async function runPreviewFetch(text: string, signal: AbortSignal): Promise<PreviewOutcome> {
		previewLoading = true;
		try {
			const response = await fetch(`${base}/posts/${post.slug}/preview`, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ source: text }),
				signal
			});
			if (!response.ok) {
				const body = (await response.json().catch(() => null)) as { message?: string } | null;
				return {
					kind: 'error',
					message: body?.message ?? `preview failed with status ${response.status}`
				};
			}
			return { kind: 'ok', result: (await response.json()) as PreviewResult };
		} catch {
			return { kind: 'error', message: 'preview request failed' };
		} finally {
			previewLoading = false;
		}
	}

	function applyPreviewOutcome(outcome: PreviewOutcome): void {
		if (outcome.kind === 'ok') {
			previewResult = outcome.result;
			previewError = null;
		} else {
			previewError = outcome.message;
			previewResult = null;
		}
	}

	const previewController = createPreviewController(runPreviewFetch);
	const scheduleAutoPreview = debounce(
		(text: string) => previewController.request(text, applyPreviewOutcome),
		900
	);

	let previewedPairKey: string | null = null;
	$effect(() => {
		const pairKey = `${post.slug}::${language}`;
		const current = source;
		if (pairKey !== previewedPairKey) {
			previewedPairKey = pairKey;
			scheduleAutoPreview.cancel();
			previewController.reset();
			previewResult = null;
			previewError = null;
			previewLoading = false;
			previewController.request(current, applyPreviewOutcome);
			return;
		}
		scheduleAutoPreview(current);
	});

	$effect(() => () => {
		scheduleAutoPreview.cancel();
		previewController.dispose();
	});

	const saveSubmit: SubmitFunction = () => {
		saving = true;
		return async ({ result, update }) => {
			saving = false;
			if (result.type === 'success' && result.data && typeof result.data.revision === 'number') {
				revision = result.data.revision;
			}
			await update({ invalidateAll: false });
		};
	};

	function handleKeydown(event: KeyboardEvent): void {
		if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== 's') return;
		event.preventDefault();
		formEl?.requestSubmit();
	}
</script>

<svelte:head>
	<title>write · {post.slug}/{language} · snowblog admin</title>
</svelte:head>

<svelte:window onkeydown={handleKeydown} />

<form
	bind:this={formEl}
	method="POST"
	action="?/save"
	use:enhance={saveSubmit}
	class="flex min-h-0 flex-1 flex-col"
>
	<input type="hidden" name="revision" value={revision} />
	<input type="hidden" name="source" value={source} />

	<div class="flex flex-wrap items-center justify-between gap-3 border-b border-line px-4 py-2">
		<div class="flex items-center gap-3">
			<Link href={`${base}/posts/${post.slug}`}>Back</Link>
			<span class="font-display text-sm font-semibold text-ink">{post.slug}</span>
			<Badge tone="neutral">{language}</Badge>
			<span class="text-xs text-ink-muted">revision {revision}</span>
		</div>
		<div class="flex items-center gap-3">
			{#if saving}
				<span class="flex items-center gap-2 text-sm text-ink-muted">
					<Spinner size="sm" /> saving…
				</span>
			{/if}
			<Button type="submit" disabled={saving}>Save</Button>
		</div>
	</div>

	{#if form?.conflict}
		<div class="border-b border-line px-4 py-2">
			<ConflictNotice slug={post.slug} message={form.message} />
		</div>
	{:else if form}
		<div class="border-b border-line px-4 py-2">
			<OutcomeBanner result={form} />
		</div>
	{/if}

	<details class="border-b border-line px-4 py-2">
		<summary class="cursor-pointer text-sm font-semibold text-ink-secondary">
			title &amp; description
		</summary>
		<div class="flex flex-col gap-2 pt-2 sm:flex-row">
			<label class="flex flex-1 flex-col gap-1 text-sm text-ink-secondary">
				title
				<Input name="title" bind:value={title} required minlength={1} />
			</label>
			<label class="flex flex-1 flex-col gap-1 text-sm text-ink-secondary">
				description
				<Input name="description" bind:value={description} />
			</label>
		</div>
	</details>

	<div class="grid min-h-0 flex-1 grid-cols-1 grid-rows-2 md:grid-cols-2 md:grid-rows-1">
		<div class="flex min-h-0 flex-col border-b border-line md:border-r md:border-b-0">
			<div class="min-h-0 flex-1">
				<TypstEditor bind:value={source} placeholder="Start writing…" />
			</div>
		</div>

		<div class="flex min-h-0 flex-col">
			<div class="flex items-center gap-2 border-b border-line px-4 py-1.5">
				<span class="text-xs font-semibold text-ink-secondary">Preview</span>
				{#if previewLoading}
					<Spinner size="sm" />
					<span class="text-xs text-ink-muted">compiling…</span>
				{/if}
			</div>
			<div class="min-h-0 flex-1 overflow-y-auto p-4">
				{#if previewError}
					<Callout tone="warn">
						<p class="text-sm">{previewError}</p>
					</Callout>
				{:else if previewResult?.status === 'ok'}
					{@const result = previewResult}
					<div class="flex flex-col gap-3">
						<Prose html={result.html} />
						{#if result.warnings.length > 0}
							<Callout tone="warn" class="flex flex-col gap-1">
								{#each result.warnings as warning, index (index)}
									<p class="text-sm">{warning.severity}: {warning.message}</p>
								{/each}
							</Callout>
						{/if}
					</div>
				{:else if previewResult?.status === 'failed'}
					{@const result = previewResult}
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
				{:else}
					<p class="text-sm text-ink-muted">Preview will appear here.</p>
				{/if}
			</div>
		</div>
	</div>
</form>
