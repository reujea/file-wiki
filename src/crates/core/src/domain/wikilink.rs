//! Phase 83: 위키링크 추출
//!
//! 마크다운 본문에서 `[[xxx]]` 또는 `[[xxx#section]]` 패턴 추출.
//! crossref 처리 시 명시적 관계(UserWikilink origin)로 등록.

/// `[[xxx]]` 또는 `[[xxx#section]]` 또는 `[[xxx|alias]]` 패턴에서 target 식별자 추출.
/// 반환은 소문자화·trim된 unique 리스트.
pub fn extract_wikilinks(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 3 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            // 종료 ]] 찾기
            let start = i + 2;
            let mut end = start;
            while end + 1 < bytes.len() && !(bytes[end] == b']' && bytes[end + 1] == b']') {
                end += 1;
            }
            if end + 1 >= bytes.len() { break; }
            // 내용 슬라이스
            if let Ok(content) = std::str::from_utf8(&bytes[start..end]) {
                // alias는 | 앞부분만, section은 # 앞부분만
                let target = content
                    .split('|').next().unwrap_or("")
                    .split('#').next().unwrap_or("")
                    .trim()
                    .to_lowercase();
                if !target.is_empty() && seen.insert(target.clone()) {
                    out.push(target);
                }
            }
            i = end + 2;
        } else {
            i += 1;
        }
    }
    out
}

/// 위키링크 target → 코퍼스 doc_id 매칭.
/// 단순: 가공본 path의 파일명(확장자 제거)과 case-insensitive 비교.
/// 향후 정교화: 제목 metadata, alias 사전 등.
pub fn resolve_wikilink_target(
    target: &str,
    docs: &[crate::domain::models::StoredDocSummary],
) -> Option<String> {
    let t = target.to_lowercase();
    docs.iter().find_map(|d| {
        let stem = std::path::Path::new(&d.path)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        if stem == t { Some(d.id.clone()) } else { None }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple() {
        let text = "이 문서는 [[other-doc]]를 참조합니다.";
        let links = extract_wikilinks(text);
        assert_eq!(links, vec!["other-doc"]);
    }

    #[test]
    fn test_extract_with_section() {
        let text = "[[doc-a#section1]] 참고";
        let links = extract_wikilinks(text);
        assert_eq!(links, vec!["doc-a"]);
    }

    #[test]
    fn test_extract_with_alias() {
        let text = "[[real-target|보이는 이름]] 사용";
        let links = extract_wikilinks(text);
        assert_eq!(links, vec!["real-target"]);
    }

    #[test]
    fn test_extract_multiple_dedup() {
        let text = "[[a]] 그리고 [[b]] 또 [[a]]";
        let links = extract_wikilinks(text);
        assert_eq!(links, vec!["a", "b"]);
    }

    #[test]
    fn test_extract_empty() {
        assert!(extract_wikilinks("일반 텍스트").is_empty());
        assert!(extract_wikilinks("").is_empty());
    }

    #[test]
    fn test_extract_unclosed() {
        // 닫히지 않은 [[는 무시
        assert!(extract_wikilinks("[[unclosed").is_empty());
    }

    #[test]
    fn test_extract_korean() {
        let text = "[[한국어-제목]] 참고";
        let links = extract_wikilinks(text);
        assert_eq!(links, vec!["한국어-제목"]);
    }
}
