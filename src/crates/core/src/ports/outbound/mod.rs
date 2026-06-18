//! `OutboundManifest` 우산 — outbound-umbrella-1 step-o1 (2026-06-16) 정합.
//!
//! ⚠️ **폐기 예정 (DEPRECATED, 본질 재정의 3차 2026-06-17 / transport-flatten-1)**:
//! "Output adapter = raw I/O 만" 재정의로 본 도메인 메타데이터 우산은 `raw_transport` 4 채널로 대체.
//! 도메인 로직(capabilities/modes/config_keys)은 plugin manifest 이관. 본 trait 들의 완전 폐기 +
//! deprecated.md 위임 = step-t6. 현재(step-t1)는 폐기 마킹만 — 기존 impl 영향 0 (점진, lesson #25).
//!
//! 외부로 나가는 모든 어댑터 (storage / embedding / llm / notify / rerank / verify) 의 공통 메타데이터
//! trait. 기존 `RemoteStoragePort::capabilities()` 패턴 (Phase 92 H5) 을 6 카테고리 전체로 확장.
//!
//! ## 본문 (prd/research/plugin-architecture-2026-06-04.md §3-C 정합)
//!
//! - **outbound 우산** = storage 만이 아니라 외부 호출하는 모든 구현체 통일
//! - **6 카테고리** = Storage / Embedding / Llm / Notify / Rerank / Verify (총 25 어댑터)
//! - **공통 메타** = id (`fp-outbound-{category}-{name}`) + category + capabilities + modes + config_keys
//! - **mode 분기 예** = telegram storage = ["document", "text", "channel"], telegram notify = ["alert", "event"]
//!
//! ## 본 step-o1 영역 (skeleton)
//!
//! 본 mod = trait 정의만 박힘. 기존 6 port trait (output.rs 의 RemoteStoragePort 등) = 손대지 않음
//! (step-o2 시점 super-trait `OutboundManifest` 박힘). 신규 어댑터 manifest impl = step-o2/o3 영역.
//!
//! ## step-o1 부재 영역 (host 결정 / 후속 step)
//!
//! - 6 port super-trait 박힘 (RemoteStoragePort: OutboundManifest + ...) — step-o2
//! - 기존 어댑터 25종 manifest impl — step-o2
//! - telegram outbound (storage + notify 양쪽) 어댑터 신설 — step-o3
//! - adapters/driven/* 디렉토리 정합 — step-o4

use crate::ports::output::ResourceCapabilities;

/// outbound 카테고리 — 6종 고정 (Storage / Embedding / Llm / Notify / Rerank / Verify).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutboundCategory {
    /// 외부 저장소 (s3 / webdav / network / notion / telegram / zstd).
    Storage,
    /// 임베딩 (claude / openai / fastembed / local / python-onnx).
    Embedding,
    /// LLM (claude / anthropic / openai / gemini / ollama / fallback / chunked-agent).
    Llm,
    /// 알림 (telegram / slack).
    Notify,
    /// 리랭킹 (claude / fastembed / null).
    Rerank,
    /// 검증 (claude).
    Verify,
}

impl OutboundCategory {
    /// 카테고리 키 — outbound id prefix 정합 (`fp-outbound-{key}-{name}`).
    pub fn as_key(&self) -> &'static str {
        match self {
            OutboundCategory::Storage => "storage",
            OutboundCategory::Embedding => "embedding",
            OutboundCategory::Llm => "llm",
            OutboundCategory::Notify => "notify",
            OutboundCategory::Rerank => "rerank",
            OutboundCategory::Verify => "verify",
        }
    }
}

/// 어댑터 config key 1건 메타 — UI / settings.db 표면 노출 정합.
#[derive(Debug, Clone)]
pub struct ConfigKey {
    /// pipeline.toml 경로 (예: "remote_storage.notion_token", "notification.telegram.bot_token").
    pub path: &'static str,
    /// 표시 라벨 (i18n 대응 부재 — 영문 placeholder).
    pub label: &'static str,
    /// 필수 여부 (false 면 디폴트 값 박힘).
    pub required: bool,
    /// secret 영역 (UI 마스킹 + log 부재 의무).
    pub secret: bool,
}

/// outbound 어댑터 공통 메타데이터 — 6 port trait 의 super-trait.
///
/// 본 trait 구현체 = `id()` 가 outbound 식별자 (예: "fp-outbound-storage-telegram"),
/// `category()` 가 enum 영역, `capabilities()` 가 백엔드별 동작 표면, `modes()` 가 다중 모드 옵션,
/// `config_keys()` 가 사용자 설정 영역.
///
/// 본 trait 의 의도 = (1) plugin registry 자동 분류, (2) UI 자동 폼 생성, (3) 회귀 가드 자동화
/// (manifest 박힘 부재 시 컴파일 에러 = 누락 가드).
pub trait OutboundManifest {
    /// outbound 식별자 — `fp-outbound-{category}-{name}` 정합 (예: "fp-outbound-storage-telegram").
    fn id(&self) -> &str;

    /// 카테고리 — 6 enum 중 1건.
    fn category(&self) -> OutboundCategory;

    /// 백엔드별 동작 표면 (can_upload / can_download / can_list / can_delete / mode_options 등).
    fn capabilities(&self) -> ResourceCapabilities;

    /// 다중 모드 옵션 (예: telegram storage = ["document", "text", "channel"]).
    fn modes(&self) -> &[&str] {
        &[]
    }

    /// 사용자 설정 key 목록 — UI 자동 폼 + 회귀 가드 영역.
    fn config_keys(&self) -> &[ConfigKey] {
        &[]
    }
}

/// outbound storage 어댑터 — step-o2 시점 기존 `RemoteStoragePort` 가 본 trait alias 정합.
///
/// 본 step-o1 = skeleton (trait 본문 부재). step-o2 = `pub trait OutboundStoragePort: OutboundManifest +
/// RemoteStoragePort + Send + Sync {}` 형태 박힘 가능 영역 (host 결정).
pub trait OutboundStoragePort: OutboundManifest + Send + Sync {}

/// outbound embedding 어댑터 — step-o2 시점 기존 `EmbeddingPort` 정합.
pub trait OutboundEmbeddingPort: OutboundManifest + Send + Sync {}

/// outbound LLM 어댑터 — step-o2 시점 기존 `LLMPort` 정합.
pub trait OutboundLlmPort: OutboundManifest + Send + Sync {}

/// outbound notify 어댑터 — step-o2 시점 기존 `NotificationPort` 정합.
pub trait OutboundNotifyPort: OutboundManifest + Send + Sync {}

/// outbound rerank 어댑터 — step-o2 시점 기존 `RerankerPort` 정합.
pub trait OutboundRerankPort: OutboundManifest + Send + Sync {}

/// outbound verify 어댑터 — step-o2 시점 기존 `VerificationPort` 정합.
pub trait OutboundVerifyPort: OutboundManifest + Send + Sync {}
