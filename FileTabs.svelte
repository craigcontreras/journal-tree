<script lang="ts">
	let {
		files,
		activePath,
		onSelect,
		onClose
	}: {
		files: readonly { path: string; filename: string }[];
		activePath: string | null;
		onSelect: (path: string) => void;
		onClose: (index: number) => void;
	} = $props();

	const leftCurve =
		'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M0 12A12 12 0 0 0 12 0V12Z"/></svg>';
	const rightCurve =
		'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path d="M0 0A12 12 0 0 0 12 12H0Z"/></svg>';
</script>

<div class="tab-strip" testid="tab-strip" role="tablist">
	{#each files as file, index (file.path)}
		{@const active = file.path === activePath}
		{@const prevActive = index > 0 && files[index - 1].path === activePath}
		<div class="tab" class:active>
			{let showCloseButton = $state(false)}
			<div
				onmouseenter={() => {
					console.log("mouse entered tab");
					showCloseButton = true;
				}} onmouseleave={() => {
					showCloseButton = false;
				}} 
				class="tab-label"
				class:active
				testid={`file-tab-${index}`}
				role="tab"
				aria-label={file.filename}
				aria-selected={active}
				tabindex={0}
				onclick={() => onSelect(file.path)}
				onkeydown={(event) => {
					if (event.key === "enter" || event.key === "space") {
						event.preventDefault();
						onSelect(file.path);
					}
				}}>
				<text class="filename">{file.filename}</text>
				<div class="close-btn" class:show-close-button={showCloseButton} class:active onclick={(event) => {
					event.stopPropagation();
					onClose(index);
                }}>
					<text>x</text>
				</div>
			</div>
			{#if active && index !== 0}
				<div class="curve left">
					<svg class="tab-curve" source={leftCurve} />
				</div>
			{/if}
			{#if prevActive}
                <div class="curve prev-right">
                    <svg class="tab-curve" source={rightCurve} />
                </div>
            {/if}
			{#if active}
				<div class="curve right">
					<svg class="tab-curve" source={rightCurve} />
				</div>
			{/if}
		</div>
	{/each}
</div>

<style>
	.tab-strip {
		display: flex;
		align-items: flex-end;
		overflow: hidden;
		flex-grow: 1;
		min-width: 0;
	}

	.tab {
		position: relative;
		display: flex;
		align-items: flex-end;
		min-width: 50px;
		max-width: 250px;
	}

	.tab.active {
		margin-bottom: -2px;
	}

	.tab-label {
		display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 8px;
        
        pointer-events: auto;
        cursor: pointer;
        min-width: 0;
        flex-grow: 1;
        padding: 0.5rem 1rem;

		color: var(--tab-text-muted);
		background-color: var(--tab-inactive);
		border-top-left-radius: 12px;
		border-top-right-radius: 12px;
	}

	.tab-label:hover {
		background-color: rgba(255, 255, 255, 0.1);
	}
	
	.tab-label.active:hover {
		background-color: var(--panel);
	}

	.filename {
		white-space: nowrap;
        text-overflow: ellipsis;
        overflow: hidden;
        min-width: 0;
	}

	.tab-label.active {
		padding-bottom: 10px;
		color: var(--text);
		background-color: var(--panel);
	}

	.curve {
		position: absolute;
        width: 12px;
        height: 12px;
        pointer-events: none;
    }
	
	.curve.left {
		left: -12px;
	}

	.curve.right {
		right: -12px;
	}

	.curve.prev-right {
		margin-bottom: -2px;
	}

	.tab-curve {
		width: 12px;
		height: 12px;
		flex-shrink: 0;
		color: var(--panel);
		pointer-events: none;
	}

	.close-btn {
		visibility: hidden;
        display: flex;
        align-items: center;
        justify-content: center;
        flex-shrink: 0;
        border-radius: 4px;
        padding: 2px 6px;
    }

	.close-btn.active {
		visibility: visible;
	}

	.show-close-button {
		visibility: visible;
	}

	.close-btn:hover {
		background-color: rgba(255, 255, 255, 0.1);
	}
</style>
