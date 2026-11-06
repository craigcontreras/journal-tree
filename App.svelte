<script lang="ts">
    import { backendState } from "./lib/backend-process.ts";
	import axios from "axios";
	import { onMount } from "svelte";
	import { on_window_key, prompt_for_paths, set_css_vars } from "gpuix-svelte";
	import MarkdownEditor from './MarkdownEditor.svelte';
	import FileTabs from './FileTabs.svelte';
	import { editorState } from './lib/editor-state.svelte.ts';
	import { fileState, openFiles, selectFile } from './lib/file-state.svelte.ts';
	import { appState } from "./state.svelte.ts";

	set_css_vars({ surface: '#11111b', panel: '#1e1e2e', divider: '#1e1e2e', text: '#ffffff', 'editor-inset': '32px', 'tab-inactive': '#181825', 'tab-text-muted': '#a6adc8' });

	let count = $state(0);
	let url = backendState.baseUrl;
	let response = $state(null);

	const activeNote = $derived(fileState.files.find(file => file.path === fileState.activePath) ?? editorState);

	onMount(() => {
		let opening = false;
		return on_window_key('keydown', async (e) => {
			if (opening || !e.modifiers?.cmd || e.key !== 'o') return;
			opening = true;
			try {
				const paths = await prompt_for_paths({
					files: true,
					directories: false,
					multiple: true,
					prompt: "Select"
				});
				if (paths) await openFiles(paths);
			} catch (error) {
				console.error('Could not open files:', error);
			} finally {
				opening = false;
			}
		});
	});

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
	<div class="menubar-row" testid="menubar-row" hitbox="self">
		<div class="side-menubar"></div>
		<div class="sidebar-spacer"></div>
		<FileTabs files={fileState.files} activePath={fileState.activePath} onSelect={selectFile} />
		<div class="sidebar-spacer"></div>
	</div>
	<div class="workspace" testid="workspace">
		<div class="side-menubar"></div>
		<div class="sidebar" testid="left-sidebar"></div>
		<div class="container" testid="editor-column">
			<div class="card">
				{#each [activeNote] as note (note)}
					{#if 'document' in note}
						<MarkdownEditor fileBufferId={note.document.id} scrollbar />
					{:else}
						<MarkdownEditor value={note.source} scrollbar startAtTop={'path' in note} onchange={(e) => note.source = e.value} />
					{/if}
				{/each}
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
		window-drag: move;
		user-select: none;
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

	.card {
		width: 100%;
		height: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 16px;
		padding-top: var(--editor-inset);
		padding-left: var(--editor-inset);
		background-color: var(--panel);
	}

</style>
