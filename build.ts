/// <reference types="bun" />
// `bun build.ts` (npm run build): compiles standalone.ts into one executable for the platform it runs on.

import type { BunPlugin } from 'bun';
import { load_module, load_svelte, resolve_svelte } from 'gpuix-svelte/plugin';
import { join } from 'node:path';
import { assertNativeBuild, buildRoot } from './native/config.ts';

assertNativeBuild();

const conditions = ['custom-renderer', 'production'];
const outfile = process.platform === 'win32' ? 'dist/app.exe' : 'dist/app';

// A `.svelte` import that misses the plugin silently becomes a file asset, so count what went through it.
let components = 0;
const plugin: BunPlugin = {
	name: 'gpuix-svelte',
	setup(build) {
		// A literal .node import lets Bun embed our patched addon in the binary.
		build.onResolve({ filter: /^@gpuix\/native$/ }, () => ({ path: join(buildRoot, 'standalone-binding.cjs') }));
		build.onResolve({ filter: /^svelte(\/|$)/ }, resolve_svelte(conditions));
		build.onLoad({ filter: /\.svelte\.[jt]s$/ }, load_module);
		build.onLoad({ filter: /\.svelte$/ }, (args) => {
			components++;
			return load_svelte(args);
		});
	}
};

const result = await Bun.build({
	entrypoints: ['./standalone.ts'],
	target: 'bun',
	conditions,
	define: { 'process.env.NODE_ENV': '"production"' },
	minify: true,
	plugins: [plugin],
	throw: false,
	compile: {
		outfile,
		autoloadBunfig: false,
		autoloadDotenv: false,
		...(process.platform === 'win32' && { windows: { hideConsole: true } })
	}
});

if (!result.success || components === 0) {
	for (const log of result.logs) console.error(log);
	if (components === 0) console.error('no .svelte file went through the plugin');
	process.exit(1);
}
console.log(`built ${outfile}`);
