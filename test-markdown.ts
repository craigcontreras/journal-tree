import { mount_headless, click_test_id, click_at, bounds, find_test_id, press, type, settle, painted, check, screenshot, wait, finish } from 'gpuix-svelte/test';
import App from './App.svelte';
import { editorState } from './lib/editor-state.svelte.ts';

const sample = '# Heading\n\n## Subheading\n\n> A quote\n\n- First\n- [x] Completed\n\n3. Third\n1. Fourth\n\n~~Removed~~ and [a link](https://example.com)\n\n| Name | Value |\n| :--- | ---: |\n| Alpha | 42 |\n\nend';
editorState.source = sample;
mount_headless(App, { width: 1280, height: 500 });
check('headings paint without hash markers', painted().includes('Heading') && !painted().includes('# '));
check('quotes and bullet lists have native markers', painted().includes('│ A quote') && painted().includes('• First'));
check('task lists paint checkbox symbols', painted().includes('☑ Completed'));
check('ordered lists number following items', painted().includes('4. Fourth'));
check('links and strikethrough hide their syntax', painted().includes('Removed and a link') && !painted().includes('~~') && !painted().includes('example.com'));
check('tables paint aligned rows and grid borders', painted().includes('Alpha') && painted().includes('│') && painted().includes('┼'));
check('all source syntax survives the rendered view', editorState.source, sample);

click_test_id('editor');
press('cmd-up');
check('heading syntax reappears when the cursor enters it', painted().includes('# Heading'));
press('down');
check('leaving the heading hides its syntax again', !painted().includes('# Heading'));
if (process.env.GPUIX_MARKDOWN_SCREENSHOT) screenshot(process.env.GPUIX_MARKDOWN_SCREENSHOT);

editorState.source = 'Title\n=====\n\n```ts\nconst value = "**literal**";\n```\n\n---\n\nend';
settle();
press('cmd-down');
check('setext headings hide their underline markers', !painted().includes('====='));
check('fenced blocks retain literal code and hide fences', painted().includes('**literal**') && !painted().includes('```'));
check('thematic breaks render as rules', painted().includes('────'));

editorState.source = 'Escaped \\*stars\\* and &amp; entity\n\n[reference][id]\n\n[id]: https://example.com\n\nend';
settle();
press('cmd-down');
check('escapes and entities render as their characters', painted().includes('Escaped *stars* and & entity'));
check('reference link definitions are hidden outside editing', painted().includes('reference') && !painted().includes('[id]:'));

const image = '![caption](data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAgAAAAICAIAAABLbSncAAAAFElEQVR4nGN8aWXFgA0wYRUdtBIAFbQBbYt+OG8AAAAASUVORK5CYII=)\n\nend';
editorState.source = image;
settle();
press('cmd-down');
await wait(200);
check('images reserve a native preview and caption', painted().includes('caption') && !painted().includes('data:image'));
press('cmd-up');
press('down');
press('cmd-down');
if (process.env.GPUIX_IMAGE_SCREENSHOT) screenshot(process.env.GPUIX_IMAGE_SCREENSHOT);
const viewport = bounds(find_test_id('editor')!)!;
click_at(viewport.x + 100, viewport.y + 60);
check('clicking an image reveals its editable source', painted().includes('![caption]('));
press('cmd-a');
type('q');
press('cmd-z');
check('undo restores an image and its exact data URL', editorState.source, image);

const code = '```ts\nconst greeting: string = "Hello 🙂";\nconst count = 42; // a comment\n```\n\n```python\ndef greet(name):\n    return f"Hello {name}"\n```\n\nend';
editorState.source = code;
settle();
press('cmd-down');
check('highlighted fences preserve literal code and whitespace', painted().includes('const greeting: string = "Hello 🙂";') && painted().includes('    return f"Hello {name}"') && !painted().includes('```'));
check('highlighting preserves the complete Markdown source', editorState.source, code);
if (process.env.GPUIX_SYNTAX_SCREENSHOT) screenshot(process.env.GPUIX_SYNTAX_SCREENSHOT);
press('cmd-up');
press('down');
press('cmd-right');
type(';');
check('highlighted code stays editable at its native caret', editorState.source, code.replace('"Hello 🙂";', '"Hello 🙂";;'));
press('cmd-z');
check('Undo restores highlighted code and Unicode exactly', editorState.source, code);
press('cmd-up');
press('cmd-right');
press('backspace');
press('backspace');
type('python');
check('the fence language remains directly editable', editorState.source, code.replace('```ts', '```python'));

finish('native Markdown blocks', 22);
