use super::markdown_input::{parser, Construct, Edit};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, Tag, TagEnd};
use std::ops::Range;
use unicode_width::UnicodeWidthStr;

pub(super) struct IndentEdit {
    pub(super) edit: Edit,
    pub(super) selection: Range<usize>,
}

// Only parser-recognized items qualify: a dash inside code, a rule, or an
// escaped marker must keep normal Enter behavior.
pub(super) fn list_newline(source: &str, selection: &Range<usize>) -> Option<Edit> {
    if !selection.is_empty() {
        return None;
    }
    let cursor = selection.start;
    let start = source[..cursor].rfind('\n').map_or(0, |index| index + 1);
    let end = source[cursor..]
        .find('\n')
        .map_or(source.len(), |index| cursor + index);
    if cursor != end {
        return None;
    }
    let line = &source[start..end];
    let indent = line.len() - line.trim_start_matches([' ', '\t']).len();
    let marker = start + indent;
    let (mut item, mut in_list, mut literal) = (false, false, false);
    for (event, range) in parser(source) {
        if matches!(event, Event::Start(Tag::Item)) && range.start >= start && range.start <= marker
        {
            item |= source[range.start..marker]
                .bytes()
                .all(|byte| matches!(byte, b' ' | b'\t'));
        }
        if range.start <= marker && range.end >= cursor {
            in_list |= matches!(event, Event::Start(Tag::List(_)));
            literal |= matches!(
                event,
                Event::Start(Tag::CodeBlock(_) | Tag::HtmlBlock)
                    | Event::Code(_)
                    | Event::Html(_)
                    | Event::InlineHtml(_)
            );
        }
    }
    let rest = &line[indent..];
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let (marker_len, next_marker) = if matches!(rest.as_bytes().first(), Some(b'-' | b'+' | b'*')) {
        (1, rest[..1].to_owned())
    } else if digits > 0 && matches!(rest.as_bytes().get(digits), Some(b'.' | b')')) {
        let number = rest[..digits].parse::<u64>().ok()?;
        (
            digits + 1,
            format!("{}{}", number.checked_add(1)?, &rest[digits..digits + 1]),
        )
    } else {
        return None;
    };
    let after = &rest[marker_len..];
    if !after.is_empty() && !after.starts_with([' ', '\t']) {
        return None;
    }
    let gap = after.len() - after.trim_start_matches([' ', '\t']).len();
    let body = &after[gap..];
    let task = body.starts_with("[ ]") || body.starts_with("[x]") || body.starts_with("[X]");
    let task = task && (body.len() == 3 || body[3..].starts_with([' ', '\t']));
    let empty = if task {
        body[3..].trim().is_empty()
    } else {
        body.trim().is_empty()
    };
    // An empty indented '-' can parse as the parent item's setext underline.
    // Treat it as an empty bullet while editing an existing list, but never
    // apply this fallback to literal code or HTML.
    let completed_bullet = gap > 0 && matches!(rest.as_bytes().first(), Some(b'-' | b'+' | b'*'));
    if !item && !((empty && in_list || completed_bullet) && !literal) {
        return None;
    }
    if empty {
        // Separate the following paragraph from the list so CommonMark does
        // not treat subsequently typed text as a lazy list continuation.
        let needs_break = start > 0
            && !source[..start - 1]
                .rsplit('\n')
                .next()
                .unwrap_or("")
                .trim()
                .is_empty();
        return Some(Edit {
            range: start..end,
            text: if needs_break { "\n" } else { "" }.into(),
        });
    }
    let spacing = if gap == 0 { " " } else { &after[..gap] };
    Some(Edit {
        range: cursor..cursor,
        text: format!(
            "\n{}{next_marker}{spacing}{}",
            &line[..indent],
            if task { "[ ] " } else { "" }
        ),
    })
}

pub(super) fn indent_lines(
    source: &str,
    selection: &Range<usize>,
    outdent: bool,
) -> Option<IndentEdit> {
    let start = source[..selection.start]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    // A selection ending at the next line's start does not include that line.
    let last = if !selection.is_empty() && source.as_bytes().get(selection.end - 1) == Some(&b'\n')
    {
        selection.end - 1
    } else {
        selection.end
    };
    let end = source[last..]
        .find('\n')
        .map_or(source.len(), |index| last + index);
    let mut edits = Vec::new();
    let mut offset = start;
    for line in source[start..end].split('\n') {
        let remove = if outdent {
            if line.starts_with('\t') {
                1
            } else {
                line.bytes()
                    .take(4)
                    .take_while(|byte| *byte == b' ')
                    .count()
            }
        } else {
            0
        };
        if !outdent || remove > 0 {
            edits.push(Edit {
                range: offset..offset + remove,
                text: if outdent { "" } else { "\t" }.into(),
            });
        }
        offset += line.len() + 1;
    }
    if edits.is_empty() {
        return None;
    }
    let remap = |position: usize| {
        let mut delta = 0isize;
        for edit in &edits {
            if position < edit.range.start {
                break;
            }
            if position < edit.range.end {
                return (edit.range.start as isize + delta) as usize;
            }
            delta += edit.text.len() as isize - edit.range.len() as isize;
        }
        (position as isize + delta) as usize
    };
    let new_start = if !selection.is_empty() && selection.start == start {
        start
    } else {
        remap(selection.start)
    };
    let new_end = remap(selection.end);
    let mut text = String::new();
    let mut previous = start;
    for edit in edits {
        text.push_str(&source[previous..edit.range.start]);
        text.push_str(&edit.text);
        previous = edit.range.end;
    }
    text.push_str(&source[previous..end]);
    Some(IndentEdit {
        edit: Edit {
            range: start..end,
            text,
        },
        selection: new_start..new_end,
    })
}

#[derive(Clone, Debug)]
pub(super) struct ImageSpec {
    pub(super) range: Range<usize>,
    pub(super) src: String,
    pub(super) alt: String,
}

#[derive(Default)]
pub(super) struct BlockFeatures {
    pub(super) constructs: Vec<Construct>,
    pub(super) bullets: Vec<Edit>,
    pub(super) headings: Vec<(Range<usize>, u8)>,
    pub(super) images: Vec<ImageSpec>,
    pub(super) links: Vec<(Range<usize>, String)>,
    pub(super) tables: Vec<Range<usize>>,
}

pub(super) fn bare_links(text: &str) -> Vec<(Range<usize>, String)> {
    let mut finder = linkify::LinkFinder::new();
    finder.url_must_have_scheme(false);
    finder
        .links(text)
        .filter_map(|link| {
            let value = link.as_str();
            let lower = value.to_ascii_lowercase();
            let url = if link.kind() == &linkify::LinkKind::Email {
                format!("mailto:{value}")
            } else if lower.starts_with("www.") {
                format!("http://{value}")
            } else if lower.starts_with("https://") || lower.starts_with("http://") {
                value.to_owned()
            } else {
                return None;
            };
            Some((link.start()..link.end(), url))
        })
        .collect()
}

fn change(
    features: &mut BlockFeatures,
    owner: Range<usize>,
    range: Range<usize>,
    text: impl Into<String>,
) {
    features.constructs.push(Construct {
        range: owner,
        edits: vec![Edit {
            range,
            text: text.into(),
        }],
    });
}

pub(super) fn parse(source: &str) -> BlockFeatures {
    let mut features = BlockFeatures::default();
    let mut items = Vec::<Range<usize>>::new();
    let mut lists = Vec::<Option<u64>>::new();
    let mut labels = Vec::<(Range<usize>, String, bool, Option<Range<usize>>, String)>::new();
    let mut table: Option<(Range<usize>, Vec<Alignment>, Vec<Vec<String>>)> = None;
    let mut cell = None;
    let mut code_depth = 0usize;
    for (event, mut range) in parser(source) {
        let is_text = matches!(&event, Event::Text(_));
        if matches!(&event, Event::Start(_) | Event::Rule | Event::HardBreak) {
            while range.end > range.start
                && matches!(source.as_bytes()[range.end - 1], b'\r' | b'\n')
            {
                range.end -= 1;
            }
        }
        match event {
            Event::Start(Tag::List(start)) => {
                lists.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                lists.pop();
            }
            Event::Start(Tag::Heading { level, .. }) => {
                features.headings.push((range.clone(), level as u8));
                let raw = &source[range.clone()];
                let trimmed = raw.trim_start_matches(' ');
                if trimmed.starts_with('#') {
                    let count = trimmed.bytes().take_while(|byte| *byte == b'#').count();
                    let content = trimmed[count..].trim_start_matches([' ', '\t']);
                    let start = range.start + raw.len() - content.len();
                    change(&mut features, range.clone(), range.start..start, "");
                    let end = raw.trim_end_matches(['\n', '\r', ' ', '\t']);
                    let without_hash = end.trim_end_matches('#');
                    if without_hash.len() != end.len() && without_hash.ends_with([' ', '\t']) {
                        change(
                            &mut features,
                            range.clone(),
                            range.start + without_hash.trim_end_matches([' ', '\t']).len()
                                ..range.start + end.len(),
                            "",
                        );
                    }
                } else if let Some(newline) = raw.trim_end_matches(['\r', '\n']).rfind('\n') {
                    change(
                        &mut features,
                        range.clone(),
                        range.start + newline + 1
                            ..range.start + raw.trim_end_matches(['\r', '\n']).len(),
                        "",
                    );
                }
            }
            Event::Start(Tag::BlockQuote(_)) => {
                let mut offset = range.start;
                for line in source[range.clone()].split_inclusive('\n') {
                    let indent = line.len() - line.trim_start_matches(' ').len();
                    if line[indent..].starts_with('>') {
                        change(
                            &mut features,
                            offset..offset + line.trim_end_matches(['\r', '\n']).len(),
                            offset + indent..offset + indent + 1,
                            "│",
                        );
                    }
                    offset += line.len();
                }
            }
            Event::Start(Tag::Item) => {
                items.push(range.clone());
                let raw = &source[range.clone()];
                let indent = raw.len() - raw.trim_start_matches([' ', '\t']).len();
                if matches!(raw.as_bytes().get(indent), Some(b'-' | b'+' | b'*')) {
                    if matches!(raw.as_bytes().get(indent + 1), Some(b' ' | b'\t')) {
                        features.bullets.push(Edit {
                            range: range.start + indent..range.start + indent + 1,
                            text: "•".into(),
                        });
                    }
                } else if let Some(Some(number)) = lists.last_mut() {
                    let digits = raw[indent..].bytes().take_while(u8::is_ascii_digit).count();
                    if digits > 0 {
                        change(
                            &mut features,
                            range.clone(),
                            range.start + indent..range.start + indent + digits + 1,
                            format!("{number}."),
                        );
                        *number += 1;
                    }
                }
            }
            Event::End(TagEnd::Item) => {
                items.pop();
            }
            Event::TaskListMarker(checked) => {
                change(
                    &mut features,
                    items.last().cloned().unwrap_or(range.clone()),
                    range,
                    if checked { "☑" } else { "☐" },
                );
            }
            Event::Rule => {
                let end = range.start + source[range.clone()].trim_end_matches(['\r', '\n']).len();
                change(
                    &mut features,
                    range.clone(),
                    range.start..end,
                    "────────────────────────────────",
                );
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_))) => {
                code_depth += 1;
                let raw = &source[range.clone()];
                if let Some(newline) = raw.find('\n') {
                    change(
                        &mut features,
                        range.clone(),
                        range.start..range.start + newline,
                        "",
                    );
                    let end = raw.trim_end_matches(['\r', '\n']);
                    if let Some(last) = end.rfind('\n') {
                        let tail = end[last + 1..].trim();
                        if tail.starts_with("```") || tail.starts_with("~~~") {
                            change(
                                &mut features,
                                range.clone(),
                                range.start + last + 1..range.start + end.len(),
                                "",
                            );
                        }
                    }
                }
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Indented)) => {
                code_depth += 1;
                let start = source[..range.start]
                    .rfind('\n')
                    .map_or(0, |index| index + 1);
                let owner = start..range.end;
                let mut offset = start;
                for line in source[owner.clone()].split_inclusive('\n') {
                    let (mut columns, mut bytes) = (0usize, 0usize);
                    for byte in line.bytes() {
                        match byte {
                            b' ' => columns += 1,
                            b'\t' => columns += 4 - columns % 4,
                            _ => break,
                        }
                        bytes += 1;
                        if columns >= 4 {
                            break;
                        }
                    }
                    if bytes > 0 {
                        change(&mut features, owner.clone(), offset..offset + bytes, "");
                    }
                    offset += line.len();
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                code_depth = code_depth.saturating_sub(1);
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                features.links.push((range.clone(), dest_url.to_string()));
                labels.push((range, dest_url.to_string(), false, None, String::new()));
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                labels.push((range, dest_url.to_string(), true, None, String::new()));
            }
            Event::End(TagEnd::Link) | Event::End(TagEnd::Image) => {
                if let Some((owner, url, image, label, alt)) = labels.pop() {
                    if image {
                        features.images.push(ImageSpec {
                            range: owner.clone(),
                            src: url,
                            alt: alt.clone(),
                        });
                        change(&mut features, owner.clone(), owner, alt);
                    } else if let Some(label) = label {
                        change(&mut features, owner.clone(), owner.start..label.start, "");
                        change(&mut features, owner.clone(), label.end..owner.end, "");
                    } else {
                        change(&mut features, owner.clone(), owner, "");
                    }
                }
            }
            Event::Start(Tag::Table(align)) => {
                features.tables.push(range.clone());
                table = Some((range, align, Vec::new()));
            }
            Event::Start(Tag::TableHead) | Event::Start(Tag::TableRow) => {
                if let Some((_, _, rows)) = &mut table {
                    rows.push(Vec::new());
                }
            }
            Event::Start(Tag::TableCell) => {
                cell = Some(String::new());
            }
            Event::End(TagEnd::TableCell) => {
                if let (Some((_, _, rows)), Some(value)) = (&mut table, cell.take()) {
                    if let Some(row) = rows.last_mut() {
                        row.push(value);
                    }
                }
            }
            Event::End(TagEnd::Table) => {
                if let Some((owner, align, rows)) = table.take() {
                    change(
                        &mut features,
                        owner.clone(),
                        owner,
                        table_grid(&rows, &align),
                    );
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if is_text && code_depth == 0 && labels.is_empty() {
                    // An indented marker can be reported as lazy paragraph
                    // text, especially around partial tab stops.
                    // It is still an editable bullet prefix, never literal code.
                    let line_start = source[..range.start]
                        .rfind('\n')
                        .map_or(0, |index| index + 1);
                    let prefix = &source[line_start..range.start];
                    if prefix.bytes().all(|byte| matches!(byte, b' ' | b'\t'))
                        && matches!(source.as_bytes().get(range.start), Some(b'-' | b'+' | b'*'))
                        && matches!(source.as_bytes().get(range.start + 1), Some(b' ' | b'\t'))
                    {
                        features.bullets.push(Edit {
                            range: range.start..range.start + 1,
                            text: "•".into(),
                        });
                    }
                    for (link, url) in bare_links(&source[range.clone()]) {
                        features
                            .links
                            .push((range.start + link.start..range.start + link.end, url));
                    }
                }
                if let Some(cell) = &mut cell {
                    cell.push_str(&text);
                }
                for (_, _, _, label, alt) in &mut labels {
                    if let Some(label) = label {
                        label.end = range.end;
                    } else {
                        *label = Some(range.clone());
                    }
                    alt.push_str(&text);
                }
                let raw = &source[range.clone()];
                let escaped = source[..range.start]
                    .bytes()
                    .rev()
                    .take_while(|byte| *byte == b'\\')
                    .count()
                    % 2
                    == 1
                    && text
                        .chars()
                        .next()
                        .is_some_and(|character| character.is_ascii_punctuation());
                if escaped {
                    let escaped_range = range.start - 1..range.end;
                    change(
                        &mut features,
                        escaped_range.clone(),
                        escaped_range,
                        text.to_string(),
                    );
                    continue;
                }
                if (raw.contains('\\') || raw.contains('&') || raw.contains('\0'))
                    && raw != text.as_ref()
                {
                    change(&mut features, range.clone(), range, text.to_string());
                }
            }
            Event::HardBreak => {
                let end = range.start + source[range.clone()].trim_end_matches(['\r', '\n']).len();
                change(&mut features, range.clone(), range.start..end, "");
            }
            _ => {}
        }
    }
    let parsed = pulldown_cmark::Parser::new(source);
    for (_, definition) in parsed.reference_definitions().iter() {
        let mut range = definition.span.clone();
        while range.end > range.start && matches!(source.as_bytes()[range.end - 1], b'\r' | b'\n') {
            range.end -= 1;
        }
        change(&mut features, range.clone(), range, "");
    }
    features
}

fn table_grid(rows: &[Vec<String>], align: &[Alignment]) -> String {
    let widths: Vec<usize> = (0..align.len())
        .map(|column| {
            rows.iter()
                .filter_map(|row| row.get(column))
                .map(|value| UnicodeWidthStr::width(value.as_str()))
                .max()
                .unwrap_or(1)
                .max(1)
        })
        .collect();
    let rule = format!(
        "├{}┤\n",
        widths
            .iter()
            .map(|width| "─".repeat(width + 2))
            .collect::<Vec<_>>()
            .join("┼")
    );
    let mut result = String::new();
    for (index, row) in rows.iter().enumerate() {
        result.push('│');
        for (column, width) in widths.iter().enumerate() {
            let value = row.get(column).map_or("", |value| value.as_str());
            let padding = width.saturating_sub(UnicodeWidthStr::width(value));
            let left = match align[column] {
                Alignment::Right => padding,
                Alignment::Center => padding / 2,
                _ => 0,
            };
            result.push_str(&format!(
                " {}{}{} │",
                " ".repeat(left),
                value,
                " ".repeat(padding - left)
            ));
        }
        result.push('\n');
        if index == 0 {
            result.push_str(&rule);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::super::markdown_input::MarkdownCache;
    use super::*;

    fn rendered(source: &str) -> String {
        let source = format!("{source}\n\nend");
        MarkdownCache::default()
            .project(&source, &(source.len()..source.len()), None)
            .text
            .to_string()
    }

    #[test]
    fn enter_continues_list_style_nesting_and_tasks() {
        for (source, expected) in [
            ("- first", "\n- "),
            ("+ first", "\n+ "),
            ("* first", "\n* "),
            ("- parent\n    - café 🙂", "\n    - "),
            ("- [x] completed", "\n- [ ] "),
            ("9) item", "\n10) "),
        ] {
            let edit = list_newline(source, &(source.len()..source.len())).unwrap();
            assert_eq!(edit.text, expected, "{source}");
            assert_eq!(edit.range, source.len()..source.len());
        }
    }

    #[test]
    fn enter_removes_empty_items_and_their_indentation() {
        for source in [
            "- ",
            "*",
            "- [ ] ",
            "- parent\n    - ",
            "- parent\n    - [x] ",
        ] {
            let edit = list_newline(source, &(source.len()..source.len())).unwrap();
            assert_eq!(edit.text, if source.contains('\n') { "\n" } else { "" });
            assert_eq!(edit.range.end, source.len());
            assert_eq!(
                edit.range.start,
                source.rfind('\n').map_or(0, |index| index + 1)
            );
        }
    }

    #[test]
    fn list_enter_does_not_change_code_rules_selections_or_midline_edits() {
        for source in [
            "```\n- literal",
            "    - code",
            "- parent\n\n      - ",
            "---",
            "\\- escaped",
            "ordinary text",
        ] {
            assert!(
                list_newline(source, &(source.len()..source.len())).is_none(),
                "{source}"
            );
        }
        assert!(list_newline("- first", &(4..4)).is_none());
        assert!(list_newline("- first", &(2..7)).is_none());
    }

    #[test]
    fn indenting_and_outdenting_preserve_unicode_caret_positions() {
        let source = "- parent\n- café 🙂";
        let change = indent_lines(source, &(source.len()..source.len()), false).unwrap();
        let mut indented = source.to_owned();
        indented.replace_range(change.edit.range, &change.edit.text);
        assert_eq!(indented, "- parent\n\t- café 🙂");
        assert_eq!(change.selection, indented.len()..indented.len());
        assert!(rendered(&indented).contains("\n    • café 🙂"));
        let change = indent_lines(&indented, &change.selection, true).unwrap();
        indented.replace_range(change.edit.range, &change.edit.text);
        assert_eq!(indented, source);
        assert_eq!(change.selection, source.len()..source.len());
    }

    #[test]
    fn indent_selection_excludes_the_line_after_its_trailing_newline() {
        let source = "- first\n- second\n- third";
        let selected = 0..source.find("- third").unwrap();
        let change = indent_lines(source, &selected, false).unwrap();
        let mut result = source.to_owned();
        result.replace_range(change.edit.range, &change.edit.text);
        assert_eq!(result, "\t- first\n\t- second\n- third");
        assert_eq!(change.selection, 0..selected.end + 2);
        let change = indent_lines(&result, &change.selection, true).unwrap();
        result.replace_range(change.edit.range, &change.edit.text);
        assert_eq!(result, source);
        assert_eq!(change.selection, selected);
        assert!(indent_lines(source, &(0..0), true).is_none());
        let change = indent_lines("\t- child", &(8..8), true).unwrap();
        assert_eq!(change.edit.text, "- child");
        assert_eq!(change.selection, 7..7);
    }

    #[test]
    fn heading_forms_have_levels_and_hide_their_markers() {
        assert_eq!(
            parse("# One\n## Two\nTitle\n=====\n")
                .headings
                .iter()
                .map(|(_, level)| *level)
                .collect::<Vec<_>>(),
            vec![1, 2, 1]
        );
        assert!(rendered("# Title ###").starts_with("Title"));
        assert!(!rendered("Title\n=====").contains('='));
    }

    #[test]
    fn nested_lists_quotes_and_tasks_keep_structure() {
        let view = rendered("> quoted\n\n- first\n  - nested\n- [x] done\n\n3. third\n1. fourth");
        assert!(view.contains("│ quoted"));
        assert!(view.contains("• first") && view.contains("• nested"));
        assert!(view.contains("☑ done"));
        assert!(view.contains("3. third") && view.contains("4. fourth"));
    }

    #[test]
    fn links_references_and_autolinks_show_labels_and_preserve_destinations() {
        let source = "[label](https://example.com) and <https://example.org> and [ref][id]\n\n[id]: https://example.net";
        let view = rendered(source);
        assert!(view.contains("label and https://example.org and ref"));
        assert!(!view.contains("example.net"));
        let features = parse(source);
        assert_eq!(features.links.len(), 3);
        assert_eq!(features.links[2].1, "https://example.net");
    }

    #[test]
    fn fenced_code_preserves_its_contents_and_hides_only_fences() {
        let view = rendered("```rust\nlet x = \"**literal**\";\n```");
        assert!(view.contains("**literal**") && !view.contains("```"));
    }

    #[test]
    fn tables_render_all_columns_and_rows_with_alignment() {
        let view = rendered("| A | B |\n| :-- | --: |\n| left | right |");
        assert!(view.contains("│ A    │     B │"));
        assert!(view.contains("│ left │ right │") && view.contains('┼'));
    }

    #[test]
    fn entities_escapes_and_rules_have_native_presentation() {
        assert!(rendered("a &amp; b \\*literal\\*\n\n---").contains("a & b *literal*"));
        assert!(rendered("---").contains('─'));
    }

    #[test]
    fn image_source_and_caption_are_preserved_in_native_preview() {
        let source = "![caption](image.png)\n\nend";
        let mut cache = MarkdownCache::default();
        let view = cache.project_with_rows(source, &(source.len()..source.len()), None, 4);
        assert_eq!(view.images.len(), 1);
        assert_eq!(view.images[0].src, "image.png");
        assert!(view.text.starts_with("caption\n\n\n\n\n"));
        assert_eq!(cache.project(source, &(4..4), None).text.as_ref(), source);
    }

    #[test]
    fn bare_gfm_urls_and_emails_are_links_but_code_is_not() {
        let source = "Visit https://example.com, www.example.org and a@example.net. `https://code.test`\n\n    https://literal.test";
        let features = parse(source);
        assert_eq!(features.links.len(), 3);
        assert!(features
            .links
            .iter()
            .any(|(_, url)| url == "mailto:a@example.net"));
        assert!(features
            .links
            .iter()
            .any(|(_, url)| url == "http://www.example.org"));
    }

    #[test]
    fn indented_code_hides_its_indent_and_preserves_code_characters() {
        let view = rendered("    **literal**\n    second");
        assert!(view.starts_with("**literal**\nsecond"));
    }

    #[test]
    fn table_padding_accounts_for_wide_unicode_characters() {
        let grid = table_grid(&[vec!["文字".into()], vec!["a".into()]], &[Alignment::Left]);
        assert!(grid.contains("│ a    │"));
    }
}
