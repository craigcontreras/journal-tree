<script lang="ts">
    import { backendState } from "./lib/backend-process.ts";
	import axios from "axios";
	import { onMount } from "svelte";
	import { set_css_vars } from "gpuix-svelte";
	import MarkdownEditor from './MarkdownEditor.svelte';
	import { editorState } from './lib/editor-state.svelte.ts';

	set_css_vars({ surface: '#11111b', panel: '#1e1e2e', divider: '#1e1e2e', text: '#ffffff', 'editor-inset': '32px' });

	let count = $state(0);
	let url = backendState.baseUrl;
	let response = $state(null);

	// onMount(async () => {
	// 	try {
	// 		const sentence = "The unanimous Declaration of the thirteen united States of America, When in the Course of human events, it becomes necessary for one people to dissolve the political bands which have connected them with another, and to assume among the powers of the earth, the separate and equal station to which the Laws of Nature and of Nature's God entitle them, a decent respect to the opinions of mankind requires that they should declare the causes which impel them to the separation.";
	// 		const words = sentence.match(/\b\w+\b/g);

	// 		response = await axios.post(url + "/topics", {
	// 			documents: words
	// 		});
	// 	} catch (err) {
	// 		console.error(err);
	// 	}
	// });
</script>

<div class="app">
	<div class="menubar-row" testid="menubar-row">
		<div class="side-menubar"></div>
		<div class="sidebar-spacer"></div>
		<div class="menubar" testid="tab-strip">
			<div class="tab">
				<div class="tab-curve left" testid="tab-left-curve"><div class="tab-cutout left"></div></div>
				<text>file</text>
				<div class="tab-curve right"><div class="tab-cutout right"></div></div>
			</div>
		</div>
		<div class="sidebar-spacer"></div>
	</div>
	<div class="workspace" testid="workspace">
		<div class="side-menubar"></div>
		<div class="sidebar" testid="left-sidebar"></div>
		<div class="container" testid="editor-column">
			<div class="card">
				<MarkdownEditor value={editorState.source} onchange={(e) => editorState.source = e.value} />
			</div>
		</div>
		<div class="sidebar"></div>
	</div>
</div>

<style>
	.side-menubar {
		width: 4rem;
		flex-shrink: 0;
	}

	.sidebar-spacer {
		width: 15%;
		flex-shrink: 0;
	}

	.sidebar {
		border-left-width: 2px;
		border-right-width: 2px;
		border-color: var(--divider);

		width: 15%;
		height: 100%;
		flex-shrink: 0;
	}

	.app {
		display: flex;
		flex-direction: column;
		width: 100%;
		height: 100%;
		background-color: var(--surface);
	}

	.menubar-row {
		display: flex;
		width: 100%;
		height: 54px;
		flex-shrink: 0;
		border-bottom-width: 2px;
		border-color: var(--divider);
	}

	.workspace {
		display: flex;
		width: 100%;
		flex-grow: 1;
		min-height: 0;
	}

	.container {
		display: flex;
		flex-direction: column;
		height: 100%;
		flex-grow: 1;
		min-width: 0;
	}

	.menubar {
		display: flex;
		align-items: flex-end;
		padding-left: 8px;
		overflow: hidden;
		flex-grow: 1;
		min-width: 0;
	}

	.tab {
		position: relative;
		margin-bottom: -2px;
		padding: 0.5rem 1.25rem;
		color: var(--text);
		background-color: var(--panel);
		border-top-left-radius: 12px;
		border-top-right-radius: 12px;
	}

	.tab-curve {
		position: absolute;
		bottom: 0;
		width: 24px;
		height: 24px;
		background-color: var(--panel);
		pointer-events: none;
	}

	.tab-curve.left { left: -24px; }
	.tab-curve.right { right: -24px; }

	.tab-cutout {
		width: 100%;
		height: 100%;
		background-color: var(--surface);
	}

	.tab-cutout.left { border-bottom-right-radius: 24px; }
	.tab-cutout.right { border-bottom-left-radius: 24px; }

	.card {
		width: 100%;
		height: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 16px;
		padding: var(--editor-inset);
		background-color: var(--panel);
	}
</style>
