//! Simple Markdown renderer for TUI
//!
//! Converts Markdown text to styled ratatui Lines and Spans.
//! Supports: bold, italic, inline code, code blocks, headers, lists,
//! blockquotes, horizontal rules, and links.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Maximum width for code block preview in lines
const CODE_BLOCK_MAX_LINES: usize = 10;

/// Maximum characters per line before wrapping
const MAX_LINE_WIDTH: usize = 120;

/// Parse a markdown string into styled Lines
pub fn parse_markdown(input: &str) -> Vec<Line<'static>> {
    let mut result: Vec<Line<'static>> = Vec::new();
    let lines: Vec<&str> = input.split('\n').collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        // Code block delimiter (```)
        if line.trim_start().starts_with("```") {
            let code_lines: Vec<String> = {
                let mut codes: Vec<String> = Vec::new();
                i += 1;
                while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                    codes.push(lines[i].to_string());
                    i += 1;
                }
                // Skip the closing ```
                i += 1;
                codes
            };
            let code_text = code_lines.join("\n");
            let styled = render_code_block(&code_text);
            result.push(Line::from(styled));
            continue;
        }

        // Horizontal rule
        if line.trim().is_empty() {
            result.push(Line::from(Span::raw("")));
            i += 1;
            continue;
        }

        // Headers (# ## ### etc)
        if line.starts_with('#') {
            let (level, text) = parse_header(line);
            let styled = render_header(level, text);
            result.push(Line::from(styled));
            i += 1;
            continue;
        }

        // Blockquote (> )
        if line.trim_start().starts_with('>') {
            let text = line.trim_start().trim_start_matches('>').trim();
            result.push(render_blockquote(text));
            i += 1;
            continue;
        }

        // Unordered list (- or * or +)
        if line.trim_start().starts_with('-')
            || line.trim_start().starts_with('*')
            || line.trim_start().starts_with('+')
        {
            let text = line.trim_start()[1..].trim();
            result.push(render_unordered_list(text));
            i += 1;
            continue;
        }

        // Ordered list (1. 2. etc)
        if let Some(dot_pos) = line.find(". ") {
            let prefix = &line[..dot_pos];
            if prefix.chars().all(|c| c.is_ascii_digit()) {
                let text = &line[dot_pos + 2..];
                result.push(render_ordered_list(prefix, text));
                i += 1;
                continue;
            }
        }

        // Regular paragraph - render inline markdown
        result.push(render_inline(line));
        i += 1;
    }

    // Remove trailing empty lines
    while result.len() > 1 && result.last().is_some_and(|l| l.spans.is_empty()) {
        result.pop();
    }

    result
}

/// Parse header line, returning (level, text)
fn parse_header(line: &str) -> (usize, &str) {
    let mut level = 0;
    for c in line.chars() {
        if c == '#' {
            level += 1;
        } else {
            break;
        }
    }
    let text = line[level..].trim();
    (level.min(6), text)
}

/// Render a header with appropriate styling
fn render_header(level: usize, text: &str) -> Span<'static> {
    let style = match level {
        1 => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::UNDERLINED),
        2 => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        3 => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        _ => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    };
    Span::styled(text.to_string(), style)
}

/// Render a blockquote line
fn render_blockquote(text: &str) -> Line<'static> {
    let styled_inner = render_inline_inner(text);
    let mut combined: Vec<Span> = vec![Span::styled("│ ", Style::default().fg(Color::DarkGray))];
    combined.extend(styled_inner);
    Line::from(combined)
}

/// Render an unordered list item
fn render_unordered_list(text: &str) -> Line<'static> {
    let prefix = Span::styled("• ", Style::default().fg(Color::Green));
    let content = render_inline(text);
    let mut combined: Vec<Span> = vec![prefix];
    combined.extend(content.spans);
    Line::from(combined)
}

/// Render an ordered list item
fn render_ordered_list(number: &str, text: &str) -> Line<'static> {
    let prefix = Span::styled(
        format!("{}. ", number),
        Style::default().fg(Color::Green),
    );
    let content = render_inline(text);
    let mut combined: Vec<Span> = vec![prefix];
    combined.extend(content.spans);
    Line::from(combined)
}

/// Render a code block with monospace styling
fn render_code_block(code: &str) -> Vec<Span<'static>> {
    let bg_style = Style::default()
        .bg(Color::Rgb(30, 30, 35))
        .fg(Color::Rgb(200, 200, 210));

    let code_lines: Vec<&str> = code.split('\n').collect();
    let display_lines: Vec<&str> = if code_lines.len() > CODE_BLOCK_MAX_LINES {
        let mut lines: Vec<&str> = code_lines[..CODE_BLOCK_MAX_LINES].to_vec();
        lines.push("  ...");
        lines
    } else {
        code_lines
    };

    display_lines
        .iter()
        .map(|l| {
            let display = if l.len() > MAX_LINE_WIDTH {
                format!("{}...", &l[..MAX_LINE_WIDTH])
            } else {
                l.to_string()
            };
            Span::styled(format!("  {}\n", display), bg_style)
        })
        .collect()
}

/// Render a line with inline markdown (bold, italic, code, links)
pub fn render_inline(line: &str) -> Line<'static> {
    Line::from(render_inline_inner(line))
}

/// Core inline rendering - returns Vec<Span>
fn render_inline_inner(text: &str) -> Vec<Span<'static>> {
    // Check for inline code first (backticks)
    if text.contains('`') {
        return render_inline_code(text);
    }

    // Check for links [text](url)
    if text.contains('[') && text.contains(']') && text.contains("(http") {
        return render_inline_links(text);
    }

    // Parse inline styles: **bold**, *italic*, __bold__, _italic_
    parse_inline_styles(text)
}

/// Render inline code (backtick-enclosed)
fn render_inline_code(text: &str) -> Vec<Span<'static>> {
    let mut result: Vec<Span<'static>> = Vec::new();
    let mut remaining = text;
    let code_style = Style::default()
        .fg(Color::Rgb(255, 180, 100))
        .add_modifier(Modifier::DIM);

    while let Some(start) = remaining.find('`') {
        // Text before the backtick
        if start > 0 {
            let prefix = &remaining[..start];
            result.extend(parse_inline_styles_non_code(prefix));
        }

        let after = &remaining[start + 1..];
        if let Some(end) = after.find('`') {
            // Found closing backtick
            let code = &after[..end];
            result.push(Span::styled(code.to_string(), code_style));
            remaining = &after[end + 1..];
        } else {
            // No closing backtick, treat rest as text
            result.extend(parse_inline_styles_non_code(&remaining[start..]));
            break;
        }
    }

    // Remaining text
    if !remaining.is_empty() {
        result.extend(parse_inline_styles_non_code(remaining));
    }

    result
}

/// Render inline links [text](url)
fn render_inline_links(text: &str) -> Vec<Span<'static>> {
    let mut result: Vec<Span<'static>> = Vec::new();
    let mut remaining = text;
    let link_style = Style::default()
        .fg(Color::Blue)
        .add_modifier(Modifier::UNDERLINED);

    while let Some(start) = remaining.find('[') {
        // Text before the link
        if start > 0 {
            let prefix = &remaining[..start];
            result.extend(parse_inline_styles_non_code(prefix));
        }

        let after = &remaining[start + 1..];
        if let Some(text_end) = after.find(']') {
            let link_text = &after[..text_end];
            let after_bracket = &after[text_end + 1..];
            if after_bracket.starts_with('(') {
                if let Some(url_end) = after_bracket.find(')') {
                    let url = &after_bracket[1..url_end];
                    // Display link text with link-style, append URL
                    result.push(Span::styled(
                        format!("{} ({})", link_text, url),
                        link_style,
                    ));
                    remaining = &after_bracket[url_end + 1..];
                } else {
                    result.extend(parse_inline_styles_non_code(&remaining[start..]));
                    break;
                }
            } else {
                result.extend(parse_inline_styles_non_code(&remaining[start..]));
                break;
            }
        } else {
            result.extend(parse_inline_styles_non_code(&remaining[start..]));
            break;
        }
    }

    if !remaining.is_empty() {
        result.extend(parse_inline_styles_non_code(remaining));
    }

    result
}

/// Parse inline styles without code handling (for performance)
fn parse_inline_styles_non_code(text: &str) -> Vec<Span<'static>> {
    // Parse **bold**, *italic*, __bold__, _italic_
    let mut result: Vec<Span<'static>> = Vec::new();
    let bold_style = Style::default().add_modifier(Modifier::BOLD);
    let italic_style = Style::default().add_modifier(Modifier::ITALIC);
    let bold_italic_style = Style::default()
        .add_modifier(Modifier::BOLD)
        .add_modifier(Modifier::ITALIC);

    let remaining = text;
    // Track which patterns we've found at each position to avoid overlapping
    // We'll use a simple approach: check for ** first, then __, then *, then _
    let chars: Vec<char> = remaining.chars().collect();
    let len = chars.len();
    let mut pos = 0;

    while pos < len {
        let rest: String = chars[pos..].iter().collect();

        // Check for **bold**
        if rest.starts_with("**") {
            if let Some(end) = find_closing(&chars[pos + 2..], "**") {
                let bold_content: String = chars[pos + 2..pos + 2 + end].iter().collect();
                result.push(Span::styled(bold_content, bold_style));
                pos += 2 + end + 2;
                continue;
            }
        }

        // Check for __bold__
        if rest.starts_with("__") {
            if let Some(end) = find_closing(&chars[pos + 2..], "__") {
                let bold_content: String = chars[pos + 2..pos + 2 + end].iter().collect();
                result.push(Span::styled(bold_content, bold_style));
                pos += 2 + end + 2;
                continue;
            }
        }

        // Check for ***bold+italic***
        if rest.starts_with("***") {
            if let Some(end) = find_closing(&chars[pos + 3..], "***") {
                let content: String = chars[pos + 3..pos + 3 + end].iter().collect();
                result.push(Span::styled(content, bold_italic_style));
                pos += 3 + end + 3;
                continue;
            }
        }

        // Check for *italic*
        if rest.starts_with('*') && !rest.starts_with("**") {
            if let Some(end) = find_closing(&chars[pos + 1..], "*") {
                let italic_content: String = chars[pos + 1..pos + 1 + end].iter().collect();
                result.push(Span::styled(italic_content, italic_style));
                pos += 1 + end + 1;
                continue;
            }
        }

        // Check for _italic_
        if rest.starts_with('_') && !rest.starts_with("__") {
            // Only trigger if not preceded by alphanumeric (avoid word boundaries)
            let prev_is_alnum = pos > 0 && chars[pos - 1].is_alphanumeric();
            if !prev_is_alnum {
                if let Some(end) = find_closing(&chars[pos + 1..], "_") {
                    let italic_content: String = chars[pos + 1..pos + 1 + end].iter().collect();
                    result.push(Span::styled(italic_content, italic_style));
                    pos += 1 + end + 1;
                    continue;
                }
            }
        }

        // No pattern matched, collect the character and move on
        result.push(Span::raw(chars[pos].to_string()));
        pos += 1;
    }

    result
}

/// Parse inline styles - main entry point with code handling
fn parse_inline_styles(text: &str) -> Vec<Span<'static>> {
    parse_inline_styles_non_code(text)
}

/// Find closing delimiter, returning the content length
fn find_closing(chars: &[char], delimiter: &str) -> Option<usize> {
    let delim_chars: Vec<char> = delimiter.chars().collect();
    let delim_len = delim_chars.len();

    for (i, window) in chars.windows(delim_len).enumerate() {
        if window == delim_chars {
            return Some(i);
        }
    }
    None
}

/// Check if text contains any markdown formatting
pub fn contains_markdown(text: &str) -> bool {
    text.contains("**")
        || text.contains("__")
        || text.contains("*")
        || text.contains("_")
        || text.contains('`')
        || text.contains("# ")
        || text.starts_with('-')
        || text.starts_with('>')
        || text.contains("[..](")
}

/// Check if a message content likely contains structured markdown
/// (more than just plain text with occasional formatting)
pub fn is_markdown_heavy(text: &str) -> bool {
    let markdown_indicators = [
        "```", "**", "__", "# ", "## ", "### ", "- [ ]", "- [x]",
        "```", "&&", "||", "| ", "```rust", "```python", "```javascript",
        "```js", "```ts", "```go", "```rs", "```cpp",
    ];
    markdown_indicators
        .iter()
        .any(|m| text.contains(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bold() {
        let result = parse_markdown("This is **bold** text");
        assert!(!result.is_empty());
        let line = &result[0];
        let spans = &line.spans;
        assert!(spans.iter().any(|s| s.style == Style::default().add_modifier(Modifier::BOLD)));
    }

    #[test]
    fn test_parse_italic() {
        let result = parse_markdown("This is *italic* text");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_parse_inline_code() {
        let result = parse_markdown("Use `echo hello` to print");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_parse_header() {
        let result = parse_markdown("# Header 1");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_parse_code_block() {
        let result = parse_markdown("```\nhello world\n```");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_parse_list() {
        let result = parse_markdown("- item 1\n- item 2");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_contains_markdown() {
        assert!(contains_markdown("**bold**"));
        assert!(contains_markdown("*italic*"));
        assert!(contains_markdown("# header"));
        assert!(!contains_markdown("plain text"));
    }

    #[test]
    fn test_is_markdown_heavy() {
        assert!(is_markdown_heavy("```rust\nfn main() {}\n```"));
        assert!(is_markdown_heavy("- [ ] task"));
        assert!(!is_markdown_heavy("plain text"));
    }
}
