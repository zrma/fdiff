use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use similar::{ChangeTag, TextDiff};

pub const MAX_INLINE_FILE_SIZE: u64 = 4 * 1024 * 1024;
const DIFF_TIMEOUT: Duration = Duration::from_millis(250);
const BINARY_PREVIEW_BYTES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentRowKind {
    Equal,
    Modified,
    LeftOnly,
    RightOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentRow {
    pub left_number: Option<usize>,
    pub left_text: String,
    pub right_number: Option<usize>,
    pub right_text: String,
    pub kind: ContentRowKind,
}

impl ContentRow {
    pub fn is_difference(&self) -> bool {
        self.kind != ContentRowKind::Equal
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextContentDiff {
    pub rows: Vec<ContentRow>,
    pub difference_rows: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryContentDiff {
    pub left_bytes: u64,
    pub right_bytes: u64,
    pub first_difference: Option<usize>,
    pub left_preview: String,
    pub right_preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentDiffKind {
    Text(TextContentDiff),
    Binary(BinaryContentDiff),
    TooLarge {
        left_bytes: u64,
        right_bytes: u64,
        limit_bytes: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileContentDiff {
    pub relative_path: PathBuf,
    pub kind: ContentDiffKind,
}

#[derive(Debug)]
struct SideLine {
    number: usize,
    text: String,
    newline_terminated: bool,
}

pub fn load_content_diff(
    left_root: &Path,
    right_root: &Path,
    relative_path: &Path,
) -> Result<FileContentDiff> {
    let left_path = left_root.join(relative_path);
    let right_path = right_root.join(relative_path);
    let left_bytes = fs::metadata(&left_path)
        .with_context(|| format!("failed to inspect left file {}", relative_path.display()))?
        .len();
    let right_bytes = fs::metadata(&right_path)
        .with_context(|| format!("failed to inspect right file {}", relative_path.display()))?
        .len();

    if left_bytes > MAX_INLINE_FILE_SIZE || right_bytes > MAX_INLINE_FILE_SIZE {
        return Ok(FileContentDiff {
            relative_path: relative_path.to_path_buf(),
            kind: ContentDiffKind::TooLarge {
                left_bytes,
                right_bytes,
                limit_bytes: MAX_INLINE_FILE_SIZE,
            },
        });
    }

    let left = fs::read(&left_path)
        .with_context(|| format!("failed to read left file {}", relative_path.display()))?;
    let right = fs::read(&right_path)
        .with_context(|| format!("failed to read right file {}", relative_path.display()))?;

    let kind = match (std::str::from_utf8(&left), std::str::from_utf8(&right)) {
        (Ok(left_text), Ok(right_text)) if !left.contains(&0) && !right.contains(&0) => {
            ContentDiffKind::Text(build_text_diff(left_text, right_text))
        }
        _ => ContentDiffKind::Binary(build_binary_diff(&left, &right)),
    };

    Ok(FileContentDiff {
        relative_path: relative_path.to_path_buf(),
        kind,
    })
}

fn build_text_diff(left: &str, right: &str) -> TextContentDiff {
    let mut config = TextDiff::configure();
    config.timeout(DIFF_TIMEOUT);
    let diff = config.diff_lines(left, right);
    let mut rows = Vec::new();
    let mut deleted = Vec::new();
    let mut inserted = Vec::new();

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                flush_changed_block(&mut rows, &mut deleted, &mut inserted);
                let value = change.value();
                rows.push(ContentRow {
                    left_number: change.old_index().map(|index| index + 1),
                    left_text: display_text(value),
                    right_number: change.new_index().map(|index| index + 1),
                    right_text: display_text(value),
                    kind: ContentRowKind::Equal,
                });
            }
            ChangeTag::Delete => deleted.push(side_line(
                change.old_index().expect("deleted line has an old index") + 1,
                change.value(),
            )),
            ChangeTag::Insert => inserted.push(side_line(
                change.new_index().expect("inserted line has a new index") + 1,
                change.value(),
            )),
        }
    }
    flush_changed_block(&mut rows, &mut deleted, &mut inserted);

    TextContentDiff {
        difference_rows: rows.iter().filter(|row| row.is_difference()).count(),
        rows,
    }
}

fn flush_changed_block(
    rows: &mut Vec<ContentRow>,
    deleted: &mut Vec<SideLine>,
    inserted: &mut Vec<SideLine>,
) {
    let count = deleted.len().max(inserted.len());
    for index in 0..count {
        let left = deleted.get(index);
        let right = inserted.get(index);
        let (left_text, right_text) = paired_display_text(left, right);
        rows.push(ContentRow {
            left_number: left.map(|line| line.number),
            left_text,
            right_number: right.map(|line| line.number),
            right_text,
            kind: match (left, right) {
                (Some(_), Some(_)) => ContentRowKind::Modified,
                (Some(_), None) => ContentRowKind::LeftOnly,
                (None, Some(_)) => ContentRowKind::RightOnly,
                (None, None) => unreachable!("changed block contains at least one line"),
            },
        });
    }
    deleted.clear();
    inserted.clear();
}

fn side_line(number: usize, value: &str) -> SideLine {
    SideLine {
        number,
        text: display_text(value),
        newline_terminated: value.ends_with('\n'),
    }
}

fn paired_display_text(left: Option<&SideLine>, right: Option<&SideLine>) -> (String, String) {
    let mut left_text = left.map(|line| line.text.clone()).unwrap_or_default();
    let mut right_text = right.map(|line| line.text.clone()).unwrap_or_default();
    if let (Some(left), Some(right)) = (left, right)
        && left.text == right.text
        && left.newline_terminated != right.newline_terminated
    {
        left_text.push_str(if left.newline_terminated {
            "  ↵"
        } else {
            "  [no newline]"
        });
        right_text.push_str(if right.newline_terminated {
            "  ↵"
        } else {
            "  [no newline]"
        });
    }
    (left_text, right_text)
}

fn display_text(value: &str) -> String {
    let without_newline = value.strip_suffix('\n').unwrap_or(value);
    let without_line_ending = without_newline
        .strip_suffix('\r')
        .unwrap_or(without_newline);
    let mut output = String::with_capacity(without_line_ending.len());
    for character in without_line_ending.chars() {
        match character {
            '\t' => output.push_str("    "),
            character if character.is_control() => output.push('�'),
            character => output.push(character),
        }
    }
    output
}

fn build_binary_diff(left: &[u8], right: &[u8]) -> BinaryContentDiff {
    let first_difference = left
        .iter()
        .zip(right)
        .position(|(left, right)| left != right)
        .or_else(|| (left.len() != right.len()).then_some(left.len().min(right.len())));
    let preview_start = first_difference
        .unwrap_or(0)
        .saturating_sub(BINARY_PREVIEW_BYTES / 2);

    BinaryContentDiff {
        left_bytes: left.len() as u64,
        right_bytes: right.len() as u64,
        first_difference,
        left_preview: hex_preview(left, preview_start),
        right_preview: hex_preview(right, preview_start),
    }
}

fn hex_preview(bytes: &[u8], start: usize) -> String {
    bytes
        .iter()
        .skip(start)
        .take(BINARY_PREVIEW_BYTES)
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn text_diff_aligns_replaced_added_and_removed_lines() {
        let diff = build_text_diff(
            "same\nold\nremoved\ntail\n",
            "same\nnew\nadded-one\nadded-two\ntail\n",
        );

        assert_eq!(diff.rows[0].kind, ContentRowKind::Equal);
        assert_eq!(diff.rows[1].kind, ContentRowKind::Modified);
        assert_eq!(diff.rows[1].left_text, "old");
        assert_eq!(diff.rows[1].right_text, "new");
        assert_eq!(diff.rows[3].kind, ContentRowKind::RightOnly);
        assert_eq!(diff.rows[4].kind, ContentRowKind::Equal);
        assert_eq!(diff.difference_rows, 3);
    }

    #[test]
    fn text_diff_exposes_missing_final_newline() {
        let diff = build_text_diff("value\n", "value");

        assert_eq!(diff.rows.len(), 1);
        assert_eq!(diff.rows[0].kind, ContentRowKind::Modified);
        assert!(diff.rows[0].left_text.contains('↵'));
        assert!(diff.rows[0].right_text.contains("no newline"));
    }

    #[test]
    fn text_diff_sanitizes_terminal_control_characters() {
        let diff = build_text_diff("safe\n", "unsafe\u{1b}[31m\n");

        assert!(!diff.rows[0].right_text.contains('\u{1b}'));
        assert!(diff.rows[0].right_text.contains('�'));
    }

    #[test]
    fn binary_diff_reports_first_different_byte_and_safe_preview() {
        let left = tempdir().unwrap();
        let right = tempdir().unwrap();
        fs::write(left.path().join("value.bin"), [0, 1, 2, 3]).unwrap();
        fs::write(right.path().join("value.bin"), [0, 1, 9, 3]).unwrap();

        let diff = load_content_diff(left.path(), right.path(), Path::new("value.bin")).unwrap();
        let ContentDiffKind::Binary(binary) = diff.kind else {
            panic!("binary fixture should produce a binary summary");
        };

        assert_eq!(binary.first_difference, Some(2));
        assert_eq!(binary.left_preview, "00 01 02 03");
        assert_eq!(binary.right_preview, "00 01 09 03");
    }

    #[test]
    fn oversized_files_are_not_loaded_for_inline_diff() {
        let left = tempdir().unwrap();
        let right = tempdir().unwrap();
        let left_file = File::create(left.path().join("large")).unwrap();
        left_file.set_len(MAX_INLINE_FILE_SIZE + 1).unwrap();
        let right_file = File::create(right.path().join("large")).unwrap();
        right_file.set_len(MAX_INLINE_FILE_SIZE + 1).unwrap();

        let diff = load_content_diff(left.path(), right.path(), Path::new("large")).unwrap();
        assert!(matches!(diff.kind, ContentDiffKind::TooLarge { .. }));
    }
}
