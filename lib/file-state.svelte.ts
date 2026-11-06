import { GpuixRenderer } from '@gpuix/native';
import { basename, resolve } from 'node:path';

type LargeTextDocument = { readonly id: number; text(): string };
type OpenFile = { path: string; filename: string } & (
	{ source: string } | { document: LargeTextDocument }
);

type OpenTextFileResult =
	| { source: string; document?: never }
	| { source?: never; document: LargeTextDocument };
const native = GpuixRenderer as typeof GpuixRenderer & {
	openTextFile(path: string): Promise<OpenTextFileResult>;
};
const openingFiles = new Map<string, Promise<void>>();

export const fileState = $state<{ files: OpenFile[]; activePath: string | null }>({
	files: [],
	activePath: null,
});

export function selectFile(path: string) {
	if (fileState.files.some(file => file.path === path)) fileState.activePath = path;
}

export async function openFiles(paths: readonly string[]) {
	for (const inputPath of paths) {
		const path = resolve(inputPath);
		if (!fileState.files.some(file => file.path === path)) {
			let pending = openingFiles.get(path);
			if (!pending) {
				pending = native.openTextFile(path).then((file) => {
					const contents = file.document ? { document: file.document } : { source: file.source };
					fileState.files.push({ path, filename: basename(path), ...contents });
				}).finally(() => openingFiles.delete(path));
				openingFiles.set(path, pending);
			}
			await pending;
		}
		selectFile(path);
	}
}
