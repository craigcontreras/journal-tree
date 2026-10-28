import { mount_headless, click_test_id, click_at, find_test_id, bounds, press, type, settle, painted, check, screenshot, finish } from 'gpuix-svelte/test';
import App from './App.svelte';
import { editorState } from './lib/editor-state.svelte.ts';

const source = '**Bold**, *italic*, and `inline code`\nNested **bold and _italic_** — café 🙂';
editorState.source = source;
mount_headless(App, { width: 1280, height: 500 });

check('inactive formatted words paint without Markdown markers', painted().includes('Bold, italic, and inline code\nNested bold and italic — café 🙂'));
check('hidden markers remain in the note source', editorState.source, source);
if (process.env.GPUIX_EDITOR_SCREENSHOT) screenshot(process.env.GPUIX_EDITOR_SCREENSHOT);

click_test_id('editor');
press('cmd-a');
type('x');
check('typing replaces the native selection and reports the new source', editorState.source, 'x');
press('cmd-z');
check('undo restores Markdown and Unicode exactly', editorState.source, source);
press('cmd-shift-z');
check('redo restores the edited source', editorState.source, 'x');

press('cmd-down');
press('enter');
type('y');
check('Enter inserts a newline in the native editor', editorState.source, 'x\ny');

editorState.source = '**updated** _from state_ `代码`';
settle();
check('external source updates preserve the Markdown bytes', editorState.source, '**updated** _from state_ `代码`');
click_test_id('editor');
press('cmd-a');
press('backspace');
check('deleting the document leaves an editable empty note', editorState.source, '');
check('empty editor paints its placeholder', painted().includes('Write Markdown…'));

editorState.source = Array.from({ length: 80 }, (_, i) => `**Row ${i}** body`).join('\n');
settle();
click_test_id('editor');
press('cmd-down');
const viewport = bounds(find_test_id('editor')!)!;
click_at(viewport.x + 5, viewport.y + 8);
type('q');
check('long notes scroll the caret into the visible editor viewport', editorState.source.indexOf('q') > editorState.source.length / 2);

editorState.source = '**word**';
settle();
press('cmd-down');
check('syntax stays editable while the cursor is at the formatted word', painted().includes('**word**'));
type('space');
check('space after the word hides the syntax', painted().includes('word ') && !painted().includes('**'));
check('space preserves the original Markdown', editorState.source, '**word** ');
press('enter');
check('moving to the next line keeps the syntax hidden', !painted().includes('**'));
press('cmd-up');
check('moving the cursor back to the word reveals its syntax', painted().includes('**word**'));
press('cmd-down');
click_at(viewport.x + 12, viewport.y + 12);
check('clicking the rendered word reveals editable Markdown', painted().includes('**word**'));
type('q');
check('editing a revealed word preserves its surrounding syntax', editorState.source.startsWith('**') && editorState.source.endsWith('** \n') && editorState.source.includes('q'));
press('cmd-up');
press('right');
press('backspace');
check('revealed Markdown delimiters can be edited', editorState.source.startsWith('*') && !editorState.source.startsWith('**'));

finish('native Markdown editor', 18);
