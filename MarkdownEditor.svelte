<script lang="ts">
	import type { GpuixEvent } from 'gpuix-svelte';

	type Props = {
		value: string;
		onchange?: (event: GpuixEvent & { value: string }) => void;
		placeholder?: string;
		readOnly?: boolean;
		markdown?: boolean;
		imageHeight?: number;
		testid?: string;
	};

	let { value, onchange, placeholder = 'Write Markdown…', readOnly = false, markdown = true, imageHeight = 180, testid = 'editor' }: Props = $props();
	
	const nativeOptions = $derived({ markdown, theme: { metrics: { mdHeadingSizes: [32, 26, 22, 19], mdHeadingLineHeights: [42, 36, 32, 29], mdEditorImageHeight: imageHeight } } });
</script>

<textarea
	{...nativeOptions}
	{value}
	{onchange}
	{placeholder}
	{readOnly}
	{testid}
	minRows={1}
	maxRows={1000000}
	aria-label="Markdown editor"
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
