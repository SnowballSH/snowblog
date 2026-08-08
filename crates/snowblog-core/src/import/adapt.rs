#[derive(Clone, Debug)]
pub struct SourceAdaptation {
    pub strip_page_setup: bool,
    pub wrap_cetz_canvas: bool,
}

impl Default for SourceAdaptation {
    fn default() -> Self {
        Self {
            strip_page_setup: true,
            wrap_cetz_canvas: true,
        }
    }
}

pub fn adapt_source(source: &str, adaptation: &SourceAdaptation) -> String {
    let mut result = source.to_string();
    if adaptation.strip_page_setup {
        result = strip_calls(&result, "#set page(");
    }
    if adaptation.wrap_cetz_canvas {
        result = wrap_calls(
            &result,
            "#context cetz.canvas(",
            "#html.frame(context cetz.canvas(",
        );
        result = wrap_calls(&result, "#cetz.canvas(", "#html.frame(cetz.canvas(");
    }
    result
}

fn strip_calls(source: &str, pattern: &str) -> String {
    transform_calls(source, pattern, |_| String::new())
}

fn wrap_calls(source: &str, pattern: &str, replacement_head: &str) -> String {
    transform_calls(source, pattern, |call| {
        format!("{replacement_head}{})", &call[pattern.len()..])
    })
}

fn transform_calls(source: &str, pattern: &str, transform: impl Fn(&str) -> String) -> String {
    let mut result = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(index) = rest.find(pattern) {
        result.push_str(&rest[..index]);
        let call_start = &rest[index..];
        match balanced_call_end(call_start, pattern.len()) {
            Some(end) => {
                result.push_str(&transform(&call_start[..end]));
                rest = &call_start[end..];
            }
            None => {
                result.push_str(call_start);
                return result;
            }
        }
    }
    result.push_str(rest);
    result
}

fn balanced_call_end(text: &str, open_index: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut in_string = false;
    let mut previous = '\0';
    for (offset, c) in text[open_index..].char_indices() {
        if in_string {
            if c == '"' && previous != '\\' {
                in_string = false;
            }
        } else {
            match c {
                '"' => in_string = true,
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(open_index + offset + c.len_utf8());
                    }
                }
                _ => {}
            }
        }
        previous = c;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_page_setup_lines() {
        let source = "#set page(height: auto, margin: 0.7em)\n= Title";
        let adapted = adapt_source(source, &SourceAdaptation::default());
        assert!(!adapted.contains("#set page"));
        assert!(adapted.contains("= Title"));
    }

    #[test]
    fn wraps_context_cetz_canvas() {
        let source = "#context cetz.canvas({\n  line((0, 0), (1, 1))\n})\nafter";
        let adapted = adapt_source(source, &SourceAdaptation::default());
        assert!(
            adapted.contains("#html.frame(context cetz.canvas({\n  line((0, 0), (1, 1))\n}))"),
            "{adapted}"
        );
        assert!(adapted.ends_with("after"));
    }

    #[test]
    fn respects_strings_with_parens() {
        let source = "#set page(numbering: \"(1)\")\n= T";
        let adapted = adapt_source(source, &SourceAdaptation::default());
        assert!(adapted.contains("= T"));
    }

    #[test]
    fn disabled_adaptation_is_identity() {
        let source = "#set page(x: 1)\n#context cetz.canvas({})";
        let off = SourceAdaptation {
            strip_page_setup: false,
            wrap_cetz_canvas: false,
        };
        assert_eq!(adapt_source(source, &off), source);
    }
}
