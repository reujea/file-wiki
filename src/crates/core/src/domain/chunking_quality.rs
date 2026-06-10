//! Adaptive Chunking 품질 지표 (arxiv 2603.25333 흡수, lesson 30 인프라 선구현)
//!
//! 4지표 계산: SC / BI / ICC / DCC
//! - SC: 100~1100 토큰 범위 청크 비율 (Size Compliance)
//! - BI: 블록(표/코드 펜스/헤딩 단락) 무결성 — τ=5자 허용 오차
//! - ICC: 청크 내 문장-청크 임베딩 평균 코사인 (Intrachunk Cohesion)
//! - DCC: 청크 ↔ 3000토큰 윈도우 임베딩 코사인 (Document Contextual Coherence)
//!
//! RC(Reference Completeness)는 한국어 coref 도구 도달 시 별도 추가 (현재 보류).
//!
//! 호출: `chunking.compute_quality` config가 true일 때만. 디폴트 false (lesson 30 패턴).

use serde::{Deserialize, Serialize};

use super::chunking::SemanticChunk;

/// Adaptive Chunking 4지표 + 메타. Metadata.chunk_quality에 부착.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ChunkQualityMetrics {
    /// Size Compliance — 100~1100 토큰 범위 청크 비율 (0.0~1.0)
    pub sc: f32,
    /// Block Integrity — 구조 블록 완전 유지 비율 (0.0~1.0)
    pub bi: f32,
    /// Intrachunk Cohesion — 청크 내 평균 코사인 (-1.0~1.0). None이면 임베딩 미적용
    pub icc: Option<f32>,
    /// Document Contextual Coherence — 윈도우 코사인 (-1.0~1.0). None이면 임베딩 미적용
    pub dcc: Option<f32>,
    /// 측정 시점 청크 수 (재현 추적용)
    pub n_chunks: usize,
}

/// SC 계산용 토큰 범위 (논문 §SC 기본값)
pub const SC_MIN_TOKENS: usize = 100;
pub const SC_MAX_TOKENS: usize = 1100;
/// BI τ — 블록 경계 허용 오차 (논문 §BI)
pub const BI_TAU_CHARS: usize = 5;
/// DCC 윈도우 크기 (논문 §DCC)
pub const DCC_WINDOW_TOKENS: usize = 3000;

/// 토큰 수 추정 — 영문 ~4자/토큰, 한글 ~2자/토큰 평균 ~3자/토큰. 임시 휴리스틱.
/// fastembed BGE-M3 tokenizer 도달 시 정확 토큰화로 대체 가능.
pub fn estimate_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    chars / 3
}

/// SC — 100~1100 토큰 범위 청크 비율
pub fn compute_sc(chunks: &[SemanticChunk]) -> f32 {
    if chunks.is_empty() {
        return 0.0;
    }
    let in_range = chunks
        .iter()
        .filter(|c| {
            let t = estimate_tokens(&c.text);
            t >= SC_MIN_TOKENS && t <= SC_MAX_TOKENS
        })
        .count();
    in_range as f32 / chunks.len() as f32
}

/// 구조 블록 — 표 / 코드 펜스 / 헤딩 단락 등 분할되면 안 되는 영역
#[derive(Debug, Clone, PartialEq)]
pub enum BlockKind {
    CodeFence,
    Table,
    Heading,
}

#[derive(Debug, Clone)]
pub struct DocBlock {
    pub kind: BlockKind,
    pub start: usize,
    pub end: usize,
}

/// 원본 문서에서 보존 대상 블록 추출.
/// 정밀도는 휴리스틱 — 정확한 markdown AST는 별도 통합 phase에서.
pub fn extract_blocks(content: &str) -> Vec<DocBlock> {
    let mut blocks = Vec::new();
    let bytes = content.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // 코드 펜스 ``` 시작
        if bytes[i..].starts_with(b"```") {
            let start = i;
            i += 3;
            while i + 3 <= bytes.len() && !bytes[i..].starts_with(b"```") {
                i += 1;
            }
            if i + 3 <= bytes.len() {
                i += 3;
                blocks.push(DocBlock { kind: BlockKind::CodeFence, start, end: i });
            }
            continue;
        }

        // 표 — 연속 라인이 `|`로 시작하고 `|`로 끝나는 단락
        if bytes[i] == b'|' && (i == 0 || bytes[i - 1] == b'\n') {
            let start = i;
            while i < bytes.len() {
                let line_end = content[i..].find('\n').map(|p| i + p).unwrap_or(bytes.len());
                let line = &content[i..line_end];
                if !line.trim_start().starts_with('|') || !line.trim_end().ends_with('|') {
                    break;
                }
                i = line_end + 1;
            }
            if i > start + 1 {
                blocks.push(DocBlock { kind: BlockKind::Table, start, end: i });
            }
            continue;
        }

        // 헤딩 라인 (#~######)
        if bytes[i] == b'#' && (i == 0 || bytes[i - 1] == b'\n') {
            let start = i;
            let line_end = content[i..].find('\n').map(|p| i + p).unwrap_or(bytes.len());
            blocks.push(DocBlock { kind: BlockKind::Heading, start, end: line_end });
            i = line_end + 1;
            continue;
        }

        i += 1;
    }

    blocks
}

/// BI — 보존 블록이 청크 안에서 무결한지. τ=BI_TAU_CHARS 허용 오차.
///
/// 각 블록이 어느 한 청크 안에 완전히 포함되면 무결.
/// 청크 경계가 블록 내부를 가로지르면 무결 깨짐 (단, τ자 이내는 허용).
pub fn compute_bi(content: &str, chunks: &[SemanticChunk], blocks: &[DocBlock]) -> f32 {
    if blocks.is_empty() {
        return 1.0;
    }
    let chunk_spans = chunk_spans_in_doc(content, chunks);
    let mut intact = 0usize;
    for block in blocks {
        let block_intact = chunk_spans.iter().any(|(cs, ce)| {
            // 블록 시작이 청크 안 + 블록 끝이 청크 안 (τ 여유)
            let start_in = block.start + BI_TAU_CHARS >= *cs && block.start <= *ce;
            let end_in = *cs <= block.end + BI_TAU_CHARS && block.end <= *ce + BI_TAU_CHARS;
            start_in && end_in
        });
        if block_intact {
            intact += 1;
        }
    }
    intact as f32 / blocks.len() as f32
}

/// 청크 텍스트가 원본 어느 byte span에 대응하는지 추정.
/// SemanticChunk가 직접 span을 보유하지 않으므로 텍스트 find로 추적.
fn chunk_spans_in_doc(content: &str, chunks: &[SemanticChunk]) -> Vec<(usize, usize)> {
    let mut cursor = 0usize;
    let mut spans = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        // overlap_prefix 제외하고 본문만 매칭
        let body = chunk.text.trim();
        if body.is_empty() {
            spans.push((cursor, cursor));
            continue;
        }
        // 청크 본문 첫 30자로 위치 추정
        let head: String = body.chars().take(30).collect();
        if let Some(found) = content[cursor..].find(&head) {
            let start = cursor + found;
            let end = (start + body.len()).min(content.len());
            spans.push((start, end));
            cursor = end;
        } else {
            spans.push((cursor, cursor + body.len()));
            cursor += body.len();
        }
    }
    spans
}

/// 코사인 유사도 — 임베딩 벡터.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// ICC — 청크 i에 대해, 청크 내 문장 임베딩과 청크 임베딩 간 평균 코사인.
/// 모든 청크 평균.
///
/// `sentence_embs[i]`: 청크 i의 문장별 임베딩 목록
/// `chunk_embs[i]`: 청크 i 전체 임베딩
pub fn compute_icc(sentence_embs: &[Vec<Vec<f32>>], chunk_embs: &[Vec<f32>]) -> Option<f32> {
    if sentence_embs.len() != chunk_embs.len() || chunk_embs.is_empty() {
        return None;
    }
    let mut sum = 0.0f32;
    let mut count = 0usize;
    for (sents, chunk) in sentence_embs.iter().zip(chunk_embs.iter()) {
        if sents.is_empty() {
            continue;
        }
        let avg: f32 = sents.iter().map(|s| cosine(s, chunk)).sum::<f32>() / sents.len() as f32;
        sum += avg;
        count += 1;
    }
    if count == 0 {
        None
    } else {
        Some(sum / count as f32)
    }
}

/// DCC — 청크 i 임베딩과 i를 중심으로 하는 3000토큰 윈도우 임베딩 간 코사인 평균.
///
/// `chunk_embs[i]`: 청크 i 임베딩
/// `window_embs[i]`: 청크 i 윈도우 임베딩
pub fn compute_dcc(chunk_embs: &[Vec<f32>], window_embs: &[Vec<f32>]) -> Option<f32> {
    if chunk_embs.len() != window_embs.len() || chunk_embs.is_empty() {
        return None;
    }
    let sum: f32 = chunk_embs
        .iter()
        .zip(window_embs.iter())
        .map(|(c, w)| cosine(c, w))
        .sum();
    Some(sum / chunk_embs.len() as f32)
}

/// 전체 지표 한 번에 계산 (임베딩이 없으면 ICC/DCC=None).
pub fn compute_all(
    content: &str,
    chunks: &[SemanticChunk],
    blocks: &[DocBlock],
    sentence_embs: Option<&[Vec<Vec<f32>>]>,
    chunk_embs: Option<&[Vec<f32>]>,
    window_embs: Option<&[Vec<f32>]>,
) -> ChunkQualityMetrics {
    ChunkQualityMetrics {
        sc: compute_sc(chunks),
        bi: compute_bi(content, chunks, blocks),
        icc: match (sentence_embs, chunk_embs) {
            (Some(s), Some(c)) => compute_icc(s, c),
            _ => None,
        },
        dcc: match (chunk_embs, window_embs) {
            (Some(c), Some(w)) => compute_dcc(c, w),
            _ => None,
        },
        n_chunks: chunks.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chunking::SemanticChunk;

    fn chunk(idx: usize, text: &str) -> SemanticChunk {
        SemanticChunk {
            index: idx,
            text: text.to_string(),
            overlap_prefix: String::new(),
            title_path: vec![],
        }
    }

    #[test]
    fn estimate_tokens_basic() {
        // 영문 60자 ≈ 20토큰
        assert!(estimate_tokens(&"a".repeat(60)) >= 18 && estimate_tokens(&"a".repeat(60)) <= 22);
    }

    #[test]
    fn sc_all_in_range() {
        // 300자 ≈ 100토큰, 3000자 ≈ 1000토큰 — 모두 100~1100 범위
        let chunks = vec![chunk(0, &"a".repeat(300)), chunk(1, &"b".repeat(3000))];
        assert!((compute_sc(&chunks) - 1.0).abs() < 0.01);
    }

    #[test]
    fn sc_out_of_range() {
        // 30자 ≈ 10토큰 (너무 짧음), 6000자 ≈ 2000토큰 (너무 김)
        let chunks = vec![
            chunk(0, &"a".repeat(30)),
            chunk(1, &"b".repeat(6000)),
            chunk(2, &"c".repeat(600)), // 200토큰 — 정상
        ];
        let sc = compute_sc(&chunks);
        assert!((sc - 0.333).abs() < 0.01, "got {}", sc);
    }

    #[test]
    fn sc_empty() {
        assert_eq!(compute_sc(&[]), 0.0);
    }

    #[test]
    fn extract_blocks_code_fence() {
        let content = "before\n```rust\nfn main() {}\n```\nafter";
        let blocks = extract_blocks(content);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].kind, BlockKind::CodeFence);
    }

    #[test]
    fn extract_blocks_table() {
        let content = "before\n| a | b |\n| c | d |\nafter\n";
        let blocks = extract_blocks(content);
        assert!(blocks.iter().any(|b| b.kind == BlockKind::Table));
    }

    #[test]
    fn extract_blocks_heading() {
        let content = "# Title\nbody\n## Sub\nmore\n";
        let blocks = extract_blocks(content);
        let headings: Vec<_> = blocks.iter().filter(|b| b.kind == BlockKind::Heading).collect();
        assert_eq!(headings.len(), 2);
    }

    #[test]
    fn bi_intact() {
        let content = "before\n```rust\nfn main() {}\n```\nafter";
        let chunks = vec![chunk(0, content)];
        let blocks = extract_blocks(content);
        let bi = compute_bi(content, &chunks, &blocks);
        assert!((bi - 1.0).abs() < 0.01, "BI={}", bi);
    }

    #[test]
    fn bi_split_block() {
        // 청크 경계가 코드 펜스 내부를 가로지름
        let content = "before\n```rust\nfn main() {}\n```\nafter";
        let chunks = vec![
            chunk(0, "before\n```rust\nfn"),
            chunk(1, "main() {}\n```\nafter"),
        ];
        let blocks = extract_blocks(content);
        let bi = compute_bi(content, &chunks, &blocks);
        // 블록 1개 깨짐 → 0.0
        assert!(bi < 0.5, "BI should be low, got {}", bi);
    }

    #[test]
    fn cosine_basic() {
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 0.001);
        assert!((cosine(&[1.0, 0.0], &[0.0, 1.0])).abs() < 0.001);
        assert!((cosine(&[1.0, 0.0], &[-1.0, 0.0]) - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn icc_basic() {
        // 청크 2개, 각 문장 임베딩이 청크 임베딩과 동일 → ICC=1.0
        let chunk_embs = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let sentence_embs = vec![
            vec![vec![1.0, 0.0], vec![1.0, 0.0]],
            vec![vec![0.0, 1.0]],
        ];
        let icc = compute_icc(&sentence_embs, &chunk_embs).unwrap();
        assert!((icc - 1.0).abs() < 0.001, "ICC={}", icc);
    }

    #[test]
    fn dcc_basic() {
        let chunk_embs = vec![vec![1.0, 0.0]];
        let window_embs = vec![vec![1.0, 0.0]];
        let dcc = compute_dcc(&chunk_embs, &window_embs).unwrap();
        assert!((dcc - 1.0).abs() < 0.001, "DCC={}", dcc);
    }

    #[test]
    fn compute_all_no_embeds() {
        let content = "# A\nbody\n";
        let chunks = vec![chunk(0, &"a".repeat(400))];
        let blocks = extract_blocks(content);
        let m = compute_all(content, &chunks, &blocks, None, None, None);
        assert!(m.icc.is_none());
        assert!(m.dcc.is_none());
        assert_eq!(m.n_chunks, 1);
    }
}
