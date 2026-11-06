<script lang="ts">
	let { files, activePath, onSelect }: {
		files: readonly { path: string; filename: string }[];
		activePath: string | null;
		onSelect: (path: string) => void;
	} = $props();

	const leftCurve = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M0 12A12 12 0 0 0 12 0V12Z"/></svg>';
	const rightCurve = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M0 0A12 12 0 0 0 12 12H0Z"/></svg>';
</script>

<div class="tab-strip" testid="tab-strip" role="tablist">
	{#each files as file, index (file.path)}
		{@const active = file.path === activePath}
		<div class="tab" class:active>
			{#if active}
				<svg class="tab-curve" source={leftCurve} />
			{/if}
			<text
				class="tab-label"
				class:active
				testid={`file-tab-${index}`}
				role="tab"
				aria-label={file.filename}
				aria-selected={active}
				tabindex={0}
				onclick={() => onSelect(file.path)}
				onkeydown={(event) => {
					if (event.key === 'enter' || event.key === 'space') {
						event.preventDefault();
						onSelect(file.path);
					}
				}}
			>{file.filename}</text>
			{#if active}
				<svg class="tab-curve" source={rightCurve} />
			{/if}
		</div>
	{/each}
</div>

<style>
	.tab-strip {
		display: flex;
		align-items: flex-end;
		padding-left: 8px;
		gap: 4px;
		overflow: hidden;
		flex-grow: 1;
		min-width: 0;
	}

	.tab {
		display: flex;
		align-items: flex-end;
		min-width: 0;
		max-width: 240px;
		overflow: hidden;
	}

	.tab.active {
		margin-bottom: -2px;
		flex-shrink: 0.15;
	}

	.tab-label {
		pointer-events: auto;
		cursor: pointer;
		min-width: 0;
		padding: 0.5rem 1rem;
		white-space: nowrap;
		text-overflow: ellipsis;
		overflow: hidden;
		color: var(--tab-text-muted);
		background-color: var(--tab-inactive);
		border-top-left-radius: 12px;
		border-top-right-radius: 12px;
	}

	.tab-label.active {
		padding-bottom: 10px;
		color: var(--text);
		background-color: var(--panel);
	}

	.tab-curve {
		width: 12px;
		height: 12px;
		flex-shrink: 0;
		color: var(--panel);
		pointer-events: none;
	}
</style>
