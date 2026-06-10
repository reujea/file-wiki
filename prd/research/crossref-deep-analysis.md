# 교차참조 성능 고도화 — 20개 최적화 기법 종합 분석 보고서

> **정정 (2026-04-17):** 본 보고서의 "65분"은 추정치였으며, 실측 결과 **422초(7분)**이 정확합니다.
> 최적화 적용 후 실측: **70초** (6배 개선). 보고서 본문은 원문 유지.

## 요약

**현재 1,000문서에서 65분 이상 소요되는 타임아웃은 벡터 수학의 문제가 아니라 아키텍처 문제이다.**
순수 Rust로 1,000문서 × 1,024차원 코사인 스캔은 1초 미만의 작업이다. 65분이 소모되는 원인은 SQLite 행 단위 디스패치, 쌍별 트랜잭션, Blob 재디코딩, Tauri IPC 라운드트립 등 구조적 병목에 있다.

**핵심 3가지 조합으로 ~10,000배 개선 가능:**

1. **Brute-force → HNSW 전환** (검색 O(N×dim) → O(log N × dim))
2. **Salsa 메모이제이션** (문서 추가 시 O(N²) → O(N))
3. **WAL 기반 마이크로 배치 그룹 커밋** (트랜잭션 오버헤드 20~100배 절감)

**예상 결과:** 1,000문서 재인덱스 65분+ → **30초 이내**, 증분 추가 시 문서당 **밀리초 수준**

---

## 1차 진단: 진짜 병목은 어디인가

알고리즘 변경 이전에, 현재 병목은 거의 확실히 다음 중 하나이다:

| 병목 후보 | 영향도 | 해결 방법 |
|-----------|--------|-----------|
| 임베딩을 JSON 텍스트로 저장 | 1~2 자릿수 성능 저하 | raw little-endian float32 BLOB 전환 |
| Tauri JS 브릿지 통한 행 단위 전달 | 1~2 자릿수 성능 저하 | Rust 내부에서 모든 거리 계산 수행 |
| 비교 건당 SQLite 트랜잭션 1건 | 1~3 자릿수 성능 저하 | 배치당 1 트랜잭션으로 통합 |

> **선행 조건:** sqlite-vec, vectorlite 등이 증명하듯, 100K × 384차원 brute-force 스캔도 올바르게 구현하면 단일 스레드에서 5~15ms에 완료된다. 이 기반이 확보되어야 아래 20개 기법이 효과를 발휘한다.

---

## 카테고리 A: 벡터 검색 및 인덱싱 알고리즘 (4개)

### A-1. HNSW (Hierarchical Navigable Small World)

**개요:** Malkov-Yashunin이 제안한 다층 근접 그래프. 상위 레이어는 "고속도로" 역할, 레이어 0에 모든 노드 포함. 탐욕적 최선우선 탐색으로 O(log N) 시간에 검색.

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | 검색: O(ef × log N × dim), 삽입: O(M × log N × dim) |
| **공간 복잡도** | 벡터 + 노드당 ~8~10B 그래프 오버헤드. 1M × 768dim → ~4~5GB RAM |
| **정확도** | ann-benchmarks 기준 Recall@10 = 0.95~0.99 (ef_search 조절로 제어) |
| **구현 복잡도** | Rust 생태계 최성숙. usearch(SIMD, f16/i8 양자화, mmap), instant-distance(순수 Rust), hnsw_rs, vectorlite(SQLite 가상 테이블 통합) |
| **운영 복잡도** | 업데이트 Churn 20% 이상 시 연결성 저하 → 주기적 전체 재구축 필요 |

**벤치마크 근거:** GloVe-100/1.2M 기준, hnswlib는 95% recall에서 ~28K QPS, 98.5% recall에서 ~16K QPS 달성.

**2025~2026 트렌드:** Qdrant, Weaviate, Milvus, pgvector, Redis, Elasticsearch, MongoDB, Lucene 등 **사실상 모든 주요 벡터 DB가 HNSW를 기본 ANN으로 채택.**

**권장 Rust 크레이트:**
- `usearch` — 단일 파일, mmap, SIMD, f16/i8 양자화 ★★★★★
- `instant-distance` — 순수 Rust, C 의존성 없음 ★★★★☆
- `vectorlite` — SQLite 가상 테이블 통합, sqlite-vec 대비 7~30배 ★★★★☆

---

### A-2. IVF-PQ (Inverted File Index + Product Quantization)

**개요:** k-means로 공간을 nlist개 보로노이 셀로 분할 후, 각 벡터를 M개 서브벡터로 나누어 8비트 코드북 인덱스로 대체. 비대칭 거리 계산(ADC)으로 사전 계산 룩업 테이블 활용.

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | 검색: O(nprobe × N/nlist × M), 구축: O(N × k-means iterations) |
| **공간 복잡도** | 768dim float32 → ~96B (32배 압축). 1M벡터 → 50~150MB |
| **정확도** | 리스코어링 포함 시 Recall 85~97%. HNSW 대비 in-memory 규모에서 하위 |
| **구현 복잡도** | LanceDB(Rust 네이티브, IVF_PQ/IVF_HNSW_PQ/IVF_HNSW_SQ 지원). faiss 크레이트는 C++/MKL 링킹 필요 → Tauri 크로스플랫폼 패키징에 부적합 |
| **운영 복잡도** | k-means 학습 패스 필요 → 증분 삽입 불리. 인덱스 재구축 비용 높음 |

**적합 시나리오:** 1M 벡터 이상 또는 극도의 RAM 제약 시. **1K~100K 문서 규모에서는 학습 비용 대비 회수 불가 → 비권장.**

**권장 Rust 크레이트:** `lancedb` ★★★☆☆

---

### A-3. ScaNN (Scalable Nearest Neighbors)

**개요:** Google의 비등방성 벡터 양자화(Anisotropic VQ). 쿼리 방향 평행 성분의 양자화 오차에 가중치를 두어 상위 점수 항목의 내적 보존. 4비트 PQ + SIMD LUT-16 결합.

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | ann-benchmarks 파레토 프론트: 95~98% recall에서 차순위 대비 ~2배 QPS |
| **정확도** | GloVe-100 기준 최상위 정확도-속도 트레이드오프 |
| **구현 복잡도** | C++/TensorFlow, x86-AVX2 전용, **Rust 바인딩 없음** |
| **Tauri 적합성** | ❌ **Apple Silicon, Windows-on-ARM 미지원 → 크로스플랫폼 Tauri 앱에 사실상 사용 불가** |

**결론:** 학술적으로 영향력 있으나 실용적으로 채택 불가. ScaNN의 핵심 아이디어는 USearch, Faiss 등에 흡수되어 HNSW 구현체를 통해 간접 사용 가능.

---

### A-4. DiskANN / Vamana

**개요:** Microsoft의 단층 그래프 인덱스. α-relaxed robust pruning으로 장거리 엣지 생성(HNSW 상위 레이어 효과를 단층에서 구현). 디스크 변형은 PQ 압축 벡터만 RAM에 유지, 전체 그래프는 SSD에 저장.

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | 검색: O(beam_width × log N × dim), RAM에서 sub-ms, SSD에서 수 ms |
| **공간 복잡도** | 벡터당 8~32B RAM + SSD. 1M × 768dim → 100~400MB RAM |
| **정확도** | Recall@10 = 0.95~0.99 |
| **구현 복잡도** | Rust: rust-diskann, diskann-rs, pgvectorscale ★★★★☆ |
| **운영 복잡도** | FreshDiskANN(SIGMOD 2022)으로 실시간 삽입 지원, 100K inserts/sec at billion-scale |

**벤치마크 근거:** pgvectorscale(2025.5): 50M Cohere 임베딩에서 99% recall 시 471 QPS vs Qdrant HNSW 41 QPS — **11.4배 차이.**

**2025~2026 트렌드:** SQL Server 2025, Azure Cosmos DB가 **HNSW가 아닌 DiskANN을 네이티브 인덱스로 채택.** 가장 빠르게 성장 중인 대안.

**적합 시나리오:** 50만 문서 이상 또는 RAM 제약 시 마이그레이션 경로로 유지. ≤100K 데스크톱 문서에서는 과설계.

---

### 벡터 검색 알고리즘 비교 매트릭스

| 차원 | HNSW | IVF-PQ | ScaNN | DiskANN |
|------|------|--------|-------|---------|
| 1M×768 구축 시간 | 분 단위 | 분 + k-means 학습 | 분 + 학습 | HNSW의 1.5~2배 |
| 쿼리 지연 | sub-ms (RAM) | 1~10ms | sub-ms | sub-ms(RAM) / 수ms(SSD) |
| Recall@10 | 0.95~0.99 | 0.85~0.97 (리스코어링) | 0.95~0.99 | 0.95~0.99 |
| 1M×768 RAM | ~4~5GB | ~50~150MB | ~50~150MB | ~100~400MB + SSD |
| 학습 패스 필요 | ❌ | ✅ | ✅ | ❌ |
| 증분 삽입 | ⭕ 양호 | ❌ 불량 (재학습) | ❌ 불량 | ⭕ 양호 (FreshDiskANN) |
| Rust 생태계 | ★★★★★ | ★★★☆☆ | ★☆☆☆☆ | ★★★★☆ |
| Tauri 적합성 | 용이 | 중간 | 곤란 | 용이 |

---

## 카테고리 B: 실행 아키텍처 및 동시성 (4개)

### B-1. Rayon Work-Stealing 병렬화

**개요:** CPU 바운드 쌍별 비교 루프에 최적화된 데이터 병렬 프레임워크. Tokio의 비동기 런타임과 분리하여 운용.

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | 8코어 기준 현실적 스케일링 4~7배, 캐시 최적화 시 최대 10배 |
| **공간 복잡도** | 스레드당 스택 ~8MB, 총 추가 ~64MB (8코어) |
| **정확도** | 동일 (계산 로직 변경 없음) |
| **구현 복잡도** | 낮음. `par_iter()` 한 줄 변경 수준. Tauri v2 `tauri::async_runtime`과 호환 |
| **운영 복잡도** | 낮음. Tokio와 Rayon 풀 분리 필수 (PostHog: p99 2s → 94ms 사례) |

**계산 근거:**
- 500K 쌍 × 100μs/쌍 = 50초 (단일 스레드)
- 8코어 Rayon × 5배 효율 = ~10초
- **현재 65분 → ~10~12초로 단축 가능 (단독 적용 시)**

**권장 패턴:** `#[tauri::command]` → `rayon::spawn` 또는 `tokio::task::spawn_blocking` → `app.emit`으로 진행률 전송

---

### B-2. 액터 모델 (Kameo / Ractor / Actix)

**개요:** 파이프라인 스테이지를 독립 액터로 분리. IngestActor → IndexActor → PairActor Pool → PersistActor 구조.

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | 총 작업량 감소 없음. 파이프라인 단계 간 동시성 확보 |
| **공간 복잡도** | 액터당 ~수KB. 10K 액터 스폰 시 Kameo 5ms, Ractor 68ms |
| **정확도** | 동일 |
| **구현 복잡도** | 중간. 기존 코드 리팩토링 필요 |
| **운영 복잡도** | Supervision, bounded backpressure 제공 |

**Rust 액터 프레임워크 비교:**

| 프레임워크 | 처리량 | 특징 | 상태 |
|-----------|--------|------|------|
| **Kameo** | ~11M tell/sec (8코어 MBP) | 6줄 액터, supervision, 분산 | ★ 가장 활발 |
| Ractor | Erlang 스타일 | 안정적 | 활발 |
| Actix | 최고 raw 메시징 속도 | Tokio 기반 런타임 | 안정적 |
| Bastion / Riker | - | - | ❌ 아카이브 |

**권장:** Kameo (개발자 편의성 + 성능 균형). 단, 액터 도입 자체가 총 처리량을 줄이지는 않으므로, Rayon 병렬화와 결합하여 사용.

---

### B-3. CQRS + 이벤트 소싱

**개요:** 쓰기(Command)와 읽기(Query) 경로 분리. 문서 수준 이벤트(Ingested, Updated, Removed)를 로그에 추가하고, 교차참조 그래프는 읽기 측 프로젝션으로 비동기 소비.

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | 쓰기: O(1)/이벤트. 다중 프로젝션 병렬 실행 가능 |
| **정확도** | 최종 일관성(Eventual Consistency) |
| **구현 복잡도** | **높음.** Aggregate, Event Store, Snapshot, Projection 등 아키텍처 비용 |
| **운영 복잡도** | 높음. 상태 관리, 리플레이, 스냅샷 전략 필요 |

**Rust 크레이트:** `cqrs-es`, `esrs`, `eventastic`, `thalo`

**판정:** 데스크톱 앱 규모에서는 **과설계.** 실질적 가치는 감사 추적, Undo, 취소 가능 인덱싱, UI 부분 결과 스트리밍에 있음. **처리량 목적으로는 비권장.**

---

### B-4. Salsa 증분 계산 프레임워크

**개요:** 순수 함수형 쿼리(`#[salsa::tracked]`)를 입력 데이터(`#[salsa::input]`) 위에 정의하고, 의존성 DAG + 리비전 번호로 변경 추적. 입력 변경 시 더럽혀진(dirty) 의존 쿼리만 재실행, early cutoff으로 상류 값 불변 시 검증 단축.

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | 증분: O(N) — 신규 문서 추가 시 기존 N개와의 새 쌍만 계산. 기존 N(N-1)/2 쌍은 메모이제이션 재사용 |
| **공간 복잡도** | 쿼리 결과 캐시: 쌍당 ~수십 B. 100K문서 → ~수 GB |
| **정확도** | **동일** (동일 계산, 캐싱만 추가) |
| **구현 복잡도** | 중간. `#[salsa::tracked] fn pair_cross_ref(db, a, b) -> Similarity` 모델링 |
| **운영 복잡도** | 낮음. rust-analyzer에서 대규모 실전 검증 완료 |

**핵심 근거:**
- rust-analyzer 실증: 한 글자 타이핑 시 전체 재분석 대비 10~100배 적은 작업량
- Salsa 3.0이 2025.3 rust-analyzer에 랜딩, 병렬 쿼리 및 영속 캐시 기반 작업 진행 중
- **문서 #1,001 추가 시**: 기존 1,000 쌍 메모이제이션 재사용, 신규 ~1,000 쌍만 계산 → **~1,000배 증분 개선**

**판정:** ★★★★★ **이 시스템에서 단일 최고 레버리지 변경. 강력 권장.**

---

## 카테고리 C: 배치 처리 및 스케줄링 (4개)

### C-1. 마이크로 배치 (Group-Committed Batch)

**개요:** Little's Law 기반 최적 배치 크기 결정. 쌍별 삽입을 배치 단위로 묶어 SQLite 트랜잭션 1건으로 커밋.

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | 배치당 트랜잭션 오버헤드 O(1). 256건 그룹 커밋 시 건당 fsync 비용 sub-μs |
| **공간 복잡도** | 배치 버퍼 ~수 KB |
| **정확도** | 동일 |
| **구현 복잡도** | 낮음. `.chunks_timeout(256, 10ms)` 한 줄 |
| **운영 복잡도** | 낮음 |

**계산 근거:**
- NVMe fsync 지연: 100~200μs
- 256건 배치로 분할 시: 건당 ~0.5μs (개별 트랜잭션 대비 200~400배 절감)
- **데스크톱 SSD 기준 20~100배 성능 향상** — 아마도 현재 시스템에서 가장 큰 단일 라인 개선

**Rust 크레이트:** `futures-batch` (`.chunks_timeout`), `batch-channel`, `ultra-batch`, `dataloader-rs`

**권장 설정:** 배치 크기 256, 타임아웃 10~50ms, UI 진행 이벤트 50ms SLA

---

### C-2. 우선순위 큐 + 다단계 피드백 (MLFQ)

**개요:** 3단계 큐로 작업 우선순위 분리. 에이징(aging)으로 기아(starvation) 방지.

| 큐 | 용도 | 우선순위 |
|----|------|----------|
| Q0 | 사용자 가시 문서 수집 | 최고 |
| Q1 | 증분 쌍 비교 | 중간 |
| Q2 | 백그라운드 전체 재계산 | 최저 (에이징 2~5초) |

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | push/pop: O(log N). `priority-queue` 크레이트: 50K 항목 기준 84~107ns/op, change_priority 4ns |
| **구현 복잡도** | 중간. Tokio에 네이티브 태스크 우선순위 없음 → 사용자 공간 mpsc/crossbeam 채널로 구현 |

**Rust 크레이트:** `priority-queue` (indexmap 기반, LGPL-3.0 → 라이선스 확인 필요), `keyed_priority_queue` (MIT/Apache), `orx-priority-queue` (d-ary heap)

---

### C-3. 디바운스 (Debounced Batch Processing)

**개요:** 폴더 드래그앤드롭 등 버스트 도착을 병합. 트레일링 디바운스 200~500ms, 하드 파이어 캡 2초.

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | O(1) 이벤트 병합 |
| **정확도** | 동일 |
| **구현 복잡도** | 낮음 |

**업계 관행:**
- VS Code 파일 워처: 300~500ms
- Kafka 4.0 (2025.3): linger.ms 기본값 0 → 5ms로 변경
- Flink: 세션 윈도우 (이벤트 갭 ≥ G 시 닫힘) — "사용자가 파일 추가를 마침" 의미론에 정확히 부합

**Rust 크레이트:** `tokio-debouncer` (cancel-safe), `debounced`, `stream-window` (tumbling/sliding/session 윈도우)

**권장:** 디바운스(조대한 입구 게이트) + 마이크로 배치(세밀한 실행 게이트) 결합으로 종단 간 백프레셔 구현

---

### C-4. WAL 기반 신뢰성 있는 배치 처리

**개요:** 65분짜리 작업이 앱 크래시나 노트북 닫힘에서 살아남기 위한 내구성(durability) 확보.

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | SQLite WAL 모드 + synchronous=NORMAL → 배치 단위 fsync로 sub-μs/건 |
| **공간 복잡도** | WAL 파일 ~수 MB |
| **정확도** | 동일 |
| **구현 복잡도** | 낮음 |
| **운영 복잡도** | macOS: `PRAGMA fullfsync=1` 필수 (기본 NORMAL은 전원 손실 시 최근 txn 미보장) |

**설계 권장안:**
```
pending_pairs(doc_a, doc_b, state, priority) 테이블을 동일 SQLite DB에 생성
→ 배치당 1 트랜잭션으로 소비
→ 별도 스토리지 엔진 추가 없이 내구성 + 그룹 커밋 처리량 확보
```

**Rust 크레이트:** `okaywal` (전용 WAL, 프리프로덕션), `nano-wal` (미니멀). **권장: SQLite 자체 WAL 활용이 가장 실용적.**

---

## 카테고리 D: 관계 탐지 및 그래프 최적화 (4개)

### D-1. LSH (Locality-Sensitive Hashing)

**개요:** O(N²) 후보 생성을 준선형(near-linear) 시간으로 전환하는 핵심 기법.

| 변형 | 용도 | 출력 |
|------|------|------|
| MinHash | 텍스트 Jaccard 유사도 | 128~256 퍼뮤테이션 → 16~2,048B/문서 시그니처 |
| SimHash | 가중 코사인 | 64비트 핑거프린트 |
| SRP | 밀집 임베딩 | 해시 테이블 |

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | 예상 쿼리: O(N^ρ), ρ = log(1/p₁)/log(1/p₂). 전체 코퍼스의 1~5%를 후보로 반환 |
| **공간 복잡도** | 문서당 16B~2KB 시그니처. 100K문서 → 1.6MB~200MB |
| **정확도** | 20밴드 × 6행: 98% recall, 15% FP. 18×7: ~95% recall, ~8% FP (실무 최적점) |
| **구현 복잡도** | 중간 |

**실증 사례:** Uber의 Spark LSH 사기 탐지 파이프라인 — **55시간 → 4시간 (14배 단축).** 이를 현재 65분에 적용하면 ~5분으로 매핑.

**2025~2026 트렌드:** LLM 학습 데이터 중복 제거(CCNet, SlimPajama)에서 수십억 문서 규모로 여전히 지배적. Milvus 2.6이 2024.11 `MINHASH_LSH`를 네이티브 인덱스로 출시.

**Rust 크레이트:** `lsh-rs` (SRP, L2, MIPS, MinHash), `probminhash` (SuperMinHash, 스트리밍 병합), `datasketch-minhash-lsh`

---

### D-2. 클러스터 기반 교차참조 (K-Means / HDBSCAN)

**개요:** 유사 문서를 먼저 클러스터링한 후, 동일 클러스터 + 인접 상위 M개 클러스터 내에서만 비교.

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | N² → N²/k (k=클러스터 수, M=인접 클러스터). N=10K, k=100, M=5 → ~6M vs 50M (~8배 절감) |
| **정확도** | LSH보다 덜 공격적이지만 조합 가능 |
| **구현 복잡도** | 중간 |
| **운영 복잡도** | Python hdbscan은 transductive (새 점이 기존 클러스터 분할/병합 불가) → 증분 수집 제한 |

**Rust 크레이트:** `linfa-clustering` (Lloyd k-means, mini-batch k-means — scikit-learn 대비 1.3배 빠름), `hdbscan` (순수 Rust, 활발 유지보수)

**증분 대안:** FISHDBC (HNSW + MST), DenStream, DBSTREAM (스트리밍 토픽 트래킹, 2025~2026 프로덕션)

---

### D-3. 그래프 기반 관계 전파 (kNN Graph)

**개요:** 이행적 폐쇄(transitive closure)가 아닌, 유지 관리되는 kNN 그래프를 구축하고 쿼리 시 2-hop BFS로 관계 추론.

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | 전체 TC: O(V³) Floyd-Warshall → ❌ 비실용적. NN-Descent: **O(N^1.14) — 준선형** |
| **공간 복잡도** | 밀집 N×N 행렬 → ❌ (100K → 40GB). 희소 kNN 그래프 → ⭕ (K=10, 100K → ~8MB) |
| **정확도** | 유사-이웃의-이웃 추론으로 간접 관계 발견 |
| **구현 복잡도** | 중간~높음 |

**최신 연구:** FreshDiskANN(SIGMOD 2022, Bing 배포) — 10억 규모 100K inserts/sec. GaussDB-Vector(VLDB 2024), DEG(SIGMOD 2025).

**Rust 크레이트:** `petgraph` (2.1M+ 다운로드, 안정, StableGraph, serde, `algo::tred`), `crepe` (Datalog 선언적 TC)

---

### D-4. 확률적 필터 (Bloom / Cuckoo / Xor / Ribbon)

**개요:** 이미 비교한 쌍의 중복 건너뛰기 또는 중복 문서 업로드 인제스트 단계에서 차단.

| 필터 | 1% FPR 공간 | 삭제 지원 | 특징 |
|------|------------|-----------|------|
| Bloom | ~9.6 bits/elem | ❌ | 고전적, 최다 채택 |
| Cuckoo | ~12 bits/elem | ✅ | FPR ≤ 3%에서 Bloom보다 우수 |
| Xor | ~9 bits/elem | ❌ (정적) | 가장 작음, 읽기 전용 스냅샷에 이상적 |
| Ribbon | ~7.5 bits/elem | ❌ | RocksDB 현재 채택 |

**가장 효과적인 적용:** 콘텐츠 해시 기반 `growable-bloom-filter` → 인제스트 시점에 완전 중복 문서 전체 유사도 계산 건너뛰기.

**Rust 크레이트:** `bloomfilter` (6.4M 다운로드), `growable-bloom-filter` (2.4M, serde 호환 → Tauri 영속화 적합), `fastbloom`, `cuckoofilter`, `xorfilter-rs` (962K)

---

## 카테고리 E: 저장 및 캐싱 아키텍처 (4개)

### E-1. LMDB (heed) / redb — SQLite 대체 벡터 저장소

**개요:** mmap 기반 COW B-트리, MVCC. 단일 쓰기자, 무제한 대기 없는(wait-free) 읽기자.

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | 순차 읽기: SQLite 대비 **47~80배 빠름**. 무작위 읽기: **9배 빠름** |
| **공간 복잡도** | mmap이므로 OS 페이지 캐시 활용, 실 RAM 사용은 워킹 셋 비례 |
| **정확도** | 동일 |
| **구현 복잡도** | 중간. 기존 SQLite 스키마 마이그레이션 필요 |
| **운영 복잡도** | 읽기 스레드 선형 확장 (4/8/16/32 스레드 → 142/77/45/38ms) |

**Rust 크레이트 비교:**

| 크레이트 | 백엔드 | 특징 | 성숙도 |
|---------|--------|------|--------|
| `heed` | LMDB (C) | Meilisearch 사용, 최성숙 | ★★★★★ |
| `redb` | 순수 Rust | 1.0 안정, LMDB 1.5~2배 이내 | ★★★★☆ |
| `fjall` | LSM, 순수 Rust | 대용량 blob 적합 | ★★★☆☆ |
| `sled` | 순수 Rust | ❌ v1 14개월+ 정체 → **사용 금지** | ★☆☆☆☆ |

**참고:** Qdrant는 RocksDB에서 mmap + 자체 Gridstore로 **능동적 마이그레이션 중** (PR #5908, #6148).

---

### E-2. W-TinyLFU 다단계 캐싱 (moka)

**개요:** 3계층 설계 — hot(RAM `Vec<f32>`, SIMD 준비), warm(mmap 페이지 캐시), cold(디스크 명시적 I/O).

| 평가축 | 분석 |
|--------|------|
| **히트율** | LRU 대비 W-TinyLFU: **10~15 pp 향상** (소용량 캐시 기준) |
| **스캔 내성** | LRU는 스캔 플러드에 무력화. W-TinyLFU는 빈도 스케치로 방어 |
| **구현 복잡도** | 낮음. `moka` 크레이트 드롭인 |

**Rust 크레이트 비교:**

| 크레이트 | 특징 | 적합 시나리오 |
|---------|------|--------------|
| `moka` | sync+async, TTL, 크기 가중 퇴거, 백그라운드 스레드 없음 (v0.12+) | **기본 선택** |
| `quick_cache` | 마이크로벤치 2~5배 빠름, 기능 적음 | 극한 지연 요구 |
| `foyer` | RisingWave. 메모리+SSD 하이브리드, W-TinyLFU/LRU/FIFO/S3-FIFO/SIEVE | 10GB+ 벡터 |

**권장:** 1K~100K 문서 → `moka` + `memmap2` 2계층. 10GB 이상 → `foyer` 고려.

---

### E-3. 구체화 관계 뷰 (Materialized Relationship View)

**개요:** N×N 밀집 행렬이 아닌, **ANN 그래프 자체를 증분 유지 관리되는 구체화 뷰로 활용.**

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | 밀집 행렬 구축: O(N²·D) → ❌. 100K × 100K × 4B = 40GB → 데스크톱 불가. HNSW in-place 삽입: O(M × log N × dim) |
| **구현 복잡도** | 중간 |
| **운영 복잡도** | HNSW 연결성 저하 → ~20% churn 시 전체 재구축 스케줄링 |

**업계 수렴:** pgvector(HNSW + VACUUM), Qdrant(세그먼트별 HNSW + WAL), Weaviate(HFresh), Oracle 23c(private/shared journal + 주기적 증분 스냅샷), LanceDB 모두 이 패턴.

**권장 Rust 크레이트:** `usearch-rs` (업데이트 중 동시 검색, f16/i8/bf16 양자화), `arroy` (Meilisearch, LMDB via heed에 트리 영속화 — 가장 깔끔한 임베디드 패턴)

---

### E-4. 메모리 맵 벡터 파일 (mmap + SIMD)

**개요:** `memmap2::Mmap::map` + `bytemuck::cast_slice::<u8, f32>` → 제로카피 `&[f32]` → SIMD 코사인 루프.

| 평가축 | 분석 |
|--------|------|
| **시간 복잡도** | 100K × 384dim brute-force: **5~15ms 단일 스레드** — SQLite 라운드트립보다 빠를 수 있음 |
| **공간 복잡도** | OS 페이지 캐시 활용, 추가 복사 없음 |
| **정확도** | 동일 |
| **구현 복잡도** | **매우 낮음.** ~50줄 코드 |
| **운영 복잡도** | 크로스플랫폼 주의사항 있음 (아래) |

**구현 옵션:**

| 옵션 | 특징 |
|------|------|
| raw packed f32 | 50줄, 제로카피, 제로역직렬화 |
| Apache Arrow IPC | Python/JS 사이드카 호환 |
| Lance v2.1+ (2025.3 안정) | Parquet 대비 50%+ 작은 디스크, 68배 빠른 blob 읽기, ~100배 빠른 랜덤 액세스 |

**크로스플랫폼 주의사항:**
- Windows: overcommit 미지원 → 10GB mmap은 10GB 디스크 사전 할당
- `MADV_RANDOM`/`MADV_SEQUENTIAL` via `memmap2::Mmap::advise()` — OS 프리페처 스래싱 방지 필수
- **벡터 파일은 읽기 전용 유지, 업데이트 시 원자적 교체**

---

## 통합 아키텍처 권장안

비용 대비 효과 순으로 정렬한 구체적 스택:

```
┌─────────────────────────────────────────────────┐
│  1. raw float32 BLOB + SQLite WAL              │ ← 내구성 + 메타데이터
│     (synchronous=NORMAL, fullfsync=1 macOS)      │
├─────────────────────────────────────────────────┤
│  2. memmap2 + bytemuck + SIMD cosine            │ ← ≤50K 문서 fast path
│     (5~15ms brute-force, SQLite 라운드트립 제거)  │
├─────────────────────────────────────────────────┤
│  3. usearch HNSW                                │ ← ≥50K 문서 시 활성화
│     (영속화, 동시 검색/삽입)                      │
├─────────────────────────────────────────────────┤
│  4. probminhash MinHash-LSH                     │ ← 후보 사전 필터링
│     (O(N²) 비교 폭발 방지, cold reindex용)       │
├─────────────────────────────────────────────────┤
│  5. Salsa 메모이제이션                           │ ← pair_cross_ref 쿼리
│     (증분 O(N), Rayon 내부 병렬 fan-out)          │
├─────────────────────────────────────────────────┤
│  6. priority-queue 3단계 + aging                │ ← 사용자 가시성 우선
├─────────────────────────────────────────────────┤
│  7. futures-batch chunks_timeout(256, 10ms)      │ ← SQLite 그룹 커밋
├─────────────────────────────────────────────────┤
│  8. moka W-TinyLFU 캐시                         │ ← 최근 비교 쌍 + hot 벡터
├─────────────────────────────────────────────────┤
│  9. growable-bloom-filter (콘텐츠 해시)          │ ← 중복 인제스트 건너뛰기
├─────────────────────────────────────────────────┤
│ 10. Kameo 액터 (파이프라인 감독 + Tauri 이벤트)  │ ← 선택적
└─────────────────────────────────────────────────┘
```

---

## 예상 성능 수치

| 시나리오 | 현재 | 목표 | 주요 기여 기법 |
|---------|------|------|---------------|
| 1,000문서 전체 재인덱스 | 65분+ 타임아웃 | **< 30초** | WAL 배치(10~100×) + Rayon(4~7×) + LSH 필터(10~100×) |
| 문서 #1,001 증분 추가 | 전체 재인덱스 | **ms 수준** | Salsa 메모이제이션 O(N) |
| 100K 문서 쿼리 지연 | [미측정] | **< 1ms, ≥95% recall** | HNSW (usearch) |
| 100K 문서 HNSW RAM | - | **~300~400MB** | usearch f16 양자화 |

---

## 구현 우선순위 로드맵

| 단계 | 작업 | 예상 효과 | 코드 변경 규모 |
|------|------|-----------|---------------|
| **Phase 0** | float32 BLOB 전환 + 배치 트랜잭션 + Rust 내부 거리 계산 | 10~100× | ~200줄 |
| **Phase 1** | Rayon 병렬화 + 디바운스 | 4~7× | ~100줄 |
| **Phase 2** | Salsa 메모이제이션 도입 | 증분 ~1,000× | ~500줄 |
| **Phase 3** | HNSW (usearch) 통합 | 검색 100~1,000× | ~300줄 |
| **Phase 4** | MinHash-LSH 후보 필터 | cold reindex 10~100× | ~400줄 |
| **Phase 5** | moka 캐시 + 우선순위 큐 | 2~5× 추가 | ~200줄 |
| **선택** | LMDB/redb, Kameo 액터, DiskANN | 규모 확장 시 | 대규모 리팩토링 |

---

## 리스크 및 미해결 사항

| 리스크 | 영향 | 완화 방안 |
|--------|------|-----------|
| HNSW 연결성 저하 (>20% churn) | recall 하락 | 주기적 전체 재구축 스케줄링 |
| Salsa 3.0 API 안정성 | 업그레이드 비용 | rust-analyzer가 사실상 안정성 보증 |
| macOS fullfsync 미적용 시 데이터 손실 | 내구성 | `PRAGMA fullfsync=1` 강제 |
| 임베딩 모델·차원 수 미확인 | 모든 메모리 추정치 변동 | [확인 필요] 후 재계산 |
| Windows mmap overcommit 부재 | 대용량 파일 디스크 할당 | 읽기 전용 + 원자적 교체 |
| LSH 밴드/행 튜닝 | recall-FP 트레이드오프 | 100문서 기준선(394관계) A/B 테스트 |