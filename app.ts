import { render_hot } from 'gpuix-svelte';
import { startBackend } from "./lib/backend-process.ts";

await startBackend();

render_hot(new URL('./App.svelte', import.meta.url), {
	title: 'Journal Tree',
	width: 1280,
	height: 720,
	titlebarTransparent: true,
	trafficLightX: 16,
	trafficLightY: 19,
	windowBackground: "transparent",
	transparent: true
});

//GPUIX-svelte doesn't call exit handlers!!!!
