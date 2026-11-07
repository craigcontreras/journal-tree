import { mount_headless, find_test_id, bounds, check, screenshot, finish, settle, click_at, click_test_id, drag, focus, painted, press, wheel, type } from 'gpuix-svelte/test';
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import App from './App.svelte';
import { fileState, openFiles } from './lib/file-state.svelte.ts';

mount_headless(App, { width: 1280, height: 500 });

const header = bounds(find_test_id('menubar-row')!)!;
const workspace = bounds(find_test_id('workspace')!)!;
const sidebar = bounds(find_test_id('left-sidebar')!)!;
const tabStrip = bounds(find_test_id('tab-strip')!)!;
const editorColumn = bounds(find_test_id('editor-column')!)!;

check('menubar spans the workspace', header.width, workspace.width);
// Native bounds omit the menubar's 2px bottom border.
check('workspace begins below the menubar divider', workspace.y, header.y + header.height + 2);
check('sidebar divider stays below the window controls', sidebar.y, workspace.y);
check('tab sits close to the editor edge', tabStrip.x, editorColumn.x);

if (process.env.GPUIX_SCREENSHOT) screenshot(process.env.GPUIX_SCREENSHOT);

const folder = await mkdtemp(join(tmpdir(), 'journal-tree-tabs-'));
try {
	const first = join(folder, 'first', 'notes.md');
	const second = join(folder, 'second', 'notes.md');
	await mkdir(join(folder, 'first'));
	await mkdir(join(folder, 'second'));
	await writeFile(first, 'First note');
	const failedOpen = await openFiles([second]).then(() => false, () => true);
	check('a failed background open leaves the tab list alone and can be retried', failedOpen && fileState.files.length === 0);
	await writeFile(second, 'Second note');
	await openFiles([first, second]);
	settle();
	check('the last opened file is shown in the editor', painted().includes('Second note'));

	click_test_id('file-tab-0');
	check('clicking a tab selects its file even when names match', fileState.activePath, first);
	check('the editor shows the clicked file', painted().includes('First note') && !painted().includes('Second note'));
	click_test_id('editor');
	press('cmd-down');
	type('!');

	click_test_id('file-tab-1');
	check('switching tabs shows the other file without carrying over edits', painted().includes('Second note') && !painted().includes('First note!'));
	click_test_id('editor');
	press('cmd-down');
	type('?');

	focus(find_test_id('file-tab-0')!);
	press('enter');
	check('keyboard activation selects a tab', fileState.activePath, first);
	check('unsaved edits are restored when returning to the first file', painted().includes('First note!'));
	click_test_id('file-tab-1');
	check('each file keeps its own unsaved edits', painted().includes('Second note?'));

	await openFiles([first]);
	settle();
	check('reopening a file selects its existing tab', fileState.files.length === 2 && fileState.activePath === first);
	check('reopening a file preserves its in-memory edits', painted().includes('First note!'));

	const large = join(folder, 'large.md');
	const body = 'A row in a large file. '.repeat(3);
	const source = Array.from({ length: 12_001 }, (_, row) => {
		if (row === 0) return '**First row**';
		if (row === 6_000) return 'Middle row';
		if (row === 12_000) return 'Last row 👩‍💻';
		return body;
	}).join('\r\n');
	await writeFile(large, source);
	const opening = openFiles([large]);
	check('the previous tab stays active while the file is loading', fileState.activePath, first);
	await Promise.all([opening, openFiles([large])]);
	settle();
	const note = fileState.files.find(file => file.path === large)!;
	if (!('document' in note)) throw new Error('Expected a native large-file buffer');
	const document = note.document;
	check('concurrent opens reuse one tab and one native document', fileState.files.filter(file => file.path === large).length, 1);
	check('large files open in editable plain text', painted().includes('**First row**'));
	check('large files initially paint only the top viewport', !painted().includes('Middle row') && !painted().includes('Last row'));

	const editor = bounds(find_test_id('editor')!)!;
	const x = editor.x + 10;
	const y = editor.y + 10;
	const scrollbarX = editor.x + editor.width - 6;
	const editorBottom = editor.y + editor.height;
	click_at(scrollbarX, editorBottom - 2);
	check('clicking the scrollbar track jumps to the end', painted().includes('Last row 👩‍💻') && !painted().includes('**First row**'));
	drag({ x: scrollbarX, y: editorBottom - 12 }, { x: scrollbarX, y: editor.y - 4 });
	check('dragging the scrollbar thumb moves the viewport without editing', painted().includes('**First row**') && !painted().includes('Last row') && document.text() === source);

	click_at(x, y);
	press('cmd-right');
	type('!');
	check('typing at the end of a row preserves CRLF', document.text().startsWith('**First row**!\r\n'));
	press('cmd-z');

	wheel({ x, y }, 0, -6_000 * 26);
	check('the native textarea scrolls to distant rope rows', painted().includes('Middle row') && !painted().includes('First row'));
	click_at(editor.x + 1, y);
	press('cmd-left');
	type('!');
	check('a click after scrolling edits the correct source row', document.text() === source.replace('Middle row', '!Middle row'));

	press('cmd-down');
	check('jumping to the end lays out the last row', painted().includes('Last row 👩‍💻') && !painted().includes('Middle row'));
	press('backspace');
	check('large-file backspace removes a whole grapheme', document.text().endsWith('Last row '));
	press('cmd-z');
	check('undo restores the Unicode text', document.text().endsWith('Last row 👩‍💻'));
	press('cmd-shift-z');
	check('redo reapplies the large-file edit', document.text().endsWith('Last row '));
	press('enter');
	type('!');
	check('inserting a new row updates the rope layout', document.text().endsWith('Last row \n!') && painted().includes('Last row \n!'));

	click_test_id('file-tab-0');
	check('normal notes continue to use the regular editor', painted().includes('First note!'));
	click_test_id('file-tab-2');
	click_test_id('editor');
	press('cmd-down');
	check('large-file edits survive switching tabs', document.text().includes('!Middle row') && painted().includes('Last row \n!'));

	const regular = join(folder, 'regular.md');
	const regularSource = Array.from({ length: 81 }, (_, row) => `Regular row ${row}`).join('\n');
	await writeFile(regular, regularSource);
	await openFiles([regular]);
	settle();
	const regularNote = fileState.files.find(file => file.path === regular)!;
	if (!('source' in regularNote)) throw new Error('Expected a regular Markdown file');
	focus(find_test_id('editor')!);
	type('!');
	check('a regular file opens with its caret at the start', regularNote.source === `!${regularSource}`);
	click_at(x, y);
	press('cmd-left');
	type('?');
	check('a regular file opens with its first line in view', regularNote.source === `?!${regularSource}`);
	click_at(scrollbarX, editorBottom - 2);
	click_at(x, y);
	press('cmd-left');
	type('#');
	check('the scrollbar also scrolls regular Markdown files', regularNote.source.indexOf('#') > regularNote.source.indexOf('Regular row 60'));

	const wide = join(folder, 'wide.txt');
	const wideRow = '0'.repeat(480) + 'Right edge';
	const wideSource = `${wideRow}${'\nA short row'.repeat(4998)}\n${wideRow} at the bottom`;
	await writeFile(wide, wideSource);
	await openFiles([wide]);
	settle();
	const wideNote = fileState.files.find(file => file.path === wide)!;
	if (!('document' in wideNote)) throw new Error('Expected an unwrapped large file');
	const rightOfText = editor.x + editor.width - 14;
	const bottomTrack = editorBottom - 6;

	click_at(rightOfText, bottomTrack);
	click_at(rightOfText, y);
	type('!');
	check('the horizontal track scrolls to the right end of a wide row', wideNote.document.text().startsWith(`${wideRow}!\n`));

	drag({ x: rightOfText - 12, y: bottomTrack }, { x: editor.x - 4, y: bottomTrack });
	click_at(editor.x + 1, y);
	type('?');
	check('dragging the horizontal thumb returns to the left edge', wideNote.document.text().startsWith(`?${wideRow}!\n`));

	click_at(scrollbarX, editorBottom - 14);
	click_at(editor.x + 1, editorBottom - 14);
	type('@');
	check('the vertical scrollbar reaches the bottom while both bars are visible', wideNote.document.text().endsWith(`\n@${wideRow} at the bottom`));
} finally {
	await rm(folder, { recursive: true, force: true });
}

finish('journal layout, file tabs, and large files', 36);
