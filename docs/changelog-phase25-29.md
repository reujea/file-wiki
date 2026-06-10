# Phase 25~29 변경 사항 + 테스트 가이드

> 2026-04-14 세션 산출물

---

## 1. 기존 동작 변경

### 1-1. process_file이 파이프라인 경유로 통합

**이전**: `process_file()`은 독립적인 400줄 처리 로직을 가지고 있었고, 파이프라인 미매칭 파일은 이 경로로 처리됨.
**이후**: `process_file()`은 내부적으로 Default 파이프라인을 생성하여 `process_file_with_pipeline()`에 위임. 모든 파일이 파이프라인을 경유.

**영향**:
- 기존 `process_file()` API는 동일하게 동작 (시그니처 변경 없음)
- 검증 메트릭이 파이프라인 경로에서도 기록됨 (이전에는 누락)
- watcher에서 파이프라인 미매칭 시에도 Default 파이프라인으로 처리

**확인 방법**:
```bash
# 1. inbox에 파일 투입
# 2. Dashboard > Processing 탭에서 "Default" 파이프라인으로 처리되는지 확인
# 3. Dashboard > Verification 탭에서 메트릭이 기록되는지 확인
```

### 1-2. process_file_with_pipeline 시그니처 변경

**이전**: `llm_override: Option<Arc<dyn LLMPort>>` — 단일 LLM 오버라이드
**이후**: `llm_overrides: &HashMap<String, Arc<dyn LLMPort>>` — 역할별 LLM 맵

**영향**:
- 직접 `process_file_with_pipeline`을 호출하는 외부 코드가 있으면 컴파일 에러
- watcher 내부에서만 호출하므로 사용자 코드에는 영향 없음

### 1-3. PipelineStep에 credential 필드 추가

**이전**: `Verify`와 `Embedding` 스텝에 credential 없음. 항상 글로벌 LLM 사용.
**이후**: 각 스텝에 `credential: Option<String>` 추가. 미설정 시 글로벌 LLM fallback.

**영향**:
- 기존 pipeline.toml은 100% 호환 (`#[serde(default)]`)
- 새 필드를 설정하면 해당 스텝만 다른 LLM 사용 가능

**확인 방법**:
```toml
# pipeline.toml 예시
[[pipelines]]
name = "고품질 처리"
pattern = "*.md"
priority = 10
enabled = true
postprocess_credential = "ollama_local"  # Todo/교차참조에 로컬 LLM 사용

[[pipelines.steps]]
type = "llm"
credential = "claude_api"  # 분류/가공에 Claude API 사용

[[pipelines.steps]]
type = "verify"
enabled = true
credential = "claude_cli"  # 검증에 Claude CLI 사용

[[pipelines.steps]]
type = "embedding"
credential = "openai_api"  # 임베딩에 OpenAI 사용
```

---

## 2. 신규 기능

### 2-1. Cross-Encoder 리랭킹

검색 결과를 Claude CLI가 관련도를 재평가하여 순서를 재정렬합니다.

**설정** (`pipeline.toml`):
```toml
[rerank]
enabled = true        # 기본: false
provider = "claude_cli"
top_n = 20            # 리랭킹 대상 상위 N개
```

**동작 흐름**:
```
검색 쿼리 → 벡터 검색 (top_k * 3) → 필터 → [리랭킹] → 최종 top_k 반환
```

**확인 방법**:
```bash
# 1. pipeline.toml에 [rerank] enabled = true 추가
# 2. MCP로 검색 실행
# 3. Claude CLI가 호출되어 결과 순서가 변경되는지 로그 확인
# 4. 비활성 시 (enabled = false) 기존과 동일하게 동작
```

**주의**: 리랭킹 활성화 시 검색마다 Claude CLI 호출이 추가되어 1~3초 지연 발생.

### 2-2. 외부 저장소

가공본과 원본을 외부 저장소에 자동 업로드합니다.

**지원 저장소**:
| 유형 | 설정 | 의존성 |
|------|------|--------|
| Network (SMB/NFS) | `network_path` | 없음 (fs::copy) |
| WebDAV (Nextcloud/Synology) | `webdav_url`, `webdav_user`, `webdav_password` | reqwest (기존) |
| S3 (AWS/MinIO/R2) | 설정만 존재, 어댑터 미구현 | aws-sdk-s3 (미추가) |

**설정** (`pipeline.toml`):
```toml
[remote_storage]
enabled = true
provider = "network"                    # "network" | "webdav"
network_path = "\\\\NAS\\share\\backup" # Windows UNC 경로 또는 /mnt/nas
```

WebDAV 예시:
```toml
[remote_storage]
enabled = true
provider = "webdav"
webdav_url = "https://nextcloud.example.com/remote.php/dav/files/user/"
webdav_user = "사용자명"
webdav_password = "비밀번호"
```

**동작**: 파일 처리 완료 후 벡터DB 색인 직후에 자동 업로드. 실패 시 warn 로그만 남기고 처리 자체는 성공.

**확인 방법**:
```bash
# Network 테스트
# 1. pipeline.toml에 [remote_storage] enabled = true, network_path 설정
# 2. inbox에 파일 투입
# 3. network_path/processed/ 에 .zst 파일이 생성되는지 확인
# 4. network_path/originals/ 에 원본 .zst 파일이 생성되는지 확인
```

### 2-3. 역할별 크레덴셜

LLM이 사용되는 6개 지점에 각각 다른 크레덴셜을 설정할 수 있습니다.

| 역할 | 설정 위치 | fallback |
|------|-----------|----------|
| 분류/가공 | `PipelineStep::Llm { credential }` | default_credential → 글로벌 LLM |
| 검증/2-Pass | `PipelineStep::Verify { credential }` | default_credential → 글로벌 LLM |
| 임베딩 | `PipelineStep::Embedding { credential }` | 글로벌 임베딩 |
| Todo 병합 | `PipelineDefinition.postprocess_credential` | 글로벌 LLM |
| 교차참조 | `PipelineDefinition.postprocess_credential` | 글로벌 LLM |
| 토픽 병합 | `PipelineDefinition.postprocess_credential` | 글로벌 LLM |

**확인 방법**:
```bash
# 1. Settings > 크레덴셜에서 2개 이상 크레덴셜 등록
#    예: "claude_api" (Anthropic API), "ollama_local" (Ollama)
# 2. Pipeline 탭에서 파이프라인 편집
#    - LLM 노드: credential = "claude_api"
#    - Verify 노드: credential = "ollama_local"
# 3. 파일 처리 시 로그에서 각 단계별 다른 프로바이더가 사용되는지 확인
```

---

## 3. Settings UI 추가 항목

Dashboard > Settings > 시스템 그룹에 추가된 섹션:

| 섹션 | 항목 | 설명 |
|------|------|------|
| 스케줄 | retention_days, purge_cron, lint_cron, lint_stale_days, fragment_threshold | 정리/점검 주기 |
| 경로 | extra_inboxes | 추가 감시 디렉토리 (쉼표 구분) |
| 동시성 | max_workers | 최대 동시 처리 수 (기본 4, 재시작 필요) |
| 외부 저장소 | enabled, provider, network_path, webdav_* | 원격 백업 설정 |
| 리랭킹 | enabled, provider, top_n | 검색 리랭킹 설정 |

**확인 방법**:
```bash
# 1. Dashboard 실행
# 2. Settings 탭 > 시스템 그룹
# 3. 스케줄/경로/동시성/외부 저장소/리랭킹 섹션이 보이는지 확인
# 4. 값 변경 후 저장 → pipeline.toml에 반영되는지 확인
```

---

## 4. 테스트 현황

### 자동 테스트 (172개, 전체 통과)

```bash
cd src
cargo test -p file-pipeline-core -p file-pipeline-adapters -p file-pipeline-shared
```

| 크레이트 | 테스트 수 | 신규 추가 |
|----------|----------|----------|
| core | 116 | +1 (파이프라인 위임 검증) |
| adapters | 48 | +27 (zstd 3, claude 4, notification 3, verifier 5, qdrant 4, reranker 4, remote_storage 2, watcher 7) |
| shared | 8 | +8 (config 5, mcp 3) |

### 수동 통합 테스트 (env guard, 선택적)

```bash
# Telegram 통합 테스트 (봇 토큰 필요)
export TELEGRAM_BOT_TOKEN="your_token"
export TELEGRAM_CHAT_ID="-1003990184767"
cargo test -p file-pipeline-cli notification_integration

# Slack 통합 테스트 (봇 토큰 필요)
export SLACK_BOT_TOKEN="your_token"
export SLACK_CHANNEL="#test"
cargo test -p file-pipeline-cli notification_integration
```

---

## 5. 호환성

| 항목 | 호환 여부 | 비고 |
|------|-----------|------|
| 기존 pipeline.toml | 100% 호환 | 신규 필드는 `#[serde(default)]` |
| process_file() API | 100% 호환 | 시그니처 변경 없음 |
| MCP 도구 | 100% 호환 | 검색 결과에 리랭킹 추가만 (비활성 시 무변경) |
| Dashboard | 하위 호환 | 신규 섹션 추가만 |
| Tauri commands | 100% 호환 | get_config/save_config가 제네릭 처리 |

---

## 6. 체크리스트

### 빌드 확인
- [ ] `cd src && cargo check --all` — 경고 0건 (기존 1건 제외)
- [ ] `cd src/modals/app && cargo check` — Tauri 빌드 성공

### 기본 동작 확인
- [ ] pipeline.exe 실행 → Dashboard 표시
- [ ] inbox에 파일 투입 → 가공 완료
- [ ] MCP 검색 → 결과 반환

### 신규 기능 확인
- [ ] Settings > 시스템에 스케줄/경로/동시성/외부저장소/리랭킹 표시
- [ ] `[rerank] enabled = true` 설정 후 MCP 검색 → 리랭킹 로그 확인
- [ ] `[remote_storage] enabled = true` + network_path 설정 → 가공 후 파일 복사 확인
- [ ] 파이프라인 credential 설정 → 역할별 다른 LLM 사용 로그 확인

### 회귀 확인
- [ ] Verification 탭에 메트릭 정상 기록
- [ ] Processing 탭에 상태 표시 정상
- [ ] 실패 항목 재처리 버튼 동작
