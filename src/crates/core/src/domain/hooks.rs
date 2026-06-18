//! Hook 실행 로직 — 순수 타입(HookEvent/HookDefinition)은 fp-domain-types로 추출됨 (cycle 7 step-d2).
//!
//! `HookRegistry::fire`는 tokio/reqwest 의존 도메인 로직이므로 core 잔류.
//! 기존 `file_pipeline_core::domain::hooks::{HookEvent, HookDefinition}` 경로는 re-export로 유지.

pub use fp_domain_types::hooks::{HookDefinition, HookEvent};

/// 훅 레지스트리
pub struct HookRegistry {
    hooks: Vec<HookDefinition>,
}

impl HookRegistry {
    pub fn new(hooks: Vec<HookDefinition>) -> Self {
        Self { hooks }
    }

    pub fn empty() -> Self {
        Self { hooks: vec![] }
    }

    /// 이벤트에 매칭되는 활성 훅 목록
    pub fn get_hooks(&self, event: &HookEvent) -> Vec<&HookDefinition> {
        let event_str = event.to_string();
        self.hooks
            .iter()
            .filter(|h| h.enabled && h.event == event_str)
            .collect()
    }

    /// 이벤트 발생 시 훅 실행 (비동기 HTTP POST 또는 명령어)
    pub async fn fire(&self, event: &HookEvent, payload: &serde_json::Value) {
        for hook in self.get_hooks(event) {
            if let Some(ref url) = hook.webhook_url {
                // 비동기 HTTP POST (fire-and-forget)
                let url = url.clone();
                let payload = payload.clone();
                tokio::spawn(async move {
                    let client = reqwest::Client::new();
                    let _ = client.post(&url).json(&payload).send().await;
                });
            }
            if let Some(ref cmd) = hook.command {
                tracing::info!("훅 실행: {} → {}", event, cmd);
                // 명령어 실행 (fire-and-forget)
                let cmd = cmd.clone();
                tokio::spawn(async move {
                    #[cfg(not(windows))]
                    {
                        let mut command = tokio::process::Command::new("sh");
                        command.arg("-c").arg(&cmd);
                        let _ = command.output().await;
                    }
                    #[cfg(windows)]
                    {
                        let mut command = tokio::process::Command::new("cmd");
                        command.args(["/C", &cmd]);
                        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
                        let _ = command.output().await;
                    }
                });
            }
        }
    }
}
