//! Notion 원격 저장소 어댑터 (Phase 90 신규)
//!
//! Notion API v1 (Notion-Version: 2022-06-28) 사용. 일반 파일 시스템 저장소와 달리
//! Notion은 **페이지/블록 기반 콘텐츠 플랫폼**이므로 두 가지 모드를 지원:
//!
//! - `mode="page"` (디폴트): 가공본 텍스트(decompress 후)를 자식 페이지로 생성.
//!   `remote_key`는 페이지 제목. 마크다운 본문은 paragraph 블록 배열로 변환.
//! - `mode="attach"`: 외부 호스팅 URL을 페이지에 외부 파일로 참조 등록.
//!   Notion 공식 API는 직접 파일 업로드를 지원하지 않으므로(file_upload v2024 별도)
//!   본 어댑터는 **명시적 미지원 + anyhow::bail!**. attach 모드 활성화 시
//!   S3/WebDAV 등 다른 백엔드 권장.
//!
//! ## 인증
//!
//! Notion Internal Integration을 생성하여 token 획득 + parent_page_id에 공유.
//! `notion.so/my-integrations` → New integration → Internal → token 복사.
//! 대상 페이지 우측 상단 ⋯ → Connect to integration.
//!
//! ## 제약
//!
//! - rate limit: 평균 3 req/s (Notion API 가이드)
//! - 블록 children 한 번에 최대 100개 (긴 문서는 분할 필요)
//! - 페이지 제목 길이 무제한이지만 표시상 100자 제한 권장

use std::path::Path;

use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use file_pipeline_core::ports::output::{RemoteStoragePort, ResourceCapabilities};
use reqwest::Client;
use serde_json::{json, Value};

const NOTION_API: &str = "https://api.notion.com/v1";
const NOTION_VERSION: &str = "2022-06-28";

/// Notion API 어댑터 — page/attach 모드 지원
pub struct NotionStorageAdapter {
    token: String,
    parent_page_id: String,
    mode: NotionMode,
    database_id: Option<String>,
    client: Client,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NotionMode {
    Page,
    Attach,
}

impl NotionStorageAdapter {
    pub fn new(token: String, parent_page_id: String, mode: &str, database_id: Option<String>) -> Self {
        let mode = match mode {
            "attach" => NotionMode::Attach,
            _ => NotionMode::Page, // 디폴트 page
        };
        Self {
            token,
            parent_page_id,
            mode,
            database_id,
            client: Client::new(),
        }
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut h = reqwest::header::HeaderMap::new();
        h.insert("Authorization", format!("Bearer {}", self.token).parse().expect("Bearer token"));
        h.insert("Notion-Version", NOTION_VERSION.parse().expect("Notion-Version"));
        h.insert("Content-Type", "application/json".parse().expect("Content-Type"));
        h
    }

    /// remote_key(파일명)에서 Notion 페이지 제목 생성
    fn key_to_title(remote_key: &str) -> String {
        // ".zst" 확장자 제거, 경로 구분자를 " / "로
        let stripped = remote_key.trim_end_matches(".zst");
        stripped.replace(['\\', '/'], " / ")
    }

    /// 가공본 텍스트를 Notion 블록 배열로 변환 (paragraph만 사용, 단순화)
    /// Notion은 children 한 요청에 최대 100개. 본 함수는 paragraph 블록의 Vec를 반환.
    fn text_to_blocks(text: &str) -> Vec<Value> {
        // 빈 줄 단위로 분할, 각 단락을 paragraph 블록으로
        let mut blocks = Vec::new();
        let mut current = String::new();
        for line in text.lines() {
            if line.trim().is_empty() {
                if !current.is_empty() {
                    blocks.push(Self::paragraph_block(&current));
                    current.clear();
                }
            } else {
                if !current.is_empty() { current.push('\n'); }
                current.push_str(line);
            }
        }
        if !current.is_empty() {
            blocks.push(Self::paragraph_block(&current));
        }
        // Notion rich_text 단일 항목당 2000자 제한 — 초과 시 분할
        blocks.into_iter().flat_map(Self::split_long_block).collect()
    }

    fn paragraph_block(content: &str) -> Value {
        json!({
            "object": "block",
            "type": "paragraph",
            "paragraph": {
                "rich_text": [{
                    "type": "text",
                    "text": { "content": content }
                }]
            }
        })
    }

    /// rich_text content 2000자 제한 회피 — 긴 단락은 여러 블록으로 분할
    fn split_long_block(block: Value) -> Vec<Value> {
        let content = block.get("paragraph")
            .and_then(|p| p.get("rich_text"))
            .and_then(|r| r.get(0))
            .and_then(|t| t.get("text"))
            .and_then(|t| t.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("");

        if content.chars().count() <= 2000 {
            return vec![block];
        }

        let mut blocks = Vec::new();
        let chars: Vec<char> = content.chars().collect();
        for chunk in chars.chunks(2000) {
            let s: String = chunk.iter().collect();
            blocks.push(Self::paragraph_block(&s));
        }
        blocks
    }

    /// page 모드: 자식 페이지 생성 + 블록 children 추가
    async fn create_page(&self, title: &str, blocks: Vec<Value>) -> Result<String> {
        // properties: database_id가 있으면 Name 컬럼 매핑, 없으면 title
        let (parent, properties) = if let Some(db) = &self.database_id {
            (
                json!({ "database_id": db }),
                json!({
                    "Name": {
                        "title": [{ "text": { "content": title } }]
                    }
                }),
            )
        } else {
            (
                json!({ "page_id": self.parent_page_id }),
                json!({
                    "title": {
                        "title": [{ "text": { "content": title } }]
                    }
                }),
            )
        };

        // children은 최초 생성 시 최대 100개. 초과분은 별도 PATCH 필요.
        let initial_children: Vec<Value> = blocks.iter().take(100).cloned().collect();
        let body = json!({
            "parent": parent,
            "properties": properties,
            "children": initial_children,
        });

        let resp = self.client.post(format!("{}/pages", NOTION_API))
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Notion 페이지 생성 요청 실패: {}", e))?;

        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            bail!("Notion API 에러 {}: {}", status, body_text);
        }

        let resp_json: Value = serde_json::from_str(&body_text)
            .map_err(|e| anyhow!("Notion 응답 파싱 실패: {}", e))?;
        let page_id = resp_json.get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Notion 응답에 id 없음"))?
            .to_string();

        // 100개 초과 블록은 별도 PATCH로 append
        if blocks.len() > 100 {
            for chunk in blocks[100..].chunks(100) {
                let append_body = json!({ "children": chunk });
                let resp = self.client.patch(format!("{}/blocks/{}/children", NOTION_API, page_id))
                    .headers(self.headers())
                    .json(&append_body)
                    .send()
                    .await
                    .map_err(|e| anyhow!("블록 append 실패: {}", e))?;
                if !resp.status().is_success() {
                    let s = resp.status();
                    let b = resp.text().await.unwrap_or_default();
                    bail!("블록 append 에러 {}: {}", s, b);
                }
            }
        }

        Ok(page_id)
    }

    /// 부모 페이지의 자식 블록 중 child_page를 검색하여 page_id 매핑
    async fn find_page_by_title(&self, title: &str) -> Result<Option<String>> {
        let url = format!("{}/blocks/{}/children?page_size=100", NOTION_API, self.parent_page_id);
        let resp = self.client.get(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| anyhow!("자식 블록 조회 실패: {}", e))?;

        if !resp.status().is_success() {
            let s = resp.status();
            let b = resp.text().await.unwrap_or_default();
            bail!("자식 블록 조회 에러 {}: {}", s, b);
        }

        let json: Value = resp.json().await
            .map_err(|e| anyhow!("응답 JSON 파싱 실패: {}", e))?;
        let results = json.get("results").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        for block in results {
            if block.get("type").and_then(|t| t.as_str()) == Some("child_page") {
                let block_title = block.get("child_page")
                    .and_then(|p| p.get("title"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("");
                if block_title == title {
                    let id = block.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    return Ok(Some(id));
                }
            }
        }
        Ok(None)
    }
}

#[async_trait]
impl RemoteStoragePort for NotionStorageAdapter {
    /// page 모드: 압축 해제된 텍스트를 페이지로 변환 후 자식 페이지 생성.
    /// attach 모드: 미지원 (Notion API 제약). S3/WebDAV 권장.
    async fn upload(&self, local_path: &Path, remote_key: &str) -> Result<()> {
        match self.mode {
            NotionMode::Attach => {
                bail!(
                    "Notion attach 모드는 미지원 — Notion API는 zst 직접 업로드 불가. \
                     mode=\"page\"로 가공본 텍스트 업로드 또는 S3/WebDAV 백엔드 사용 권장. \
                     local={}, remote_key={}", local_path.display(), remote_key
                );
            }
            NotionMode::Page => {
                // 압축 해제는 호출자(또는 zstd_storage)가 수행해야 함.
                // 본 어댑터는 local_path가 평문 텍스트라고 가정 (가공본 .txt).
                // zst 직접 업로드면 텍스트로 변환되지 않으므로 명확한 에러.
                let content = std::fs::read_to_string(local_path)
                    .map_err(|e| anyhow!(
                        "Notion page 모드는 텍스트 파일을 요구 (zst 직접 입력 시 압축 해제 후 호출). local={}, err={}",
                        local_path.display(), e
                    ))?;
                let title = Self::key_to_title(remote_key);
                let blocks = Self::text_to_blocks(&content);
                let page_id = self.create_page(&title, blocks).await?;
                tracing::info!("Notion 페이지 생성: title={} page_id={}", title, page_id);
                Ok(())
            }
        }
    }

    /// 자식 페이지의 모든 paragraph 블록을 합쳐 local_path에 텍스트로 저장.
    async fn download(&self, remote_key: &str, local_path: &Path) -> Result<()> {
        if self.mode == NotionMode::Attach {
            bail!("Notion attach 모드는 download 미지원");
        }
        let title = Self::key_to_title(remote_key);
        let page_id = self.find_page_by_title(&title).await?
            .ok_or_else(|| anyhow!("Notion 페이지 없음: title={}", title))?;

        let url = format!("{}/blocks/{}/children?page_size=100", NOTION_API, page_id);
        let resp = self.client.get(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| anyhow!("페이지 children 조회 실패: {}", e))?;

        if !resp.status().is_success() {
            let s = resp.status();
            let b = resp.text().await.unwrap_or_default();
            bail!("페이지 children 조회 에러 {}: {}", s, b);
        }

        let json: Value = resp.json().await
            .map_err(|e| anyhow!("응답 JSON 파싱 실패: {}", e))?;
        let results = json.get("results").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let mut content = String::new();
        for block in results {
            if block.get("type").and_then(|t| t.as_str()) == Some("paragraph") {
                if let Some(rich_text) = block.get("paragraph").and_then(|p| p.get("rich_text")).and_then(|r| r.as_array()) {
                    for rt in rich_text {
                        if let Some(text) = rt.get("text").and_then(|t| t.get("content")).and_then(|c| c.as_str()) {
                            content.push_str(text);
                        }
                    }
                    content.push_str("\n\n");
                }
            }
        }
        std::fs::write(local_path, content)
            .map_err(|e| anyhow!("download 결과 저장 실패: {}", e))?;
        Ok(())
    }

    /// 부모 페이지의 자식 child_page 제목 목록 반환.
    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        if self.mode == NotionMode::Attach {
            return Ok(vec![]);
        }
        let url = format!("{}/blocks/{}/children?page_size=100", NOTION_API, self.parent_page_id);
        let resp = self.client.get(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| anyhow!("list 요청 실패: {}", e))?;

        if !resp.status().is_success() {
            let s = resp.status();
            let b = resp.text().await.unwrap_or_default();
            bail!("list 에러 {}: {}", s, b);
        }

        let json: Value = resp.json().await
            .map_err(|e| anyhow!("list 응답 파싱 실패: {}", e))?;
        let results = json.get("results").and_then(|v| v.as_array()).cloned().unwrap_or_default();
        let mut titles = Vec::new();
        for block in results {
            if block.get("type").and_then(|t| t.as_str()) == Some("child_page") {
                let t = block.get("child_page")
                    .and_then(|p| p.get("title"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("");
                if t.starts_with(prefix) {
                    titles.push(t.to_string());
                }
            }
        }
        Ok(titles)
    }

    /// 페이지를 archived=true로 마킹 (Notion은 hard delete 미지원).
    async fn delete(&self, remote_key: &str) -> Result<()> {
        if self.mode == NotionMode::Attach {
            bail!("Notion attach 모드는 delete 미지원");
        }
        let title = Self::key_to_title(remote_key);
        let page_id = self.find_page_by_title(&title).await?
            .ok_or_else(|| anyhow!("Notion 페이지 없음: title={}", title))?;

        let body = json!({ "archived": true });
        let resp = self.client.patch(format!("{}/pages/{}", NOTION_API, page_id))
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("페이지 archive 요청 실패: {}", e))?;

        if !resp.status().is_success() {
            let s = resp.status();
            let b = resp.text().await.unwrap_or_default();
            bail!("페이지 archive 에러 {}: {}", s, b);
        }
        Ok(())
    }

    fn is_configured(&self) -> bool {
        !self.token.is_empty() && !self.parent_page_id.is_empty()
    }

    /// Phase 92 H5: Notion capability — mode=page는 upload 가능, mode=attach는 upload 불가.
    /// delete는 archived=true PATCH (hard delete 미지원).
    fn capabilities(&self) -> ResourceCapabilities {
        let active = match self.mode {
            NotionMode::Page => "page",
            NotionMode::Attach => "attach",
        };
        ResourceCapabilities {
            backend: "notion",
            can_upload: matches!(self.mode, NotionMode::Page),
            can_download: matches!(self.mode, NotionMode::Page),
            can_list: true,
            can_delete: true,
            mode_options: &["page", "attach"],
            active_mode: active.to_string(),
            supports_hard_delete: false, // archived=true PATCH
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_to_title_strips_zst_and_normalizes_separators() {
        assert_eq!(NotionStorageAdapter::key_to_title("processed/foo.txt.zst"), "processed / foo.txt");
        assert_eq!(NotionStorageAdapter::key_to_title("a\\b\\c.md.zst"), "a / b / c.md");
        assert_eq!(NotionStorageAdapter::key_to_title("just-name.txt"), "just-name.txt");
    }

    #[test]
    fn text_to_blocks_splits_by_blank_lines() {
        let text = "첫째 단락.\n둘째 줄.\n\n둘째 단락.\n";
        let blocks = NotionStorageAdapter::text_to_blocks(text);
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn text_to_blocks_splits_long_content() {
        let long = "가".repeat(3000);
        let blocks = NotionStorageAdapter::text_to_blocks(&long);
        // 2000자 제한이라 최소 2개 블록
        assert!(blocks.len() >= 2);
    }

    #[test]
    fn mode_attach_defaults_to_attach_variant() {
        let a = NotionStorageAdapter::new("t".into(), "p".into(), "attach", None);
        assert_eq!(a.mode, NotionMode::Attach);
        let p = NotionStorageAdapter::new("t".into(), "p".into(), "page", None);
        assert_eq!(p.mode, NotionMode::Page);
        // 알 수 없는 값은 page로 폴백
        let unknown = NotionStorageAdapter::new("t".into(), "p".into(), "xxx", None);
        assert_eq!(unknown.mode, NotionMode::Page);
    }

    #[tokio::test]
    async fn attach_mode_upload_returns_error() {
        let a = NotionStorageAdapter::new("t".into(), "p".into(), "attach", None);
        let tmp = tempfile::NamedTempFile::new().expect("tmp");
        let result = a.upload(tmp.path(), "key").await;
        assert!(result.is_err(), "attach 모드는 명시적 미지원이어야 함");
    }

    #[test]
    fn is_configured_requires_both_token_and_parent() {
        let none = NotionStorageAdapter::new("".into(), "".into(), "page", None);
        assert!(!none.is_configured());
        let only_token = NotionStorageAdapter::new("t".into(), "".into(), "page", None);
        assert!(!only_token.is_configured());
        let both = NotionStorageAdapter::new("t".into(), "p".into(), "page", None);
        assert!(both.is_configured());
    }

    // Phase 92 H5: ResourceCapabilities 테스트
    #[test]
    fn capabilities_page_mode_allows_upload() {
        let a = NotionStorageAdapter::new("t".into(), "p".into(), "page", None);
        let caps = a.capabilities();
        assert_eq!(caps.backend, "notion");
        assert!(caps.can_upload);
        assert!(caps.can_download);
        assert_eq!(caps.active_mode, "page");
        assert!(!caps.supports_hard_delete);
        assert_eq!(caps.mode_options, &["page", "attach"]);
    }

    #[test]
    fn capabilities_attach_mode_blocks_upload() {
        let a = NotionStorageAdapter::new("t".into(), "p".into(), "attach", None);
        let caps = a.capabilities();
        assert!(!caps.can_upload, "attach 모드는 upload 불가");
        assert!(!caps.can_download, "attach 모드는 download 불가");
        assert_eq!(caps.active_mode, "attach");
    }
}

// step-o2 (2026-06-16, outbound-umbrella-1): OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for NotionStorageAdapter {
    fn id(&self) -> &str { "fp-outbound-storage-notion" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Storage
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("notion")
    }
}
