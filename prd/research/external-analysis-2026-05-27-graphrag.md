---
created: 2026-05-27
purpose: 외부 프로젝트 분석 단일 진실원 — AWS GraphRAG Toolkit (Apache-2.0)
predecessor: prd/research/external-analysis-2026-05-22.md (JAMES + Mirage), tfm-tabpfn-analysis.md
related: spec/lesson-learned/62_phase103-graphrag-absorption-batch.md
trigger: Phase 103 GraphRAG 흡수 4건 묶음 + 메타 룰 21 정식 승격 (3건 누적 도달)
---

# 외부 프로젝트 분석 — AWS GraphRAG Toolkit

## 1. 메타데이터

| 항목 | 값 |
|------|-----|
| **저장소** | https://github.com/awslabs/graphrag-toolkit |
| **블로그 (한글)** | https://aws.amazon.com/ko/blogs/tech/introducing-the-graphrag-toolkit-03/ |
| **문서 사이트** | https://awslabs.github.io/graphrag-toolkit/ |
| **라이선스** | Apache-2.0 (상업 자유) |
| **언어 비율** | Python 88.2% / MDX 10.8% |
| **활성도 (2026-04)** | 1,328 commits / 85 releases / 398 stars / 88 forks |
| **본질 도메인** | 엔터프라이즈 RAG (AWS 클라우드) |
| **메타 룰 16 차원 B** | 🟡 부분 매칭 (Statement 3계층은 부수, AWS 인프라는 🔴) |
| **메타 룰 20 분류** | 외부 프로젝트 도메인 가정 정렬 — **본질 불일치, 부수 영역만 흡수** |
| **메타 룰 21 분류** | 외부 도메인 도구 흡수 본질/부수 분리 — **3건 누적 도달** (TFM + Mirage + 본 분석) |

## 2. 기술 스택

### 핵심 노드 3계층 (Lexical Graph)
- **Statement** — FM에 반환되는 사실 단위 (가장 세밀)
- **Topic** — 동일 문서 내 관련 문장 로컬 연결
- **Fact** — 여러 문서 간 동일 사실 글로벌 연결

### 엣지
- LLM 추출 의미 관계 ("사용한다", "제휴하다" 등 자유 동사)

### 백엔드
| 영역 | 지원 |
|------|------|
| 그래프 DB | Neptune / Neo4j / FalkorDB (3종) |
| 벡터 DB | Neptune / OpenSearch / Postgres / S3 Vectors (4종) |
| LLM | Bedrock (Claude / Titan) + LlamaIndex 통합 |

### 검색 알고리즘
- **TraversalBasedRetriever**: 모든 경로 체계 탐색 (ChunkBasedSearch / EntityBasedSearch / EntityNetworkSearch)
- **SemanticGuidedRetriever**: 유망한 경로 선택 탐색 (빔 검색 + TF-IDF 재순위 + 다양성 필터)

### 저장소 구조 (모노레포)
```
graphrag-toolkit/
├── lexical-graph/             핵심 그래프 추출 (PyPI: graphrag-lexical-graph)
├── byokg-rag/                 KG 질의응답 (PyPI: graphrag-toolkit-byokg-rag)
├── lexical-graph-contrib/
│   └── falkordb/              백엔드 확장 패턴 (단일 진실원 + contrib)
├── examples/                  Workshop 노트북
├── docs-site/                 awslabs.github.io
└── integration-tests/         테스트
```

## 3. 본 프로젝트와 비교

### 본질 도메인 가정 차이 (메타 룰 20)

| 차원 | file-pipeline | GraphRAG |
|------|--------|-----------|
| 사용자 | 단일 사용자 데스크톱 | 엔터프라이즈 클라우드 |
| 운영 환경 | Windows 단일 바이너리 21MB | AWS Neptune + OpenSearch |
| 데이터 위치 | 로컬 inbox/ | 클라우드 저장소 |
| 비용 | 0원 (LLM API 제외) | Neptune + OpenSearch 이중 운영 |
| 보안 | PII 5종 + 사용자 정의 격리 | (미언급) |
| 인터페이스 | MCP 25 도구 + Tauri GUI 7탭 | 코드/노트북 |

### 본 프로젝트 KG 구조 (현행)

| 영역 | 현황 |
|------|------|
| 노드 계층 | 문서 단위 (Document 1계층) |
| RelationType | 5종 (Supersedes/Updates/RelatedTopic/References/ReferencedBy) |
| RelationOrigin | 5종 (AutoSimilarity/UserWikilink/LlmExtracted/UserManual/LintAutoFix) |
| Entity 종류 | 7종 (person/organization/place/technology/amount/project/concept) |
| 검색 | Dense + Sparse(BM25) RRF + 리랭킹 + KG 1-hop (디폴트 비활성) |

## 4. 흡수 후보 평가 (메타 룰 16 차원 B 라벨)

### 🟢 즉시 흡수 (본질/부수 일치)

| ID | 후보 | 매핑 | Phase 103 처리 |
|----|------|------|----------|
| **G4** | TF-IDF 재순위 (검색 다양성) | 검색 후처리 단계에 TF-IDF 필터 추가. fastembed reranker와 결합 | ✅ **즉시 구현** (검증된 즉시 측정 가능) |

### 🟡 인프라 선구현 (lesson 30 패턴, 디폴트 비활성)

| ID | 후보 | 매핑 | Phase 103 처리 |
|----|------|------|----------|
| **G1** | Statement 노드 추출 | Metadata.statements 신규 + prompts.toml `statements` 필드 + needs_verification 결합 | ✅ **인프라 선구현** (디폴트 비활성, 측정 후 활성화 트리거) |
| **G2** | 의미 관계 LLM 추출 | RelationType `Semantic { verb }` 추가 + prompts.toml `semantic_relations` 필드 | ✅ **인프라 선구현** (디폴트 비활성) |
| **G3** | Multi-hop 빔 검색 | 기존 A2 KG hop 인프라 확장. SearchConfig.kg_beam_search 추가 | ✅ **인프라 선구현** (디폴트 비활성) |

### 🔴 영구 보류 (도메인 불일치)

| 항목 | 사유 |
|------|------|
| Neptune / Neo4j / FalkorDB | 외부 인프라 의존 (Phase 44 Qdrant 제거 결정 자기 적용) |
| OpenSearch / Postgres / S3 Vectors | 동일 사유 |
| Bedrock LLM 전용 | LLM 어댑터 5종으로 이미 충족 |
| LlamaIndex 의존 | Python 의존 + Rust 단일 바이너리 정책 |
| AWS boto3 SDK | 클라우드 종속 |
| 모노레포 PyPI 패키지화 | 단일 사용자 정책. 다만 file-pipeline 형제 module 10종 패턴은 contrib 유사 |
| Topic 계층 노드 | doc_types 17종 + Phase 61 hierarchy 메타로 부분 충족, 중복 위험 |

## 5. 메타 룰 적용

| 룰 | 적용 |
|----|------|
| 메타 룰 8 (사전 grep) | ✅ RelationType 5종 + Metadata 필드 + SearchConfig 사전 grep 완료 |
| 메타 룰 9 (외부 문서 권고 3단계) | ✅ 본 문서가 단일 진실원 (필드/구조 → 로직 → UI 노출) |
| 메타 룰 16 차원 B | ✅ 🟢/🟡/🔴 라벨 부착 |
| 메타 룰 18 (추정 재검증) | ✅ 1차 WebFetch 모호 → docs site + GitHub 직접 재확인 |
| 메타 룰 20 (도메인 정렬) | ✅ 본질 불일치 식별 + 부수 흡수만 결정 |
| 메타 룰 21 (본질/부수 분리) | ✅ 3건 누적 도달 → Phase 103 정식 승격 |
| 메타 룰 22 (사용자 정책 합의) | ✅ 사용자 명시 "흡수 가치 종합 목록 전체 구현" 합의 |
| 메타 룰 23 (승격 기준) | ✅ 메타 룰 21 3요소 모두 충족 |
| 메타 룰 25 (자기 적용 의무) | ✅ 메타 룰 21 정식 승격 직후 본 분석에 즉시 자기 적용 |
| 메타 룰 30 (외부 문서 흡수 정형화 — 본 phase 신규 후보 가능) | (검토 대기) |

## 6. 본 프로젝트 우위 영역 (보존)

| 영역 | 본 프로젝트 우위 |
|------|----------------|
| 단일 바이너리 | 21MB vs AWS 인프라 의존 |
| 외부 인프라 0건 | LocalVectorStore 인프로세스 |
| MCP 인터페이스 | 25 도구 (Phase 102 optimize 신규) |
| Tauri GUI | 7탭 + 5 운영 카드 (Phase 100 IA) |
| 메타 작업 정형화 | 정식 메타 룰 23 + 후보 5 |
| 회귀 게이트 자동화 | 7+1종 (Phase 97 + 98) |
| PII / 보안 | C2 + A2 출력 마스킹 |
| 한국어 | prompts.toml 한국어 콘텐츠 |
| 세션 컨텍스트 복원 | CLAUDE.md + 61 lesson |

## 7. 트리거 매핑 (G1~G4 활성화 조건)

| 후보 | 트리거 | 비고 |
|------|--------|------|
| G4 TF-IDF | ✅ Phase 103 즉시 측정 가능 | 사용자 검색 30회+ 후 MRR before/after |
| G1 Statement | 가공 50파일+ + needs_verification 누적 5건+ | 측정 후 디폴트 활성화 검토 |
| G2 의미 관계 | KG 관계 평균 <2 + 도메인 다양성 확보 | 트리거 #G2 등록 |
| G3 Multi-hop | A2 활성화(expand_kg_hops > 0) + 실 사용자 만족도 신호 | 트리거 #G3 등록 |

## 8. 결정 요약

- **즉시 흡수**: G4 (검증된 알고리즘 + 자동 측정 가능)
- **인프라 선구현 (디폴트 비활성, lesson 30 패턴)**: G1 / G2 / G3
- **영구 보류**: AWS 인프라 7건 / LlamaIndex / Python 의존 / Topic 계층 / 모노레포 패키징
- **메타 룰 21 정식 승격**: 누적 3건 도달
- **사용자 우위 영역**: 단일 바이너리 / MCP / GUI / PII / 한국어 / 메타 작업 — 보존
