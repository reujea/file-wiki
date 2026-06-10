---
phase: 92
date: 2026-05-22
topics: Mirage Resource 패턴 흡수 / MCP 카탈로그 다차원 분류 / audit_anomaly 자동 감지 / 메타 룰 20 정식 승격 / 메타 룰 21 후보 등록 / 사용자 명시 합의로 외부 협업 보류 정책의 경계 결정
related_lessons: 14, 30, 45, 50
related_meta_rules: 1, 13, 16, 18, 20, 21
---

# 51. Phase 92 — Mirage 흡수 + MCP 다차원 + audit_anomaly + 메타 룰 20/21 정형화

## 상황

Phase 91 직후 사용자가 외부 프로젝트 비교 분석 요청 (JAMES v0.3.0 재검증 + Mirage v0.0.1).
**사용자 명시**: "원격 저장소 표준화도 포함해서 구현 진행해" — 외부 협업/연계 보류 정책의 경계 결정.

JAMES 5일 변동 없음 (재검증) + Mirage는 본질 도메인 불일치 (VFS 통합) 확인. 사용자 명시 합의로 Mirage Resource 패턴 부분 흡수 결정.

## 문제

### 문제 1: 외부 협업/연계 보류 정책 vs 원격 저장소 표준화 경계

이전 사용자 정책: "외부 협업, 외부 솔루션, 외부 연계 항목 보류".
Mirage Resource 패턴 = 다양한 외부 백엔드 추상화 — 정책 위반처럼 보임.

**해소**:
- Mirage 전체 흡수 (S3/GDrive/Slack/Gmail 등 새 백엔드 추가) = 외부 연계 추가 → 🔴 보류
- **기존 어댑터(S3/WebDAV/Network/Null/Notion)의 capability 표준화** = 메타데이터 통일만 → 🟢 진행
- 사용자 명시 합의로 후자 결정 — 새 외부 서비스 추가 아닌 기존 표준화

### 문제 2: Mirage 본질 도메인 불일치 (메타 룰 21 후보 첫 사례)

| Mirage 가정 | file-pipeline 도메인 | 정렬 |
|------------|---------------------|------|
| AI 에이전트 사용자 + bash 명령 | MCP 도구 (도메인 특화) | 🔴 |
| TypeScript + Python | Rust 단일 바이너리 | 🔴 |
| 분산 캐시 (Redis 옵션) | 인프로세스 단일 사용자 | 🔴 |
| VFS mount tree | 헥사고날 + 도메인 특화 | 🔴 |
| **Resource capability 메타데이터** | RemoteStoragePort 5 메서드 + is_configured만 | **🟡 흡수 가능** |
| **Command 3차원 등록** | mutates_state 단일 차원 (Phase 91 B2) | **🟡 흡수 가능** |

→ 본질 영역 전부 불일치, **메타데이터 표준화 영역만 부분 일치**. 메타 룰 21 후보 패턴 정확 일치 (본질/부수 분리).

### 문제 3: 추정 빗나감 4번째 누적 (가벼운 사례)

본 분석 초기 추정: "JAMES v0.3.0 이후 메이저 변동 있을 것". 실측: 5일간 메이저 변동 없음.
이전 3건(lesson 46 G-1/G-4 + lesson 50 service.rs)보다 가벼운 사례이지만 패턴 유지.

## 원인

### 직접 원인

- "외부 협업/연계 보류" 정책의 경계가 명문화 안 됨 — 사용자 명시 합의로 해소
- Mirage의 좋은 패턴(Resource capability 메타)을 본질 영역까지 흡수하려는 욕구
- JAMES 변동 추정도 메타 룰 18 검증 안 함

### 구조적 원인

- 외부 프로젝트 흡수 시 "본질 도메인" vs "부수 도메인" 분리 메커니즘 부재 (메타 룰 21 후보 필요성)
- 메타 룰 20 누적 사례 4건 임계 도달했지만 META 정식 승격 안 함
- 사용자 명시 보류 정책의 예외/경계 정형화 부재

## 개선

### 개선 1 — H3 MCP 카탈로그 다차원 분류 (Mirage Command 3차원 등록 패턴)

Phase 91 B2 `mcp_tool_catalog` (단일 차원, 26 도구)를 Mirage 3차원 패턴으로 확장:

```rust
pub struct McpToolMetadata {
    pub name: &'static str,
    pub mutates: bool,                   // Phase 91 B2
    pub category: McpToolCategory,       // Phase 92 H3 신규
    pub cost: McpToolCost,               // Phase 92 H3 신규
}
```

- `McpToolCategory` 7종 (Search / Kg / Settings / Todo / Signal / Snapshot / Lint)
- `McpToolCost` 3종 (Free / LlmCall / HeavyCompute)
- 호환성 wrapper: `mcp_tool_mutates_state` + `mcp_tool_catalog` 유지
- **단일↔다차원 일치성 자체 테스트** (메타 룰 1 자기 적용)

### 개선 2 — H5 RemoteStoragePort 표준화 (Mirage Resource 패턴, mode 분기 흡수)

```rust
pub struct ResourceCapabilities {
    pub backend: &'static str,
    pub can_upload: bool,
    pub can_download: bool,
    pub can_list: bool,
    pub can_delete: bool,
    pub mode_options: &'static [&'static str],
    pub active_mode: String,
    pub supports_hard_delete: bool,
}

pub trait RemoteStoragePort: Send + Sync {
    // 기존 5 메서드 ...

    fn capabilities(&self) -> ResourceCapabilities {  // 디폴트 메서드 (호환성)
        ResourceCapabilities::standard("unknown")
    }
}
```

- 5 어댑터 모두 `capabilities()` 오버라이드 (S3/WebDAV/Network/Null/Notion)
- **Notion 핵심**: page 모드는 `can_upload=true`, attach 모드는 `can_upload=false` → 호출자 사전 차단 가능
- `supports_hard_delete=false` (Notion archived=true PATCH)

### 개선 3 — H1 audit_anomaly 자동 이상 감지 + 사용자 권고

Phase 91 A3 `audit_trace` 인프라 자연 확장 (메타 룰 13 2단계):

```rust
pub fn analyze_recent_audit(db: &SettingsDb, thresholds: &AnomalyThresholds) -> Result<AnomalyReport>;
```

- JAMES 자체 진화 게이트 패턴 흡수, RBAC 인간 승인 게이트는 보류 (단일 사용자)
- **자동 롤백 아닌 사용자 검토 권고만** (메타 룰 20 자기 적용 — RBAC 보류)
- 신규 `SettingsDb::list_recent_audit_events(limit)` 메서드
- lesson 46 G-1 "Claude CLI 산발 실패" 같은 추정 사례를 trace 누적으로 root cause 확정 인프라

### 개선 4 — 메타 룰 20 META 정식 승격 (누적 4건 도달)

"외부 프로젝트 패턴 흡수 시 도메인 가정 정렬":
- 3축 분리 (패턴 추출 / 도메인 가정 검증 / 부분 흡수 결정)
- 흡수 결정 라벨 (🟢/🟡/🔴) + 메타 룰 16 차원 B 결합
- 사전 분류 체크리스트 5항

### 개선 5 — 메타 룰 21 후보 정식 등록 (누적 2건)

"외부 도메인 도구 흡수 시 본질/부수 도메인 분리":
- TFM (XGBoost 도메인) + Mirage (VFS 도메인) 모두 본질 도메인 불일치
- 부수 도메인(운영 지표 / 메타데이터)만 흡수 후보
- 1건 추가 누적 시 META 정식 승격

### 개선 6 — "사용자 명시 합의로 정책 경계 결정" 패턴 명문화

사용자 정책 위반처럼 보이는 항목도 사용자 명시 합의로 진행 가능. 본 phase 사례:
- 정책: "외부 협업/연계 보류"
- 항목: Mirage Resource 표준화 (외부 연계처럼 보이나 메타데이터 표준화만)
- 사용자 합의: "원격 저장소 표준화도 포함해서 구현 진행해"
- 결정: 새 외부 서비스 추가 아닌 기존 어댑터 표준화만 진행

**메타 룰 후보 22**: "사용자 정책의 경계 결정은 명시 합의로 케이스 기록 의무". 1건 누적 (본 lesson).

## 공통 교훈

1. **외부 프로젝트 흡수는 본질/부수 도메인 분리가 핵심** — 같은 도메인은 메타 룰 20, 다른 도메인은 메타 룰 21
2. **사용자 정책의 경계는 명시 합의로 케이스 단위 결정** — 정책 자체를 바꾸지 않고 예외 기록
3. **추정 빗나감 4건째 누적 (메타 룰 18)** — JAMES 변동 추정도 검증 안 함, 가벼운 사례지만 패턴
4. **호환성 wrapper로 단계적 마이그레이션** — `mcp_tool_catalog`(단일) + `mcp_tool_catalog_full`(다차원) 병행. 메타 룰 1 자기 적용 (일치성 테스트)
5. **인프라 활성화 단계 분리 (메타 룰 13)** — H1 audit_anomaly는 2단계만 (로직 활성화), 호출처 부착 + UI는 후속

## 잘한 것 (재사용 가능)

1. **사전 검증 grep 의무 적용** — Task 8에서 RemoteStoragePort 실제 코드 확인 후 진행 (메타 룰 18 자기 적용)
2. **추정 빗나감 4건째 발견 즉시 lesson 51 본문에 기록** — 메타 룰 18 누적 사례 등재 의무 (lesson 50 후속)
3. **호환성 wrapper 패턴** — 기존 호출처 0건 변경. 신규 API는 별도 함수로 노출 (단일 차원 ↔ 다차원)
4. **메타 룰 20 + 21 동시 정형화** — 같은 phase에서 정식 승격(20) + 후보 등록(21) 묶음. 외부 프로젝트 흡수 패턴 완성도
5. **`-j 2` 즉시 적용** — 메타 룰 9 자기 적용 누적 (Phase 91 + 92)

## 메타 룰 1 추가 사례 (META.md 갱신)

본 lesson을 메타 룰 1의 18번째 누적 사례로 등재:

| lesson | 패턴 | 단일화 |
|--------|------|--------|
| **51** | **MCP 도구 분류 단일 차원 (mutates_state) → 다차원 (mutates + category + cost)** | **McpToolMetadata + 단일↔다차원 일치성 테스트** (Mirage 3차원 등록 패턴) |

## 다음 세션 플래그

- [ ] H1 audit_anomaly 호출처 부착 (메타 룰 13 3단계 — service.rs 가공 종료 후 주기 호출)
- [ ] H1 GUI Verification 탭 이상 신호 카드 (메타 룰 13 4단계)
- [ ] H3 GUI Settings 카드에 MCP 도구 다차원 분류 표시 (Phase 91 후속 P0 3번 + 본 Phase H3 결합)
- [ ] H5 GUI Pipeline 외부 저장소 인스펙터에 capability 노출 (Notion mode 분기 시각화)
- [ ] 메타 룰 22 후보 ("사용자 정책 경계 명시 합의 기록") 1건 추가 누적 시 META 등록 검토
- [ ] 메타 룰 21 1건 추가 누적 시 META 정식 승격
- [ ] Tauri release 재빌드 (메타 룰 17) — H3/H5/H1 모두 라이브러리 변경이라 GUI에는 자동 반영 안 됨
