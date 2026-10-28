# Journal Tree

An entirely local, native application that automatically sorts your journaling into tree branches. Built in GPUIX-Svelte and BERTopic. --- basically my own recreation of Obsidian (just for practice)

## Native Markdown editing

The note editor uses GPUIX's native textarea with a local Rust extension and a
CommonMark parser with GitHub-flavored Markdown extensions. It supports:

- Bold, italic, strikethrough, inline code, links, and autolinks.
- ATX and setext headings, with native font sizes and line heights.
- Nested quotes, bullet/ordered lists, and task lists.
- Fenced code with native language-aware syntax highlighting; space-indented code.
- Thematic breaks, escapes, and entities.
- Aligned tables, including Unicode column widths and horizontal scrolling.
- Native image previews from local paths, HTTP(S), and data URLs.
- Reference links and image references.

Formatting markers hide when the cursor leaves the word, including after a
space or newline. Moving or clicking back onto the word reveals editable
Markdown. Tables and images reopen their source when entered. Cmd-click
(Ctrl-click on Windows/Linux) opens supported link destinations. The original
Markdown is preserved for editing, selection, and undo.

Tab indents the current line or selected lines with a literal tab character,
displayed at four-column tab stops. Shift-Tab removes one tab (or up to four
legacy spaces). Tab-indented paragraphs retain Markdown formatting rather than
becoming code blocks; fenced and space-indented code remain supported.
Enter at the end of a list item continues its bullet style
and indentation (and starts a fresh unchecked task for task lists). Enter on an
empty item removes its marker and indentation, returning to plain text.
Typing a space after `-`, `*`, or `+` immediately displays a bullet. Completed
bullet prefixes stay rendered while editing, including after Enter continuation;
the original Markdown marker remains in the note source.
A single dash completed with a space starts a bullet after a paragraph too;
use `---` for an underlined heading in that case.
Shift-Enter inserts a plain newline without continuing a list.
Within lists, a lone indented dash stays an empty bullet instead of turning the
previous item into a heading. Setext headings in lists can use `---` or `===`.

Typing `--` after prose text inserts an em dash (`—`). Undo restores the literal
hyphens. Code, escaped hyphens, URLs, and Markdown separators remain literal;
hyphens at the beginning of a line stay available for rules and underlines.
Loading existing Markdown does not automatically rewrite its punctuation.

Raw HTML remains editable literal text; the native editor does not execute or
render HTML. Obsidian-specific wiki links, embeds, math, and plugins are outside
the CommonMark/GFM scope. Task checkboxes are edited through their Markdown source.

Code fences use their language label for native syntax colours, including while
editing. Supported labels include `js`/`javascript`, `ts`/`typescript`, `jsx`,
`tsx`, `rust`/`rs`, `python`/`py`, `go`, `json`/`jsonc`, `bash`/`sh`, `toml`,
`md`/`markdown`, `html`, `css`, `yaml`/`yml`, and `c`. Backtick and tilde fences,
multiline tokens, and unfinished fences are supported. Unknown or missing labels,
inline code, and space-indented code keep plain monospace styling. Blocks above
512 KiB fall back to plain code to bound highlighting work.

```sh
npm install
npm run native:build
npm start
```

The first native build downloads pinned GPUIX/GPUI sources and installs a
project-local Rust toolchain into the ignored `.native-build/` folder. It needs
the platform's native development tools (Xcode on macOS). It does not change
your shell configuration or modify installed npm packages. Later builds reuse
the compiler cache. Sources match `@gpuix/native` 0.10.0 and `gpuix-svelte` 0.2.0.

```sh
npm run typecheck
npm test               # native layout and editing tests
npm run native:test    # Rust Markdown/run/composition tests
npm run build          # standalone app; also requires Bun >= 1.4
```

`npm start` and `npm test` load the patched addon explicitly and reject missing
or stale builds. The standalone build embeds the same addon. Notes survive
Svelte hot reloads in memory; this editor change does not add disk persistence.

The maintained extension lives in `native/markdown_input.rs`,
`native/markdown_blocks.rs`, `native/markdown_syntax.rs`, and `native/textarea.patch`;
`MarkdownEditor.svelte` exposes it to the app. Markdown
mode fills the editor's visible height so long notes scroll within that area.
The native build and standalone editor tests have been verified on macOS arm64.

The native tests exercise source/display integrity across all 652 official
CommonMark 0.31.2 examples. This checks offsets and source preservation, not
pixel-for-pixel HTML rendering conformance. The corpus in `native/fixtures/`
is licensed separately under CC BY-SA 4.0; see its license file.
