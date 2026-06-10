## 상황 (Phase 70+71, 2026-05-06)

Phase 67 인스펙터 고도화 + Phase 68 시각 마커 + Phase 69 일부 보완 후, 사용자가 "step에 포함 안 된 설정 검토"를 요청. 검토 결과 코드 상수로 묶여 UI 미노출인 항목 7건 + 노드 누락 3건 발견.

## 문제

- Memory Tier 임계(hot 7/warm 30/cold 90)가 `domain/memory_tier.rs` 코드 상수
- 검색 후처리 파라미터(window_lines/mmr_lambda/sparse_weight/time_weight)가 코드 상수
- flush_crossref 30초가 코드 상수
- 알림 배치 요약 30초가 코드 상수
- Quarantine 분기, Memory Tier 갱신, Lint 주기 검사가 노드 시각화에 없음
- HookDefinition은 config에 있으나 UI 매핑 없음

## 원인

1. **코드 상수의 누적**: 각 phase가 기능 추가 시 임시로 코드 상수를 사용하고, 이후 사용자가 조정할 일이 거의 없을 거라 가정해 config로 끌어내지 않음.
2. **노드 시각화의 보수성**: 분기 흐름(Verify FAIL → Quarantine)은 단순 직선 플로우와 맞지 않아 노드화 보류.
3. **config-노드 매핑의 불완전성**: configSections는 섹션 단위 매핑만 지원해, 한 노드에서 여러 섹션의 일부 필드만 매핑할 수 없었음.

## 개선

### 패턴 A — config 신규 섹션은 도메인 단위로 분리

```
[memory_tier]   — 도메인 정책
[search]        — 도메인 후처리 파라미터
[notification_batch] — 채널과 분리된 배치 동작 정책
```

상수 4개를 한 섹션에 몰아넣지 않고, 도메인 의미별로 섹션 분리. 향후 추가될 검색 파라미터(예: BM25 k1/b)는 [search]에 흡수, 알림 정책은 [notification_batch]에 흡수.

### 패턴 B — configFields 메타 (Phase 69 유산)

```javascript
// configSections: 섹션 전체 매핑
fragment: { configSections: ['schedule'] }  // ← schedule 전체 노출 (lint_interval_hours까지 같이)

// configFields: 섹션 내 특정 필드만
fragment: { configFields: [['schedule', 'fragment_threshold']] }  // ← 정확히 한 필드
```

한 섹션을 여러 노드가 공유하면서 필드만 다르게 매핑할 때 필수. 예: `schedule.fragment_threshold` (Fragment 노드) vs `schedule.lint_interval_hours` (Lint 노드).

### 패턴 C — 분기 노드는 일단 단일 노드로

Quarantine 분기는 본래 Verify FAIL 분기 그래프지만, 노드 플로우가 단순 직선이라 Cytoscape 같은 라이브러리 없이는 분기 표현이 어려움. 임시로 Quarantine을 단일 노드(Verify 다음)로 두고, 인스펙터에서 `verification.on_fail` 옵션을 노출. 사용자가 quarantine vs skip_with_notify를 선택.

향후 분기 그래프 라이브러리 도입 시 노드를 이동(Verify와 평행 배치)할 수 있도록 PB_NODES 정의는 그래프 친화적으로 유지.

### 패턴 D — Settings 그룹은 유사 책임끼리 묶기

```
크레덴셜 관리 — 자원 등록
일반          — 시스템 출력 (로깅)
이벤트 훅     — 자동화 통합 (HookDefinition)
마이그레이션  — 운영 작업
```

이벤트 훅은 가공 흐름의 일부가 아니라 외부 시스템 통합이므로 Settings에 둠. 가공 노드 인스펙터에 흡수하면 노드 흐름과 외부 통합이 섞여 멘탈 모델 혼란.

### 패턴 E — HookDefinition은 UI 편집 미루고 readonly 표시

복잡한 데이터 구조(HookEvent enum + action JSON)는 UI 편집이 큰 비용. 일단 읽기 전용 목록만 노출하고 편집은 TOML 직접 수정 안내. 사용자 사용 빈도 측정 후 편집 UI 결정.

## 적용 범위

- 향후 코드 상수 발견 시: 즉시 config로 끌어내기 vs 사용 빈도 낮으면 보류 결정
- 신규 노드 추가 시: configSections + configFields 메타 양쪽 활용
- 분기 노드는 단일 노드 + 인스펙터 분기 옵션으로 처리 (라이브러리 도입 전까지)
- Settings 그룹은 책임 단위로만 분리, 자원 단위로는 분리 안 함

## 사후 점검

- 사용자가 새 [memory_tier] / [search] / [notification_batch] 임계를 실제로 조정하는지
- HookDefinition readonly 표시가 사용자 요구를 충족하는지 (편집 요청 빈도)
- Quarantine 단일 노드 + on_fail 옵션이 분기 흐름 멘탈 모델과 일치하는지

## 참고
- lesson 22: 인스펙터 기반 config 편집 5패턴
- 코드 상수 발견 시 처리 정책 — 사용 빈도 + 도메인 중요도 판단
