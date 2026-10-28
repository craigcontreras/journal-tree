import { mount_headless, click_test_id, press, type, settle, check, finish } from 'gpuix-svelte/test';
import App from './App.svelte';
import { editorState } from './lib/editor-state.svelte.ts';

function typeHyphens(count = 2) {
	for (let i = 0; i < count; i++) type('-');
}

editorState.source = 'hello';
mount_headless(App, { width: 1280, height: 500 });
click_test_id('editor');
press('cmd-down');
typeHyphens();
check('typing two hyphens after prose inserts an em dash', editorState.source, 'hello—');
type('world');
check('the caret stays after the inserted em dash', editorState.source, 'hello—world');
press('cmd-z');
check('undoing subsequent typing retains the em dash', editorState.source, 'hello—');
press('cmd-z');
check('one Undo cancels the substitution to literal hyphens', editorState.source, 'hello--');
press('cmd-shift-z');
check('redo restores the em dash', editorState.source, 'hello—');

editorState.source = 'café 🙂';
settle();
press('cmd-down');
typeHyphens();
type('x');
check('em dash substitution preserves Unicode and caret offsets', editorState.source, 'café 🙂—x');
press('backspace');
press('backspace');
check('Backspace deletes the em dash as one character', editorState.source, 'café 🙂');

for (const [source, expected] of [
	['', '--'],
	['Title\n', 'Title\n--'],
	['| :', '| :--'],
	['```\ncode', '```\ncode--'],
	['    code', '    code--'],
	['`code', '`code--'],
	['Visit https://example.com/path', 'Visit https://example.com/path--'],
	['escaped \\', 'escaped \\--'],
] as const) {
	editorState.source = source;
	settle();
	press('cmd-down');
	typeHyphens();
	check(`hyphens remain literal in ${JSON.stringify(source)}`, editorState.source, expected);
}

editorState.source = '';
settle();
press('cmd-down');
typeHyphens(3);
check('typing a thematic break still preserves all three hyphens', editorState.source, '---');

editorState.source = 'Existing -- source and an em dash —';
settle();
check('existing Markdown is not rewritten when loaded', editorState.source, 'Existing -- source and an em dash —');

finish('native typography', 17);
