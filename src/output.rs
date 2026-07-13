use std::fmt::Write;

use crate::diff::{DiffKind, DiffReport};

pub fn render_plain(report: &DiffReport, show_identical: bool) -> String {
    let mut output = String::new();
    let summary = &report.summary;
    writeln!(output, "fdiff").unwrap();
    writeln!(output, "left   {}", report.left_root.display()).unwrap();
    writeln!(output, "right  {}", report.right_root.display()).unwrap();
    writeln!(
        output,
        "summary  {} left-only · {} changed · {} type · {} right-only · {} same",
        summary.left_only,
        summary.modified,
        summary.type_changed,
        summary.right_only,
        summary.identical,
    )
    .unwrap();
    writeln!(output, "────────────────────────────────────────").unwrap();

    for entry in report
        .entries
        .iter()
        .filter(|entry| show_identical || entry.kind != DiffKind::Identical)
    {
        writeln!(
            output,
            "{:<10} {}",
            entry.kind.plain_label(),
            entry.path.display()
        )
        .unwrap();
    }

    if !show_identical && !report.has_differences() {
        writeln!(output, "no differences").unwrap();
    }
    output
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::diff::DiffEngine;

    use super::*;

    #[test]
    fn plain_output_is_stable_and_hides_identical_entries_by_default() {
        let left = tempdir().unwrap();
        let right = tempdir().unwrap();
        fs::write(left.path().join("same"), "same").unwrap();
        fs::write(right.path().join("same"), "same").unwrap();
        fs::write(left.path().join("left"), "left").unwrap();

        let report = DiffEngine::new(left.path(), right.path())
            .unwrap()
            .scan()
            .unwrap();
        let output = render_plain(&report, false);

        assert!(output.contains("1 left-only"));
        assert!(output.contains("← left     left"));
        assert!(!output.lines().any(|line| line.starts_with("= same")));
    }
}
