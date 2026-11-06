<script lang="ts">
	import type { GpuixEvent } from 'gpuix-svelte';
	import { untrack } from 'svelte';

	type Props = {
		value?: string;
		onchange?: (event: GpuixEvent & { value: string }) => void;
		placeholder?: string;
		readOnly?: boolean;
		markdown?: boolean;
		largeFile?: boolean;
		fileBufferId?: number;
		scrollbar?: boolean;
		startAtTop?: boolean;
		imageHeight?: number;
		testid?: string;
	};

	let { value = '', onchange, placeholder = 'Write Markdown…', readOnly = false, markdown = true, largeFile = false, fileBufferId, scrollbar = false, startAtTop = false, imageHeight = 180, testid = 'editor' }: Props = $props();

	const nativeOptions = $derived({ markdown, largeFile, fileBufferId, scrollbar, startAtTop, theme: { metrics: { mdHeadingSizes: [32, 26, 22, 19], mdHeadingLineHeights: [42, 36, 32, 29], mdEditorImageHeight: imageHeight } } });
	// Keep the native rope authoritative while this editor is mounted.
	let source = untrack(() => value);
	let nativeValue = $state(source);
	let wasLargeFile = false;

	$effect.pre(() => {
		if (largeFile && (!wasLargeFile || value !== source)) {
			source = value;
			nativeValue = value;
		}
		wasLargeFile = largeFile;
	});

	function handleChange(event: GpuixEvent & { value: string }) {
		if (typeof event.startIndex === 'number' && typeof event.endIndex === 'number') {
			source = source.slice(0, event.startIndex) + event.value + source.slice(event.endIndex);
			event.value = source;
		} else {
			source = event.value;
		}
		onchange?.(event);
	}
</script>

<textarea
	{...nativeOptions}
	value={largeFile ? nativeValue : value}
	onchange={largeFile && onchange ? handleChange : onchange}
	{placeholder}
	{readOnly}
	{testid}
	minRows={1}
	maxRows={1000000}
	class="markdown-editor"
></textarea>

<style>
	.markdown-editor {
		width: 100%;
		height: 100%;
		min-width: 0;
		min-height: 0;
		font-size: 16px;
		line-height: 26px;
		color: var(--text);
		background-color: var(--panel);
	}
</style>
