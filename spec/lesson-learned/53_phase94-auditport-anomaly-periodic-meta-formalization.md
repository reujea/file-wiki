---
phase: 94
date: 2026-05-22
topics: AuditPort 헥사고날 정공법 / H1 주기 호출 / 메타 룰 1 sub-rule 분리 / 메타 룰 19 META 정식 승격
related_lessons: 14, 21, 27, 30, 49, 50, 51, 52
related_meta_rules: 1, 5, 13, 14, 18, 19, 20
---

# 53. Phase 94 — AuditPort 헥사고날 + H1 주기 + 메타 룰 정형화 4건 묶음

## 상황

Phase 91~93 누적 후속 4건 (P0 1·2 + P1 5·6번) 묶음 진행. 사용자 명시 합의로 A 옵션(AuditPort trait 헥사고날 정공법) 선택.

## 문제

### 문제 1: 헥사고날 보존 vs 광범위 변경 트레이드오프

A3 trace_id 호출처 부착 시 service.rs(헥사고날 코어)가 settings.db 직접 의존 없음 발견. 3 옵션:
- **A (사용자 선택)**: AuditPort trait + NullAuditAdapter 디폴트 + SettingsAuditAdapter 구현 + 6+ 호출처 부착
- B: shared만 (mcp_server.rs + adapter Notion + Tauri commands)
- C: trait + null adapter만 (lesson 30 Ruflo 선구현)

### 문제 2: 8 파일 직접 생성처 — lesson 21/27 회귀 위험

`FileProcessingService { ... }` 직접 생성처 8곳 발견:
- shared/test_helpers.rs ServiceBuilder
- shared/lib.rs build_service
- service.rs 자체 단위 테스트 helper
- 5 통합 테스트 파일 (modals/cli/tests/)

추정으로는 5 통합 테스트 모두 갱신 필요. **사전 검증 grep으로 ServiceBuilder 사용 확인 → 5 통합 테스트 변경 0건**.

### 문제 3: 메타 룰 1의 19건 누적 — 단일 표 가독성 임계

19건 도달로 신규 작업 사전 체크리스트 lookup 비용 큼. 사용자 결정 영역(직전 phase 후속 플래그)이라 분리 진행.

### 문제 4: 메타 룰 19 후보 → 정식 승격 기준 불명

기존 후보 단계는 "2~3건 누적 시" 기준. Phase 91~93에서 4건 추가 누적 (총 5건) → 명확한 임계 도달. 메타 룰 16 (차원 A/B)의 누적 1건만 승격 선례와 비교 일관성 검토.

## 원인

### 직접 원인

- 헥사고날 정공법은 광범위 변경 — 사용자 명시 합의로 결정 (메타 룰 22 후보 자기 적용)
- 메타 룰 1 sub-rule 분리는 19건 도달로 미룰 수 없음
- 메타 룰 19 정식 승격은 lesson 49 도입 + 4 phase 추가 누적으로 자연 임계 도달

### 구조적 원인

- file-pipeline의 헥사고날이 잘 보존됨 — settings.db 같은 인프라가 core에서 보이지 않음. 본 Phase가 정공법 시험대
- ServiceBuilder 패턴이 lesson 21/27 회귀 차단을 자동화 (메타 룰 5 강화 + 메타 룰 1 1b 카테고리)
- 메타 룰 정식 승격은 명문화된 기준 부재 — lesson 53 후속으로 정형화 필요 (다음 메타 룰 후보)

## 개선

### 개선 1 — AuditPort 헥사고날 (A 옵션)

```rust
// core/ports/output.rs
pub trait AuditPort: Send + Sync {
    fn record(&self, trace_id: &str, stage: &str,
              inputs_hash: Option<&str>, output_summary: Option<&str>,
              applied_rule: Option<&str>);
}
pub struct NullAuditAdapter;  // 디폴트 no-op (lesson 14 회피)

// shared/settings_audit_adapter.rs
pub struct SettingsAuditAdapter { db_path: PathBuf }
impl AuditPort for SettingsAuditAdapter { ... }
```

호출처 부착:
- service.rs LLM 호출 3 지점 (classify_text / classify / verify reprocess)
- mcp_server.rs handle_search 2 지점 (캐시 hit + 일반)
- FileProcessingService 신규 필드 `audit` + 8 생성처 갱신 (ServiceBuilder 자동, 통합 테스트 0건)

### 개선 2 — H1 주기 호출 (메타 룰 13 3단계)

`modals/app/src/service.rs::c1-periodic` 주기 task에 `analyze_recent_audit` 호출. lesson 36 LRU GC 패턴과 동일 위치 사용:

```rust
let thresholds = AnomalyThresholds::default();
match analyze_recent_audit(&db, &thresholds) {
    Ok(report) if report.has_anomaly() => warn!(...),
    Ok(report) => debug!("정상 ({}건)", report.examined_events),
    Err(e) => warn!("분석 실패: {}", e),
}
```

GUI는 Phase 93 anomaly-report-card에서 이미 사용자에게 노출 — 메타 룰 13 4단계 결합 완성.

### 개선 3 — 메타 룰 1 sub-rule 분리

19건 시계열 표 보존 (메타 룰 12 "잔존 종결 의무" 변형) + 7 카테고리 표 신설:

| Sub-rule | 영역 | 누적 사례 |
|----------|------|---------|
| 1a UI 제거 | 13, 19, 19+, 47 | UI 10단계 체크리스트 + 통합 테스트 grep |
| 1b 구조체 필드 추가 | 21, 27, 35 | ServiceBuilder 패턴 (lesson 53 검증) |
| 1c DB 스키마 | 10, 26 | SETTINGS_DB_SCHEMA 단일 상수 |
| 1d 미연결 포트/함수 | 14, 31, 35 | NullXxxAdapter 디폴트 (메타 룰 5) |
| 1e 직렬화 4계층 | 32 | 도메인→어댑터→영속→파일 일괄 |
| 1f 함수/검사 분산 | 29, 38, 50-A, 50-B, 51, 52 | 단일 진입점 (메타 룰 19) |
| 1g spec 자기 위반 | 49, 28 | 단일 진실원 위임 (메타 룰 19 분기) |

### 개선 4 — 메타 룰 19 META 정식 승격

3축 분리 (What/Why/Link) + 3요소 동반 필수 (선언/grep/규칙) 명문화:

| lesson | 영역 | 진실원 |
|--------|------|--------|
| 49 | spec 문서 | deprecated.md |
| 50-A | 검사 함수 | classifier.rs check_sensitive_and_pii |
| 50-B | 검증 함수 | reasoning/verifier.rs Verifier |
| 51 | MCP 카탈로그 | mcp_tool_catalog_full() |
| 52 | 백엔드→frontend | 백엔드 단일 진입점 |

5건 누적 → META 정식 승격. 후보 섹션 삭제 (단일 진실원 자기 적용).

## 공통 교훈

1. **헥사고날 정공법은 ServiceBuilder가 있으면 안전** — 통합 테스트 변경 0건 (lesson 21/27 회피)
2. **사전 검증 grep으로 추정 빗나감 6번째 차단 성공** — `paths.base` 없음 즉시 발견 → settings_db_path 사용
3. **메타 룰 정식 승격은 누적 사례 + 자기 적용 명확화로 결정** — 본 Phase에서 메타 룰 19 자기 적용 사례 lesson 49/50-A/50-B/51/52 모두 누적 → 5건 도달
4. **메타 룰 1의 19건은 sub-rule 분리 임계 명확** — 시계열 보존 + 카테고리 신설로 가독성/연속성 양립

## 잘한 것 (재사용 가능)

1. **A vs B vs C 옵션 사용자 명시 합의 패턴** — lesson 51 메타 룰 22 후보 패턴 재적용 ("사용자 정책 경계 명시 합의")
2. **사전 검증 grep으로 ServiceBuilder 사용 확인** — 5 통합 테스트 갱신 의무 자동 해소 발견
3. **메타 룰 1 sub-rule 분리 시 본문 보존** — 시계열 누적 사례 표 그대로 + 카테고리 표 헤더 추가 (메타 룰 12 자기 적용)
4. **메타 룰 19 후보 섹션 단일 진실원 자기 적용 삭제** — 정식 룰로 승격 후 후보 섹션 잔존 제거

## 메타 룰 자기 적용 6건 (Phase 91 5건 + 본 Phase 6건)

| 룰 | Phase 94 자기 적용 |
|----|------------------|
| 메타 룰 1 (다중 위치) | AuditPort 8 생성처 동시 갱신 + sub-rule 분리 |
| 메타 룰 5 (트리거 인프라 3요소) | AuditPort = trait + Null 디폴트 + 호출처 분기 완성 |
| 메타 룰 13 (4단계 활성화) | A3 2단계 + H1 3단계 진척 |
| 메타 룰 14 (다중 진입점 통일) | service.rs LLM 호출 3 지점 동일 trait |
| 메타 룰 18 (추정 재검증) | 사전 검증 6번째 차단 성공 |
| 메타 룰 19 (단일 진실원 위임) | **META 정식 승격 + 자기 적용 — 후보 섹션 삭제** |

## 메타 룰 후보 23 등록 (메타 룰 정식 승격 기준 명문화)

(본 Phase에서 식별, lesson 53 본문에 첫 사례)

신규 메타 룰 후보 → 정식 승격 임계:

| 누적 사례 수 | 결정 |
|------------|------|
| 1건 | 후보 등록만 |
| 2~3건 | 후보 강화 (잠재 적용 후보 명시) |
| **4건+** | **META 정식 승격 검토 (메타 룰 19 패턴)** |
| 1건 + 본질 명확 | 즉시 META 정식 승격 (메타 룰 16 차원 A/B 선례) |

본 룰 자체가 메타 룰 23 후보 — 1건 누적 후 본문 등록.

## 다음 세션 플래그

- [ ] A3 trace_id 부착 영역 확장 (Notion adapter / commands.rs search / kg_paths 등 — 메타 룰 13 2단계 완성도 향상)
- [ ] H1 audit_trace 누적 후 실측 (lesson 46 G-1 같은 산발 실패 root cause 확정)
- [ ] 메타 룰 22 후보(사용자 정책 경계) 1건 추가 누적 시 META 등록 (Phase 91/92/94 명시 합의 패턴)
- [ ] 메타 룰 23 후보(승격 기준) 정형화 검토 — 본 Phase 자기 적용
- [ ] Tauri release 재빌드 (메타 룰 17 의무) — A3 부착으로 service.rs 변경
