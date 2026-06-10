use similar::{ChangeTag, TextDiff};
use std::fmt::Write;

/// git diff 스타일로 두 텍스트의 차이를 렌더링
pub fn render_diff(old_label: &str, new_label: &str, old_text: &str, new_text: &str) -> String {
    let diff = TextDiff::from_lines(old_text, new_text);
    let mut output = String::new();

    writeln!(output, "── DIFF ─────────────────────────────────────").unwrap();
    writeln!(output, "--- 기존: {}", old_label).unwrap();
    writeln!(output, "+++ 신규: {}", new_label).unwrap();

    let mut added = 0usize;
    let mut removed = 0usize;

    for group in diff.grouped_ops(3) {
        for op in &group {
            for change in diff.iter_changes(op) {
                let (sign, _line) = match change.tag() {
                    ChangeTag::Delete => {
                        removed += 1;
                        ('-', change.value())
                    }
                    ChangeTag::Insert => {
                        added += 1;
                        ('+', change.value())
                    }
                    ChangeTag::Equal => (' ', change.value()),
                };
                write!(output, "{}{}", sign, change.value()).unwrap();
                if !change.value().ends_with('\n') {
                    writeln!(output).unwrap();
                }
            }
        }
    }

    writeln!(output, "──────────────────────────────────────────────").unwrap();
    writeln!(output, "  +{} 추가  -{} 삭제", added, removed).unwrap();

    output
}

/// 두 텍스트 간 유사도 계산 (0.0 ~ 1.0)
pub fn text_similarity(a: &str, b: &str) -> f64 {
    let diff = TextDiff::from_lines(a, b);
    diff.ratio().into()
}

/// 코사인 유사도 계산
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

/// 코사인 거리 (1 - similarity). 값이 작을수록 유사.
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    1.0 - cosine_similarity(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_diff() {
        let old = "줄1\n줄2\n줄3\n";
        let new = "줄1\n줄2 수정\n줄3\n줄4\n";
        let diff = render_diff("old.txt", "new.txt", old, new);
        assert!(diff.contains("+"));
        assert!(diff.contains("-"));
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 2.0, 3.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_distance() {
        let v = vec![1.0, 0.0];
        let w = vec![0.0, 1.0];
        assert!((cosine_distance(&v, &w) - 1.0).abs() < 1e-6);
    }
}
