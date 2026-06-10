use serde::{Serialize, Deserialize};

/// 이벤트 유형
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookEvent {
    FileDetected,      // inbox 파일 감지
    ProcessStart,      // 가공 시작
    ProcessComplete,   // 가공 완료
    VerifyFail,        // 검증 실패
    SearchQuery,       // 검색 쿼리 수신
}

impl std::fmt::Display for HookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookEvent::FileDetected => write!(f, "file_detected"),
            HookEvent::ProcessStart => write!(f, "process_start"),
            HookEvent::ProcessComplete => write!(f, "process_complete"),
            HookEvent::VerifyFail => write!(f, "verify_fail"),
            HookEvent::SearchQuery => write!(f, "search_query"),
        }
    }
}

/// 훅 정의
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDefinition {
    pub event: String,
    #[serde(default)]
    pub webhook_url: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub enabled: bool,
}

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
        self.hooks.iter()
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
                    let _ = client.post(&url)
                        .json(&payload)
                        .send()
                        .await;
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
