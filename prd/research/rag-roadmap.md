---
created: 2026-04-29
status: 통합 — file-pipeline RAG 고도화 단일 진입점
supersedes:
  - rag-enhancement-plan.md (Phase 51-52 완료, 흡수)
  - embedding-comparison-improvements.md (P0/P1 완료 + 트리거 대기, 흡수)
  - embedding-enhancement-benchmark.md (50항목 분석, 핵심만 흡수)
  - smart-chunking-rag.md (G1~G7 갭 분석, 흡수)
  - bge-m3-value-review.md (트리거 대기 #3 검토, 흡수)
  - crossref-deep-analysis.md (Phase 47-52 완료, 참조만)
  - crossref-optimization.md (Phase 46-49 완료, 참조만)
---

# file-pipeline RAG 고도화 통합 로드맵

> 본 문서는 6건의 RAG 관련 research 자료를 단일 진입점으로 통합한 것. 검색·임베딩·청킹·교차참조 4축의 현재 상태와 향후 방향을 한 곳에서 관리.

## 1. 4축 현재 상태 (2026-04-29)

### 축 1: 임베딩

| 항목 | 현재 | 측정 |
|------|------|------|
| 기본 임베더 | Claude CLI 128축 의미 벡터 | MRR ~0.65 |
| Fallback | HashEmbedder (키워드 해시) | MRR 0.525 (단위 테스트) |
| **Phase 62 도입 예정** | **fastembed BGE-M3 (순수 Rust)** | **MRR 0.975 (PythonOnnx 실측 동일 기대)** |
| Sparse | keyword_index + search_hints + summary | BM25. fastembed Sparse로 대체 검토 |

**결정 사항** (전문가 자문 2026-04-29):
- 옵션 A/B/C(Python subprocess) 모두 폐기
- **fastembed v5.12.0 채택** — pykeio/ort 기반, Tokio 의존 없음, Dense+Sparse+Reranker 통합
- Python 의존 완전 제거 → Tauri 단일 바이너리 UX 보존
- 상세: `bge-m3-fastembed-decision.md`

### 축 2: 청킹

| 항목 | 현재 |
|------|------|
| 기본 분할 | `SemanticChunkConfig` (target_bytes 1500, overlap 2문장, 코드펜스 보존) |
| H1/H2/H3 인식 | `split_by_headings` ✅ |
| 단락 강제 분할 | `split_section_by_paragraphs` ✅ |
| 대용량 LLM 입력 | `split_into_chunks` (40KB) + `module-llm-chunked::ByteSplitter` |
| 미구현 갭 | G1 계층 메타데이터 부착 / G2 표 보존 / G3 Parent-Child / G4 PDF→MD / G7 인덱싱 표준화 |

**결정 사항**:
- G1+G7 묶음을 Phase 61로 즉시 진행
- G2/G3/G4는 도메인/규모 트리거 대기

### 축 3: 검색

| 단계 | 구현 |
|------|------|
| Dense 검색 | LocalVectorStore cosine + HNSW(instant-distance) |
| Sparse 검색 | keyword_index BM25 |
| 결합 | RRF (가중치 정적, 동적 조정 트리거 대기) |
| 시간 가중 | recent 모드 10% boost |
| 메타데이터 필터 | doc_type + date |
| 다양성 | MMR ✅ |
| 리랭킹 | ClaudeReranker (LLM 점수 0~10) |
| 검색 모드 | default/exact/related/recent/fusion (다중 쿼리 RRF) |
| 검색 후 처리 | Sentence Window (query 매칭 ±5줄) + CRAG (신뢰도 3단계 + 보완 검색) |

**결정 사항**:
- Cross-Encoder 리랭커는 BGE-M3 ONNX 도입 후 (트리거 대기 #9)
- HyDE 폴백은 실사용 "검색 안 됨" 피드백 후 (트리거 대기 #6)

### 축 4: 교차참조 (지식 그래프)

| 항목 | 현재 |
|------|------|
| 관계 유형 | 5종 (Supersedes, Updates, RelatedTopic, References, ReferencedBy) |
| 후보 필터 | MinHash LSH (옵션) + 메타데이터 블로킹 (옵션) |
| 임계값 | threshold 기반 전체 스캔 (cap_supersedes/updates/related/references/incoming) |
| 비동기 처리 | `flush_crossref` (EmbeddingSnapshot 1회 로드 + 인라인 cosine, 30초 유휴 트리거) |
| 성능 | 1K문서 105초, 59K 관계 (HashEmbedder 기준) |

**결정 사항**:
- threshold 디폴트 0.7→0.8 상향은 실 임베딩 검증 후 (트리거 대기 #1)
- MinHash 자동 활성 임계치 3K는 5K+ 코퍼스 도달 시 검토 (트리거 대기 #2)

---

## 2. 통합 트리거 대기 표

기존 5건 + 신규 4건 + 분리 1건 = **9건**.

| # | 항목 | 트리거 조건 | 준비 상태 | 예상 비용 |
|---|------|------------|----------|----------|
| 1 | threshold 디폴트 상향 (0.7→0.8) | 실 임베딩 "관계 노이즈" 피드백 | Phase 59 옵션화 완료. 디폴트 변경만 대기 | 10분 |
| 2 | MinHash 자동 활성 임계치 조정 | 5K+ 문서 도달 시 부하 측정 | Phase 59 force/min_docs 노출. 강제 활성 가능 | 30분 |
| ~~3a~~ | ~~BGE-M3 Python production~~ | ❌ 폐기 — Phase 62 fastembed 채택으로 대체 | (전문가 자문 결과) | — |
| ~~3b~~ | ~~BGE-M3 Rust 네이티브 (ort 대기)~~ | ❌ 폐기 — fastembed가 ort 기반이므로 즉시 가능 | (전문가 자문 결과) | — |
| ~~3c~~ | ~~BGE-M3 Sparse + Cross-Encoder~~ | ✅ Phase 62에 흡수 — fastembed 동시 제공 | RerankerModel::BGERerankerV2M3 내장 | (Phase 62 포함) |
| 4 | 메타데이터 블로킹 디폴트 활성 | 검색 정확도 불만 + 코퍼스 다양성 검증 | Phase 59 옵션화. HashEmbedder 100문서: 효과 0% | 30분 |
| 6 | HyDE 폴백 검색 | 실사용 "검색 안 됨" 피드백 | LLMPort.summarize_text 재사용 가능 | 1일 |
| 7 | Parent-Child 청크 구조 | 1K+ 코퍼스 MRR 회귀 | 스키마 변경 + mmap 영향 큼 | 3일 |
| 8 | 표 마크다운 보존 청킹 | 표 비중 높은 도메인 진입 | preserve_code_blocks 패턴 재사용 | 2일 |

(기존 5번 ColBERT는 BGE-M3 3c와 통합되어 별도 항목 제거)

---

## 3. 즉시 착수 후보 — Phase 61

### Phase 61: 청킹 메타데이터 고도화 (G1 + G7)

목적: 원문 스마트 청킹 자료의 ① 계층적 청킹 + ⑦ 인덱싱 메타데이터 표준화 통합 적용. 비용 낮음(2일), 호환성 높음.

**구현 항목**:
1. `SemanticChunk`에 `title_path: Vec<String>` 추가 (H1>H2>H3 경로)
2. `split_by_headings`가 path를 추적하도록 수정
3. `Metadata`에 `hierarchy: Option<Vec<String>>`, `content_type: String` 추가 (lesson 5: 타입 변경 없이 필드 추가)
4. 인덱싱 시 hierarchy를 sparse vector(키워드)에도 추가 → 검색 시 제목 매칭 향상
5. Documents 탭에서 hierarchy 표시 (검색 결과 리스트)
6. `bench_search_curve` 회귀 검증 (MRR@5, Recall@10 회귀 0% 확인)
7. 마이그레이션: 기존 인덱스는 hierarchy 없이 운영 + 재인덱싱 시 자동 부착

**선행**: 없음 (Phase 60 완료 상태)
**후행**: Phase 62 BGE-M3 (3a) 또는 트리거 대기

---

## 4. 권장 구현 순서

### A안 (기본 권장)

```
Phase 61 (청킹 메타데이터, 2일) → Phase 62 (BGE-M3 Python A, 1~2일) → 트리거 대기
```

근거:
- 청킹 메타데이터는 외부 의존 0, 즉시 진행 가능
- BGE-M3 Phase A는 사용자 결정만 받으면 진행 가능
- 두 작업은 독립적이고 충돌 없음

### B안 (효과 우선)

```
Phase 62 (BGE-M3 Python A, 1~2일) → Phase 61 (청킹 메타데이터, 2일) → 트리거 대기
```

근거:
- BGE-M3가 MRR +50% 효과 (청킹 +5~10%보다 큰 격차)
- 큰 효과를 먼저 보고 청킹 메타데이터의 추가 효과 측정

**대답 권고**: A안 — 청킹 작업은 외부 의존 없으므로 사용자 의사결정 없이 즉시 시작 가능. BGE-M3는 Python 환경 부담 결정이 필요하므로 사용자 컨펌 후 진행.

---

## 5. 완료된 분석 자료 (참조용)

다음 자료들은 **Phase 51-52에서 구현 완료**되어 본 통합 문서로 흡수됨. 원문은 참조용으로 유지되나 갱신은 본 문서 우선:

| 원문 | 흡수 방향 |
|------|----------|
| `rag-enhancement-plan.md` (25방안 매트릭스) | §3 검색 축 완료 항목으로 흡수 |
| `embedding-comparison-improvements.md` (MCP 관점 우선순위) | §1 임베딩 축 결정 사항으로 흡수 |
| `embedding-enhancement-benchmark.md` (50항목 분석) | P0/P1 완료, 트리거 대기 §2로 흡수 |
| `smart-chunking-rag.md` (G1~G7 갭 분석) | Phase 61 항목 §3으로 흡수 |
| `bge-m3-value-review.md` (트리거 대기 #3 검토) | 트리거 대기 #3a/3b/3c §2로 분리 |

### 교차참조 분석 자료

| 원문 | 상태 |
|------|------|
| `crossref-deep-analysis.md` (20개 최적화 기법) | Phase 47~52에서 핵심 3가지(HNSW, mmap+행렬곱, 배치 커밋) 적용 완료. 70초 → 4.5초(flush 77% 개선) |
| `crossref-optimization.md` (50항목 카테고리) | Phase 47~49 적용. 재검토 트리거: 5K+ 문서 도달 |

---

## 6. 본 문서의 갱신 트리거

다음 시점에 본 문서를 갱신:
- 트리거 대기 항목 착수/완료 시
- 실측 MRR/Recall 회귀 발견 시
- 외부 의존(ort, BGE-M3 모델) 상태 변화 시
- 신규 RAG 자료 도입 시 (handsoff 또는 외부 소스)

본 문서가 단일 진입점이며, 신규 RAG 연구는 본 문서에 직접 추가하거나 별도 파일 작성 후 즉시 흡수.
