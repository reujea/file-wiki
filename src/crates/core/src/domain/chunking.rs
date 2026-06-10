//! 대용량 파일 청크 분할
//!
//! CHUNK_SIZE(40KB) 초과 파일을 의미 단위로 분할.
//! 각 청크는 독립적으로 에이전트에게 위임.

/// 텍스트를 지정 크기 이하 청크로 분할 (줄 단위 경계)
pub fn split_into_chunks(content: &str, chunk_size: usize) -> Vec<(usize, String)> {
    if content.len() <= chunk_size {
        return vec![(0, content.to_string())];
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    let mut chunk_idx = 0;

    while start < content.len() {
        let mut end = (start + chunk_size).min(content.len());

        // UTF-8 안전한 경계
        while end < content.len() && !content.is_char_boundary(end) {
            end -= 1;
        }

        // 줄 단위 경계로 조정 (뒤로)
        if end < content.len() {
            if let Some(newline_pos) = content[start..end].rfind('\n') {
                end = start + newline_pos + 1;
            }
        }

        let chunk = content[start..end].to_string();
        if !chunk.trim().is_empty() {
            chunks.push((chunk_idx, chunk));
            chunk_idx += 1;
        }
        start = end;
    }

    chunks
}

/// 청크 수 + 전체 크기 정보
pub struct ChunkInfo {
    pub total_chunks: usize,
    pub total_bytes: usize,
    pub avg_chunk_bytes: usize,
}

pub fn chunk_info(content: &str, chunk_size: usize) -> ChunkInfo {
    let chunks = split_into_chunks(content, chunk_size);
    let total = chunks.len();
    ChunkInfo {
        total_chunks: total,
        total_bytes: content.len(),
        avg_chunk_bytes: if total > 0 { content.len() / total } else { 0 },
    }
}

// ── 의미 단위 청킹 (Phase A) ────────────────────────────────

/// 의미 단위 청킹 설정
#[derive(Debug, Clone)]
pub struct SemanticChunkConfig {
    /// 목표 청크 크기 (바이트, 대략 토큰*4)
    pub target_bytes: usize,
    /// 최대 청크 크기 (이 이상이면 강제 분할)
    pub max_bytes: usize,
    /// 오버랩 문장 수 (앞 청크 마지막 N문장을 다음 청크 앞에 복사)
    pub overlap_sentences: usize,
    /// 코드 펜스 보존 (true면 ``` 블록 내부 절단 금지)
    pub preserve_code_blocks: bool,
    /// 표 마크다운 보존 (true면 `|...|` 표 블록 내부 절단 금지) — Phase 85 트리거 #8 인프라
    pub preserve_tables: bool,
}

impl Default for SemanticChunkConfig {
    fn default() -> Self {
        Self {
            target_bytes: 1500,  // ~375토큰
            max_bytes: 2500,     // ~625토큰
            overlap_sentences: 2,
            preserve_code_blocks: true,
            preserve_tables: false, // 트리거 #8 대기 — 표 비중 높은 도메인 진입 시 활성화
        }
    }
}

/// 의미 단위 청크 결과
#[derive(Debug, Clone)]
pub struct SemanticChunk {
    pub index: usize,
    pub text: String,
    /// 오버랩으로 추가된 접두사 (없으면 빈 문자열)
    pub overlap_prefix: String,
    /// 상위 제목 계층 (H1>H2>H3 순서). 빈 Vec이면 헤딩 없음.
    /// Phase 61 G1 — 검색 시 문맥 파악 + sparse 키워드 매칭에 활용.
    pub title_path: Vec<String>,
}

/// 마크다운 문서를 의미 단위로 청킹
///
/// 분할 우선순위:
/// 1. `##` / `###` 헤딩 경계
/// 2. `---` 수평선
/// 3. 빈 줄 (단락 경계)
/// 4. 줄 단위 (fallback)
///
/// 코드 펜스(```) 내부는 절단하지 않음.
pub fn split_semantic(content: &str, config: &SemanticChunkConfig) -> Vec<SemanticChunk> {
    let sections_with_path = split_by_headings_with_path(content, config.preserve_code_blocks, config.preserve_tables);

    let mut chunks: Vec<SemanticChunk> = Vec::new();
    let mut chunk_idx = 0;

    for (path, section) in &sections_with_path {
        if section.trim().is_empty() {
            continue;
        }

        if section.len() <= config.max_bytes {
            chunks.push(SemanticChunk {
                index: chunk_idx,
                text: section.clone(),
                overlap_prefix: String::new(),
                title_path: path.clone(),
            });
            chunk_idx += 1;
        } else {
            let sub_chunks = split_section_by_paragraphs(section, config);
            for sub in sub_chunks {
                chunks.push(SemanticChunk {
                    index: chunk_idx,
                    text: sub,
                    overlap_prefix: String::new(),
                    title_path: path.clone(),
                });
                chunk_idx += 1;
            }
        }
    }

    if config.overlap_sentences > 0 && chunks.len() > 1 {
        apply_overlap(&mut chunks, config.overlap_sentences);
    }

    chunks
}

/// 표 마크다운 라인 여부 — `|...|` 형식 (구분선 `|---|---|` 포함)
fn is_table_line(trimmed: &str) -> bool {
    trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() >= 2
}

/// 헤딩/수평선 기준으로 섹션 분리 + H1/H2/H3 path 추적 (Phase 61 G1)
///
/// 반환: `Vec<(title_path, section_text)>` — title_path는 H1>H2>H3 경로.
fn split_by_headings_with_path(content: &str, preserve_code: bool, preserve_tables: bool) -> Vec<(Vec<String>, String)> {
    let mut sections: Vec<(Vec<String>, String)> = Vec::new();
    let mut current = String::new();
    let mut in_code_block = false;
    let mut in_table = false;
    // H1, H2, H3별 현재 제목 (없으면 None)
    let mut headings: [Option<String>; 3] = [None, None, None];
    // current 섹션 시작 시점의 path
    let mut current_path: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if preserve_code && trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            current.push_str(line);
            current.push('\n');
            continue;
        }

        if in_code_block {
            current.push_str(line);
            current.push('\n');
            continue;
        }

        // 표 상태 갱신 — preserve_tables 활성 시에만 추적
        if preserve_tables {
            in_table = is_table_line(trimmed);
        }

        // 헤딩 레벨 감지 (# H1, ## H2, ### H3)
        let heading_level: Option<usize> = if trimmed.starts_with("# ") && !trimmed.starts_with("## ") {
            Some(0)
        } else if trimmed.starts_with("## ") && !trimmed.starts_with("### ") {
            Some(1)
        } else if trimmed.starts_with("### ") {
            Some(2)
        } else {
            None
        };

        let is_hr = trimmed == "---" || trimmed == "***" || trimmed == "___";

        // in_table 중에는 분할 금지 (표 내부 절단 방지)
        if (heading_level.is_some() || is_hr) && !current.trim().is_empty() && !in_table {
            sections.push((current_path.clone(), current.clone()));
            current.clear();
        }

        if let Some(level) = heading_level {
            // 헤딩 텍스트 추출 ("## 제목" → "제목")
            let title = trimmed.trim_start_matches('#').trim().to_string();
            headings[level] = Some(title);
            // 하위 헤딩 초기화 (H1 변경 시 H2/H3도 리셋)
            for h in headings.iter_mut().skip(level + 1).take(3 - level - 1) {
                *h = None;
            }
            // current_path 갱신
            current_path = headings.iter().filter_map(|h| h.clone()).collect();
        }

        current.push_str(line);
        current.push('\n');
    }

    if !current.trim().is_empty() {
        sections.push((current_path, current));
    }

    sections
}

/// 헤딩/수평선 기준 섹션 분리 (path 정보 없음, 호환용)
#[allow(dead_code)]
fn split_by_headings(content: &str, preserve_code: bool) -> Vec<String> {
    let mut sections: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_code_block = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // 코드 펜스 토글
        if preserve_code && trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            current.push_str(line);
            current.push('\n');
            continue;
        }

        if in_code_block {
            current.push_str(line);
            current.push('\n');
            continue;
        }

        // 헤딩 또는 수평선에서 분할
        let is_heading = trimmed.starts_with("## ") || trimmed.starts_with("### ");
        let is_hr = trimmed == "---" || trimmed == "***" || trimmed == "___";

        if (is_heading || is_hr) && !current.trim().is_empty() {
            sections.push(current.clone());
            current.clear();
        }

        current.push_str(line);
        current.push('\n');
    }

    if !current.trim().is_empty() {
        sections.push(current);
    }

    sections
}

/// 단락(빈 줄) 기준으로 큰 섹션을 target_bytes 이하로 분할
fn split_section_by_paragraphs(section: &str, config: &SemanticChunkConfig) -> Vec<String> {
    let paragraphs = split_paragraphs(section, config.preserve_code_blocks, config.preserve_tables);
    let mut chunks: Vec<String> = Vec::new();
    let mut current = String::new();

    for para in &paragraphs {
        if current.len() + para.len() > config.max_bytes && !current.trim().is_empty() {
            chunks.push(current.clone());
            current.clear();
        }
        current.push_str(para);
    }

    if !current.trim().is_empty() {
        chunks.push(current);
    }

    // 여전히 max 초과인 청크는 줄 단위로 강제 분할
    let mut result = Vec::new();
    for chunk in chunks {
        if chunk.len() <= config.max_bytes {
            result.push(chunk);
        } else {
            result.extend(force_split_lines(&chunk, config.target_bytes));
        }
    }

    result
}

/// 빈 줄 기준 단락 분리 (코드 펜스 + 표 마크다운 내부 보호)
fn split_paragraphs(text: &str, preserve_code: bool, preserve_tables: bool) -> Vec<String> {
    let mut paragraphs: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_code = false;
    let mut in_table = false;
    let mut prev_empty = false;

    for line in text.lines() {
        let trimmed = line.trim();

        if preserve_code && trimmed.starts_with("```") {
            in_code = !in_code;
        }

        // 표 상태 — 표 라인 사이의 빈 줄도 표 내부로 간주 (대부분 표는 연속 라인이라 실제 영향 적음)
        if preserve_tables && !in_code {
            in_table = is_table_line(trimmed);
        }

        if !in_code && !in_table && trimmed.is_empty() {
            if !prev_empty && !current.trim().is_empty() {
                current.push('\n');
                paragraphs.push(current.clone());
                current.clear();
            }
            prev_empty = true;
            continue;
        }

        prev_empty = false;
        current.push_str(line);
        current.push('\n');
    }

    if !current.trim().is_empty() {
        paragraphs.push(current);
    }

    paragraphs
}

/// 줄 단위 강제 분할 (최후 수단)
fn force_split_lines(text: &str, target: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for line in text.lines() {
        if current.len() + line.len() + 1 > target && !current.trim().is_empty() {
            chunks.push(current.clone());
            current.clear();
        }
        current.push_str(line);
        current.push('\n');
    }

    if !current.trim().is_empty() {
        chunks.push(current);
    }

    chunks
}

/// 오버랩 적용: 이전 청크 마지막 N문장을 다음 청크 앞에 추가
fn apply_overlap(chunks: &mut [SemanticChunk], overlap_sentences: usize) {
    if chunks.len() < 2 {
        return;
    }

    let prev_tails: Vec<String> = chunks.iter().map(|c| {
        let sentences: Vec<&str> = c.text.lines()
            .filter(|l| !l.trim().is_empty())
            .collect();
        let start = sentences.len().saturating_sub(overlap_sentences);
        sentences[start..].join("\n")
    }).collect();

    for i in 1..chunks.len() {
        let prefix = &prev_tails[i - 1];
        if !prefix.trim().is_empty() {
            chunks[i].overlap_prefix = prefix.clone();
            chunks[i].text = format!("{}\n\n{}", prefix, chunks[i].text);
        }
    }
}

// ── Phase B: ChunkingStrategy 추상화 (Adaptive Chunking 인프라 선구현) ──
// arxiv 2603.25333 흡수 — prd/research/external-analysis-2026-06-04-adaptive-chunking.md
// lesson 30 패턴 (인프라 + 디폴트 비활성 + 트리거 도달 시 활성화)

/// 청킹 전략. config `chunking.strategy` 디폴트 "semantic" (호환).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkingStrategy {
    /// 고정 크기 — `split_into_chunks` (바이트 단위). target_bytes만 사용
    Fixed,
    /// 의미 단위 (현재 디폴트) — `split_semantic`
    Semantic,
    /// 재귀 분할 — 헤딩 → 단락 → 문장 순서 폴백. Semantic의 단순 변형
    Recursive,
    /// Adaptive (논문 본체) — 본 인프라 단계에서는 Semantic으로 fallback.
    /// Phase C에서 4지표 측정 기반 동적 선택 본체 추가.
    Adaptive,
}

impl ChunkingStrategy {
    pub fn from_str_or_default(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "fixed" => Self::Fixed,
            "semantic" => Self::Semantic,
            "recursive" => Self::Recursive,
            "adaptive" => Self::Adaptive,
            _ => Self::Semantic,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::Semantic => "semantic",
            Self::Recursive => "recursive",
            Self::Adaptive => "adaptive",
        }
    }
}

impl Default for ChunkingStrategy {
    fn default() -> Self {
        Self::Semantic
    }
}

/// 단일 진입점 (메타 룰 1 sub-rule 1f) — 모든 청킹 호출은 본 함수 경유 권장.
/// 기존 `split_semantic` 직접 호출도 호환 유지.
pub fn chunk_by_strategy(
    content: &str,
    strategy: ChunkingStrategy,
    config: &SemanticChunkConfig,
) -> Vec<SemanticChunk> {
    match strategy {
        ChunkingStrategy::Fixed => chunk_fixed(content, config.target_bytes),
        ChunkingStrategy::Semantic => split_semantic(content, config),
        ChunkingStrategy::Recursive => chunk_recursive(content, config),
        // Phase C 진입 전까지 Semantic으로 위임 (lesson 30 인프라 디폴트)
        ChunkingStrategy::Adaptive => split_semantic(content, config),
    }
}

/// 고정 크기 청킹 (기존 split_into_chunks를 SemanticChunk로 래핑)
fn chunk_fixed(content: &str, chunk_size: usize) -> Vec<SemanticChunk> {
    split_into_chunks(content, chunk_size)
        .into_iter()
        .map(|(idx, text)| SemanticChunk {
            index: idx,
            text,
            overlap_prefix: String::new(),
            title_path: vec![],
        })
        .collect()
}

/// 재귀 분할 — Semantic이 이미 헤딩 → 단락 폴백 수행하므로 현 단계에서는 Semantic 위임.
/// Phase C 진입 시 LangChain RecursiveCharacterTextSplitter 패턴 추가 검토.
fn chunk_recursive(content: &str, config: &SemanticChunkConfig) -> Vec<SemanticChunk> {
    split_semantic(content, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_content_single_chunk() {
        let chunks = split_into_chunks("hello world", 1000);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].0, 0);
        assert_eq!(chunks[0].1, "hello world");
    }

    #[test]
    fn test_split_at_newline() {
        let content = "line1\nline2\nline3\nline4\nline5\n";
        let chunks = split_into_chunks(content, 12); // ~2줄씩
        assert!(chunks.len() >= 2);
        // 각 청크가 줄 단위로 끊어져야 함
        for (_, chunk) in &chunks {
            assert!(chunk.ends_with('\n') || chunk == chunks.last().unwrap().1.as_str());
        }
    }

    #[test]
    fn test_korean_utf8_safe() {
        let content = "가나다라마바사아자차카타파하\n".repeat(100);
        let chunks = split_into_chunks(&content, 50);
        assert!(chunks.len() > 1);
        // 모든 청크가 유효한 UTF-8
        for (_, chunk) in &chunks {
            assert!(chunk.is_ascii() || !chunk.is_empty());
        }
    }

    #[test]
    fn test_chunk_info() {
        let content = "a".repeat(100_000);
        let info = chunk_info(&content, 40_000);
        assert_eq!(info.total_chunks, 3); // 40K + 40K + 20K
        assert_eq!(info.total_bytes, 100_000);
    }

    // ── 의미 단위 청킹 테스트 ──

    #[test]
    fn test_semantic_split_by_heading() {
        let content = "\
## 서론
첫 번째 섹션 내용입니다.

## 본론
두 번째 섹션 내용입니다.

## 결론
세 번째 섹션입니다.
";
        let config = SemanticChunkConfig { max_bytes: 5000, ..Default::default() };
        let chunks = split_semantic(content, &config);
        assert_eq!(chunks.len(), 3, "헤딩 3개 → 청크 3개");
        assert!(chunks[0].text.contains("서론"));
        assert!(chunks[1].text.contains("본론"));
        assert!(chunks[2].text.contains("결론"));
    }

    #[test]
    fn test_semantic_preserve_code_block() {
        let content = "\
## 설정

설정 파일 예시:

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: test
```

설정 설명 계속.
";
        let config = SemanticChunkConfig { max_bytes: 50, preserve_code_blocks: true, ..Default::default() };
        let chunks = split_semantic(content, &config);
        // 코드 블록이 한 청크 안에 온전히 포함되어야 함
        let code_chunk = chunks.iter().find(|c| c.text.contains("```yaml"));
        assert!(code_chunk.is_some(), "코드 블록을 포함하는 청크가 있어야 함");
        let code_text = &code_chunk.unwrap().text;
        let fence_count = code_text.matches("```").count();
        assert!(fence_count >= 2, "코드 펜스 열기/닫기가 같은 청크에 있어야 함 (found {})", fence_count);
    }

    #[test]
    fn test_semantic_preserve_table_off_by_default() {
        // preserve_tables=false(디폴트)일 때 표는 일반 단락처럼 처리
        let content = "\
## 표 섹션

| 컬럼 A | 컬럼 B |
|--------|--------|
| 값 1   | 값 2   |

이후 문단.
";
        let config = SemanticChunkConfig { max_bytes: 5000, ..Default::default() };
        let chunks = split_semantic(content, &config);
        // 디폴트 비활성이라 표 자체가 분할되지는 않지만(짧음) 동작이 비활성임을 확인
        assert!(!config.preserve_tables, "디폴트는 비활성");
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_semantic_preserve_table_when_enabled() {
        // preserve_tables=true일 때 표 내부에서 `---` 수평선이 분할 트리거되지 않아야 함
        // (표 구분선 `|---|`은 trim 시 `|---|` 그대로 — 수평선 `---`와 다르지만 안전 보장)
        let content = "\
## 헤더

| 컬럼 A | 컬럼 B |
|--------|--------|
| 값 1   | 값 2   |
| 값 3   | 값 4   |

다음 단락.
";
        let config = SemanticChunkConfig {
            max_bytes: 5000,
            preserve_tables: true,
            ..Default::default()
        };
        let chunks = split_semantic(content, &config);
        let table_chunk = chunks.iter().find(|c| c.text.contains("| 컬럼 A |"));
        assert!(table_chunk.is_some(), "표를 포함한 청크가 있어야 함");
        let txt = &table_chunk.unwrap().text;
        assert!(txt.contains("| 값 1   |") && txt.contains("| 값 3   |"),
            "표 모든 행이 같은 청크에 있어야 함");
    }

    #[test]
    fn test_semantic_overlap() {
        let content = "\
## A
문장 A1.
문장 A2.
문장 A3.

## B
문장 B1.
문장 B2.
";
        let config = SemanticChunkConfig {
            max_bytes: 5000,
            overlap_sentences: 1,
            ..Default::default()
        };
        let chunks = split_semantic(content, &config);
        assert!(chunks.len() >= 2);
        // 두 번째 청크에 첫 번째 청크의 마지막 문장이 오버랩되어야 함
        assert!(!chunks[1].overlap_prefix.is_empty(), "오버랩 접두사가 있어야 함");
    }

    #[test]
    fn test_semantic_small_content_single_chunk() {
        let content = "짧은 문서입니다.";
        let config = SemanticChunkConfig::default();
        let chunks = split_semantic(content, &config);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn test_semantic_hr_split() {
        let content = "\
파트 1 내용

---

파트 2 내용
";
        let config = SemanticChunkConfig { max_bytes: 5000, ..Default::default() };
        let chunks = split_semantic(content, &config);
        assert_eq!(chunks.len(), 2, "수평선에서 분할되어야 함");
    }

    #[test]
    fn test_semantic_large_section_paragraph_split() {
        // 하나의 섹션이 max_bytes를 초과하면 단락 단위로 재분할
        let para = "이것은 테스트 단락입니다. ".repeat(20);
        let content = format!("## 큰 섹션\n\n{}\n\n{}\n\n{}", para, para, para);
        let config = SemanticChunkConfig {
            target_bytes: 200,
            max_bytes: 400,
            overlap_sentences: 0,
            preserve_code_blocks: true,
            preserve_tables: false,
        };
        let chunks = split_semantic(&content, &config);
        assert!(chunks.len() > 1, "큰 섹션은 재분할되어야 함");
        for chunk in &chunks {
            // 강제 분할 후에도 max_bytes * 2 이하여야 함 (줄 단위 분할 허용 오차)
            assert!(chunk.text.len() <= config.max_bytes * 2,
                "청크가 너무 큼: {} bytes", chunk.text.len());
        }
    }

    #[test]
    fn test_semantic_numbered_list_not_split() {
        // 번호 목록이 target 이하면 하나의 청크로 유지
        let content = "\
## 절차

1. 첫 번째 단계
2. 두 번째 단계
3. 세 번째 단계
4. 네 번째 단계
5. 다섯 번째 단계
";
        let config = SemanticChunkConfig { max_bytes: 5000, ..Default::default() };
        let chunks = split_semantic(content, &config);
        assert_eq!(chunks.len(), 1, "짧은 번호 목록은 분할하면 안 됨");
        assert!(chunks[0].text.contains("1.") && chunks[0].text.contains("5."));
    }

    // ── Phase B: ChunkingStrategy 추상화 테스트 ──

    #[test]
    fn strategy_from_str_known() {
        assert_eq!(ChunkingStrategy::from_str_or_default("fixed"), ChunkingStrategy::Fixed);
        assert_eq!(ChunkingStrategy::from_str_or_default("Semantic"), ChunkingStrategy::Semantic);
        assert_eq!(ChunkingStrategy::from_str_or_default("RECURSIVE"), ChunkingStrategy::Recursive);
        assert_eq!(ChunkingStrategy::from_str_or_default("adaptive"), ChunkingStrategy::Adaptive);
    }

    #[test]
    fn strategy_from_str_unknown_falls_back_to_semantic() {
        assert_eq!(ChunkingStrategy::from_str_or_default(""), ChunkingStrategy::Semantic);
        assert_eq!(ChunkingStrategy::from_str_or_default("foo"), ChunkingStrategy::Semantic);
    }

    #[test]
    fn strategy_as_str_roundtrip() {
        for s in [ChunkingStrategy::Fixed, ChunkingStrategy::Semantic, ChunkingStrategy::Recursive, ChunkingStrategy::Adaptive] {
            assert_eq!(ChunkingStrategy::from_str_or_default(s.as_str()), s);
        }
    }

    #[test]
    fn strategy_default_is_semantic() {
        assert_eq!(ChunkingStrategy::default(), ChunkingStrategy::Semantic);
    }

    #[test]
    fn chunk_by_strategy_fixed() {
        let content = "a".repeat(1000);
        let config = SemanticChunkConfig { target_bytes: 300, ..Default::default() };
        let chunks = chunk_by_strategy(&content, ChunkingStrategy::Fixed, &config);
        assert!(chunks.len() >= 3, "1000 bytes / 300 -> 3+ chunks, got {}", chunks.len());
        for c in &chunks {
            assert!(c.title_path.is_empty(), "fixed는 title_path 없음");
        }
    }

    #[test]
    fn chunk_by_strategy_semantic_equiv_split_semantic() {
        let content = "## A\n본문 A\n\n## B\n본문 B\n";
        let config = SemanticChunkConfig::default();
        let via_strategy = chunk_by_strategy(content, ChunkingStrategy::Semantic, &config);
        let direct = split_semantic(content, &config);
        assert_eq!(via_strategy.len(), direct.len());
    }

    #[test]
    fn chunk_by_strategy_adaptive_falls_back_to_semantic_in_phase_b() {
        // Phase C 진입 전까지 Adaptive는 Semantic으로 위임 (lesson 30 인프라 디폴트)
        let content = "## A\n본문 A\n\n## B\n본문 B\n";
        let config = SemanticChunkConfig::default();
        let adaptive = chunk_by_strategy(content, ChunkingStrategy::Adaptive, &config);
        let semantic = chunk_by_strategy(content, ChunkingStrategy::Semantic, &config);
        assert_eq!(adaptive.len(), semantic.len());
    }
}
