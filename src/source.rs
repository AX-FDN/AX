use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn join(left: Self, right: Self) -> Self {
        Self {
            start: left.start.min(right.start),
            end: left.end.max(right.end),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    path: PathBuf,
    text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)?;
        Ok(Self::new(path.to_path_buf(), text))
    }

    pub fn new(path: impl Into<PathBuf>, text: String) -> Self {
        let line_starts = compute_line_starts(&text);
        Self {
            path: path.into(),
            text,
            line_starts,
        }
    }

    pub fn anonymous(text: impl Into<String>) -> Self {
        Self::new("<memory>", text.into())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn display_path(&self) -> String {
        self.path.display().to_string()
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn slice(&self, span: Span) -> &str {
        &self.text[span.start.min(self.text.len())..span.end.min(self.text.len())]
    }

    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.text.len());
        let index = match self.line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        let line_start = self.line_starts[index];
        (index + 1, offset.saturating_sub(line_start) + 1)
    }

    pub fn line_text(&self, line_number: usize) -> &str {
        let index = line_number
            .saturating_sub(1)
            .min(self.line_starts.len().saturating_sub(1));
        let start = self.line_starts[index];
        let end = self
            .line_starts
            .get(index + 1)
            .copied()
            .unwrap_or(self.text.len());
        let mut slice = &self.text[start..end];
        if let Some(stripped) = slice.strip_suffix('\n') {
            slice = stripped;
        }
        if let Some(stripped) = slice.strip_suffix('\r') {
            slice = stripped;
        }
        slice
    }
}

fn compute_line_starts(text: &str) -> Vec<usize> {
    let mut line_starts = vec![0];
    for (index, ch) in text.char_indices() {
        if ch == '\n' && index + 1 < text.len() {
            line_starts.push(index + 1);
        }
    }
    line_starts
}

#[cfg(test)]
mod tests {
    use super::SourceFile;

    #[test]
    fn computes_line_and_column() {
        let source = SourceFile::anonymous("first\nsecond\nthird");
        assert_eq!(source.line_col(0), (1, 1));
        assert_eq!(source.line_col(7), (2, 2));
        assert_eq!(source.line_col(13), (3, 1));
    }
}
