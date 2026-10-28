import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { chmod, copyFile, mkdir, readFile, writeFile, rename, access } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { binaryPath, buildRoot, gpuixRevision, manifestPath, projectRoot, rustVersion, sourceHash, zedRevision } from './config.ts';

const sourceRoot = join(buildRoot, 'gpuix');
const toolsRoot = join(buildRoot, 'tools');
const cargoHome = join(toolsRoot, 'cargo');
const rustupHome = join(toolsRoot, 'rustup');
const executableSuffix = process.platform === 'win32' ? '.exe' : '';
const cargo = join(cargoHome, 'bin', `cargo${executableSuffix}`);
const env = {
	...process.env,
	CARGO_HOME: cargoHome,
	RUSTUP_HOME: rustupHome,
	CARGO_TARGET_DIR: join(buildRoot, 'target'),
	PATH: `${join(cargoHome, 'bin')}${process.platform === 'win32' ? ';' : ':'}${process.env.PATH ?? ''}`
};

async function exists(path: string): Promise<boolean> {
	try { await access(path); return true; } catch { return false; }
}

async function run(command: string, args: string[], quiet = false): Promise<number> {
	return new Promise((resolve, reject) => {
		const child = spawn(command, args, { cwd: projectRoot, env, stdio: quiet ? 'ignore' : 'inherit' });
		child.on('error', reject);
		child.on('exit', (code) => resolve(code ?? 1));
	});
}

async function checked(command: string, args: string[]): Promise<void> {
	if (await run(command, args)) throw new Error(`${command} failed. See the build output above.`);
}

async function download(url: string, destination: string): Promise<void> {
	const response = await fetch(url);
	if (!response.ok || !response.body) throw new Error(`Download failed: ${url} (${response.status})`);
	await writeFile(destination, new Uint8Array(await response.arrayBuffer()));
}

async function sourceArchive(repo: string, revision: string, target: string): Promise<void> {
	const marker = join(target, '.journal-tree-revision');
	if (await exists(marker) && (await readFile(marker, 'utf8')) === revision) return;
	const archive = join(buildRoot, 'downloads', `${repo}-${revision}.tar.gz`);
	console.log(`Preparing ${repo} at ${revision.slice(0, 8)}…`);
	if (!(await exists(archive))) await download(`https://github.com/remorses/${repo}/archive/${revision}.tar.gz`, archive);
	await mkdir(target, { recursive: true });
	await checked('tar', ['-xzf', archive, '-C', target, '--strip-components=1']);
	await writeFile(marker, revision);
}

await mkdir(join(buildRoot, 'downloads'), { recursive: true });
await mkdir(toolsRoot, { recursive: true });
const host = ({ 'darwin-arm64': 'aarch64-apple-darwin', 'linux-x64': 'x86_64-unknown-linux-gnu', 'win32-x64': 'x86_64-pc-windows-msvc' } as Record<string, string>)[`${process.platform}-${process.arch}`];
if (!host) throw new Error('Native Markdown builds support macOS arm64, Linux x64, and Windows x64.');

if (!(await exists(cargo))) {
	console.log('Installing a project-local Rust toolchain (shell settings are unchanged)…');
	const installer = join(toolsRoot, `rustup-init${executableSuffix}`);
	const installerUrl = `https://static.rust-lang.org/rustup/dist/${host}/rustup-init${executableSuffix}`;
	await download(installerUrl, installer);
	const checksumResponse = await fetch(`${installerUrl}.sha256`);
	if (!checksumResponse.ok) throw new Error('Could not verify the Rust installer checksum.');
	const expectedHash = (await checksumResponse.text()).trim().split(/\s+/)[0];
	if (createHash('sha256').update(await readFile(installer)).digest('hex') !== expectedHash) throw new Error('Rust installer checksum mismatch.');
	if (process.platform !== 'win32') await chmod(installer, 0o755);
	await checked(installer, ['-y', '--no-modify-path', '--profile', 'minimal', '--default-toolchain', rustVersion]);
}

await sourceArchive('gpuix', gpuixRevision, sourceRoot);
await sourceArchive('zed', zedRevision, join(sourceRoot, 'zed'));

const patchArgs = ['--unsafe-paths', `--directory=${relative(projectRoot, sourceRoot)}`, join(projectRoot, 'native/textarea.patch')];
if (await run('git', ['apply', '--reverse', '--check', ...patchArgs], true)) {
	await checked('git', ['apply', '--check', ...patchArgs]);
	await checked('git', ['apply', ...patchArgs]);
}
for (const name of ['markdown_input.rs', 'markdown_blocks.rs', 'markdown_syntax.rs']) {
	const moduleSource = join(projectRoot, 'native', name);
	const moduleDestination = join(sourceRoot, 'packages/native/src/custom_elements', name);
	if (!(await exists(moduleDestination)) || !(await readFile(moduleSource)).equals(await readFile(moduleDestination))) {
		await copyFile(moduleSource, moduleDestination);
	}
}
const corpusSource = join(projectRoot, 'native/fixtures/commonmark.json');
const corpusDestination = join(sourceRoot, 'packages/native/src/custom_elements/commonmark.json');
if (!(await exists(corpusDestination)) || !(await readFile(corpusSource)).equals(await readFile(corpusDestination))) {
	await copyFile(corpusSource, corpusDestination);
}

const manifest = join(sourceRoot, 'packages/native/Cargo.toml');
if (process.argv.includes('--test')) {
	await checked(cargo, ['test', '--locked', '--release', '--manifest-path', manifest, '--lib', 'custom_elements::markdown_']);
} else {
	console.log('Building the native Markdown editor…');
	await checked(cargo, ['build', '--locked', '--release', '--manifest-path', manifest]);
	await mkdir(join(buildRoot, 'artifacts'), { recursive: true });
	const library = process.platform === 'darwin' ? 'libgpuix_native.dylib' : process.platform === 'win32' ? 'gpuix_native.dll' : 'libgpuix_native.so';
	await copyFile(join(buildRoot, 'target/release', library), `${binaryPath}.next`);
	await rename(`${binaryPath}.next`, binaryPath);
	await writeFile(manifestPath, JSON.stringify({ hash: sourceHash(), gpuixRevision, zedRevision, rustVersion }, null, 2));
	const exports = ['AvailableUpdate', 'GpuixRenderer', 'TestGpuixRenderer', 'checkUpdate', 'hasTestGpuixRenderer'];
	await writeFile(join(buildRoot, 'standalone-binding.cjs'), [
		`const binding = require('./artifacts/${process.platform}-${process.arch}.node');`,
		...exports.map((name) => `exports.${name} = binding.${name};`),
		`exports.__napiBindingTarget = 'native';`, ''
	].join('\n'));
	console.log(`Native Markdown editor ready: ${binaryPath}`);
}
