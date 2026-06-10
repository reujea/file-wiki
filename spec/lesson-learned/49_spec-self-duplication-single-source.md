---
phase: post-G-7 spec hygiene
date: 2026-05-20
topics: spec 문서 자체의 메타 룰 1 위반 / architecture-archive.md ↔ deprecated.md 중복 / 단일 진실원 위임 패턴
related_lessons: 14, 19, 20, 38, 39
related_meta_rules: 1, 12
---

# 49. spec 문서 자체의 메타 룰 1 위반 — 자기 모순 해소 (단일 진실원 위임 패턴)

## 상황

옵션 A 적용 직전 spec 폴더 분석 중 발견. `spec/architecture-archive.md` 와 `spec/deprecated.md` 가 **같은 삭제·폐기 사실을 양쪽에 기재**하고 있었음. deprecated.md 머리말이 "단일 진실원"을 명시했지만 archive에 동일 사실이 6건 이상 중복 존재.

중복 매핑 (6건 확인):

| 사실 | architecture-archive.md | deprecated.md |
|------|------------------------|---------------|
| onnx feature 폐기 + vendor/onnxruntime 394MB | 트리거 #11 처리 (398~417줄, 상세 20줄) | `vendor/onnxruntime/` + `ort/tokenizers dep` 별도 표 2건 |
| `feedback_*` Tauri commands 7건 | Phase 64 본문 (451줄, "11개 dead 삭제: feedback_* + credential_store_*") | 별도 표 (74~83줄) |
| `credential_store_*` Tauri commands 4건 | Phase 64 본문 (같은 라인) | 별도 표 (84~90줄) |
| `cli.rs` 파일 | Phase 64 본문 (452줄, "modals/cli/src/cli.rs 완전 삭제") | 별도 표 (99~104줄) |
| dead config 5건 (qdrant_url 등) | Phase 65-2 본문 (385줄) | **누락** (단방향) |
| `get_health`/`get_lint`/`delete_document` 외 7건 | Phase 64 본문 (API dead 함수 9개) | 별도 표 (92~97줄) |

## 문제

1. **메타 룰 1 자기 적용 실패** — "같은 사실 다중 위치 동기화 누락" 룰의 본 문서들이 패턴 위반. 14건 누적된 룰의 15번째 사례가 spec 메타 문서 자신
2. **갱신 비대칭 위험** — architecture-archive.md는 "**읽기 전용 아카이브**" 선언 (10줄). 신규 삭제는 deprecated.md만 갱신될 가능성 → 양쪽 stale 발산
3. **단일 진실원 위반** — deprecated.md 머리말이 단일 진실원 선언했음에도 archive가 인벤토리 중복 보유
4. **단방향 누락 발견** — dead config 5건 (Phase 65-2)은 archive에만 있고 deprecated.md에 누락. lesson 20 §44 grep 검증 사실이 deprecated.md 인벤토리에 없어 "왜 없는가" 질문 시 추적 분기 발생

## 원인

### 직접 원인

- 두 문서의 **역할 경계가 명문화되지 않음**:
  - architecture-archive.md = Phase별 결정 이력 (시간축)
  - deprecated.md = 현재 없는 항목 인벤토리 (상태축)
  - 그러나 "삭제된 항목"은 양쪽 모두에 자연스럽게 등장 → 작성자가 어디에 쓸지 매 phase 판단
- deprecated.md 도입(Phase 86 A-4) 시점에 architecture-archive.md 기존 인벤토리 마이그레이션 누락 — 신규 항목만 deprecated.md에 추가하고 옛 사실은 archive에 잔존

### 구조적 원인

- 두 축(시간축 vs 상태축)이 같은 사실을 다른 관점으로 표현 → "왜(Why)"와 "무엇(What)"이 자연 분리되지 않으면 양쪽에 모두 쓰는 경향
- 메타 룰 1의 14건 누적 사례 (lesson 10/13/14/19/19+/21/26/27/28/29/31/32/35/47) 모두 **코드 또는 UI 영역**의 다중 위치 — spec 문서 자체가 위반자가 될 수 있다는 인식 부재
- "단일 진실원"이라는 머리말 선언만으로 위반이 차단되지 않음 (검증 메커니즘 부재)

## 개선

### 옵션 A 적용 (단일 진실원 강제)

**deprecated.md 갱신**:
1. 머리말에 단일 진실원 원칙 명시 추가 ("`architecture.md` / `architecture-archive.md`는 결정 맥락만, 본 문서는 인벤토리")
2. **dead config 5건 신규 항목** 추가 (Phase 65-2, 기존 누락분) — `vector_db.qdrant_url` / `collection` / `auto_start` / `embedding.sensitive_model` / `embedding.onnx_model_dir`

**architecture-archive.md 갱신**:
1. 머리말에 단일 진실원 위임 선언 추가
2. **트리거 #11** 인벤토리 6줄 → 링크 1줄. 상세 인벤토리(`onnxruntime.dll`, `1.24.4 archive`, `260줄 onnx_embed.rs`)는 deprecated.md만 보존. **결정 맥락 / build_service 영향 / 검증 결과는 archive 보존**
3. **Phase 64** Tauri commands 11건 + cli.rs 인벤토리 → 링크 5줄. Frontend 측 변경(`renderSearchResults` silent fail / `_renderCredBindings` 117줄)은 architectural 변경이므로 archive 보존
4. **Phase 65-2 dead config 5건** → 링크 1줄

### 검증

옵션 A 적용 후 grep 검증:

```bash
grep -nE 'feedback_\*|credential_store_\*|cli\.rs 완전 삭제|vendor/onnxruntime|dead config 제거|onnxruntime.*394MB' \
  spec/architecture-archive.md spec/deprecated.md
```

잔존 매치 9건 모두 정당화:
- archive: 목차 / 섹션 제목 / 링크 본문 항목 식별용 (정상)
- deprecated: 단일 진실원 본문 (정상)

상세 인벤토리는 deprecated.md로 단일화 완료.

### 측정 가능한 효과

- archive 본문 약 **17줄 감소** (3 구간 축약, 결정 맥락 보존)
- deprecated.md **+8줄** (dead config 5건 신규 항목 — 단방향 누락 해소)
- 메타 룰 1 위반 6건 → 0건 (옵션 A 적용 후)

### 갱신 규칙 (재발 방지)

신규 삭제 시:
- **deprecated.md만 갱신** (인벤토리 = 무엇이 없는가)
- architecture.md / architecture-archive.md는 **결정 맥락만 추가** (왜 그 Phase에 그 결정)
- 인벤토리는 deprecated.md 링크로 위임 (`→ spec/deprecated.md 의 "..." 참조`)

phase 종결 시 grep 체크리스트 추가 후보:

```bash
# 메타 룰 1 자기 적용 검증
grep -nE '삭제 함수|삭제 항목|dead.*삭제|폐기' spec/architecture.md spec/architecture-archive.md
# → 결과 1+건이면 deprecated.md에 동일 사실 존재 여부 확인
```

## 공통 교훈

1. **단일 진실원 선언만으로는 부족** — 검증 메커니즘(grep 체크리스트 + 갱신 규칙) 동반 필수
2. **메타 룰 1은 코드뿐 아니라 spec 문서 자신에도 적용** — 같은 사실이 두 곳에 표현되면 동기화 누락
3. **시간축 ↔ 상태축 분리** — "왜(Why)"는 시간축(architecture-archive), "무엇(What)"은 상태축(deprecated). 한 사실이 둘 다 필요해 보이면 What만 본문, Why는 What을 링크 참조
4. **읽기 전용 아카이브 선언**은 신규 항목 차단일 뿐, **옛 항목 마이그레이션 의무**를 면제하지 않음

## 잘한 것 (재사용 가능)

1. **분석-합의-적용 3단계 분리** — 1턴(분석/중복 매핑) → 2턴(옵션 A vs B 사용자 결정) → 3턴(적용) 패턴. lesson 19 10단계와 결합 가능
2. **단방향 누락 검출** — 분석 단계에서 archive→deprecated 한 방향만이 아니라 deprecated→archive 방향도 검증 (dead config 5건 발견)
3. **링크 위임 본문 형태** — `→ spec/deprecated.md 의 "..." 참조` 1줄 형식. archive의 결정 맥락 가독성 보존하면서 인벤토리 단일화

## 메타 룰 1 추가 사례 (META.md 갱신 의무)

본 lesson을 메타 룰 1의 **15번째 누적 사례**로 META.md에 추가:

| lesson | 패턴 | 단일화 가능 여부 |
|--------|------|------------------|
| **49** | **spec 문서 자체 — architecture-archive.md ↔ deprecated.md 같은 삭제 사실 6건 중복** | **시간축(Why) vs 상태축(What) 역할 분리 + 단방향 링크 위임** |

## 메타 룰 신규 후보: 단일 진실원 위임 패턴

본 lesson에서 정형화된 패턴이 메타 룰 신규 후보:

> **메타 룰 N 후보 — 단일 진실원 위임 패턴**:
> 같은 사실이 둘 이상 문서/위치에 등장할 때, 한 곳을 **진실원(인벤토리/상태)**으로 지정하고 나머지는 **링크 참조(맥락/이유)**로 위임한다.
>
> - 진실원 = "무엇이 그러한가" (현재 상태)
> - 참조원 = "왜 그렇게 되었는가" (결정 맥락)
> - 진실원 선언 + 검증 grep + 갱신 규칙 3요소 동반 필수

누적 1건 (본 lesson 49). 2~3건 추가 시 META 정식 승격 검토.

## 다음 세션 플래그

- [ ] META.md 메타 룰 1에 15번째 누적 사례 추가 (본 lesson 49)
- [ ] 단일 진실원 위임 패턴 메타 룰 후보 등록 (META.md 끝)
- [ ] architecture.md 본문(현재 Phase 80~90+ 활성)이 향후 archive로 이관 시 동일 grep 체크리스트 자동화 검토 (Q2 후보)
- [ ] domain-map.md ↔ architecture.md 매핑 표 잠재 중복 점검 (Q3 후보 — 별도 phase)
