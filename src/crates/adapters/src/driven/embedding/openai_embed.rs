use anyhow::{Context, Result};
use async_trait::async_trait;
use file_pipeline_core::ports::output::EmbeddingPort;
use serde::{Deserialize, Serialize};

pub struct OpenAIEmbeddingAdapter {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAIEmbeddingAdapter {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "text-embedding-3-small".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_model(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct EmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[async_trait]
impl EmbeddingPort for OpenAIEmbeddingAdapter {
    fn dim(&self) -> usize {
        1536
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let batch = self.embed_batch(&[text.to_string()]).await?;
        batch
            .into_iter()
            .next()
            .context("임베딩 결과 없음")
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let req = EmbeddingRequest {
            input: texts.to_vec(),
            model: self.model.clone(),
        };

        let resp = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&req)
            .send()
            .await
            .context("OpenAI API 호출 실패")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI API 오류 ({}): {}", status, body);
        }

        let resp: EmbeddingResponse = resp.json().await.context("응답 파싱 실패")?;
        Ok(resp.data.into_iter().map(|d| d.embedding).collect())
    }
}

// step-o2 (2026-06-16, outbound-umbrella-1): OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for OpenAIEmbeddingAdapter {
    fn id(&self) -> &str { "fp-outbound-embedding-openai" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Embedding
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("openai")
    }
}
