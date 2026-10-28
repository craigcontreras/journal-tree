import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { assertNativeBuild, binaryPath, projectRoot } from './config.ts';

assertNativeBuild();
const require = createRequire(import.meta.url);
const binding = require(binaryPath) as { journalTreeMarkdownVersion?: () => number };
if (binding.journalTreeMarkdownVersion?.() !== 3) {
	throw new Error('The native library does not contain the Markdown editor extension.');
}
const cli = join(dirname(require.resolve('gpuix-svelte/package.json')), 'bin/gpuix-svelte.js');
const child = spawn(process.execPath, [cli, ...process.argv.slice(2)], {
	cwd: projectRoot,
	stdio: 'inherit',
	env: { ...process.env, NAPI_RS_NATIVE_LIBRARY_PATH: binaryPath }
});
child.on('error', (error) => { console.error(error.message); process.exitCode = 1; });
child.on('exit', (code, signal) => { process.exitCode = code ?? (signal ? 1 : 0); });
process.on('SIGINT', () => child.kill('SIGINT'));
process.on('SIGTERM', () => child.kill('SIGTERM'));
