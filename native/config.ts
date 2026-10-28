import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export const projectRoot = fileURLToPath(new URL('../', import.meta.url));
export const buildRoot = join(projectRoot, '.native-build');
export const gpuixRevision = '9fcd628863e354e9c58019fc3bf38981a1e64158';
export const zedRevision = '81c99f816b4a5f69d3c014774068034c24d1d7af';
export const rustVersion = '1.97.1';
export const binaryPath = join(buildRoot, 'artifacts', `${process.platform}-${process.arch}.node`);
export const manifestPath = `${binaryPath}.json`;

export function sourceHash(): string {
	const hash = createHash('sha256').update(gpuixRevision).update(zedRevision).update(rustVersion);
	for (const name of ['markdown_input.rs', 'markdown_blocks.rs', 'markdown_syntax.rs', 'textarea.patch']) {
		hash.update(readFileSync(join(projectRoot, 'native', name)));
	}
	return hash.digest('hex');
}

export function assertNativeBuild(): void {
	const require = createRequire(import.meta.url);
	const nativeVersion = (JSON.parse(readFileSync(join(dirname(require.resolve('@gpuix/native')), 'package.json'), 'utf8')) as { version: string }).version;
	const svelteVersion = (require('gpuix-svelte/package.json') as { version: string }).version;
	if (nativeVersion !== '0.10.0' || svelteVersion !== '0.2.0') {
		throw new Error('The Markdown extension requires gpuix-svelte 0.2.0 and @gpuix/native 0.10.0.');
	}
	if (!existsSync(binaryPath) || !existsSync(manifestPath)) {
		throw new Error('Native Markdown editor is not built. Run npm run native:build first.');
	}
	const manifest = JSON.parse(readFileSync(manifestPath, 'utf8')) as { hash: string };
	if (manifest.hash !== sourceHash()) {
		throw new Error('Native Markdown editor source has changed. Run npm run native:build again.');
	}
}
