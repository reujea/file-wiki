---
created: 2026-04-29
status: 결정 — fastembed 채택. 옵션 A/B/C 폐기
supersedes: bge-m3-production-consultation-prompt.md (자문 입력 + 응답 모두 흡수)
---

# BGE-M3 임베더 production 결정 — fastembed 채택

## 결정 요약

전문가 자문 결과 **fastembed 크레이트(v5.12.0) 채택**. 기존 옵션 A(영구 Python 프로세스) / B(배치 호출) / C(POC 격상)는 모두 폐기.

### 핵심 근거

| 항목 | 근거 |
|------|------|
| Python 의존 완전 제거 | Tauri 단일 바이너리 UX 보존 |
| 임시 다리 → 영구 솔루션 | fastembed 자체가 ort 기반이므로 ort 정식 릴리스 대기 불필요 |
| 모델 로드 1회 + 추론 0.05~0.3초/건 | MCP 검색 응답 SLA 충족 |
| Dense + Sparse + Reranker 통합 | 한 크레이트로 하이브리드 검색 + Cross-Encoder 리랭커 모두 |
| crates.io 209K+ 다운로드/월 | 운영 안정성 검증됨 |

## fastembed 크레이트 정보

| 항목 | 값 |
|------|-----|
| 버전 | v5.12.0 |
| 라이선스 | Apache 2.0 |
| 내부 의존 | `pykeio/ort` (ONNX Runtime Rust 래퍼) + `huggingface/tokenizers` |
| Tokio 의존 | 없음 (동기 사용 지원) |
| DirectML | 지원 (자동 메모리 패턴 최적화) |

### 지원 모델

| 모델 | enum | 용도 |
|------|------|------|
| `EmbeddingModel::BGEM3` | Dense 임베딩 | 메인 검색 임베더 (1024차원) |
| `SparseModel::BGEM3` | Sparse 임베딩 | 하이브리드 검색 (BM25 대체 가능) |
| `RerankerModel::BGERerankerV2M3` | Cross-Encoder 리랭커 | 검색 결과 재정렬 |

## 기존 트리거 대기 영향

| 기존 항목 | 처리 |
|----------|------|
| #3a BGE-M3 Python production | ❌ 폐기 — fastembed가 Python 우회 |
| #3b BGE-M3 Rust 네이티브 (ort 정식 릴리스 대기) | ❌ 폐기 — fastembed가 ort 기반이므로 이미 가능 |
| #3c BGE-M3 Sparse + Cross-Encoder | ✅ Phase 62에 흡수 (fastembed 동시 제공) |
| #9 Cross-Encoder 리랭커 (트리거 대기) | ✅ Phase 62에 흡수 (Q3 결정 — ClaudeReranker 교체) |

## Phase 62 정의

**제목**: fastembed 기반 BGE-M3 임베더 + Cross-Encoder 리랭커 통합 도입

**선행**: Phase 60 완료 (module 분리)
**후행**: Phase 63 청킹 메타데이터 (G1+G7) 또는 트리거 대기

**작업 분해**:
1. 사전 검증 (DLL/벡터 동일성/속도/메모리 4항목)
2. `FastEmbedAdapter` 구현 (`EmbeddingPort` 신규 어댑터)
3. `FastEmbedReranker` 구현 (`RerankerPort`, ClaudeReranker 교체)
4. `build_service` 통합 + `EmbeddingConfig` 옵션
5. Settings UI 임베더/리랭커 선택지
6. spec 갱신 + 메모리

**기대 효과**:
- MRR@5: 0.65 (Claude CLI) → **0.975 (fastembed BGE-M3)** = +50%
- 임베딩 속도: 15초/건 → **0.05~0.3초/건** = 50~300배
- 1K문서 초기 투입: 4시간 → 5분
- Cross-Encoder 리랭커: ClaudeReranker(LLM API 호출) → fastembed BGE-Reranker-v2-M3 (로컬 ms 단위)

## 검증 필요 항목 (사전)

자문에서 명시한 4항목을 임시 크레이트로 격리 검증:

| # | 항목 | 방법 | 합격 기준 |
|---|------|------|----------|
| 1 | DLL 크래시 재현 여부 | fastembed + CPU EP 단건 임베딩 | 크래시 없음 |
| 2 | 벡터 동일성 | PythonOnnx POC vs fastembed (동일 모델, 동일 입력) | cosine similarity ≥ 0.99 |
| 3 | 속도 실측 | 단건 + 배치 10/50/100건 | 단건 ≤ 0.5초, 배치 평균 ≤ 0.1초/건 |
| 4 | 메모리 RSS | BGE-M3 모델 로드 후 측정 | 합리적 범위 (4GB 이내) |

## 모델 파일 관리

- **자동 다운로드**: 첫 사용 시 HuggingFace에서 fetch (기본 캐시 `%LOCALAPPDATA%/fastembed/`)
- **오프라인 옵션**: `try_new_from_user_defined()` — vendor에 모델 번들 가능
- **BGE-M3 ONNX 크기**: ~1.1GB [확인 필요]

## 작업 범위 외

- Candle 크로스 비교는 보류 — fastembed 단일 안으로 진행. 만약 사전 검증 1번(DLL 크래시)에서 실패하면 Candle 검토.
- ColBERT late interaction은 별도 트리거 대기 유지.
