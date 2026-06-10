---
created: 2026-05-15
phase: 87
status: 결정 — supertonic 무관 / 352523 기적용 / 353407 부분 적용 (Phase 87)
source_docs:
  - https://github.com/supertone-inc/supertonic
  - https://wikidocs.net/352523
  - https://wikidocs.net/353407
---

# 외부 문서 3종 분석 결과 + 본 프로젝트 적용 결정

## 결정 요약

| 문서 | 결정 | 적용 위치 |
|------|------|---------|
| supertone-inc/supertonic | **무관** — 직접 적용 없음 | (참고용) |
| wikidocs/352523 (자기 진화 에이전트) | **기적용** — Ruflo C1/decision_log/config_snapshot이 부분 충족 | (Phase 73~84 기존) |
| wikidocs/353407 (정리·감사 흐름) | **부분 적용** — 4건 중 3건 Phase 87 적용 | A-1/A-2/A-3 (A-4 보류) |

본 문서는 외부 분석 결과의 **단일 진실원**이다. 향후 같은 문서 재분석 시 본 문서 인용 → 결정 재반복 방지.

---

## 1. supertone-inc/supertonic 분석

### 본질
ONNX Runtime 기반 로컬 TTS (Text-to-Speech) 시스템:
- Speech Autoencoder + Flow-Matching + Self-Purifying Flow Matching
- 31개 언어, 99M 파라미터, 44.1kHz WAV 출력
- Python / Node / Browser / Java / C++ / C# / Go / Swift / iOS / Rust / Flutter 지원
- ONNX Runtime + onnxruntime-web (WebGPU/WASM)

### 본 프로젝트와의 관계

**직접 적용 가치**: 없음. file-pipeline은 텍스트 분류·검색이며 출력 음성 합성 시나리오 없음.

**간접 패턴 차용 — 이미 적용됨**:
- ONNX Runtime 정적 링크 → `fastembed` crate (Phase 62)에서 이미 채택 (ort-sys 정적 링크)
- 다국어 지원 → BGE-M3 임베더가 100+ 언어 (한국어 검증 완료)
- 로컬 실행 (Tauri/CLI 단일 바이너리) → 본 프로젝트 핵심 아키텍처

### 결정
**적용 없음**. supertonic 가치는 본 프로젝트 fastembed 채택에 이미 반영. 추가 차용 가치 없음.

향후 시나리오 확장(예: 회의록 녹음 → STT → 파이프라인 인입)이 발생할 때만 재검토.

---

## 2. wikidocs/352523 분석 — 2026 에이전트 하네스 트렌드

### 핵심 권고 (4대 흐름)

1. **자기 진화 하네스 (Meta-Harness)** — 실패 로그 분석 → 자체 하네스 수정
2. **스킬 생태계** — 도구 호출 → 재사용 학습 가능 스킬, 검색·조합·검증이 새 과제
3. **컨텍스트/메모리 외재화** — 프롬프트 → 파일시스템·메모리저장소·샌드박스
4. **자기 진화 시스템** — 스킬 생성·컨텍스트 전략 변경·워크플로우 재구성

### 본 프로젝트와의 매핑

| 권고 | 본 프로젝트 구현 | 상태 |
|------|------------------|------|
| 실패 로그 분석 → 자체 수정 | `auto_suggester.rs` (C1, Phase 80~84) — 카운터 분석 → decision_log → 사용자 확인 후 자동 toml 수정 | **기적용** |
| 컨텍스트 외재화 (파일시스템) | `settings.db` + `pipeline.toml` + `originals/` 압축 보존 | **기적용** |
| 컨텍스트 외재화 (메모리저장소) | `LocalVectorStore` + `keyword_index` + `find_related` | **기적용** |
| 스킬 검색·조합 | `setup_modules.rs` 동작 모듈 12종 + `RecommendationEngine` (Phase 76/80) | **기적용** |
| 자기 진화 트리거 | `config_snapshot.rs` + 자동 롤백 4트리거 (Phase 77) | **기적용** |

### 직접 적용 부적합 항목

| 권고 | 사유 |
|------|------|
| 스킬 마켓플레이스 (수십만 스킬 공유) | 본 프로젝트는 단일 사용자 데스크톱 도구. 마켓 시나리오 없음 |
| LLM-in-Sandbox (가상 컴퓨터 환경) | 본 프로젝트는 호스트 직접 실행. 샌드박스 분리 가치 낮음 |
| 메타 하네스 자체 수정 | 본 프로젝트는 파이프라인이고 에이전트 아님. 자동 코드 수정 위험 |

### 결정
**기적용** — Ruflo 영감 작업(Phase 73~84)이 이미 핵심 흐름 차용. **추가 도입 가치 낮음**.

만약 본 프로젝트가 향후 에이전트 형태로 확장된다면(예: claude_cli 호출 결과로 자기 룰 학습), 재검토.

---

## 3. wikidocs/353407 분석 — 정리와 감사 흐름

### 핵심 권고

**3단계 점검**:
1. **형식 점검 (Linting)** — 5필드: 한 문장 요약 + 근거 + 확인됨 + 확인 필요 + 다시 물어볼 질문
2. **근거 점검 (Evidence)** — 원천 자료 회귀 / 강한 주장 약화 / 상충 정보 보류
3. **구조 점검 (Graph)** — 임베딩 유사성 / 연결성 / 양방향 매핑

**주기**:
- 매일 — 새 문서 색인 연결
- 주 1회 — 중복·미연결 문서
- 월 1회 — 오래된 문서·상충 정보
- 산출물 전 — 근거 없는 주장 확인

### 본 프로젝트와의 매핑

| 권고 | 본 프로젝트 구현 | 상태 |
|------|------------------|------|
| 형식 1: 한 문장 요약 | `Metadata.summary` (LLM 가공) | **기적용** |
| 형식 2: 근거 | `Metadata.search_hints` + sections | **기적용** |
| 형식 3: 확인됨 | `verification.thresholds` 6종 + quarantine 분기 | **기적용** |
| 형식 4: 확인 필요 | `Metadata.needs_verification` (Phase 87 A-1) | **Phase 87 인프라 추가** |
| 형식 5: 다시 물어볼 질문 | `Metadata.open_questions` (Phase 87 A-1) | **Phase 87 인프라 추가** |
| 근거: 원천 자료 회귀 | `originals/` 압축 보존 + `storage.read_header` | **기적용** |
| 근거: 환각 탐지 | Phase 38 verification (키워드 커버리지 + ROUGE-L) | **기적용** |
| 근거: 강한 주장 약화 | `detect_strong_claims()` (Phase 87 A-2) | **Phase 87 인프라 추가** |
| 구조: 임베딩 유사성 | `LocalVectorStore` + fastembed BGE-M3 (MRR 0.975) | **기적용** |
| 구조: 연결성 분석 | `find_related` + `kg_neighbors` + relations 5종 origin | **기적용** |
| 구조: 양방향 매핑 | `References` + `ReferencedBy` (Phase 47) | **기적용** |
| 주기: 매일/주1회/월1회 | `lint_interval_hours` + `lint_weekly_hours` + `lint_monthly_hours` (Phase 87 A-3) | **Phase 87 인프라 추가** |
| 자기 진화: 사용자 승인 | Ruflo C1 `auto_suggester` + `decision_log` | **기적용** |

### Phase 87 적용 결과

3건 적용 (인프라만):
- A-1: `Metadata.needs_verification` + `Metadata.open_questions` 필드
- A-2: `detect_strong_claims()` 함수 (12종 단정 표현 마커)
- A-3: `lint_weekly_hours` (168) + `lint_monthly_hours` (720) 필드

### 보류 1건

**A-4: 수집·점검 분리** — wikidocs는 "수집일은 빠르게, 점검일은 천천히" 권고. 본 프로젝트는 가공+검증이 통합 사이클(2-Pass 피드백). 위키 운영용 권고지 자동 파이프라인엔 과한 분리.

향후 사용자가 "가공만 빠르게 → 별도 검증 사이클" 요구 시 재검토.

### 결정
**부분 적용 완료** (Phase 87, 3/4). 호출 연결은 Phase 88(부분) 진행 중.

---

## 4. 메타 관찰

### 본 프로젝트 성숙도 재확인

3 문서 분석 결과 — **wikidocs 353407 권고 ~90% 이미 구현**. 나머지 10%는 Phase 87에서 적용. 외부 best practice와의 정합성 확인됨.

### 외부 분석 도입 3단계 (메타 룰, lesson 40)

1. **필드/구조 추가** (Phase 87) — 위험 낮음, 호환성 우선
2. **로직 채우기** — LLM 프롬프트 또는 lint 룰이 필드 활용
3. **UI 노출** — 사용자 검토 가능

본 phase는 1단계만. 2~3단계는 Phase 88+에서.

### 외부 출처 추적 (lesson 40 메타 룰)

코드 주석에 `Phase 87 wikidocs 353407` 명시. 향후 권고 갱신 시 추적 가능 + 권고와 구현의 차이(우리는 Vec 반환 vs 권고 표현 약화 자동 수정)를 명확히 함.

---

## 5. 다음 분석 시점 가이드

본 문서가 다음 외부 분석 시점에 갱신 트리거:
- supertonic이 RAG/검색 기능을 추가하는 메이저 업데이트 시
- wikidocs 352523/353407 신규 챕터 또는 권고 갱신 시
- 본 프로젝트가 에이전트형으로 전환(자동 코드 수정 등) 시
- 새로운 외부 best practice 문서가 본 프로젝트와 직접 연관될 때

각 트리거 시 본 문서를 인용하면서 **새 결정만 추가**, 기존 결정 반복 금지.
