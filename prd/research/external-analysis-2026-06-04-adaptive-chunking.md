---
created: 2026-06-04
purpose: Adaptive Chunking (arxiv 2603.25333, LREC 2026) 외부 분석 단일 진실원
external_sources:
  - arxiv: "2603.25333"
  - github: "ekimetrics/adaptive-chunking"
  - license: "MIT (코어), CC BY-NC-SA 4.0 (상호참조 모듈)"
  - publication: "LREC 2026 채택"
  - source_url: "https://discuss.pytorch.kr/t/adaptive-chunking-rag/10478"
related:
  - spec/lesson-learned/META.md (메타 룰 20 누적 5건째, 메타 룰 21 부분 적용)
  - prd/research/external-analysis-2026-05-15.md (wikidocs 353407, 양식 참조)
  - prd/research/external-analysis-2026-05-22.md (Mirage)
  - prd/research/external-analysis-2026-05-27-graphrag.md (GraphRAG)
meta_rule_label:
  dimension_a: "🟢 본질 도메인 일치 (RAG 청킹)"
  dimension_b: "🟢 본질 도메인 일치 — 즉시 흡수 검토"
status: "Phase A 측정 인프라 흡수 완료 (2026-06-04 본 세션), Phase B 전략 인프라 진입"
---

# Adaptive Chunking (arxiv 2603.25333) 흡수 분석

## 0. 본 문서 위상

본 문서는 메타 룰 9 "외부 출처 단일 진실원" 자기 적용. Adaptive Chunking 흡수 결정·기준·진척의 단일 진실원이며, 본 외부 자료를 인용하는 lesson/spec/코드 주석은 본 파일 링크 (`prd/research/external-analysis-2026-06-04-adaptive-chunking.md`)만 참조한다.

메타 룰 20 누적 5건째 — JAMES v0.3.0 + TFM/TabPFN + Mirage v0.0.1 + GraphRAG에 이은 본질 도메인 일치 외부 분석.

## 1. 본 솔루션 본질 정렬

| 항목 | file-pipeline | Adaptive Chunking |
|------|--------------|--------------------|
| 본질 도메인 | 가공·검색 RAG | 청킹 품질 평가 + 전략 선택 |
| 단위 | 청크 (SemanticChunk) | 청크 (논문 §3) |
| 임베딩 | BGE-M3 fastembed | 임의 (논문은 sentence-transformers 기반) |
| 환경 | Rust 단일 바이너리 / 데스크톱 / 단일 사용자 | Python ekimetrics 패키지 |

**판정**: 🟢 본질 도메인 일치 (메타 룰 16 차원 A + B). 메타 룰 20 분류 (메타 룰 21이 아님 — 도메인 불일치 외부 도구가 아니라 같은 도메인 솔루션).

## 2. 핵심 알고리즘 — 5 내재 지표

| 지표 | 정의 | file-pipeline 매핑 | 상태 |
|------|------|-------------------|------|
| **RC** Reference Completeness | 개체-대명사 같은 청크 내 비율 | 한국어 coref 도구 미보유 | 🔴 보류 (Maverick = 영어 전용) |
| **BI** Block Integrity | 표/그림/코드 펜스 무결 유지 (τ=5자) | Phase 86 #8 트리거 (preserve_tables) 본질 일치 | ✅ Phase A 흡수 |
| **ICC** Intrachunk Cohesion | 청크 내 문장-청크 임베딩 평균 코사인 | BGE-M3 임베딩 재사용 | ✅ Phase A 흡수 |
| **DCC** Document Contextual Coherence | 청크 ↔ 3000토큰 윈도우 코사인 | BGE-M3 임베딩 재사용 | ✅ Phase A 흡수 |
| **SC** Size Compliance | 100~1100 토큰 범위 비율 | 토큰 추정 휴리스틱 (3자/토큰) | ✅ Phase A 흡수 |

**보류 (RC)**: 한국어 coref는 KLUE / UNCC 모델이 존재하지만, fastembed 통합 비용 + 본 솔루션 비전(Rust 단일 바이너리)과 정렬 어려움. 트리거: 다국어 coref 외부 모듈(`module-coref`) 도달 시 재평가.

## 3. 성능 측정 (논문 §4)

LangChain RecursiveCharacterTextSplitter 디폴트 대비:

| 지표 | baseline | Adaptive | Δ |
|------|---------|---------|---|
| 검색 완전성 (Retrieval Completeness) | 58.08% | 67.68% | +9.60pp |
| 답변 정확도 (Answer Accuracy) | 70.11% | 78.01% | +7.90pp |
| 답변 가능 질문 수 | 49 | 65 | +32.7% |

논문 핵심 발견 — 내재 지표의 미세 개선 (0.4~2.4pp)이 RAG 종단 성능에서 **8~10pp로 증폭**.

본 솔루션 적용 가치 평가:
- 현재 디폴트 청킹 (`SemanticChunkConfig::default`, target_bytes=1500)이 BI/ICC/DCC 어디에 위치하는지 미측정
- Phase A 인프라로 baseline 측정 가능
- 50파일+ 가공 후 baseline 도달 시 Phase B/C 전략 선택 진입 가치 명확화

## 4. 흡수 결정 (메타 룰 16 차원 A/B + 메타 룰 20)

### 차원 A (자동 측정 가능성)

| 영역 | 라벨 | 결정 |
|------|------|------|
| SC 측정 | 🟢 자동 측정 가능 | ✅ Phase A 흡수 |
| BI 측정 | 🟢 자동 측정 가능 | ✅ Phase A 흡수 |
| ICC 측정 | 🟢 임베딩 재사용 자동 | ✅ Phase A 흡수 |
| DCC 측정 | 🟢 임베딩 재사용 자동 | ✅ Phase A 흡수 |
| 50파일+ baseline | 🟡 사용자 코퍼스 의존 | B-1/B-2/B-9 트리거 묶음 |
| 전략 선택 효과 검증 | 🟡 코퍼스 의존 | Phase C 진입 전 baseline 필수 |
| 종단 정확도 +8pp 재현 | 🔴 사용자 만족도 의존 | Phase D UI 가시화 후 사용자 신호 |

### 차원 B (외부 솔루션 추상화 매칭)

| 영역 | 라벨 | 결정 |
|------|------|------|
| 4 지표 계산 함수 | 🟢 추상화 매칭 + module 위임 가능 | core/domain 직접 구현 (도메인 단순) |
| Adaptive 본체 알고리즘 | 🟡 부분 일치 + 전략 enum 분기 | Phase B에서 ChunkingStrategy enum |
| Maverick coref 모듈 | 🔴 한국어 미지원 + 라이선스(CC BY-NC-SA 4.0) | 명시 보류 |

### 메타 룰 20 분류 (본질 도메인 정렬)

| 흡수 항목 | 라벨 | 위치 |
|---------|------|------|
| BI 측정 / ICC / DCC / SC | 🟢 본질 일치 | `core/domain/chunking_quality.rs` (Phase A) |
| ChunkingStrategy enum 추상화 | 🟢 본질 일치 | `core/domain/chunking.rs` (Phase B) |
| Adaptive 전략 본체 | 🟢 본질 일치 | Phase C 진입 (baseline 후) |
| UI 가시화 (Verification 카드) | 🟢 본질 일치 | Phase D 진입 (메타 룰 13 4단계) |
| Maverick RC | 🔴 영어 전용 도메인 불일치 | 명시 보류 — coref 모듈 도달 시 재평가 |

## 5. Phase 진행 매트릭스 (lesson 30 인프라 선구현 패턴)

| Phase | 영역 | 위치 | 디폴트 | 상태 |
|-------|------|------|--------|------|
| **A** | 4지표 측정 인프라 | `chunking_quality.rs` + `Metadata.chunk_quality` + `ChunkingConfig.compute_quality` | false | ✅ **2026-06-04 완료** |
| **B** | 전략 enum 인프라 | `ChunkingStrategy` enum + Chunker trait + 4 impl | "semantic" | 🟡 **본 세션 진입** |
| **C** | Adaptive 본체 알고리즘 | `chunking/adaptive.rs` 신규 | "adaptive" 토글 | ⏸ baseline 도달 후 |
| **D** | UI 가시화 | Verification 탭 "📐 청킹 품질" 카드 + Pipeline 인스펙터 chunk 노드 | UI는 항상 표시 | ⏸ Phase C 측정 후 |

## 6. 외부 출처 명시 (메타 룰 9)

본 자료 인용 시 본 문서 링크만 사용:
- 코드 주석: `// Adaptive Chunking (arxiv 2603.25333, prd/research/external-analysis-2026-06-04-adaptive-chunking.md)`
- lesson: 본 문서 링크
- 외부 출처 변경 시 본 문서만 갱신 (다른 위치 갱신 금지)

## 7. 누적 사례 등재 (메타 룰 20)

`spec/lesson-learned/META.md` §메타 룰 20 누적 사례 표에 5번째 추가:

| 프로젝트 | 본질 일치 흡수 (🟢) | 부수 일치 흡수 (🟡) | 불일치 보류 (🔴) |
|---------|-------|-------|-------|
| **Adaptive Chunking (arxiv 2603.25333)** | SC/BI/ICC/DCC 측정 (Phase A) / ChunkingStrategy enum (Phase B) / Adaptive 본체 (Phase C) / UI 가시화 (Phase D) | 토큰 추정 휴리스틱 (정확 토큰화 도달 시 대체) | Maverick RC (영어 전용) |

## 8. 측정 트리거 (B-9 결합)

다음 측정 시점에 Adaptive Chunking 4지표 자동 측정 묶음 진입:

- `external-trigger-checklist.md` B-1/B-2/B-9 50파일+ 가공 세션
- `chunking.compute_quality=true` 일시 활성화 후 baseline 측정
- 결과를 `spec/benchmarks/adaptive_chunking_baseline_<date>.json`에 기록
- baseline 도달 시 Phase C 진입 가치 평가

## 9. 본 분석의 메타 가치

- **본질 도메인 일치 외부 분석 5건 누적** → 메타 룰 20 안정화 (확장 평가 1건째)
- **lesson 30 패턴 재적용** → Phase A/B/C/D 4단계 분리로 위험 0
- **메타 룰 13 4단계 자기 적용** — 인프라(A) → 로직(B) → 측정(C) → UI(D)
- **메타 룰 9 자기 적용** → 외부 출처 단일 진실원 정착
