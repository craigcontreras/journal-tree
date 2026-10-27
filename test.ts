import { mount_headless, find_test_id, bounds, check, screenshot, finish } from 'gpuix-svelte/test';
import App from './App.svelte';

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
check('tab sits close to the editor edge', tabStrip.x, editorColumn.x + 8);

if (process.env.GPUIX_SCREENSHOT) screenshot(process.env.GPUIX_SCREENSHOT);

finish('journal layout', 4);
