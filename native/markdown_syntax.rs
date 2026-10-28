//! Language-aware colours for editable Markdown code fences.

use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};

use super::markdown_input::parser;
use crate::syntax::{cache::highlight_cached, language_for_alias, HighlightSpan};

struct Piece {
    code: Range<usize>,
    source: Range<usize>,
}

struct CodeBlock {
    language: String,
    text: String,
    pieces: Vec<Piece>,
}

impl CodeBlock {
    fn push(&mut self, source: &str, range: Range<usize>, text: &str) {
        let mut code_offset = self.text.len();
        self.text.push_str(text);
        let mut source_offset = range.start;
        // Parser text excludes quote/list prefixes and may expand a partial
        // tab. Match each literal payload line to its original source line;
        // leave virtual indentation uncoloured instead of shifting tokens.
        for line in text.split_inclusive('\n') {
            let end = source[source_offset..range.end]
                .find('\n')
                .map_or(range.end, |index| source_offset + index);
            let raw = source[source_offset..end].trim_end_matches('\r');
            let payload = line.trim_end_matches(['\r', '\n']);
            let (payload, skip) = if raw.contains(payload) {
                (payload, 0)
            } else {
                let trimmed = payload.trim_start_matches([' ', '\t']);
                (trimmed, payload.len() - trimmed.len())
            };
            if !payload.is_empty() {
                if let Some(index) = raw.rfind(payload) {
                    self.pieces.push(Piece {
                        code: code_offset + skip..code_offset + skip + payload.len(),
                        source: source_offset + index..source_offset + index + payload.len(),
                    });
                }
            }
            code_offset += line.len();
            source_offset = (end + 1).min(range.end);
        }
    }

    fn finish(self, output: &mut Vec<HighlightSpan>) {
        let Some(document) = highlight_cached(&self.text, None, Some(&self.language)) else {
            return;
        };
        let (mut offset, mut piece_index) = (0usize, 0usize);
        for (line, spans) in self.text.split('\n').zip(&document.lines) {
            for span in spans {
                let range = offset + span.range.start..offset + span.range.end;
                while piece_index < self.pieces.len()
                    && self.pieces[piece_index].code.end <= range.start
                {
                    piece_index += 1;
                }
                for piece in &self.pieces[piece_index..] {
                    if piece.code.start >= range.end {
                        break;
                    }
                    let start = piece.code.start.max(range.start);
                    let end = piece.code.end.min(range.end);
                    if start < end {
                        output.push(HighlightSpan {
                            range: piece.source.start + start - piece.code.start
                                ..piece.source.start + end - piece.code.start,
                            kind: span.kind,
                        });
                    }
                }
            }
            offset += line.len() + 1;
        }
    }
}

pub(super) fn highlights(source: &str) -> Vec<HighlightSpan> {
    let mut output = Vec::new();
    let mut block = None;
    for (event, range) in parser(source) {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(language))) => {
                // A missing/unknown label stays plain, including a shebang
                // that the general-purpose code element could autodetect.
                block = language_for_alias(&language).map(|_| CodeBlock {
                    language: language.to_string(),
                    text: String::new(),
                    pieces: Vec::new(),
                });
            }
            Event::Text(text) => {
                if let Some(block) = &mut block {
                    block.push(source, range, &text);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(block) = block.take() {
                    block.finish(&mut output);
                }
            }
            _ => {}
        }
    }
    output.sort_by_key(|span| span.range.start);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::HighlightKind;

    #[test]
    fn language_labels_colour_keywords_strings_numbers_and_comments() {
        let source = "Outside const\n\n```TS title=example\nconst value: number = 42;\nconsole.log(\"hello\"); // comment\n```\nOutside";
        let spans = highlights(source);
        for kind in [
            HighlightKind::Keyword,
            HighlightKind::String,
            HighlightKind::Number,
            HighlightKind::Comment,
        ] {
            assert!(spans.iter().any(|span| span.kind == kind), "{kind:?}");
        }
        let start = source.find("const value").unwrap();
        let end = source.rfind("```").unwrap();
        assert!(spans
            .iter()
            .all(|span| start <= span.range.start && span.range.end <= end));
    }

    #[test]
    fn aliases_containers_line_endings_and_unclosed_fences_preserve_offsets() {
        for source in [
            "~~~js\nconst n = 3;\n~~~",
            "```json\r\n{\"n\": 3}\r\n```",
            "> ```py\n> value = 3\n> ```",
            "- note\n\n  ```rs\n  let n = 3;\n  ```",
            "\t```ts\n\tconst n = 3;\n\t```",
            "```typescript\nconst n = 3;",
        ] {
            let spans = highlights(source);
            assert!(
                spans.iter().any(|span| span.kind == HighlightKind::Number
                    && source[span.range.clone()].contains('3')),
                "{source}"
            );
            assert!(
                spans
                    .iter()
                    .all(|span| source.is_char_boundary(span.range.start)
                        && source.is_char_boundary(span.range.end)),
                "{source}"
            );
        }
    }

    #[test]
    fn multiline_parser_state_and_unicode_are_preserved() {
        let source = "```rust\n/* first\n still comment */\nlet café = \"🙂\";\n```";
        let spans = highlights(source);
        for word in ["first", "still comment"] {
            let start = source.find(word).unwrap();
            assert!(spans
                .iter()
                .any(|span| span.kind == HighlightKind::Comment && span.range.contains(&start)));
        }
        assert!(spans
            .iter()
            .any(|span| span.kind == HighlightKind::String
                && source[span.range.clone()].contains('🙂')));
        assert!(spans
            .iter()
            .all(|span| source.get(span.range.clone()).is_some()));
    }

    #[test]
    fn unknown_unlabelled_indented_and_inline_code_stay_plain() {
        for source in [
            "```\nconst n = 3;\n```",
            "```not-a-language\n#!/usr/bin/python\nvalue = 3\n```",
            "    const n = 3;",
            "Text `const n = 3;`",
        ] {
            assert!(highlights(source).is_empty(), "{source}");
        }
        let source = format!(
            "```js\n// {}\n```",
            "x".repeat(crate::syntax::DEFAULT_MAX_SOURCE_BYTES)
        );
        assert!(highlights(&source).is_empty());
    }
}
