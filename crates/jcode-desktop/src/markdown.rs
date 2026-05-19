use crate::single_session::{
    SingleSessionLineStyle, SingleSessionStyledLine, blank_styled_line, styled_line,
};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Tag, TagEnd};

#[derive(Default)]
pub(super) struct AssistantMarkdownRenderer {
    lines: Vec<SingleSessionStyledLine>,
    current: String,
    current_style: SingleSessionLineStyle,
    line_style_override: Option<SingleSessionLineStyle>,
    quote_depth: usize,
    list_stack: Vec<AssistantMarkdownList>,
    item_continuation_prefixes: Vec<String>,
    pending_line_prefix: String,
    continuation_prefix: String,
    in_code_block: bool,
    table: Option<AssistantMarkdownTable>,
    image_stack: Vec<AssistantMarkdownImage>,
    link_stack: Vec<AssistantMarkdownLink>,
}

#[derive(Clone, Debug)]
struct AssistantMarkdownList {
    next_number: Option<u64>,
}

#[derive(Clone, Debug)]
struct AssistantMarkdownLink {
    dest_url: String,
    start_byte: usize,
}

#[derive(Clone, Debug, Default)]
struct AssistantMarkdownImage {
    dest_url: String,
    alt_text: String,
}

#[derive(Clone, Debug, Default)]
struct AssistantMarkdownTable {
    rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    current_cell: String,
    header_rows: usize,
}

impl Default for SingleSessionLineStyle {
    fn default() -> Self {
        Self::Assistant
    }
}

impl AssistantMarkdownRenderer {
    pub(super) fn handle_event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => self.start_heading(level),
            Event::End(TagEnd::Heading(_)) => self.end_heading(),
            Event::Start(Tag::Paragraph) => self.start_paragraph(),
            Event::End(TagEnd::Paragraph) => self.end_paragraph(),
            Event::Start(Tag::BlockQuote(_)) => self.start_block_quote(),
            Event::End(TagEnd::BlockQuote(_)) => self.end_block_quote(),
            Event::Start(Tag::List(start)) => self.start_list(start),
            Event::End(TagEnd::List(_)) => self.end_list(),
            Event::Start(Tag::Item) => self.start_list_item(),
            Event::End(TagEnd::Item) => self.end_list_item(),
            Event::TaskListMarker(checked) => self.apply_task_marker(checked),
            Event::Start(Tag::CodeBlock(kind)) => self.start_code_block(kind),
            Event::End(TagEnd::CodeBlock) => self.end_code_block(),
            Event::Start(Tag::Table(_)) => self.start_table(),
            Event::End(TagEnd::Table) => self.end_table(),
            Event::Start(Tag::TableHead) => self.start_table_head(),
            Event::End(TagEnd::TableHead) => self.end_table_head(),
            Event::Start(Tag::TableRow) => self.start_table_row(),
            Event::End(TagEnd::TableRow) => self.end_table_row(),
            Event::Start(Tag::TableCell) => self.start_table_cell(),
            Event::End(TagEnd::TableCell) => self.end_table_cell(),
            Event::Start(Tag::Link { dest_url, .. }) => self.start_link(dest_url.as_ref()),
            Event::End(TagEnd::Link) => self.end_link(),
            Event::Start(Tag::Image { dest_url, .. }) => self.start_image(dest_url.as_ref()),
            Event::End(TagEnd::Image) => self.end_image(),
            Event::Start(Tag::Emphasis | Tag::Strong | Tag::Strikethrough) => {}
            Event::End(TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough) => {}
            Event::Text(text) => self.push_text(text.as_ref()),
            Event::Code(code) => self.push_inline_code(code.as_ref()),
            Event::SoftBreak => self.soft_break(),
            Event::HardBreak => self.hard_break(),
            Event::Rule => self.rule(),
            Event::Html(html) | Event::InlineHtml(html) => self.push_text(html.as_ref()),
            Event::FootnoteReference(name) => {
                self.push_text("[");
                self.push_text(name.as_ref());
                self.push_text("]");
            }
            _ => {}
        }
    }

    pub(super) fn finish(mut self) -> Vec<SingleSessionStyledLine> {
        self.flush_current_line();
        if self
            .lines
            .last()
            .is_some_and(|line| line.style == SingleSessionLineStyle::Blank)
        {
            self.lines.pop();
        }
        self.lines
    }

    fn start_heading(&mut self, level: HeadingLevel) {
        self.flush_current_line();
        self.ensure_block_gap();
        self.current_style = SingleSessionLineStyle::AssistantHeading;
        self.pending_line_prefix = heading_prefix(level).to_string();
    }

    fn end_heading(&mut self) {
        self.flush_current_line_as(SingleSessionLineStyle::AssistantHeading);
        self.current_style = self.prose_style();
        self.pending_line_prefix.clear();
    }

    fn start_paragraph(&mut self) {
        if self.list_stack.is_empty() && self.quote_depth == 0 {
            self.ensure_block_gap();
        }
        self.current_style = self.prose_style();
    }

    fn end_paragraph(&mut self) {
        self.flush_current_line();
        if !self.item_continuation_prefixes.is_empty() {
            self.pending_line_prefix = self.continuation_prefix.clone();
        }
    }

    fn start_block_quote(&mut self) {
        self.flush_current_line();
        self.ensure_block_gap();
        self.quote_depth += 1;
        self.current_style = SingleSessionLineStyle::AssistantQuote;
    }

    fn end_block_quote(&mut self) {
        self.flush_current_line_as(SingleSessionLineStyle::AssistantQuote);
        self.quote_depth = self.quote_depth.saturating_sub(1);
        self.current_style = self.prose_style();
        self.pending_line_prefix.clear();
        self.continuation_prefix.clear();
    }

    fn start_list(&mut self, start: Option<u64>) {
        self.flush_current_line();
        if self.list_stack.is_empty() && self.quote_depth == 0 {
            self.ensure_block_gap();
        }
        self.list_stack
            .push(AssistantMarkdownList { next_number: start });
    }

    fn end_list(&mut self) {
        self.flush_current_line();
        self.list_stack.pop();
        if self.list_stack.is_empty() {
            self.pending_line_prefix.clear();
            self.continuation_prefix.clear();
            self.item_continuation_prefixes.clear();
        }
    }

    fn start_list_item(&mut self) {
        self.flush_current_line();
        let (prefix, continuation) = self.list_item_prefix(false);
        self.pending_line_prefix = prefix;
        self.continuation_prefix = continuation.clone();
        self.item_continuation_prefixes.push(continuation);
        self.current_style = self.prose_style();
    }

    fn end_list_item(&mut self) {
        self.flush_current_line();
        self.item_continuation_prefixes.pop();
        self.continuation_prefix = self
            .item_continuation_prefixes
            .last()
            .cloned()
            .unwrap_or_default();
        self.pending_line_prefix.clear();
    }

    fn apply_task_marker(&mut self, checked: bool) {
        let (prefix, continuation) = self.task_item_prefix(checked);
        if self.current.is_empty() {
            self.pending_line_prefix = prefix;
            self.continuation_prefix = continuation.clone();
            if let Some(last) = self.item_continuation_prefixes.last_mut() {
                *last = continuation;
            }
        } else {
            self.current.push_str(if checked { "✓ " } else { "☐ " });
        }
    }

    fn start_code_block(&mut self, kind: CodeBlockKind<'_>) {
        self.flush_current_line();
        self.ensure_block_gap();
        self.in_code_block = true;
        if let CodeBlockKind::Fenced(language) = kind {
            let language = language.as_ref().trim();
            if !language.is_empty() {
                self.lines.push(styled_line(
                    format!("  {language}"),
                    SingleSessionLineStyle::Code,
                ));
            }
        }
    }

    fn end_code_block(&mut self) {
        self.in_code_block = false;
    }

    fn start_table(&mut self) {
        self.flush_current_line();
        self.ensure_block_gap();
        self.table = Some(AssistantMarkdownTable::default());
    }

    fn end_table(&mut self) {
        if let Some(table) = self.table.take() {
            self.render_table(table);
        }
    }

    fn start_table_head(&mut self) {}

    fn end_table_head(&mut self) {
        if let Some(table) = &mut self.table {
            if !table.current_cell.trim().is_empty() {
                table.finish_cell();
            }
            table.finish_row();
            table.header_rows = table.rows.len();
        }
    }

    fn start_table_row(&mut self) {
        if let Some(table) = &mut self.table {
            table.current_row.clear();
        }
    }

    fn end_table_row(&mut self) {
        if let Some(table) = &mut self.table {
            if !table.current_cell.trim().is_empty() {
                table.finish_cell();
            }
            table.finish_row();
        }
    }

    fn start_table_cell(&mut self) {
        if let Some(table) = &mut self.table {
            table.current_cell.clear();
        }
    }

    fn end_table_cell(&mut self) {
        if let Some(table) = &mut self.table {
            table.finish_cell();
        }
    }

    fn start_link(&mut self, dest_url: &str) {
        self.begin_line_if_needed();
        self.link_stack.push(AssistantMarkdownLink {
            dest_url: dest_url.to_string(),
            start_byte: self.current.len(),
        });
    }

    fn end_link(&mut self) {
        let Some(link) = self.link_stack.pop() else {
            return;
        };
        if link.dest_url.is_empty() {
            return;
        }
        self.begin_line_if_needed();
        let label = self
            .current
            .get(link.start_byte..)
            .unwrap_or_default()
            .trim();
        if !label.contains(&link.dest_url) {
            self.current.push_str(" ↗ ");
            self.current.push_str(&link.dest_url);
        }
        if self.current_style == SingleSessionLineStyle::Assistant {
            self.line_style_override = Some(SingleSessionLineStyle::AssistantLink);
        }
    }

    fn start_image(&mut self, dest_url: &str) {
        self.image_stack.push(AssistantMarkdownImage {
            dest_url: dest_url.to_string(),
            alt_text: String::new(),
        });
    }

    fn end_image(&mut self) {
        let Some(image) = self.image_stack.pop() else {
            return;
        };
        self.begin_line_if_needed();
        let alt = image.alt_text.trim();
        if alt.is_empty() {
            self.current.push_str("image");
        } else {
            self.current.push_str("image: ");
            self.current.push_str(alt);
        }
        if !image.dest_url.is_empty() {
            self.current.push_str(" ↗ ");
            self.current.push_str(&image.dest_url);
        }
        if self.current_style == SingleSessionLineStyle::Assistant {
            self.line_style_override = Some(SingleSessionLineStyle::AssistantLink);
        }
    }

    fn push_text(&mut self, text: &str) {
        if let Some(image) = self.image_stack.last_mut() {
            image.alt_text.push_str(text);
            return;
        }
        if let Some(table) = &mut self.table {
            table.push_text(text);
            return;
        }
        if self.in_code_block {
            self.push_code_text(text);
            return;
        }
        self.begin_line_if_needed();
        self.current.push_str(&text.replace('\n', " "));
    }

    fn push_inline_code(&mut self, code: &str) {
        if let Some(table) = &mut self.table {
            table.push_text("`");
            table.push_text(code);
            table.push_text("`");
            return;
        }
        self.begin_line_if_needed();
        self.current.push('`');
        self.current.push_str(code);
        self.current.push('`');
    }

    fn soft_break(&mut self) {
        if let Some(table) = &mut self.table {
            table.push_space();
            return;
        }
        if self.in_code_block {
            self.lines
                .push(styled_line("  ", SingleSessionLineStyle::Code));
            return;
        }
        self.push_space();
    }

    fn hard_break(&mut self) {
        if let Some(table) = &mut self.table {
            table.push_space();
            return;
        }
        self.flush_current_line();
        if !self.continuation_prefix.is_empty() {
            self.pending_line_prefix = self.continuation_prefix.clone();
        } else if self.quote_depth > 0 {
            self.pending_line_prefix = self.quote_prefix();
        }
    }

    fn rule(&mut self) {
        self.flush_current_line();
        self.ensure_block_gap();
        self.lines
            .push(styled_line("────────────", SingleSessionLineStyle::Meta));
    }

    fn begin_line_if_needed(&mut self) {
        if !self.current.is_empty() {
            return;
        }
        if !self.pending_line_prefix.is_empty() {
            self.current.push_str(&self.pending_line_prefix);
            self.pending_line_prefix.clear();
            return;
        }
        if self.quote_depth > 0 {
            self.current.push_str(&self.quote_prefix());
        }
    }

    fn push_space(&mut self) {
        self.begin_line_if_needed();
        if !self.current.chars().last().is_some_and(char::is_whitespace) {
            self.current.push(' ');
        }
    }

    fn push_code_text(&mut self, text: &str) {
        if text.is_empty() {
            self.lines
                .push(styled_line("  ", SingleSessionLineStyle::Code));
            return;
        }
        for line in text.lines() {
            self.lines.push(styled_line(
                format!("  {line}"),
                SingleSessionLineStyle::Code,
            ));
        }
    }

    fn flush_current_line(&mut self) {
        let style = self
            .line_style_override
            .take()
            .unwrap_or(self.current_style);
        self.flush_current_line_as(style);
    }

    fn flush_current_line_as(&mut self, style: SingleSessionLineStyle) {
        let trimmed = self.current.trim_end();
        if !trimmed.is_empty() {
            self.lines.push(styled_line(trimmed, style));
        }
        self.current.clear();
        self.line_style_override = None;
    }

    fn ensure_block_gap(&mut self) {
        if self
            .lines
            .last()
            .is_some_and(|line| line.style != SingleSessionLineStyle::Blank)
        {
            self.lines.push(blank_styled_line());
        }
    }

    fn prose_style(&self) -> SingleSessionLineStyle {
        if self.quote_depth > 0 {
            SingleSessionLineStyle::AssistantQuote
        } else {
            SingleSessionLineStyle::Assistant
        }
    }

    fn quote_prefix(&self) -> String {
        "│ ".repeat(self.quote_depth)
    }

    fn list_item_prefix(&mut self, task: bool) -> (String, String) {
        let quote_prefix = self.quote_prefix();
        let depth = self.list_stack.len().saturating_sub(1);
        let indent = "  ".repeat(depth);
        let marker = if task {
            "☐ ".to_string()
        } else if let Some(list) = self.list_stack.last_mut() {
            if let Some(next_number) = &mut list.next_number {
                let marker = format!("{next_number}. ");
                *next_number += 1;
                marker
            } else {
                bullet_for_depth(depth).to_string()
            }
        } else {
            "• ".to_string()
        };
        let continuation = format!(
            "{quote_prefix}{indent}{}",
            " ".repeat(marker.chars().count())
        );
        (format!("{quote_prefix}{indent}{marker}"), continuation)
    }

    fn task_item_prefix(&self, checked: bool) -> (String, String) {
        let quote_prefix = self.quote_prefix();
        let depth = self.list_stack.len().saturating_sub(1);
        let indent = "  ".repeat(depth);
        let marker = if checked { "✓ " } else { "☐ " };
        let continuation = format!(
            "{quote_prefix}{indent}{}",
            " ".repeat(marker.chars().count())
        );
        (format!("{quote_prefix}{indent}{marker}"), continuation)
    }

    fn render_table(&mut self, table: AssistantMarkdownTable) {
        let header_rows = table.header_rows;
        let rows = table.non_empty_rows();
        if rows.is_empty() {
            return;
        }
        let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
        if column_count == 0 {
            return;
        }
        let mut widths = vec![0usize; column_count];
        for row in &rows {
            for (column, cell) in row.iter().enumerate() {
                widths[column] = widths[column].max(cell.chars().count());
            }
        }
        for (row_index, row) in rows.iter().enumerate() {
            self.lines.push(styled_line(
                format_table_row(row, &widths),
                SingleSessionLineStyle::AssistantTable,
            ));
            if header_rows > 0 && row_index + 1 == header_rows.min(rows.len()) {
                self.lines.push(styled_line(
                    format_table_separator(&widths),
                    SingleSessionLineStyle::AssistantTable,
                ));
            }
        }
    }
}

impl AssistantMarkdownTable {
    fn push_text(&mut self, text: &str) {
        self.current_cell.push_str(&text.replace('\n', " "));
    }

    fn push_space(&mut self) {
        if !self
            .current_cell
            .chars()
            .last()
            .is_some_and(char::is_whitespace)
        {
            self.current_cell.push(' ');
        }
    }

    fn finish_cell(&mut self) {
        self.current_row.push(self.current_cell.trim().to_string());
        self.current_cell.clear();
    }

    fn finish_row(&mut self) {
        if !self.current_row.is_empty() {
            self.rows.push(std::mem::take(&mut self.current_row));
        }
    }

    fn non_empty_rows(mut self) -> Vec<Vec<String>> {
        if !self.current_cell.trim().is_empty() {
            self.finish_cell();
        }
        self.finish_row();
        self.rows
            .into_iter()
            .filter(|row| row.iter().any(|cell| !cell.is_empty()))
            .collect()
    }
}

fn heading_prefix(level: HeadingLevel) -> &'static str {
    match level {
        HeadingLevel::H1 | HeadingLevel::H2 => "",
        HeadingLevel::H3 => "› ",
        _ => "· ",
    }
}

fn bullet_for_depth(depth: usize) -> &'static str {
    match depth % 3 {
        0 => "• ",
        1 => "◦ ",
        _ => "▪ ",
    }
}

fn format_table_row(row: &[String], widths: &[usize]) -> String {
    let mut rendered = String::new();
    for (column, width) in widths.iter().enumerate() {
        if column > 0 {
            rendered.push_str(" │ ");
        }
        let cell = row.get(column).map(String::as_str).unwrap_or_default();
        rendered.push_str(cell);
        rendered.push_str(&" ".repeat(width.saturating_sub(cell.chars().count())));
    }
    rendered.trim_end().to_string()
}

fn format_table_separator(widths: &[usize]) -> String {
    let mut rendered = String::new();
    for (column, width) in widths.iter().enumerate() {
        if column > 0 {
            rendered.push_str("─┼─");
        }
        rendered.push_str(&"─".repeat((*width).max(1)));
    }
    rendered
}
