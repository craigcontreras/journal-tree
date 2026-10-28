import { mount_headless, click_test_id, click_at, bounds, find_test_id, press, type, settle, painted, check, screenshot, finish } from 'gpuix-svelte/test';
import App from './App.svelte';
import { editorState } from './lib/editor-state.svelte.ts';

editorState.source = '- Parent\n- Child';
mount_headless(App, { width: 1280, height: 500 });
click_test_id('editor');
press('cmd-down');
press('tab');
check('Tab indents the current bullet with a literal tab', editorState.source, '- Parent\n\t- Child');
type('!');
check('Tab keeps focus and the caret in the bullet', editorState.source, '- Parent\n\t- Child!');
press('enter');
check('Enter continues a nested bullet at the same level', editorState.source, '- Parent\n\t- Child!\n\t- ');
press('enter');
check('Enter on an empty bullet removes its marker and indent', editorState.source, '- Parent\n\t- Child!\n\n');
press('cmd-z');
check('removing an empty bullet is undoable in one step', editorState.source, '- Parent\n\t- Child!\n\t- ');
press('cmd-shift-z');
check('redo removes the empty bullet again', editorState.source, '- Parent\n\t- Child!\n\n');
type('plain');
type('space');
type('text');
check('typing after exiting a list produces plain text', editorState.source, '- Parent\n\t- Child!\n\nplain text');
check('nested bullets retain visible indentation', painted().includes('\n    • Child!'));
if (process.env.GPUIX_LIST_SCREENSHOT) screenshot(process.env.GPUIX_LIST_SCREENSHOT);

editorState.source = '- Parent\n    - café 🙂';
settle();
press('cmd-down');
press('shift-tab');
check('Shift-Tab outdents a bullet', editorState.source, '- Parent\n- café 🙂');
type('!');
check('outdenting preserves the caret after Unicode text', editorState.source, '- Parent\n- café 🙂!');
press('cmd-a');
press('tab');
press('shift-tab');
check('Tab and Shift-Tab handle a multiline selection', editorState.source, '- Parent\n- café 🙂!');
type('replacement');
check('indenting preserves the selected text range', editorState.source, 'replacement');

editorState.source = '- first';
settle();
press('cmd-down');
press('enter');
check('Enter continues a top-level bullet', editorState.source, '- first\n- ');
press('enter');
check('a second Enter exits the list', editorState.source, '- first\n\n');

editorState.source = '- [x] done';
settle();
press('cmd-down');
press('enter');
check('continuing a task starts an unchecked item', editorState.source, '- [x] done\n- [ ] ');
press('enter');
check('Enter also removes an empty task bullet', editorState.source, '- [x] done\n\n');

editorState.source = '* first';
settle();
press('cmd-down');
press('enter');
check('list continuation preserves the original bullet style', editorState.source, '* first\n* ');

editorState.source = '- hello';
settle();
press('cmd-up');
press('right'); press('right'); press('right'); press('right');
press('enter');
check('Enter within an item inserts an ordinary newline', editorState.source, '- he\nllo');

editorState.source = '- hello';
settle();
press('cmd-down');
press('shift-enter');
check('Shift-Enter inserts a newline without a bullet', editorState.source, '- hello\n');

editorState.source = '```\n- literal';
settle();
press('cmd-down');
press('enter');
check('bullet-like text inside code is not continued', editorState.source, '```\n- literal\n');

editorState.source = '- Parent\n    - ';
settle();
press('cmd-down');
press('enter');
check('an empty nested dash exits even when it resembles a heading underline', editorState.source, '- Parent\n\n');

editorState.source = '- hello\n\t- hello\n\t- \n\nend';
settle();
press('cmd-up');
press('down');
press('down');
press('cmd-right');
press('tab');
const emptyNested = '- hello\n\t- hello\n\t\t- \n\nend';
check('Tab preserves the original dash in a new nested bullet', editorState.source, emptyNested);
if (process.env.GPUIX_EMPTY_LIST_SCREENSHOT) screenshot(process.env.GPUIX_EMPTY_LIST_SCREENSHOT);
const viewport = bounds(find_test_id('editor')!)!;
// At body line height (26px), this point is on the fourth line. A setext
// heading incorrectly enlarges the empty bullet enough to intercept it.
click_at(viewport.x + 500, viewport.y + 84);
type('x');
check('an empty nested bullet keeps normal line height', editorState.source, emptyNested.replace('\n\nend', '\nx\nend'));
press('backspace');
click_at(viewport.x + 500, viewport.y + 84);
type('y');
check('clearing a nested bullet does not change its line height', editorState.source, emptyNested.replace('\n\nend', '\ny\nend'));

editorState.source = 'Plain **bold**';
settle();
press('cmd-down');
press('tab');
press('tab');
check('plain paragraphs indent with actual tab characters', editorState.source, '\t\tPlain **bold**');
type('!');
check('typing after paragraph indentation keeps the caret in place', editorState.source, '\t\tPlain **bold**!');
press('shift-tab');
check('Shift-Tab removes one literal tab', editorState.source, '\tPlain **bold**!');
press('cmd-z');
check('Undo restores paragraph indentation', editorState.source, '\t\tPlain **bold**!');
press('cmd-shift-z');
check('Redo removes one indentation level again', editorState.source, '\tPlain **bold**!');
press('enter');
type('end');
check('tab-indented paragraphs render Markdown instead of code', painted().includes('    Plain bold!') && !painted().includes('**'));
if (process.env.GPUIX_TAB_SCREENSHOT) screenshot(process.env.GPUIX_TAB_SCREENSHOT);

for (const marker of ['-', '*']) {
	editorState.source = '';
	settle();
	press('cmd-down');
	type(marker);
	check(`${marker} stays literal until a space is typed`, editorState.source === marker && !painted().includes('•'));
	type('space');
	check(`${marker} becomes a bullet immediately after a space`, painted().includes('• ') && !painted().includes(`${marker} `));
	check(`${marker} remains Markdown in the note source`, editorState.source, `${marker} `);
	press('backspace');
	check(`deleting the space restores a literal ${marker}`, editorState.source === marker && !painted().includes('•'));
	press('cmd-z');
	check(`Undo restores the rendered ${marker} bullet`, editorState.source === `${marker} ` && painted().includes('• '));
	press('cmd-right');
	type('word');
	check(`the ${marker} bullet stays rendered while editing its word`, painted().includes('• word') && !painted().includes(`${marker} word`));
	press('enter');
	check(`Enter immediately renders the continued ${marker} bullet`, editorState.source === `${marker} word\n${marker} ` && painted().includes('• word\n• '));
	press('tab');
	check(`indenting the continued ${marker} keeps its bullet glyph`, editorState.source === `${marker} word\n\t${marker} ` && painted().includes('• word\n    • '));
	press('enter');
	check(`Enter still removes the empty ${marker} bullet`, editorState.source, `${marker} word\n\n`);
}

for (const indent of ['', '\t']) {
	editorState.source = `Paragraph\n${indent}-`;
	settle();
	press('cmd-down');
	type('space');
	check(`a ${JSON.stringify(indent)} bullet starts immediately after a paragraph`, painted().includes(`Paragraph\n${indent.replace('\t', '    ')}• `));
	type('word');
	press('enter');
	check(`the ${JSON.stringify(indent)} paragraph-following bullet continues`, editorState.source === `Paragraph\n${indent}- word\n${indent}- ` && painted().includes(`${indent.replace('\t', '    ')}• `));
}

finish('native Markdown lists', 52);
