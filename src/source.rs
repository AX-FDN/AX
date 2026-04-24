use std::fs;
use std::io;
use std::ops::Range;
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
    segments: Vec<SourceSegment>,
}

#[derive(Debug, Clone)]
struct SourceSegment {
    path: PathBuf,
    range: Range<usize>,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)?;
        Ok(Self::new(path.to_path_buf(), text))
    }

    pub fn new(path: impl Into<PathBuf>, text: String) -> Self {
        let path = path.into();
        let line_starts = compute_line_starts(&text);
        let length = text.len();
        Self {
            path: path.clone(),
            text,
            segments: vec![SourceSegment {
                path,
                range: 0..length,
                line_starts,
            }],
        }
    }

    pub fn anonymous(text: impl Into<String>) -> Self {
        Self::new("<memory>", text.into())
    }

    pub fn from_segments(path: impl Into<PathBuf>, segments: Vec<(PathBuf, String)>) -> Self {
        let path = path.into();
        let mut combined = String::new();
        let mut built_segments = Vec::new();

        for (segment_path, mut segment_text) in segments {
            if !segment_text.ends_with('\n') {
                segment_text.push('\n');
            }

            let start = combined.len();
            let line_starts = compute_line_starts(&segment_text);
            combined.push_str(&segment_text);
            let end = combined.len();

            built_segments.push(SourceSegment {
                path: segment_path,
                range: start..end,
                line_starts,
            });
        }

        Self {
            path,
            text: combined,
            segments: built_segments,
        }
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

    pub fn display_path_for_offset(&self, offset: usize) -> &str {
        self.segment_for_offset(offset)
            .path
            .to_str()
            .unwrap_or("<invalid-path>")
    }

    pub fn slice(&self, span: Span) -> &str {
        &self.text[span.start.min(self.text.len())..span.end.min(self.text.len())]
    }

    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let segment = self.segment_for_offset(offset);
        let local_offset = offset
            .min(segment.range.end)
            .saturating_sub(segment.range.start);
        let index = match segment.line_starts.binary_search(&local_offset) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        let line_start = segment.line_starts[index];
        (index + 1, local_offset.saturating_sub(line_start) + 1)
    }

    pub fn line_text(&self, line_number: usize) -> &str {
        let segment = self
            .segments
            .first()
            .expect("source should always include at least one segment");
        self.line_text_in_segment(segment, line_number)
    }

    pub fn line_text_for_offset(&self, offset: usize, line_number: usize) -> &str {
        let segment = self.segment_for_offset(offset);
        self.line_text_in_segment(segment, line_number)
    }

    pub fn segment_end(&self, offset: usize) -> usize {
        self.segment_for_offset(offset).range.end
    }

    fn segment_for_offset(&self, offset: usize) -> &SourceSegment {
        let clamped = offset.min(self.text.len());

        self.segments
            .iter()
            .find(|segment| {
                if clamped == self.text.len() {
                    segment.range.end == self.text.len()
                } else {
                    segment.range.start <= clamped && clamped < segment.range.end
                }
            })
            .or_else(|| self.segments.last())
            .expect("source should always include at least one segment")
    }

    fn line_text_in_segment<'a>(&'a self, segment: &SourceSegment, line_number: usize) -> &'a str {
        let index = line_number
            .saturating_sub(1)
            .min(segment.line_starts.len().saturating_sub(1));
        let start = segment.range.start + segment.line_starts[index];
        let end = segment
            .line_starts
            .get(index + 1)
            .map(|start| segment.range.start + *start)
            .unwrap_or(segment.range.end);
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
    use std::path::PathBuf;

    #[test]
    fn computes_line_and_column() {
        let source = SourceFile::anonymous("first\nsecond\nthird");
        assert_eq!(source.line_col(0), (1, 1));
        assert_eq!(source.line_col(7), (2, 2));
        assert_eq!(source.line_col(13), (3, 1));
    }

    #[test]
    fn maps_offsets_back_to_original_segments() {
        let source = SourceFile::from_segments(
            "src/main.ax",
            vec![
                (
                    PathBuf::from("src/lib.ax"),
                    "fn helper() -> i32 {\n    return 1;\n}".to_string(),
                ),
                (
                    PathBuf::from("src/main.ax"),
                    "fn main() -> i32 {\n    return helper();\n}".to_string(),
                ),
            ],
        );

        let helper_offset = source
            .text()
            .find("return 1")
            .expect("helper return should exist");
        let main_offset = source
            .text()
            .find("return helper")
            .expect("main return should exist");

        assert_eq!(source.display_path_for_offset(helper_offset), "src/lib.ax");
        assert_eq!(source.line_col(helper_offset), (2, 5));
        assert_eq!(
            source.line_text_for_offset(helper_offset, 2),
            "    return 1;"
        );

        assert_eq!(source.display_path_for_offset(main_offset), "src/main.ax");
        assert_eq!(source.line_col(main_offset), (2, 5));
        assert_eq!(
            source.line_text_for_offset(main_offset, 2),
            "    return helper();"
        );
    }
}
