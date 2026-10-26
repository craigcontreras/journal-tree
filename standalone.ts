// The entry `npm run build` compiles: a binary carries no source to hot-reload, so it imports the
// component statically and mounts it with `render`.
import { render } from 'gpuix-svelte';
import App from './App.svelte';
import { startBackend } from './lib/backend-process.ts';

await startBackend();

render(App, {
	title: 'Journal Tree',
	width: 1280,
	height: 720,
	titlebarTransparent: true
});
