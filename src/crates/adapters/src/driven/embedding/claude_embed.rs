//! Claude CLI 기반 의미 임베딩 어댑터
//!
//! Claude에게 고정된 의미 축(카테고리)에 대한 관련도 점수를 요청하여
//! 의미 기반 벡터를 생성. 동의어/다국어/문맥 이해 가능.

use anyhow::{Context, Result};
use async_trait::async_trait;
use file_pipeline_core::ports::output::EmbeddingPort;
use std::process::Command;

pub struct ClaudeEmbeddingAdapter {
    claude_bin: String,
    dim: usize,
    config_dir: Option<String>,
}

/// 의미 축 정의 (128개)
/// 각 축은 문서의 특정 의미 차원을 나타냄.
/// Claude가 각 축에 대해 0.0~1.0 관련도를 매김.
const SEMANTIC_AXES: &[&str] = &[
    // 프로그래밍 언어/기술
    "Java", "Python", "JavaScript", "TypeScript", "Rust", "Go", "C/C++", "SQL",
    "Spring Framework", "React/Vue/Angular", "Node.js", "Docker/Kubernetes",
    // 소프트웨어 공학
    "객체지향 설계", "함수형 프로그래밍", "디자인 패턴", "리팩토링", "테스트/QA",
    "CI/CD 파이프라인", "DevOps", "마이크로서비스", "모놀리식", "API 설계",
    // 데이터/DB
    "관계형 DB", "NoSQL", "캐시(Redis)", "데이터베이스 설계", "쿼리 최적화",
    "데이터 모델링", "마이그레이션", "트랜잭션/ACID", "인덱싱", "데이터 파이프라인",
    // 인프라/운영
    "서버 운영", "모니터링/로깅", "성능 최적화", "메모리 관리", "네트워크",
    "보안/인증", "암호화", "클라우드(AWS/GCP/Azure)", "로드밸런싱", "장애 대응",
    // 아키텍처
    "시스템 아키텍처", "분산 시스템", "이벤트 기반", "메시지 큐", "헥사고날",
    "클린 아키텍처", "DDD", "CQRS", "동시성/병렬처리", "확장성",
    // 프론트엔드
    "UI/UX", "CSS/스타일링", "상태 관리", "SPA", "SSR/SSG",
    "웹 성능", "접근성", "반응형 디자인", "애니메이션", "브라우저 API",
    // AI/ML
    "머신러닝", "딥러닝", "자연어 처리(NLP)", "LLM/GPT", "임베딩/벡터",
    "RAG", "프롬프트 엔지니어링", "파인튜닝", "컴퓨터 비전", "추천 시스템",
    // 비즈니스/조직
    "프로젝트 관리", "애자일/스크럼", "회의록", "의사결정", "기획/요구사항",
    "코드 리뷰", "기술 부채", "레거시 마이그레이션", "온보딩", "문서화",
    // 문서 유형
    "튜토리얼/가이드", "트러블슈팅", "에러 분석", "성능 벤치마크", "비교 분석",
    "포스트모템", "RFC/설계문서", "릴리즈 노트", "개인 메모", "할일/체크리스트",
    // 도메인
    "금융/핀테크", "이커머스", "게임", "헬스케어", "교육",
    "미디어/콘텐츠", "IoT/임베디드", "모바일 앱", "블록체인", "SaaS",
    // 감성/수준
    "입문자용", "중급", "고급/심화", "이론적", "실용적/실전",
    "공식 문서", "블로그 포스트", "학술 논문", "코드 중심", "개념 설명 중심",
    // 추가 축
    "에러 핸들링", "로깅/디버깅", "설정/환경", "빌드/배포",
    "버전 관리(Git)", "패키지 관리", "직렬화/역직렬화", "파일 I/O",
    "날짜/시간 처리", "정규식", "국제화(i18n)", "웹소켓/실시간",
];

const N_AXES: usize = 128; // SEMANTIC_AXES.len()

impl ClaudeEmbeddingAdapter {
    pub fn new(dim: usize) -> Self {
        Self {
            claude_bin: std::env::var("CLAUDE_BIN").unwrap_or_else(|_| "claude".into()),
            dim,
            config_dir: std::env::var("CLAUDE_CONFIG_DIR").ok(),
        }
    }

    /// Claude CLI로 의미 벡터 생성
    fn embed_via_claude(&self, text: &str) -> Result<Vec<f32>> {
        let truncated = if text.len() > 4000 {
            &text[..text.char_indices().nth(4000).map(|(i,_)|i).unwrap_or(text.len())]
        } else {
            text
        };

        // Claude에게 의미 축별 관련도 점수를 요청
        let axes_str = SEMANTIC_AXES.iter().enumerate()
            .map(|(i, ax)| format!("{}: {}", i, ax))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            r#"아래 텍스트를 분석하여 각 카테고리와의 관련도를 0~9 사이 정수로 평가하라.
0=전혀 무관, 5=보통 관련, 9=핵심 주제.

카테고리 목록:
{axes}

텍스트:
{text}

출력 형식: 숫자만 공백으로 구분하여 {n}개를 한 줄에 출력하라.
예시: 0 0 3 7 9 0 2 ...
반드시 정확히 {n}개의 숫자만 출력하라. 다른 텍스트는 출력하지 마라."#,
            axes = axes_str,
            text = truncated,
            n = N_AXES,
        );

        let mut cmd = Command::new(&self.claude_bin);
        cmd.arg("-p").arg(&prompt).args(["--output-format", "text"]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        if let Some(ref dir) = self.config_dir {
            cmd.env("CLAUDE_CONFIG_DIR", dir);
        }
        let output = cmd.output().context("Claude CLI 임베딩 호출 실패")?;

        if output.status.success() {
            let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
            match Self::parse_scores(&raw, self.dim) {
                Ok(vec) => return Ok(vec),
                Err(e) => {
                    tracing::warn!("Claude 의미 벡터 파싱 실패: {} → 키워드 fallback", e);
                }
            }
        } else {
            tracing::warn!("Claude CLI 임베딩 호출 실패 → 키워드 fallback");
        }

        // fallback: 키워드 해시 (기존 방식)
        Ok(Self::keyword_fallback(text, self.dim))
    }

    /// Claude 응답에서 숫자 벡터 파싱
    fn parse_scores(raw: &str, dim: usize) -> Result<Vec<f32>> {
        // 숫자만 추출
        let scores: Vec<f32> = raw.split(|c: char| c.is_whitespace() || c == ',')
            .filter_map(|s| s.trim().parse::<f32>().ok())
            .collect();

        if scores.len() < N_AXES / 2 {
            anyhow::bail!("파싱된 숫자 부족: {} (최소 {})", scores.len(), N_AXES / 2);
        }

        // 0~9 → 0.0~1.0 정규화
        let mut vec = vec![0.0f32; dim];
        for (i, &score) in scores.iter().enumerate().take(N_AXES.min(dim)) {
            vec[i] = (score / 9.0).clamp(0.0, 1.0);
        }

        // 나머지 차원은 0으로 유지 (패딩)
        // L2 정규화
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            vec.iter_mut().for_each(|x| *x /= norm);
        }

        Ok(vec)
    }

    /// fallback: 키워드 해시 기반 (기존 방식)
    fn keyword_fallback(text: &str, dim: usize) -> Vec<f32> {
        let mut vec = vec![0.0f32; dim];
        let words: Vec<&str> = text.split(|c: char| c == ',' || c.is_whitespace())
            .map(|w| w.trim())
            .filter(|w| w.len() >= 2)
            .collect();

        for word in &words {
            let hash = word.bytes().fold(0u64, |acc, b| {
                acc.wrapping_mul(31).wrapping_add(b as u64)
            });
            vec[(hash as usize) % dim] += 1.0;
            let hash2 = hash.wrapping_mul(0x517cc1b727220a95);
            vec[(hash2 as usize) % dim] += 0.5;
        }

        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            vec.iter_mut().for_each(|x| *x /= norm);
        }
        vec
    }
}

#[async_trait]
impl EmbeddingPort for ClaudeEmbeddingAdapter {
    fn dim(&self) -> usize {
        self.dim
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let text = text.to_string();
        let claude_bin = self.claude_bin.clone();
        let config_dir = self.config_dir.clone();
        let dim = self.dim;

        tokio::task::spawn_blocking(move || {
            let adapter = ClaudeEmbeddingAdapter { claude_bin, dim, config_dir };
            adapter.embed_via_claude(&text)
        })
        .await
        .context("spawn_blocking 실패")?
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        use tokio::sync::Semaphore;
        use std::sync::Arc;

        if texts.len() <= 1 {
            let mut results = Vec::with_capacity(texts.len());
            for text in texts {
                results.push(self.embed(text).await?);
            }
            return Ok(results);
        }

        // 동시 최대 4개 프로세스
        let semaphore = Arc::new(Semaphore::new(4));
        let mut handles = Vec::with_capacity(texts.len());

        for text in texts {
            let sem = Arc::clone(&semaphore);
            let claude_bin = self.claude_bin.clone();
            let dim = self.dim;
            let config_dir = self.config_dir.clone();
            let text = text.clone();

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore poisoned");
                tokio::task::spawn_blocking(move || {
                    let adapter = ClaudeEmbeddingAdapter { claude_bin, dim, config_dir };
                    adapter.embed_via_claude(&text)
                })
                .await
                .context("spawn_blocking 실패")?
            }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            results.push(handle.await.context("task join 실패")??);
        }
        Ok(results)
    }
}

// step-o2 (2026-06-16, outbound-umbrella-1): OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for ClaudeEmbeddingAdapter {
    fn id(&self) -> &str { "fp-outbound-embedding-claude" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Embedding
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("claude")
    }
}
