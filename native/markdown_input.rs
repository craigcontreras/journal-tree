use std::ops::Range;

use crate::syntax::HighlightSpan;
use gpui::{
    px, Font, FontStyle, FontWeight, Hsla, SharedString, StrikethroughStyle, TextRun,
    UnderlineStyle,
};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

#[napi_derive::napi]
pub fn journal_tree_markdown_version() -> u32 {
    3
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct InlineStyle {
    bold: bool,
    italic: bool,
    code: bool,
    strike: bool,
    link: bool,
    quote: bool,
    heading: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceSpan {
    range: Range<usize>,
    style: InlineStyle,
}

#[derive(Clone, Debug)]
pub(super) struct Construct {
    pub(super) range: Range<usize>,
    pub(super) edits: Vec<Edit>,
}

#[derive(Clone, Debug)]
pub(super) struct Edit {
    pub(super) range: Range<usize>,
    pub(super) text: String,
}

pub(super) fn smart_em_dash(source: &str, cursor: usize) -> Option<Edit> {
    let start = cursor.checked_sub(2)?;
    if source.get(start..cursor)? != "--"
        || source[..start].ends_with('-')
        || source.as_bytes().get(cursor) == Some(&b'-')
    {
        return None;
    }
    let escaped = source[..start]
        .bytes()
        .rev()
        .take_while(|byte| *byte == b'\\')
        .count()
        % 2
        == 1;
    if escaped {
        return None;
    }
    let line_start = source[..start].rfind('\n').map_or(0, |index| index + 1);
    let line_end = source[cursor..]
        .find('\n')
        .map_or(source.len(), |index| cursor + index);
    if source[line_start..line_end]
        .bytes()
        .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'-' | b':' | b'|'))
    {
        return None;
    }
    let (mut prose, mut preceding_text) = (false, false);
    let mut code_depth = 0usize;
    for (event, range) in parser(source) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code_depth += 1,
            Event::End(TagEnd::CodeBlock) => code_depth = code_depth.saturating_sub(1),
            Event::Code(_) if range.start < cursor && range.end > start => return None,
            Event::Text(_) | Event::Code(_) if code_depth == 0 => {
                let before = range.start.max(line_start)..range.end.min(start);
                if before.start < before.end && !source[before].trim().is_empty() {
                    preceding_text = true;
                }
                if range.start <= start && range.end >= cursor {
                    if super::markdown_blocks::bare_links(&source[range.clone()])
                        .iter()
                        .any(|(link, _)| {
                            range.start + link.start < cursor && range.start + link.end > start
                        })
                    {
                        return None;
                    }
                    prose = true;
                }
            }
            _ => {}
        }
    }
    // Without preceding text, '--' must remain available for heading
    // underlines, table separators, and '---' thematic breaks.
    if !prose || !preceding_text {
        return None;
    }
    // A code span being typed has no closing delimiter yet, so the Markdown
    // parser reports it as prose. Keep its contents literal during editing.
    let paragraph = source[..start].rsplit("\n\n").next().unwrap_or("");
    let bytes = paragraph.as_bytes();
    let (mut index, mut delimiter) = (0usize, None);
    while index < bytes.len() {
        if bytes[index] != b'`' {
            index += 1;
            continue;
        }
        let run_start = index;
        while index < bytes.len() && bytes[index] == b'`' {
            index += 1;
        }
        let length = index - run_start;
        if delimiter == Some(length) {
            delimiter = None;
        } else if delimiter.is_none() {
            let slashes = bytes[..run_start]
                .iter()
                .rev()
                .take_while(|byte| **byte == b'\\')
                .count();
            if slashes % 2 == 0 {
                delimiter = Some(length);
            }
        }
    }
    if delimiter.is_some() {
        return None;
    }
    Some(Edit {
        range: start..cursor,
        text: "—".into(),
    })
}

fn raw_parser(source: &str) -> impl Iterator<Item = (Event<'_>, Range<usize>)> {
    Parser::new_ext(
        source,
        Options::ENABLE_TABLES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_GFM,
    )
    .into_offset_iter()
    .map(move |(event, mut range)| {
        // Partial tab expansion can place a nested item's reported start on
        // the preceding newline. Anchor structural ranges at their real marker.
        if matches!(
            &event,
            Event::Start(Tag::List(_) | Tag::Item) | Event::End(TagEnd::List(_) | TagEnd::Item)
        ) {
            let raw = &source[range.clone()];
            range.start += raw.len() - raw.trim_start_matches([' ', '\t', '\r', '\n']).len();
        }
        (event, range)
    })
}

fn editor_events(source: &str) -> Vec<(Event<'_>, Range<usize>)> {
    let mut events = raw_parser(source);
    let mut result = Vec::new();
    while let Some((event, range)) = events.next() {
        if matches!(
            event,
            Event::Start(Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Indented))
        ) {
            let start = source[..range.start]
                .rfind('\n')
                .map_or(0, |index| index + 1);
            let prefix = &source[start..range.start];
            if prefix.bytes().all(|byte| matches!(byte, b' ' | b'\t')) && prefix.contains('\t') {
                // Literal tabs are editor indentation, rather than an implicit
                // code fence. Reparse after removing their shared outer level;
                // keep spaces and map every event back to the original bytes.
                let lines: Vec<_> = source[start..range.end].split_inclusive('\n').collect();
                let levels = lines
                    .iter()
                    .filter_map(|line| {
                        let count = line
                            .bytes()
                            .take_while(|byte| matches!(byte, b' ' | b'\t'))
                            .filter(|byte| *byte == b'\t')
                            .count();
                        (count > 0 && !line.trim().is_empty()).then_some(count)
                    })
                    .min()
                    .unwrap_or(1);
                let (mut normalized, mut positions) = (Vec::new(), Vec::new());
                let mut offset = start;
                for line in lines {
                    let prefix_len = line
                        .bytes()
                        .take_while(|byte| matches!(byte, b' ' | b'\t'))
                        .count();
                    let mut removed = 0;
                    for (index, byte) in line.bytes().enumerate() {
                        if index < prefix_len && byte == b'\t' && removed < levels {
                            removed += 1;
                            continue;
                        }
                        normalized.push(byte);
                        positions.push(offset + index);
                    }
                    offset += line.len();
                }
                let normalized =
                    String::from_utf8(normalized).expect("Removing tabs preserves UTF-8");
                for (child, child_range) in parser(&normalized) {
                    let from = positions
                        .get(child_range.start)
                        .copied()
                        .unwrap_or(range.end);
                    let to = if child_range.is_empty() {
                        from
                    } else {
                        positions[child_range.end - 1] + 1
                    };
                    result.push((child.into_static(), from..to));
                }
                for (event, _) in events.by_ref() {
                    if matches!(event, Event::End(TagEnd::CodeBlock)) {
                        break;
                    }
                }
                continue;
            }
        }
        result.push((event, range));
    }
    result
}

pub(super) fn parser(source: &str) -> std::vec::IntoIter<(Event<'_>, Range<usize>)> {
    let events = editor_events(source);
    let mut list_depth = 0usize;
    let mut empty_bullets = Vec::new();
    for (event, range) in &events {
        match event {
            Event::Start(Tag::List(_)) => list_depth += 1,
            Event::End(TagEnd::List(_)) => list_depth = list_depth.saturating_sub(1),
            Event::Start(Tag::Heading {
                level: pulldown_cmark::HeadingLevel::H2,
                ..
            }) => {
                let end = range.start
                    + source[range.clone()]
                        .trim_end_matches(['\r', '\n', ' ', '\t'])
                        .len();
                let start = source[..end].rfind('\n').map_or(0, |index| index + 1);
                let completed = matches!(source.as_bytes().get(end), Some(b' ' | b'\t'));
                if (list_depth > 0 || completed)
                    && start > range.start
                    && source[start..end].trim_start_matches([' ', '\t']) == "-"
                {
                    empty_bullets.push(end - 1);
                }
            }
            _ => {}
        }
    }
    if empty_bullets.is_empty() {
        return events.into_iter();
    }
    // Inside a list, or after its completing space, a lone '-' is a bullet.
    // CommonMark otherwise reads it as a setext heading underline.
    // '+' has the same byte length and unambiguous list semantics. Only the
    // parser input changes; all offsets and editable source bytes stay intact.
    let mut normalized = source.as_bytes().to_vec();
    for marker in empty_bullets {
        normalized[marker] = b'+';
    }
    let normalized = String::from_utf8(normalized).expect("ASCII replacements preserve UTF-8");
    editor_events(&normalized)
        .into_iter()
        .map(|(event, range)| (event.into_static(), range))
        .collect::<Vec<_>>()
        .into_iter()
}

fn parse_constructs(source: &str) -> Vec<Construct> {
    let mut constructs = Vec::new();
    for (event, range) in parser(source) {
        let marker_len = match event {
            Event::Start(Tag::Strong) => 2,
            Event::Start(Tag::Emphasis) => 1,
            Event::Start(Tag::Strikethrough) => source[range.clone()]
                .bytes()
                .take_while(|byte| *byte == b'~')
                .count(),
            Event::Code(_) => source[range.clone()]
                .bytes()
                .take_while(|byte| *byte == b'`')
                .count(),
            _ => continue,
        };
        if marker_len > 0 && range.len() > marker_len * 2 {
            constructs.push(Construct {
                edits: vec![
                    Edit {
                        range: range.start..range.start + marker_len,
                        text: String::new(),
                    },
                    Edit {
                        range: range.end - marker_len..range.end,
                        text: String::new(),
                    },
                ],
                range,
            });
        }
    }
    constructs
}

#[derive(Debug)]
struct MapPiece {
    source: Range<usize>,
    display: Range<usize>,
    replacement: bool,
}

pub(super) struct Projection {
    pub(super) text: SharedString,
    pieces: Vec<MapPiece>,
    source_len: usize,
    pub(super) images: Vec<super::markdown_blocks::ImageSpec>,
}

impl Projection {
    fn from_edits(source: &str, mut edits: Vec<Edit>) -> Self {
        edits.sort_by_key(|edit| (edit.range.start, std::cmp::Reverse(edit.range.end)));
        let mut text = String::new();
        let mut pieces = Vec::new();
        let mut cursor = 0;
        for edit in edits.into_iter().chain(std::iter::once(Edit {
            range: source.len()..source.len(),
            text: String::new(),
        })) {
            let range = edit.range;
            if range.start < cursor {
                if edit.text.is_empty() {
                    cursor = cursor.max(range.end);
                }
                continue;
            }
            if cursor < range.start {
                let start = text.len();
                text.push_str(&source[cursor..range.start]);
                pieces.push(MapPiece {
                    source: cursor..range.start,
                    display: start..text.len(),
                    replacement: false,
                });
            }
            if !edit.text.is_empty() {
                let start = text.len();
                text.push_str(&edit.text);
                pieces.push(MapPiece {
                    source: range.clone(),
                    display: start..text.len(),
                    replacement: true,
                });
            }
            cursor = cursor.max(range.end);
        }
        Self {
            text: text.into(),
            pieces,
            source_len: source.len(),
            images: Vec::new(),
        }
    }

    pub(super) fn to_display(&self, source: usize) -> usize {
        for piece in &self.pieces {
            if source <= piece.source.start {
                return piece.display.start;
            }
            if source < piece.source.end {
                return if piece.replacement {
                    piece.display.start
                } else {
                    piece.display.start + source - piece.source.start
                };
            }
        }
        self.text.len()
    }

    pub(super) fn to_source(&self, display: usize) -> usize {
        for piece in &self.pieces {
            if display < piece.display.end {
                return if piece.replacement {
                    piece.source.start
                } else {
                    piece.source.start + display.saturating_sub(piece.display.start)
                };
            }
        }
        self.source_len
    }

    pub(super) fn project_runs(&self, runs: Vec<TextRun>) -> Vec<TextRun> {
        let mut projected = Vec::new();
        let mut source_start = 0;
        let mut piece_index = 0;
        for run in runs {
            let source_end = source_start + run.len;
            while piece_index < self.pieces.len()
                && self.pieces[piece_index].source.end <= source_start
            {
                piece_index += 1;
            }
            for piece in &self.pieces[piece_index..] {
                if piece.source.start >= source_end {
                    break;
                }
                let start = piece.source.start.max(source_start);
                let end = piece.source.end.min(source_end);
                if start < end {
                    let mut visible = run.clone();
                    if !piece.replacement || source_start <= piece.source.start {
                        visible.len = if piece.replacement {
                            piece.display.len()
                        } else {
                            end - start
                        };
                        projected.push(visible);
                    }
                }
            }
            source_start = source_end;
        }
        projected
    }
}

fn parse_spans(source: &str) -> Vec<SourceSpan> {
    let mut spans = Vec::new();
    let (mut bold, mut italic, mut code_block, mut strike, mut link, mut quote, mut table) =
        (0usize, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let mut heading = 0;
    let mut checked = Vec::new();
    for (event, range) in parser(source) {
        let code = match event {
            Event::Start(Tag::Strong) => {
                bold += 1;
                continue;
            }
            Event::End(TagEnd::Strong) => {
                bold = bold.saturating_sub(1);
                continue;
            }
            Event::Start(Tag::Emphasis) => {
                italic += 1;
                continue;
            }
            Event::End(TagEnd::Emphasis) => {
                italic = italic.saturating_sub(1);
                continue;
            }
            Event::Start(Tag::Strikethrough) => {
                strike += 1;
                continue;
            }
            Event::End(TagEnd::Strikethrough) => {
                strike = strike.saturating_sub(1);
                continue;
            }
            Event::Start(Tag::Link { .. }) | Event::Start(Tag::Image { .. }) => {
                link += 1;
                continue;
            }
            Event::End(TagEnd::Link) | Event::End(TagEnd::Image) => {
                link = link.saturating_sub(1);
                continue;
            }
            Event::Start(Tag::BlockQuote(_)) => {
                quote += 1;
                continue;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                quote = quote.saturating_sub(1);
                continue;
            }
            Event::Start(Tag::Heading { level, .. }) => {
                heading = level as u8;
                continue;
            }
            Event::End(TagEnd::Heading(_)) => {
                heading = 0;
                continue;
            }
            Event::Start(Tag::Item) => {
                checked.push(false);
                continue;
            }
            Event::End(TagEnd::Item) => {
                checked.pop();
                continue;
            }
            Event::TaskListMarker(value) => {
                if let Some(checked) = checked.last_mut() {
                    *checked = value;
                }
                continue;
            }
            Event::Start(Tag::Table(_)) => {
                table += 1;
                spans.push(SourceSpan {
                    range,
                    style: InlineStyle {
                        code: true,
                        ..InlineStyle::default()
                    },
                });
                continue;
            }
            Event::End(TagEnd::Table) => {
                table = table.saturating_sub(1);
                continue;
            }
            Event::Start(Tag::CodeBlock(_)) => {
                code_block += 1;
                continue;
            }
            Event::End(TagEnd::CodeBlock) => {
                code_block = code_block.saturating_sub(1);
                continue;
            }
            Event::Code(_) if table == 0 => true,
            Event::Text(_) if table == 0 => code_block > 0,
            Event::Html(_) | Event::InlineHtml(_) if table == 0 => true,
            _ => continue,
        };
        let style = InlineStyle {
            bold: bold > 0,
            italic: italic > 0,
            code,
            strike: strike > 0 || checked.iter().any(|value| *value),
            link: link > 0,
            quote: quote > 0,
            heading,
        };
        if !style.code && !style.link {
            let mut cursor = range.start;
            for (link, _) in super::markdown_blocks::bare_links(&source[range.clone()]) {
                let start = range.start + link.start;
                let end = range.start + link.end;
                if cursor < start && style != InlineStyle::default() {
                    spans.push(SourceSpan {
                        range: cursor..start,
                        style,
                    });
                }
                spans.push(SourceSpan {
                    range: start..end,
                    style: InlineStyle {
                        link: true,
                        ..style
                    },
                });
                cursor = end;
            }
            if cursor < range.end && style != InlineStyle::default() {
                spans.push(SourceSpan {
                    range: cursor..range.end,
                    style,
                });
            }
        } else if style != InlineStyle::default() && !range.is_empty() {
            spans.push(SourceSpan { range, style });
        }
    }
    spans
}

#[derive(Default)]
pub(super) struct MarkdownCache {
    source: String,
    spans: Vec<SourceSpan>,
    syntax: Vec<HighlightSpan>,
    constructs: Vec<Construct>,
    blocks: super::markdown_blocks::BlockFeatures,
}

impl MarkdownCache {
    fn update(&mut self, source: &str) {
        if self.source != source {
            self.spans = parse_spans(source);
            self.syntax = super::markdown_syntax::highlights(source);
            self.constructs = parse_constructs(source);
            self.blocks = super::markdown_blocks::parse(source);
            self.constructs
                .extend(self.blocks.constructs.iter().cloned());
            self.source = source.to_owned();
        }
    }

    #[cfg(test)]
    pub(super) fn project(
        &mut self,
        source: &str,
        selection: &Range<usize>,
        marked: Option<&Range<usize>>,
    ) -> Projection {
        self.project_with_rows(source, selection, marked, 0)
    }

    pub(super) fn heading_at(&self, source: usize) -> u8 {
        self.blocks
            .headings
            .iter()
            .find(|(range, _)| range.start <= source && source < range.end)
            .map_or(0, |(_, level)| *level)
    }

    pub(super) fn link_at(&self, source: usize) -> Option<&str> {
        self.blocks
            .links
            .iter()
            .find(|(range, _)| range.start <= source && source <= range.end)
            .map(|(_, url)| url.as_str())
    }

    pub(super) fn has_image(&self, src: &str) -> bool {
        self.blocks.images.iter().any(|image| image.src == src)
    }

    pub(super) fn table_at(&self, source: usize) -> bool {
        self.blocks
            .tables
            .iter()
            .any(|range| range.start <= source && source < range.end)
    }

    pub(super) fn project_with_rows(
        &mut self,
        source: &str,
        selection: &Range<usize>,
        marked: Option<&Range<usize>>,
        image_rows: usize,
    ) -> Projection {
        self.update(source);
        let active = |range: &Range<usize>, selection: &Range<usize>| {
            if selection.is_empty() {
                range.start <= selection.start && selection.start <= range.end
            } else {
                range.start < selection.end && selection.start < range.end
            }
        };
        let mut hidden: Vec<Edit> = self
            .constructs
            .iter()
            .filter(|construct| {
                !active(&construct.range, selection)
                    && !marked.is_some_and(|marked| active(&construct.range, marked))
            })
            .flat_map(|construct| construct.edits.iter().cloned())
            .collect();
        for edit in &mut hidden {
            if let Some(image) = self
                .blocks
                .images
                .iter()
                .find(|image| image.range == edit.range)
            {
                edit.text = format!("{}{}", image.alt, "\n".repeat(image_rows + 1));
            }
        }
        // Completed list prefixes stay bullets while the item is being edited.
        // The source marker remains available to cursor movement and deletion.
        hidden.extend(self.blocks.bullets.iter().cloned());
        // Paint tab stops without rewriting the note. Source/display mapping
        // keeps each tab one editable character, including in code and tables.
        let mut column = 0usize;
        for (index, character) in source.char_indices() {
            match character {
                '\n' | '\r' => column = 0,
                '\t' => {
                    let width = 4 - column % 4;
                    hidden.push(Edit {
                        range: index..index + 1,
                        text: " ".repeat(width),
                    });
                    column += width;
                }
                _ => column += unicode_width::UnicodeWidthChar::width(character).unwrap_or(0),
            }
        }
        let mut view = Projection::from_edits(source, hidden);
        view.images = self
            .blocks
            .images
            .iter()
            .filter(|image| {
                view.pieces
                    .iter()
                    .any(|piece| piece.replacement && piece.source == image.range)
            })
            .cloned()
            .collect();
        view
    }

    #[cfg(test)]
    pub(super) fn runs(
        &mut self,
        source: &str,
        base_font: Font,
        color: Hsla,
        code_font: &SharedString,
        marked: Option<&Range<usize>>,
    ) -> Vec<TextRun> {
        self.runs_themed(
            source,
            base_font,
            color,
            code_font,
            marked,
            &crate::theme::Theme::dark(),
        )
    }

    pub(super) fn runs_themed(
        &mut self,
        source: &str,
        base_font: Font,
        color: Hsla,
        code_font: &SharedString,
        marked: Option<&Range<usize>>,
        theme: &crate::theme::Theme,
    ) -> Vec<TextRun> {
        self.update(source);
        build_runs(
            source,
            &self.spans,
            &self.syntax,
            base_font,
            color,
            code_font,
            marked,
            theme,
        )
    }
}

fn build_runs(
    source: &str,
    spans: &[SourceSpan],
    syntax: &[HighlightSpan],
    base_font: Font,
    color: Hsla,
    code_font: &SharedString,
    marked: Option<&Range<usize>>,
    theme: &crate::theme::Theme,
) -> Vec<TextRun> {
    let mut boundaries = vec![0, source.len()];
    for span in spans {
        boundaries.extend([span.range.start, span.range.end]);
    }
    for span in syntax {
        boundaries.extend([span.range.start, span.range.end]);
    }
    if let Some(marked) = marked {
        boundaries.extend([marked.start, marked.end]);
    }
    boundaries.sort_unstable();
    boundaries.dedup();
    let (mut span_index, mut syntax_index) = (0, 0);
    let mut runs = Vec::with_capacity(boundaries.len().saturating_sub(1));
    for pair in boundaries.windows(2) {
        let range = pair[0]..pair[1];
        if range.is_empty() {
            continue;
        }
        debug_assert!(source.is_char_boundary(range.start) && source.is_char_boundary(range.end));
        while span_index < spans.len() && spans[span_index].range.end <= range.start {
            span_index += 1;
        }
        let style = spans
            .get(span_index)
            .filter(|span| span.range.start <= range.start && range.end <= span.range.end)
            .map(|span| span.style)
            .unwrap_or_default();
        let mut font = base_font.clone();
        if style.code {
            font.family = code_font.clone();
        }
        if (style.bold || style.heading > 0) && font.weight.0 < FontWeight::BOLD.0 {
            font.weight = FontWeight::BOLD;
        }
        if style.italic {
            font.style = FontStyle::Italic;
        }
        let underlined = marked.is_some_and(|m| m.start <= range.start && range.end <= m.end);
        while syntax_index < syntax.len() && syntax[syntax_index].range.end <= range.start {
            syntax_index += 1;
        }
        let token = syntax
            .get(syntax_index)
            .filter(|span| span.range.start <= range.start && range.end <= span.range.end);
        let color = if let Some(token) = token.filter(|_| style.code) {
            theme.syntax.color(token.kind)
        } else if style.link {
            theme.accent
        } else if style.quote {
            theme.text_muted
        } else {
            color
        };
        runs.push(TextRun {
            len: range.len(),
            font,
            color,
            background_color: style.code.then_some(theme.code_wash),
            underline: (underlined || style.link).then_some(UnderlineStyle {
                color: Some(color),
                thickness: px(1.0),
                wavy: false,
            }),
            strikethrough: style.strike.then_some(StrikethroughStyle {
                color: Some(color),
                thickness: px(1.0),
            }),
        });
    }
    runs
}

pub(super) fn shape_lines(
    text: &SharedString,
    runs: &[TextRun],
    base_size: gpui::Pixels,
    base_height: gpui::Pixels,
    width: gpui::Pixels,
    cache: &MarkdownCache,
    map: &Projection,
    metrics: &crate::theme::Metrics,
    window: &mut gpui::Window,
) -> (Vec<gpui::WrappedLine>, Vec<usize>, Vec<gpui::Pixels>) {
    let (mut lines, mut starts, mut heights) = (Vec::new(), Vec::new(), Vec::new());
    let (mut offset, mut run_index, mut run_start) = (0usize, 0usize, 0usize);
    for logical in text.split('\n') {
        let end = offset + logical.len();
        while run_index < runs.len() && run_start + runs[run_index].len <= offset {
            run_start += runs[run_index].len;
            run_index += 1;
        }
        let (mut index, mut start) = (run_index, run_start);
        let mut local_runs = Vec::new();
        while index < runs.len() && start < end {
            let run_end = start + runs[index].len;
            let a = start.max(offset);
            let b = run_end.min(end);
            if a < b {
                let mut run = runs[index].clone();
                run.len = b - a;
                local_runs.push(run);
            }
            start = run_end;
            index += 1;
        }
        let heading = if logical.is_empty() {
            0
        } else {
            cache.heading_at(map.to_source(offset))
        };
        let (font_size, line_height) = if heading > 0 {
            let (font_size, line_height) = metrics.heading(heading);
            (px(font_size), px(line_height))
        } else {
            (base_size, base_height)
        };
        if let Ok(shaped) = window.text_system().shape_text(
            logical.to_owned().into(),
            font_size,
            &local_runs,
            (!cache.table_at(map.to_source(offset))).then_some(width),
            None,
        ) {
            for line in shaped {
                starts.push(offset);
                heights.push(line_height);
                lines.push(line);
            }
        }
        offset = end + 1;
    }
    (lines, starts, heights)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runs(source: &str, marked: Option<&Range<usize>>) -> Vec<TextRun> {
        MarkdownCache::default().runs(
            source,
            gpui::font(".SystemUIFont"),
            gpui::white(),
            &"monospace".into(),
            marked,
        )
    }

    #[test]
    fn fenced_syntax_uses_theme_colours_and_preserves_composition_and_projection() {
        let source = "const prose\n\n```ts\nconst value = 42;\n```\n\nend";
        let mut cache = MarkdownCache::default();
        let mut theme = crate::theme::Theme::dark();
        theme.syntax.keyword = gpui::rgba(0xff0000ff).into();
        let keyword = source.find("const value").unwrap();
        let marked = keyword..keyword + 5;
        let make = |cache: &mut MarkdownCache, theme: &crate::theme::Theme| {
            cache.runs_themed(
                source,
                gpui::font(".SystemUIFont"),
                gpui::white(),
                &"monospace".into(),
                Some(&marked),
                theme,
            )
        };
        let text_runs = make(&mut cache, &theme);
        let mut offset = 0;
        let token = text_runs
            .iter()
            .find(|run| {
                let found = offset <= keyword && keyword < offset + run.len;
                offset += run.len;
                found
            })
            .unwrap();
        assert_eq!(token.color, theme.syntax.keyword);
        assert_eq!(token.font.family.as_ref(), "monospace");
        assert_eq!(token.background_color, Some(theme.code_wash));
        assert!(token.underline.is_some());
        assert_eq!(text_runs.first().unwrap().color, gpui::white());
        assert_eq!(
            text_runs.iter().map(|run| run.len).sum::<usize>(),
            source.len()
        );
        let view = cache.project(source, &(source.len()..source.len()), None);
        assert_eq!(
            view.project_runs(text_runs)
                .iter()
                .map(|run| run.len)
                .sum::<usize>(),
            view.text.len()
        );
        theme.syntax.keyword = gpui::rgba(0x00ff00ff).into();
        assert!(make(&mut cache, &theme)
            .iter()
            .any(|run| run.color == theme.syntax.keyword));
    }

    #[test]
    fn editing_code_or_its_language_refreshes_highlighting() {
        let mut cache = MarkdownCache::default();
        let first = "```js\nconst value = 42;\n```";
        cache.project(first, &(0..0), None);
        assert!(!cache.syntax.is_empty());
        let edited = "```python\nvalue = \"hello\"\n```";
        cache.project(edited, &(0..0), None);
        assert!(cache
            .syntax
            .iter()
            .any(|span| span.kind == crate::syntax::HighlightKind::String
                && edited[span.range.clone()].contains("hello")));
        cache.project("```unknown\nvalue = 42\n```", &(0..0), None);
        assert!(cache.syntax.is_empty());
        cache.project("", &(0..0), None);
        assert!(cache.syntax.is_empty());
    }

    #[test]
    fn completed_bullet_prefixes_stay_rendered_during_editing() {
        let mut cache = MarkdownCache::default();
        for marker in ["-", "*", "+"] {
            assert_eq!(cache.project(marker, &(1..1), None).text.as_ref(), marker);
            let source = format!("{marker} ");
            let view = cache.project(&source, &(2..2), None);
            assert_eq!(view.text.as_ref(), "• ");
            assert_eq!(view.to_source(0), 0);
            assert_eq!(view.to_source(3), 1);
            assert_eq!(view.to_display(2), 4);
            assert_eq!(
                view.project_runs(runs(&source, None))
                    .iter()
                    .map(|run| run.len)
                    .sum::<usize>(),
                4
            );
            let source = format!("{marker} word\n\t{marker} ");
            assert_eq!(
                cache
                    .project(&source, &(source.len()..source.len()), None)
                    .text
                    .as_ref(),
                "• word\n    • "
            );
        }
        for indent in ["", "\t"] {
            let source = format!("Paragraph\n{indent}- ");
            let view = cache.project(&source, &(source.len()..source.len()), None);
            assert_eq!(
                view.text.as_ref(),
                format!("Paragraph\n{}• ", indent.replace('\t', "    "))
            );
            assert!(cache.blocks.headings.is_empty());
        }
        for source in ["```\n- \n```", "    - ", r"\- word", "*italic*", "---"] {
            assert!(
                !cache
                    .project(source, &(source.len()..source.len()), None)
                    .text
                    .contains('•'),
                "{source}"
            );
        }
    }

    #[test]
    fn literal_tab_indentation_keeps_paragraph_and_list_formatting() {
        for source in [
            "\tPlain **bold**",
            "\t\tPlain **bold**",
            "\tFirst\n\tSecond **bold**",
            "\t- **bold**",
            "- parent\n\t\t- **bold**",
        ] {
            let spans = parse_spans(source);
            assert!(spans.iter().all(|span| !span.style.code), "{source}");
            assert!(spans.iter().any(|span| span.style.bold), "{source}");
            let source = format!("{source}\n\nend");
            let view =
                MarkdownCache::default().project(&source, &(source.len()..source.len()), None);
            assert!(
                view.text.contains("bold") && !view.text.contains("**"),
                "{source}"
            );
        }
        for source in [
            "    **literal**",
            "\t    **literal**",
            "```\n\t**literal**\n```",
        ] {
            let spans = parse_spans(source);
            assert!(spans.iter().any(|span| span.style.code), "{source}");
            assert!(spans.iter().all(|span| !span.style.bold), "{source}");
        }
    }

    #[test]
    fn tab_stops_keep_original_byte_offsets_and_text_runs() {
        let source = "\t🙂\ttext\n\nend";
        let mut cache = MarkdownCache::default();
        let view = cache.project(source, &(source.len()..source.len()), None);
        assert_eq!(view.text.as_ref(), "    🙂  text\n\nend");
        assert_eq!(view.to_display(1), 4);
        assert_eq!(view.to_source(4), 1);
        assert_eq!(view.to_display(6), 10);
        assert_eq!(view.to_source(9), 5);
        assert_eq!(view.to_source(view.text.len()), source.len());
        assert_eq!(
            view.project_runs(runs(source, None))
                .iter()
                .map(|run| run.len)
                .sum::<usize>(),
            view.text.len()
        );
    }

    #[test]
    fn smart_dashes_convert_prose_after_text_without_changing_its_surroundings() {
        for source in [
            "hello--",
            "hello --",
            "café 🙂--",
            "**hello** --",
            "- hello --",
            "> hello --",
            "# hello --",
            "`code` --",
            "[hello--](https://example.com)",
            r"\`hello--",
        ] {
            let cursor = source.find("--").unwrap() + 2;
            let edit = smart_em_dash(source, cursor).unwrap_or_else(|| panic!("{source}"));
            let mut result = source.to_owned();
            result.replace_range(edit.range, &edit.text);
            assert_eq!(result, source.replacen("--", "—", 1));
        }
    }

    #[test]
    fn smart_dashes_leave_markdown_structures_code_escapes_and_urls_literal() {
        for source in [
            "--",
            "---",
            "  --",
            "- --",
            "> --",
            "## --",
            "| :-- | --: |",
            "Title\n--",
            "```\nhello--",
            "    hello--",
            "`hello--",
            "`hello--`",
            "``hello--",
            "[link](https://example.com/a--)",
            "Visit https://example.com/a--",
            r"word \--",
            "<b data-x='--'>",
            "word---",
        ] {
            let cursor = source.find("--").unwrap() + 2;
            assert!(smart_em_dash(source, cursor).is_none(), "{source}");
        }
    }

    #[test]
    fn empty_nested_dashes_never_apply_heading_fonts_or_metrics() {
        let mut cache = MarkdownCache::default();
        for source in [
            "- hello\n    -",
            "- hello\n    - ",
            "- hello\n    - hello\n        -",
            "- hello\n    - hello\n        - ",
            "- hello\n    - hello\n        - text",
            "- hello\n    - hello\n        - \n\nend",
        ] {
            cache.project(source, &(source.len()..source.len()), None);
            assert!(cache.blocks.headings.is_empty(), "{source}");
            assert!(
                parse_spans(source)
                    .iter()
                    .all(|span| span.style.heading == 0 && !span.style.bold),
                "{source}"
            );
            assert!(
                runs(source, None)
                    .iter()
                    .all(|run| run.font.weight != FontWeight::BOLD),
                "{source}"
            );
            assert!(
                (0..source.len()).all(|index| cache.heading_at(index) == 0),
                "{source}"
            );
        }
    }

    #[test]
    fn list_disambiguation_preserves_headings_explicit_bold_and_original_markers() {
        for source in ["Title\n-", "Title\n---", "- Title\n  ---", "- ## Title"] {
            assert_eq!(
                super::super::markdown_blocks::parse(source).headings.len(),
                1,
                "{source}"
            );
        }
        let source = "- **bold**\n    - ";
        assert!(parse_spans(source).iter().any(|span| span.style.bold));
        let view = MarkdownCache::default().project(source, &(0..source.len()), None);
        assert_eq!(view.text.as_ref(), "• **bold**\n    • ");
        assert_eq!(view.to_source(view.text.len()), source.len());
        assert!(!view.text.contains('+'));
        let source = "```\n- hello\n    - \n```";
        assert!(parse_spans(source)
            .iter()
            .all(|span| span.style.code && span.style.heading == 0));
    }

    #[test]
    fn nested_emphasis_keeps_source_offsets_and_delimiters() {
        let source = "plain **bold _both_** end";
        let spans = parse_spans(source);
        let both = spans
            .iter()
            .find(|span| &source[span.range.clone()] == "both")
            .unwrap();
        assert!(both.style.bold && both.style.italic);
        assert_eq!(
            runs(source, None).iter().map(|run| run.len).sum::<usize>(),
            source.len()
        );
        assert_eq!(
            runs(source, None).first().unwrap().font.weight,
            FontWeight::NORMAL
        );
    }

    #[test]
    fn inline_code_does_not_parse_emphasis_and_preserves_unicode() {
        let source = "é🙂 `**文字**` *café*";
        let spans = parse_spans(source);
        let code = spans.iter().find(|span| span.style.code).unwrap();
        assert!(!code.style.bold && !code.style.italic);
        assert!(source[code.range.clone()].contains("**文字**"));
        assert!(runs(source, None)
            .iter()
            .any(|run| run.font.family.as_ref() == "monospace"));
        assert_eq!(
            runs(source, None).iter().map(|run| run.len).sum::<usize>(),
            source.len()
        );
    }

    #[test]
    fn escaped_and_unclosed_delimiters_stay_plain() {
        let source = r"\*plain\* and **unfinished";
        assert!(parse_spans(source).is_empty());
        assert_eq!(runs(source, None).len(), 1);
    }

    #[test]
    fn composition_underline_survives_style_boundaries() {
        let source = "a **bold** z";
        let marked = 2..10;
        let runs = runs(source, Some(&marked));
        assert_eq!(
            runs.iter()
                .filter(|run| run.underline.is_some())
                .map(|run| run.len)
                .sum::<usize>(),
            marked.len()
        );
        assert!(runs
            .iter()
            .any(|run| run.font.weight == FontWeight::BOLD && run.underline.is_some()));
        assert_eq!(runs.iter().map(|run| run.len).sum::<usize>(), source.len());
    }

    #[test]
    fn entities_and_line_endings_keep_the_original_byte_count() {
        let source = "**a &amp; é**\r\n_🙂_\n";
        assert_eq!(
            runs(source, None).iter().map(|run| run.len).sum::<usize>(),
            source.len()
        );
    }

    #[test]
    fn fenced_code_does_not_turn_asterisks_into_bold() {
        let source = "```\n**plain**\n```";
        let spans = parse_spans(source);
        assert!(spans.iter().all(|span| span.style.code && !span.style.bold));
    }

    #[test]
    fn reparsing_removes_formatting_after_delimiters_are_deleted() {
        let mut cache = MarkdownCache::default();
        let make = |cache: &mut MarkdownCache, source| {
            cache.runs(
                source,
                gpui::font(".SystemUIFont"),
                gpui::white(),
                &"monospace".into(),
                None,
            )
        };
        assert!(make(&mut cache, "**bold**")
            .iter()
            .any(|run| run.font.weight == FontWeight::BOLD));
        assert!(make(&mut cache, "bold")
            .iter()
            .all(|run| run.font.weight == FontWeight::NORMAL));
    }

    #[test]
    fn empty_document_has_no_runs() {
        assert!(runs("", None).is_empty());
    }

    #[test]
    fn space_and_newline_after_word_hide_markers_without_changing_source() {
        let mut cache = MarkdownCache::default();
        for source in ["**word** ", "**word**\n"] {
            let view = cache.project(source, &(source.len()..source.len()), None);
            assert_eq!(view.text.as_ref(), source.replace("**", ""));
            assert_eq!(view.to_source(view.text.len()), source.len());
        }
    }

    #[test]
    fn cursor_on_word_reveals_both_editable_delimiters() {
        let source = "**word** ";
        let mut cache = MarkdownCache::default();
        let hidden = cache.project(source, &(9..9), None);
        assert_eq!(hidden.text.as_ref(), "word ");
        let source_cursor = hidden.to_source(2);
        assert_eq!(source_cursor, 4);
        let visible = cache.project(source, &(source_cursor..source_cursor), None);
        assert_eq!(visible.text.as_ref(), source);
    }

    #[test]
    fn nested_markers_hide_and_reveal_together() {
        let source = "***both*** ";
        let mut cache = MarkdownCache::default();
        assert_eq!(
            cache
                .project(source, &(source.len()..source.len()), None)
                .text
                .as_ref(),
            "both "
        );
        assert_eq!(cache.project(source, &(4..4), None).text.as_ref(), source);
    }

    #[test]
    fn code_spans_use_matching_backtick_runs() {
        let source = "``a ` b`` ";
        assert_eq!(
            MarkdownCache::default()
                .project(source, &(source.len()..source.len()), None)
                .text
                .as_ref(),
            "a ` b "
        );
    }

    #[test]
    fn selection_and_composition_reveal_syntax() {
        let source = "**word** ";
        let mut cache = MarkdownCache::default();
        assert_eq!(
            cache
                .project(source, &(0..source.len()), None)
                .text
                .as_ref(),
            source
        );
        assert_eq!(
            cache.project(source, &(9..9), Some(&(2..6))).text.as_ref(),
            source
        );
    }

    #[test]
    fn unicode_visible_positions_round_trip_through_hidden_syntax() {
        let source = "é **🙂文字** _café_ ";
        let view = MarkdownCache::default().project(source, &(source.len()..source.len()), None);
        assert_eq!(view.text.as_ref(), "é 🙂文字 café ");
        for (index, _) in view.text.char_indices() {
            let original = view.to_source(index);
            assert!(source.is_char_boundary(original));
            assert_eq!(view.to_display(original), index);
        }
    }

    #[test]
    fn projected_runs_cover_visible_text_with_formatting_intact() {
        let source = "**bold** _italic_ `code` ";
        let mut cache = MarkdownCache::default();
        let view = cache.project(source, &(source.len()..source.len()), None);
        let projected = view.project_runs(runs(source, None));
        assert_eq!(
            projected.iter().map(|run| run.len).sum::<usize>(),
            view.text.len()
        );
        assert!(projected
            .iter()
            .any(|run| run.font.weight == FontWeight::BOLD));
        assert!(projected
            .iter()
            .any(|run| run.font.style == FontStyle::Italic));
        assert!(projected
            .iter()
            .any(|run| run.font.family.as_ref() == "monospace"));
    }

    #[test]
    fn commonmark_corpus_preserves_source_and_valid_display_offsets() {
        let examples: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("commonmark.json")).unwrap();
        assert!(examples.len() >= 650);
        for example in examples {
            let source = format!("{}\n\nend", example["markdown"].as_str().unwrap());
            let mut cache = MarkdownCache::default();
            let view = cache.project(&source, &(source.len()..source.len()), None);
            let projected = view.project_runs(runs(&source, None));
            assert_eq!(
                projected.iter().map(|run| run.len).sum::<usize>(),
                view.text.len(),
                "example {}",
                example["example"]
            );
            for (index, _) in view.text.char_indices() {
                assert!(
                    source.is_char_boundary(view.to_source(index)),
                    "example {}",
                    example["example"]
                );
            }
            assert_eq!(view.to_source(view.text.len()), source.len());
        }
    }
}
