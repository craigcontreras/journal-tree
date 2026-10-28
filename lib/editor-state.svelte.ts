// Keep the note through Svelte hot reloads. Disk persistence is a separate step.
export const editorState = $state({ source: '' });
