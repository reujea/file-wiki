---
created: 2026-04-17
updated: 2026-04-17T2
status: mostly-done
---

# 교차참조 고도화 구현 계획 — 14건 (필요 5 + 추후 9)

## 구현 순서

의존성과 효과를 고려하여 6단계로 분류.

### Step 1: mmap + SIMD (E-4) — I/O 제거

**목표**: JSON 직렬화/역직렬화 제거. raw float32 파일로 벡터 저장.

**변경**:
- `sqlite_adapter.rs`: DbSnapshot의 embedding을 별도 `.vectors` 바이너리 파일로 분리
- `memmap2` + `bytemuck` 크레이트 추가
- cosine_similarity를 mmap `&[f32]` 슬라이스에서 직접 계산
- JSON에는 메타데이터만 저장 (id, path, hash, doc_types, date, keywords)

**파일**: `sqlite_adapter.rs` (~100줄 변경)
**크레이트**: `memmap2`, `bytemuck`

### Step 2: Rayon 병렬화 (B-1) — CPU 활용

**목표**: cosine 비교 루프를 `par_iter()`로 병렬화.

**변경**:
- `sqlite_adapter.rs` search_brute_force: `docs.iter()` → `docs.par_iter()`
- `cross_reference.rs` auto_link 내부 후보 비교: 병렬 가능 부분 분리
- Rayon threadpool은 Tokio와 분리 (spawn_blocking 내에서 사용)

**파일**: `sqlite_adapter.rs`, `cross_reference.rs` (~50줄)
**크레이트**: `rayon` (workspace에 이미 간접 의존)

### Step 3: moka 캐시 (E-2) — 중복 비교 제거

**목표**: 최근 비교한 (doc_a, doc_b) 쌍의 결과를 캐시.

**변경**:
- `sqlite_adapter.rs`: `moka::sync::Cache<(String,String), f32>` 필드 추가
- search_similar에서 캐시 히트 시 cosine 재계산 스킵
- upsert/delete 시 관련 캐시 무효화
- TTL: 문서 변경 없으면 무제한, max_capacity: 100K 엔트리

**파일**: `sqlite_adapter.rs` (~60줄)
**크레이트**: `moka`

### Step 4: usearch HNSW (A-1) — 검색 O(log N)

**목표**: instant-distance(매번 재빌드) → usearch(영속화+SIMD+양자화)로 교체.

**변경**:
- `instant-distance` → `usearch` 크레이트 교체
- HNSW 인덱스를 `.usearch` 파일로 영속화
- upsert 시 인덱스에 추가 (재빌드 불필요)
- f16 양자화 옵션 (RAM 50% 절감)

**파일**: `sqlite_adapter.rs` (~150줄 교체)
**크레이트**: `usearch` (C 의존성, SIMD)

### Step 5: Salsa 메모이제이션 (B-4) — 증분 O(N)

**목표**: 문서 추가 시 기존 N개와의 새 쌍만 계산. 기존 쌍은 캐시 재사용.

**변경**:
- `salsa` 크레이트 추가
- `CrossRefDb` Salsa 데이터베이스 정의
- `pair_cross_ref(db, doc_a, doc_b) -> Option<RelationType>` tracked 쿼리
- `all_relations(db) -> Vec<(String, String, RelationType)>` derived 쿼리
- service.rs에서 Salsa DB를 FileProcessingService 필드로 보유

**파일**: `cross_reference.rs` (~300줄), `service.rs` (~100줄), 신규 `salsa_db.rs` (~100줄)
**크레이트**: `salsa`

### Step 6: 추후 항목 일괄 (9건)

#### 6a. WAL/SQLite 전환 (C-4)
- JSON 파일 → SQLite DB로 전환
- `rusqlite` 크레이트
- documents/relations/entities 테이블
- WAL 모드 + synchronous=NORMAL

#### 6b. 디바운스 (C-3)
- watcher에 session window 추가
- 파일 투입 버스트 병합 (500ms gap)

#### 6c. Bloom 필터 (D-4)
- `growable-bloom-filter` 크레이트
- 인제스트 시 콘텐츠 해시 체크 (SHA-256과 별도, 부분 해시)

#### 6d. 구체화 관계 뷰 (E-3)
- HNSW 인덱스 자체를 관계 뷰로 활용 (usearch 영속화)

#### 6e. LSH MinHash (D-1)
- `lsh-rs` 또는 `probminhash` 크레이트
- cold reindex 시 후보 사전 필터링

#### 6f. 클러스터링 HDBSCAN (D-2)
- `hdbscan` 크레이트
- 동일 클러스터 내에서만 교차참조 비교

#### 6g. 우선순위 큐 MLFQ (C-2)
- `keyed_priority_queue` 크레이트
- Q0(수집) > Q1(증분 비교) > Q2(전체 재계산)

#### 6h. LMDB/redb (E-1)
- `redb` (순수 Rust) 또는 `heed` (LMDB)
- JSON + mmap → 단일 KV 저장소

#### 6i. DiskANN (A-4) / Kameo 액터 (B-2) / kNN 그래프 (D-3)
- 50만+ 문서 시 마이그레이션 경로
- 현재 인터페이스만 준비

## 의존성 그래프

```
Step 1 (mmap) ─→ Step 2 (Rayon) ─→ Step 3 (moka) ─→ Step 5 (Salsa)
                                                  ↗
Step 4 (usearch) ─────────────────────────────────┘
                                                  
Step 6a~6i: Step 1~5 완료 후 독립 적용
```

## 예상 일정

| Step | 작업량 | 예상 효과 (1,000문서) |
|------|--------|---------------------|
| 1 | ~100줄 | 7분(422초) → 70초 ✅ 실측 |
| 2 | ~50줄 | → ~1분 |
| 3 | ~60줄 | → ~30초 |
| 4 | ~150줄 | 검색 <1ms |
| 5 | ~500줄 | 증분 추가 ms 수준 |
| 6 | ~1,000줄 | 대규모 확장 대비 |
