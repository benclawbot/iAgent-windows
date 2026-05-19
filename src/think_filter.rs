#[derive(Debug, Default, Clone)]
pub struct ThinkTagFilter {
    in_think: bool,
    pending: String,
}

impl ThinkTagFilter {
    pub fn push(&mut self, text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }

        let mut input = String::with_capacity(self.pending.len() + text.len());
        input.push_str(&self.pending);
        input.push_str(text);
        self.pending.clear();

        let mut output = String::new();
        let mut cursor = 0;
        while cursor < input.len() {
            let remaining = &input[cursor..];
            if self.in_think {
                if let Some(close_idx) = find_ascii_case_insensitive(remaining, "</think>") {
                    cursor += close_idx + "</think>".len();
                    self.in_think = false;
                } else {
                    let keep = longest_suffix_that_starts_tag(remaining, "</think>");
                    if keep > 0 {
                        self.pending.push_str(&remaining[remaining.len() - keep..]);
                    }
                    break;
                }
            } else {
                match find_think_open_tag(remaining) {
                    OpenTagSearch::Found { start, end } => {
                        output.push_str(&remaining[..start]);
                        cursor += end;
                        self.in_think = true;
                    }
                    OpenTagSearch::Partial { start } => {
                        output.push_str(&remaining[..start]);
                        self.pending.push_str(&remaining[start..]);
                        break;
                    }
                    OpenTagSearch::None => {
                        let keep = longest_suffix_that_starts_tag(remaining, "<think");
                        output.push_str(&remaining[..remaining.len() - keep]);
                        if keep > 0 {
                            self.pending.push_str(&remaining[remaining.len() - keep..]);
                        }
                        break;
                    }
                }
            }
        }

        output
    }

    pub fn flush(&mut self) -> String {
        if self.in_think {
            self.pending.clear();
            String::new()
        } else {
            std::mem::take(&mut self.pending)
        }
    }
}

pub fn strip_think_tags(text: &str) -> String {
    let mut filter = ThinkTagFilter::default();
    let mut output = filter.push(text);
    output.push_str(&filter.flush());
    output
}

enum OpenTagSearch {
    Found { start: usize, end: usize },
    Partial { start: usize },
    None,
}

fn find_think_open_tag(haystack: &str) -> OpenTagSearch {
    let mut search_start = 0;
    while search_start < haystack.len() {
        let Some(relative_idx) = find_ascii_case_insensitive(&haystack[search_start..], "<think")
        else {
            return OpenTagSearch::None;
        };
        let start = search_start + relative_idx;
        let after_name = start + "<think".len();
        let Some(next) = haystack.as_bytes().get(after_name).copied() else {
            return OpenTagSearch::Partial { start };
        };
        if next == b'>' {
            return OpenTagSearch::Found {
                start,
                end: after_name + 1,
            };
        }
        if next.is_ascii_whitespace() {
            let Some(close_relative) = haystack[after_name..].find('>') else {
                return OpenTagSearch::Partial { start };
            };
            return OpenTagSearch::Found {
                start,
                end: after_name + close_relative + 1,
            };
        }
        search_start = after_name;
    }
    OpenTagSearch::None
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())
}

fn longest_suffix_that_starts_tag(text: &str, tag: &str) -> usize {
    let max = text.len().min(tag.len() - 1);
    let text_bytes = text.as_bytes();
    let tag_bytes = tag.as_bytes();
    for len in (1..=max).rev() {
        let suffix = &text_bytes[text_bytes.len() - len..];
        if suffix
            .iter()
            .zip(tag_bytes.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
            && suffix.iter().all(u8::is_ascii)
        {
            return len;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::ThinkTagFilter;

    #[test]
    fn removes_complete_think_blocks() {
        let mut filter = ThinkTagFilter::default();
        assert_eq!(
            filter.push("before <think>hidden</think> after"),
            "before  after"
        );
        assert_eq!(filter.flush(), "");
    }

    #[test]
    fn removes_think_blocks_split_across_chunks() {
        let mut filter = ThinkTagFilter::default();
        assert_eq!(filter.push("before <thi"), "before ");
        assert_eq!(filter.push("nk>hidden</thi"), "");
        assert_eq!(filter.push("nk> after"), " after");
        assert_eq!(filter.flush(), "");
    }

    #[test]
    fn preserves_partial_open_tag_when_stream_ends() {
        let mut filter = ThinkTagFilter::default();
        assert_eq!(filter.push("before <thi"), "before ");
        assert_eq!(filter.flush(), "<thi");
    }

    #[test]
    fn drops_unclosed_think_block_on_flush() {
        let mut filter = ThinkTagFilter::default();
        assert_eq!(filter.push("before <think>hidden"), "before ");
        assert_eq!(filter.flush(), "");
    }

    #[test]
    fn matches_tags_case_insensitively() {
        let mut filter = ThinkTagFilter::default();
        assert_eq!(filter.push("a <THINK>hidden</Think> b"), "a  b");
    }

    #[test]
    fn removes_open_tags_with_attributes() {
        let mut filter = ThinkTagFilter::default();
        assert_eq!(
            filter.push("before <think type=\"internal\">hidden</think> after"),
            "before  after"
        );
    }

    #[test]
    fn removes_attributed_open_tags_split_across_chunks() {
        let mut filter = ThinkTagFilter::default();
        assert_eq!(filter.push("before <think type=\"internal"), "before ");
        assert_eq!(filter.push("\">hidden</think> after"), " after");
    }

    #[test]
    fn strip_think_tags_removes_complete_blocks() {
        assert_eq!(
            super::strip_think_tags("visible <think>hidden</think> answer"),
            "visible  answer"
        );
    }

    #[test]
    fn handles_non_ascii_text_near_tag_boundaries() {
        let mut filter = ThinkTagFilter::default();
        assert_eq!(filter.push("cafe é <thi"), "cafe é ");
        assert_eq!(filter.push("nk>hidden</think> done"), " done");
    }
}
