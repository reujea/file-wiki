//! Python subprocess 기반 ONNX 임베딩 어댑터 (legacy)
//!
//! **상태 (2026-04-29)**: Phase 62에서 `FastEmbedAdapter`(순수 Rust)가 도입되어
//! 본 어댑터는 fallback 또는 fastembed feature 비활성 환경 전용으로 격하됨.
//!
//! 신규 사용은 `fastembed_adapter`를 권장. 본 어댑터는 다음 시나리오에서만 유효:
//!   - fastembed feature 없이 빌드한 환경에서 BGE-M3를 쓰고 싶을 때
//!   - `default_model = "onnx"` + Python 환경 사전 설치된 사용자
//!
//! 사용 조건:
//!   - Python 3.10+ + onnxruntime + transformers 설치
//!   - models/bge-m3/ 디렉토리에 model.onnx + tokenizer.json

use anyhow::{Context, Result};
use async_trait::async_trait;
use file_pipeline_core::ports::output::EmbeddingPort;
use std::path::PathBuf;
use std::process::Command;

pub struct PythonOnnxEmbeddingAdapter {
    model_dir: PathBuf,
    dim: usize,
    python_bin: String,
}

impl PythonOnnxEmbeddingAdapter {
    pub fn new(model_dir: PathBuf, dim: usize) -> Result<Self> {
        if !model_dir.join("model.onnx").exists() {
            anyhow::bail!("model.onnx 없음: {}", model_dir.display());
        }
        if !model_dir.join("tokenizer.json").exists() {
            anyhow::bail!("tokenizer.json 없음: {}", model_dir.display());
        }

        // Python 실행 파일 탐색
        let python_bin = Self::find_python()?;
        tracing::info!("PythonOnnx 임베딩: model={}, python={}", model_dir.display(), python_bin);

        Ok(Self { model_dir, dim, python_bin })
    }

    fn find_python() -> Result<String> {
        for bin in &["python", "python3"] {
            let mut cmd = Command::new(bin);
            cmd.arg("--version");
            #[cfg(windows)]
            { use std::os::windows::process::CommandExt; cmd.creation_flags(0x08000000); }
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let mut check_cmd = Command::new(bin);
                    check_cmd.args(["-c", "import onnxruntime; import transformers"]);
                    #[cfg(windows)]
                    { use std::os::windows::process::CommandExt; check_cmd.creation_flags(0x08000000); }
                    if let Ok(o) = check_cmd.output() {
                        if o.status.success() {
                            return Ok(bin.to_string());
                        }
                    }
                }
            }
        }
        anyhow::bail!("Python + onnxruntime + transformers가 필요합니다.\npip install onnxruntime transformers")
    }

    fn embed_sync(&self, text: &str) -> Result<Vec<f32>> {
        let script = format!(r#"
import sys, json, numpy as np
import onnxruntime as ort
from transformers import AutoTokenizer

sess = ort.InferenceSession(r'{model_dir}/model.onnx')
tok = AutoTokenizer.from_pretrained(r'{model_dir}')
inputs = tok(sys.argv[1], return_tensors='np', padding=True, truncation=True, max_length=512)
ids = inputs['input_ids'].astype(np.int64)
mask = inputs['attention_mask'].astype(np.int64)
ttype = np.zeros_like(ids)
hidden = sess.run(None, {{'input_ids':ids,'attention_mask':mask,'token_type_ids':ttype}})[0]
m = mask[:,:,np.newaxis].astype(np.float32)
pooled = (hidden * m).sum(1) / m.sum(1)
norm = np.linalg.norm(pooled, axis=1, keepdims=True)
vec = (pooled / norm)[0].tolist()
print(json.dumps(vec))
"#, model_dir = self.model_dir.display());

        let mut cmd = Command::new(&self.python_bin);
        cmd.args(["-c", &script, text]);
        #[cfg(windows)]
        { use std::os::windows::process::CommandExt; cmd.creation_flags(0x08000000); }

        let output = cmd.output().context("Python ONNX 임베딩 실행 실패")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Python ONNX 에러: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let vec: Vec<f32> = serde_json::from_str(stdout.trim())
            .context("Python 임베딩 결과 파싱 실패")?;

        Ok(vec)
    }
}

#[async_trait]
impl EmbeddingPort for PythonOnnxEmbeddingAdapter {
    fn dim(&self) -> usize { self.dim }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let text = text.to_string();
        let model_dir = self.model_dir.clone();
        let dim = self.dim;
        let python = self.python_bin.clone();

        tokio::task::spawn_blocking(move || {
            let adapter = PythonOnnxEmbeddingAdapter {
                model_dir, dim, python_bin: python,
            };
            adapter.embed_sync(&text)
        })
        .await
        .context("spawn_blocking 실패")?
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }
}
