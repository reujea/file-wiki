//! fastembed 기반 BGE-M3 임베더 — `EmbeddingPort` 구현.
//!
//! 헥사고날 경계: Driven adapter. `pykeio/ort` (ONNX Runtime Rust 래퍼) 위에서 동작.
//!
//! # 빌드 요구사항
//!
//! - Visual Studio Build Tools 2022 v17.8+ (MSVC v14.38+)
//! - Windows SDK 10.0.19041.0+
//! - `fastembed` feature 활성화 시에만 컴파일됨
//!
//! # 모델 캐시
//!
//! 첫 사용 시 HuggingFace에서 BGE-M3 모델 자동 다운로드.
//! 기본 캐시: `%LOCALAPPDATA%/fastembed/`. `with_cache_dir`로 오버라이드 가능.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use file_pipeline_core::ports::output::EmbeddingPort;
use tokio::sync::Mutex;

const BGE_M3_DIM: usize = 1024;

/// fastembed BGE-M3 임베더 어댑터.
///
/// `embed()`가 `&mut self`를 요구하므로 `Arc<Mutex<TextEmbedding>>`로 감싸 동시 접근 처리.
pub struct FastEmbedAdapter {
    model: Arc<Mutex<TextEmbedding>>,
}

impl FastEmbedAdapter {
    /// 기본 캐시 경로(`%LOCALAPPDATA%/fastembed/`)에서 BGE-M3 로드.
    pub fn new() -> Result<Self> {
        let model = TextEmbedding::try_new(InitOptions::new(EmbeddingModel::BGEM3))
            .context("fastembed BGE-M3 모델 로드 실패")?;
        tracing::info!("fastembed BGE-M3 로드 완료 (1024차원)");
        Ok(Self { model: Arc::new(Mutex::new(model)) })
    }

    /// 커스텀 캐시 경로 지정.
    pub fn with_cache_dir(cache_dir: PathBuf) -> Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGEM3).with_cache_dir(cache_dir),
        )
        .context("fastembed BGE-M3 모델 로드 실패 (커스텀 캐시)")?;
        tracing::info!("fastembed BGE-M3 로드 완료 (1024차원, 커스텀 캐시)");
        Ok(Self { model: Arc::new(Mutex::new(model)) })
    }
}

#[async_trait]
impl EmbeddingPort for FastEmbedAdapter {
    fn dim(&self) -> usize {
        BGE_M3_DIM
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let model = self.model.clone();
        let text = text.to_string();
        tokio::task::spawn_blocking(move || -> Result<Vec<f32>> {
            // blocking_lock — spawn_blocking 컨텍스트에서 동기 lock
            let mut guard = model.blocking_lock();
            let mut embeddings = guard.embed(vec![text], None).context("fastembed embed 실패")?;
            embeddings.pop().context("fastembed 빈 응답")
        })
        .await
        .context("spawn_blocking 합류 실패")?
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let model = self.model.clone();
        let texts = texts.to_vec();
        tokio::task::spawn_blocking(move || -> Result<Vec<Vec<f32>>> {
            let mut guard = model.blocking_lock();
            guard.embed(texts, None).context("fastembed embed_batch 실패")
        })
        .await
        .context("spawn_blocking 합류 실패")?
    }
}
