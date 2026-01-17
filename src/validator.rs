use miette::{Diagnostic, SourceSpan};
use serde_json::error::Category;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum JsonError {
    #[error("trailing comma")]
    #[diagnostic(code(ferrite::trailing_comma))]
    TrailingComma(
        #[source_code] String,
        #[label("remove this comma")] SourceSpan,
        #[help] String,
    ),
    #[error("expected `,` or `]`")]
    #[diagnostic(code(ferrite::missing_comma))]
    MissingComma(
        #[source_code] String,
        #[label("add comma here")] SourceSpan,
        #[help] String,
    ),
    #[error("expected `:`")]
    #[diagnostic(code(ferrite::missing_colon))]
    MissingColon(
        #[source_code] String,
        #[label("add `:` here")] SourceSpan,
        #[help] String,
    ),
    #[error("unexpected end of file")]
    #[diagnostic(code(ferrite::unexpected_eof))]
    UnexpectedEof(
        #[source_code] String,
        #[label("file ended here")] SourceSpan,
        #[help] String,
    ),
    #[error("expected quoted string as object key")]
    #[diagnostic(code(ferrite::key_must_be_string))]
    KeyMustBeString(
        #[source_code] String,
        #[label("add quotes around this")] SourceSpan,
        #[help] String,
    ),
    #[error("invalid escape sequence")]
    #[diagnostic(code(ferrite::invalid_escape))]
    InvalidEscape(
        #[source_code] String,
        #[label("invalid escape")] SourceSpan,
        #[help] String,
    ),
    #[error("invalid number")]
    #[diagnostic(code(ferrite::invalid_number))]
    InvalidNumber(
        #[source_code] String,
        #[label("malformed number")] SourceSpan,
        #[help] String,
    ),
    #[error("invalid character in string")]
    #[diagnostic(code(ferrite::control_character))]
    InvalidControlCharacter(
        #[source_code] String,
        #[label("escape this character")] SourceSpan,
        #[help] String,
    ),
    #[error("{0}")]
    #[diagnostic(code(ferrite::syntax_error))]
    SyntaxError(
        String,
        #[source_code] String,
        #[label("error here")] SourceSpan,
    ),
}

struct ErrorContext<'a> {
    src: &'a str,
    line: usize,
    column: usize,
    span: SourceSpan,
    lines: Vec<&'a str>,
}

impl<'a> ErrorContext<'a> {
    fn new(src: &'a str, line: usize, column: usize) -> Self {
        let lines: Vec<&str> = src.lines().collect();
        let span = SourceSpan::new(Self::offset_at(src, line, column).into(), 1);
        Self {
            src,
            line,
            column,
            span,
            lines,
        }
    }

    fn current_line(&self) -> &'a str {
        self.lines
            .get(self.line.saturating_sub(1))
            .copied()
            .unwrap_or("")
    }

    fn previous_line(&self) -> Option<&'a str> {
        self.lines.get(self.line.saturating_sub(2)).copied()
    }

    fn is_trailing_comma(&self) -> bool {
        let current = self.current_line();
        let cut = Self::byte_index(current, self.column.saturating_sub(1));
        current[..cut.min(current.len())].trim_end().ends_with(',')
    }

    fn trailing_comma_hint(&self) -> String {
        let trimmed = self.current_line().trim_end();
        match trimmed
            .strip_suffix(",]")
            .or_else(|| trimmed.strip_suffix(",}"))
        {
            Some(prefix) => format!(
                "change `{}` to `{}{}`",
                trimmed,
                prefix.trim_end(),
                if trimmed.ends_with(",]") { "]" } else { "}" }
            ),
            None if trimmed.ends_with(',') => format!(
                "change `{}` to `{}`",
                trimmed,
                trimmed.trim_end_matches(',')
            ),
            _ => "remove the trailing comma".to_string(),
        }
    }

    fn missing_comma_hint(&self) -> String {
        if let Some(line) = self.previous_line() {
            let trimmed = line.trim_end();
            if !trimmed.ends_with(',') && !trimmed.ends_with('[') && !trimmed.ends_with('{') {
                return format!(
                    "add `,` after `{}`",
                    trimmed.chars().take(32).collect::<String>()
                );
            }
        }
        "add comma between items".to_string()
    }

    fn colon_hint(&self) -> String {
        let line = self.current_line();
        let end = Self::byte_index(line, self.column.saturating_sub(1)).min(line.len());
        Self::extract_last_key(&line[..end])
            .map(|key| format!("change `\"{}\"` to `\"{}\": `", key, key))
            .unwrap_or_else(|| "add `:` after the key".to_string())
    }

    fn key_hint(&self) -> String {
        let line = self.current_line();
        let token = line[..Self::byte_index(line, self.column).min(line.len())]
            .split_whitespace()
            .last()
            .unwrap_or("key")
            .trim_end_matches(':');
        let key = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        if key.is_empty() {
            "wrap the key in quotes".to_string()
        } else {
            format!("change `{}` to `\"{}\": `", key, key)
        }
    }

    fn eof_hint(&self) -> String {
        let mut msgs = vec![];
        let (open_brace, close_brace) = (
            self.src.chars().filter(|&c| c == '{').count(),
            self.src.chars().filter(|&c| c == '}').count(),
        );
        let (open_arr, close_arr) = (
            self.src.chars().filter(|&c| c == '[').count(),
            self.src.chars().filter(|&c| c == ']').count(),
        );
        if open_brace > close_brace {
            msgs.push(format!("{} missing `}}`", open_brace - close_brace));
        }
        if open_arr > close_arr {
            msgs.push(format!("{} missing `]`", open_arr - close_arr));
        }
        if msgs.is_empty() {
            "add missing closing bracket".to_string()
        } else {
            msgs.join(", ")
        }
    }

    fn escape_hint(&self) -> String {
        let line = self.current_line();
        if line.contains(":\\") {
            if let Some(end) = line.rfind('"') {
                if let Some(start) = line[..end].rfind('"') {
                    let value = &line[start + 1..end];
                    if value.contains(":\\") {
                        return format!(
                            "change `\"{}\"` to `\"{}\"`",
                            value,
                            value.replace('\\', "\\\\")
                        );
                    }
                }
            }
        }
        let window = self.line_window(5);
        if window.contains("\\n") || window.contains("\\t") || window.contains("\\r") {
            "this escape is already correct, check your quotes"
        } else if window.contains('\\') {
            "escape the backslash as \\\\"
        } else {
            "invalid escape - valid ones: \\\" \\\\ \\/ \\b \\f \\n \\r \\t \\uXXXX"
        }
        .to_string()
    }

    fn number_hint(&self) -> String {
        let window = self.line_window(8);
        let token: String = window
            .chars()
            .take_while(|ch| ch.is_ascii_digit() || matches!(ch, '-' | '+' | '.'))
            .collect();
        if token.is_empty() {
            return "fix number format".to_string();
        }
        if token.starts_with('0')
            && token.len() > 1
            && token.chars().nth(1).is_some_and(|c| c.is_ascii_digit())
        {
            return format!("change `{}` to `{}`", token, token.trim_start_matches('0'));
        }
        if token.ends_with('.') {
            return format!("change `{}` to `{}0`", token, token);
        }
        if let Some(stripped) = token.strip_prefix('+') {
            return format!("change `{}` to `{}`", token, stripped);
        }
        "fix number format".to_string()
    }

    fn line_window(&self, radius: usize) -> &str {
        let line = self.current_line();
        let left_chars = self.column.saturating_sub(radius + 1);
        let right_chars = self.column.saturating_add(radius);
        let left = Self::byte_index(line, left_chars).min(line.len());
        let right = Self::byte_index(line, right_chars).min(line.len());
        &line[left..right]
    }

    fn offset_at(content: &str, line: usize, column: usize) -> usize {
        let mut offset = 0usize;
        for (idx, current) in content.lines().enumerate() {
            if idx + 1 == line {
                offset += Self::byte_index(current, column);
                break;
            }
            offset += current.len() + 1;
        }
        offset
    }

    fn extract_last_key(text: &str) -> Option<String> {
        let end = text.rfind('"')?;
        let start = text[..end].rfind('"')?;
        Some(text[start + 1..end].to_string())
    }

    fn byte_index(text: &str, column: usize) -> usize {
        if column == 0 {
            return 0;
        }
        text.char_indices()
            .map(|(i, _)| i)
            .nth(column.saturating_sub(1))
            .unwrap_or_else(|| text.len())
    }
}

pub fn validate_json(
    content: &str,
    _filename: String,
    _context_lines: usize,
) -> miette::Result<()> {
    serde_json::from_str::<serde_json::Value>(content)
        .map(|_| ())
        .map_err(|err| {
            let ctx = ErrorContext::new(content, err.line(), err.column());
            miette::Report::new(map_error(err, &ctx))
        })
}

fn map_error(err: serde_json::Error, ctx: &ErrorContext<'_>) -> JsonError {
    let lower = err.to_string().to_ascii_lowercase();
    let (src, span) = (ctx.src.to_owned(), ctx.span);

    if lower.contains("trailing comma") || ctx.is_trailing_comma() {
        JsonError::TrailingComma(src, span, ctx.trailing_comma_hint())
    } else if lower.contains("expected") && lower.contains("comma") {
        JsonError::MissingComma(src, span, ctx.missing_comma_hint())
    } else if lower.contains("key must be a string") {
        JsonError::KeyMustBeString(src, span, ctx.key_hint())
    } else if lower.contains("expected") && lower.contains("colon") {
        JsonError::MissingColon(src, span, ctx.colon_hint())
    } else if lower.contains("control character") {
        JsonError::InvalidControlCharacter(
            src,
            span,
            "replace tabs/newlines with \\t or \\n".to_string(),
        )
    } else if lower.contains("escape") {
        JsonError::InvalidEscape(src, span, ctx.escape_hint())
    } else if lower.contains("number") {
        JsonError::InvalidNumber(src, span, ctx.number_hint())
    } else if lower.contains("eof") || err.classify() == Category::Eof {
        JsonError::UnexpectedEof(src, span, ctx.eof_hint())
    } else {
        JsonError::SyntaxError(err.to_string(), src, span)
    }
}
