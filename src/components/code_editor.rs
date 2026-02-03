//! Code editor component with line numbers and basic syntax highlighting
//!
//! A simple code display/editor component for terminal UIs.

use crate::components::{Box as TinkBox, Text};
use crate::core::{Color, Element, FlexDirection};

/// Syntax highlighting language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    Plain,
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Json,
    Yaml,
    Toml,
    Markdown,
    Shell,
}

/// Code editor component
#[derive(Debug, Clone)]
pub struct CodeEditor {
    /// Code content
    content: String,
    /// Language for syntax highlighting
    language: Language,
    /// Show line numbers
    show_line_numbers: bool,
    /// Starting line number
    start_line: usize,
    /// Highlighted line (1-indexed)
    highlighted_line: Option<usize>,
    /// Cursor position (line, column) - 1-indexed
    cursor: Option<(usize, usize)>,
    /// Line number color
    line_number_color: Color,
    /// Highlighted line background
    highlight_color: Color,
    /// Keyword color
    keyword_color: Color,
    /// String color
    string_color: Color,
    /// Comment color
    comment_color: Color,
    /// Number color
    _number_color: Color,
    /// Key for reconciliation
    key: Option<String>,
}

impl CodeEditor {
    /// Create a new code editor
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            language: Language::Plain,
            show_line_numbers: true,
            start_line: 1,
            highlighted_line: None,
            cursor: None,
            line_number_color: Color::BrightBlack,
            highlight_color: Color::Ansi256(236),
            keyword_color: Color::Magenta,
            string_color: Color::Green,
            comment_color: Color::BrightBlack,
            _number_color: Color::Yellow,
            key: None,
        }
    }

    /// Set language for syntax highlighting
    pub fn language(mut self, lang: Language) -> Self {
        self.language = lang;
        self
    }

    /// Show/hide line numbers
    pub fn show_line_numbers(mut self, show: bool) -> Self {
        self.show_line_numbers = show;
        self
    }

    /// Set starting line number
    pub fn start_line(mut self, line: usize) -> Self {
        self.start_line = line.max(1);
        self
    }

    /// Set highlighted line (1-indexed)
    pub fn highlighted_line(mut self, line: usize) -> Self {
        self.highlighted_line = Some(line);
        self
    }

    /// Set cursor position (line, column) - 1-indexed
    pub fn cursor(mut self, line: usize, column: usize) -> Self {
        self.cursor = Some((line, column));
        self
    }

    /// Set line number color
    pub fn line_number_color(mut self, color: Color) -> Self {
        self.line_number_color = color;
        self
    }

    /// Set highlight color
    pub fn highlight_color(mut self, color: Color) -> Self {
        self.highlight_color = color;
        self
    }

    /// Set keyword color
    pub fn keyword_color(mut self, color: Color) -> Self {
        self.keyword_color = color;
        self
    }

    /// Set string color
    pub fn string_color(mut self, color: Color) -> Self {
        self.string_color = color;
        self
    }

    /// Set comment color
    pub fn comment_color(mut self, color: Color) -> Self {
        self.comment_color = color;
        self
    }

    /// Set key
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Convert to element
    pub fn into_element(self) -> Element {
        let lines: Vec<&str> = self.content.lines().collect();
        let line_count = lines.len();
        let line_num_width = (self.start_line + line_count).to_string().len().max(3);

        let mut elements = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let line_num = self.start_line + i;
            let is_highlighted = self.highlighted_line == Some(line_num);
            let has_cursor = self.cursor.map(|(l, _)| l == line_num).unwrap_or(false);

            let mut row_children = Vec::new();

            // Line number
            if self.show_line_numbers {
                let num_text = format!("{:>width$} │ ", line_num, width = line_num_width);
                row_children.push(
                    Text::new(num_text)
                        .color(self.line_number_color)
                        .into_element(),
                );
            }

            // Code content with syntax highlighting
            let highlighted_code = self.highlight_line(line);
            row_children.push(Text::new(highlighted_code).into_element());

            // Cursor indicator
            if has_cursor {
                // Simple cursor indicator at end
                row_children.push(Text::new(" ◂").color(Color::Yellow).into_element());
            }

            let mut row = TinkBox::new()
                .flex_direction(FlexDirection::Row)
                .children(row_children);

            if is_highlighted {
                row = row.background(self.highlight_color);
            }

            elements.push(row.into_element());
        }

        let mut container = TinkBox::new()
            .flex_direction(FlexDirection::Column)
            .children(elements);

        if let Some(key) = self.key {
            container = container.key(key);
        }

        container.into_element()
    }

    fn highlight_line(&self, line: &str) -> String {
        match self.language {
            Language::Plain => line.to_string(),
            Language::Rust => self.highlight_rust(line),
            Language::Python => self.highlight_python(line),
            Language::JavaScript | Language::TypeScript => self.highlight_js(line),
            Language::Go => self.highlight_go(line),
            Language::Json => self.highlight_json(line),
            Language::Yaml | Language::Toml => self.highlight_config(line),
            Language::Shell => self.highlight_shell(line),
            Language::Markdown => line.to_string(),
        }
    }

    fn highlight_rust(&self, line: &str) -> String {
        let keywords = [
            "fn", "let", "mut", "const", "static", "struct", "enum", "impl", "trait",
            "pub", "use", "mod", "crate", "self", "super", "where", "for", "loop",
            "while", "if", "else", "match", "return", "break", "continue", "async",
            "await", "move", "ref", "type", "dyn", "unsafe", "extern",
        ];
        self.highlight_generic(line, &keywords, "//")
    }

    fn highlight_python(&self, line: &str) -> String {
        let keywords = [
            "def", "class", "if", "elif", "else", "for", "while", "try", "except",
            "finally", "with", "as", "import", "from", "return", "yield", "raise",
            "pass", "break", "continue", "lambda", "and", "or", "not", "in", "is",
            "None", "True", "False", "self", "async", "await",
        ];
        self.highlight_generic(line, &keywords, "#")
    }

    fn highlight_js(&self, line: &str) -> String {
        let keywords = [
            "function", "const", "let", "var", "if", "else", "for", "while", "do",
            "switch", "case", "break", "continue", "return", "try", "catch", "finally",
            "throw", "class", "extends", "new", "this", "super", "import", "export",
            "default", "async", "await", "yield", "typeof", "instanceof", "null",
            "undefined", "true", "false",
        ];
        self.highlight_generic(line, &keywords, "//")
    }

    fn highlight_go(&self, line: &str) -> String {
        let keywords = [
            "func", "var", "const", "type", "struct", "interface", "map", "chan",
            "if", "else", "for", "range", "switch", "case", "default", "break",
            "continue", "return", "go", "defer", "select", "package", "import",
            "nil", "true", "false", "make", "new", "len", "cap", "append",
        ];
        self.highlight_generic(line, &keywords, "//")
    }

    fn highlight_json(&self, line: &str) -> String {
        let mut result = String::new();
        let mut in_string = false;
        let mut chars = line.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '"' && !in_string {
                in_string = true;
                result.push_str(&format!("\x1b[32m{}", c)); // Green for strings
            } else if c == '"' && in_string {
                in_string = false;
                result.push(c);
                result.push_str("\x1b[0m");
            } else if !in_string && (c.is_ascii_digit() || c == '-' || c == '.') {
                result.push_str(&format!("\x1b[33m{}\x1b[0m", c)); // Yellow for numbers
            } else if !in_string && (c == 't' || c == 'f' || c == 'n') {
                // Check for true, false, null
                let word: String = std::iter::once(c)
                    .chain(chars.by_ref().take_while(|&ch| ch.is_alphabetic()))
                    .collect();
                if word == "true" || word == "false" || word == "null" {
                    result.push_str(&format!("\x1b[35m{}\x1b[0m", word)); // Magenta for keywords
                } else {
                    result.push_str(&word);
                }
            } else {
                result.push(c);
            }
        }

        if in_string {
            result.push_str("\x1b[0m");
        }

        result
    }

    fn highlight_config(&self, line: &str) -> String {
        // Simple YAML/TOML highlighting
        if let Some(comment_start) = line.find('#') {
            let (code, comment) = line.split_at(comment_start);
            format!(
                "{}\x1b[90m{}\x1b[0m",
                self.highlight_config_line(code),
                comment
            )
        } else {
            self.highlight_config_line(line)
        }
    }

    fn highlight_config_line(&self, line: &str) -> String {
        if let Some(colon_pos) = line.find(':') {
            let (key, value) = line.split_at(colon_pos);
            format!("\x1b[36m{}\x1b[0m{}", key, value)
        } else if let Some(eq_pos) = line.find('=') {
            let (key, value) = line.split_at(eq_pos);
            format!("\x1b[36m{}\x1b[0m{}", key, value)
        } else {
            line.to_string()
        }
    }

    fn highlight_shell(&self, line: &str) -> String {
        let keywords = [
            "if", "then", "else", "elif", "fi", "for", "while", "do", "done",
            "case", "esac", "function", "return", "exit", "export", "local",
            "echo", "cd", "ls", "rm", "cp", "mv", "mkdir", "cat", "grep",
        ];
        self.highlight_generic(line, &keywords, "#")
    }

    fn highlight_generic(&self, line: &str, keywords: &[&str], comment_prefix: &str) -> String {
        // Check for comment
        if let Some(comment_start) = line.find(comment_prefix) {
            let (code, comment) = line.split_at(comment_start);
            return format!(
                "{}\x1b[90m{}\x1b[0m",
                self.highlight_code(code, keywords),
                comment
            );
        }

        self.highlight_code(line, keywords)
    }

    fn highlight_code(&self, line: &str, keywords: &[&str]) -> String {
        let mut result = String::new();
        let mut chars = line.chars().peekable();
        let mut in_string = false;
        let mut string_char = '"';

        while let Some(c) = chars.next() {
            // String handling
            if (c == '"' || c == '\'') && !in_string {
                in_string = true;
                string_char = c;
                result.push_str("\x1b[32m"); // Green
                result.push(c);
                continue;
            }

            if in_string {
                result.push(c);
                if c == string_char {
                    in_string = false;
                    result.push_str("\x1b[0m");
                }
                continue;
            }

            // Number handling
            if c.is_ascii_digit() {
                result.push_str("\x1b[33m"); // Yellow
                result.push(c);
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_digit() || next == '.' || next == 'x' || next == 'b' {
                        result.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                result.push_str("\x1b[0m");
                continue;
            }

            // Word/keyword handling
            if c.is_alphabetic() || c == '_' {
                let mut word = String::new();
                word.push(c);
                while let Some(&next) = chars.peek() {
                    if next.is_alphanumeric() || next == '_' {
                        word.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                if keywords.contains(&word.as_str()) {
                    result.push_str(&format!("\x1b[35m{}\x1b[0m", word)); // Magenta
                } else {
                    result.push_str(&word);
                }
                continue;
            }

            result.push(c);
        }

        if in_string {
            result.push_str("\x1b[0m");
        }

        result
    }
}

impl Default for CodeEditor {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_editor_creation() {
        let editor = CodeEditor::new("fn main() {}")
            .language(Language::Rust);
        assert_eq!(editor.language, Language::Rust);
    }

    #[test]
    fn test_empty_editor() {
        let editor = CodeEditor::new("");
        let _ = editor.into_element();
    }

    #[test]
    fn test_line_numbers() {
        let editor = CodeEditor::new("line 1\nline 2\nline 3")
            .show_line_numbers(true)
            .start_line(10);
        assert_eq!(editor.start_line, 10);
    }

    #[test]
    fn test_highlighting() {
        let code = r#"fn main() {
    let x = 42;
    println!("Hello");
}"#;
        let editor = CodeEditor::new(code).language(Language::Rust);
        let _ = editor.into_element();
    }

    #[test]
    fn test_cursor() {
        let editor = CodeEditor::new("code")
            .cursor(1, 5)
            .highlighted_line(1);
        assert_eq!(editor.cursor, Some((1, 5)));
        assert_eq!(editor.highlighted_line, Some(1));
    }

    #[test]
    fn test_json_highlighting() {
        let json = r#"{"key": "value", "num": 42, "bool": true}"#;
        let editor = CodeEditor::new(json).language(Language::Json);
        let _ = editor.into_element();
    }
}
