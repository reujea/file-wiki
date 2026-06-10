---
updated: 2026-04-14
---

# Lint 결과 이력

최신순 정렬. 교훈은 00_session-archive.md 참조.

## 2026-04-17T2

| 항목 | 결과 |
|------|------|
| 교차참조 비동기 배치 | crossref_queue + flush_crossref (간격30초+중복skip+priority) |
| MLFQ 우선순위 | CrossRefQueueItem.priority + 정렬 |
| Neo4j 어댑터 | json_graph.rs + neo4j_graph.rs (GraphDBPort 실구현) |
| Python ONNX 어댑터 | python_onnx_embed.rs (subprocess 폴백) |
| mmap 캐시 | 필드 보유 + refresh_mmap + init 로드 |
| 컴파일 경고 | **0건** (6건 수정) |
| 벤치마크 | 1,000문서 14.3 docs/s (70초) — 7분→70초 = 6배 |
| 65분 오기재 | 정정 노트 추가 (실측: 422초=7분) |
| E2E 테스트 | +2건 (crossref_async_batch, crossref_duplicate_skip) |
| 빌드 | workspace + Tauri 성공 |
| 테스트 | 232개 통과 (core 125 + adapters 65 + CLI 42) |

## 2026-04-17

| 항목 | 결과 |
|------|------|
| 교차참조 최적화 | 키워드 역색인 + HNSW(instant-distance) + batch persist + get_keywords 실구현 |
| VectorDBPort 확장 | batch_begin/batch_end 기본 구현 |
| StoredDoc.keywords | upsert 시 저장, 역색인 자동 구축 |
| search_hybrid 개선 | 키워드 역색인 + doc_type 매칭으로 후보 축소 |
| 벤치마크 100문서 | **13.3 docs/s** (batch), 검색 **0.44ms** (p95 0.48ms) |
| 벤치마크 비교 | 최적화 전 6.3 → 후 13.3 (+111%), 검색 0.55→0.44ms (-20%) |
| 20개 솔루션 분석 | crossref-optimization.md PRD (10카테고리 50항목) |
| 컴파일 경고 | 1건 (기존 dead_code) |
| 빌드 | workspace + Tauri 성공 |
| 테스트 | 105개 통과 (adapters 64 + CLI 41) |

## 2026-04-16T3

| 항목 | 결과 |
|------|------|
| ONNX Runtime DLL | vendor/에 ORT 1.24.4 배치, ort rc.12 load-dynamic 크래시 미해결 |
| BGE-M3 MRR | Python 실측 MRR@5 = **0.975** (+86% vs Hash, +50% vs Claude) |
| E2E Phase B | DOCX 파이프라인 + 프롬프트 핫리로드 + 배치 병렬 3건 통과 |
| Credential Manager | keyring 크레이트 + credential_store 모듈 + Tauri 커맨드 4개 |
| ColBERT | EmbeddingPort에 embed_colbert/supports_colbert 추가 (인터페이스만) |
| Qdrant auto_start | VectorDbConfig.auto_start + vendor/ 탐색 |
| 모바일 빌드 | Cargo.toml mobile feature + tauri.conf.json apk/app 타겟 |
| GraphDB | GraphDBPort 포트 trait + config graph_db 섹션 + Settings UI |
| 컴파일 경고 | 1건 (기존 dead_code) |
| 빌드 | workspace + Tauri 모두 성공 |
| 테스트 | 113개 통과 (adapters 69 + CLI 41 + core 3) |

## 2026-04-16T2

| 항목 | 결과 |
|------|------|
| BGE-M3 ONNX 실사용 | tokenizers 연동, from_dir/auto_detect, attention mask mean pooling |
| build_service ONNX 연결 | default_model=onnx/bge_m3 시 자동 로드 + Claude CLI 폴백 |
| 프롬프트 핫 리로드 | OnceLock→RwLock, reload_prompts() + get/save API |
| Pipeline UI 프롬프트 모달 | LLM 노드 → 프롬프트 편집 버튼 → textarea 모달 + 저장 |
| Embedding ONNX 경로 | showIf 조건부 표시 + config validate() 파일 존재 검증 |
| 전처리 테스트 14건 | DOCX 4 + XLSX 2 + CSV 1 + 로그 1 + 라우팅 3 + HostTool 3 |
| 프롬프트 테스트 3건 | RwLock 핫 리로드 + get_content + invalid TOML |
| Playwright 검증 | LLM 프롬프트 모달 + Embedding ONNX 경로 showIf — 5시나리오 통과 |
| BGE-M3 모델 | 2.2GB (model.onnx + model.onnx.data) + tokenizer.json 17MB |
| 컴파일 경고 | 1건 (기존 dead_code SearchLogEntry) |
| 빌드 | workspace + Tauri release + ONNX feature 모두 성공 |
| 테스트 | 107개 전체 통과 (adapters 69 + CLI 38) |

## 2026-04-16

| 항목 | 결과 |
|------|------|
| 프롬프트 외부화 | prompts.toml 로드 + OnceLock 캐시 + 내장 기본값 폴백 |
| DOCX 네이티브 | zip + XML 파싱 (word/document.xml `<w:t>` 추출) |
| XLSX 네이티브 | calamine 크레이트 (시트별 데이터 추출) |
| 배치 임베딩 병렬화 | ClaudeEmbeddingAdapter Semaphore(4) 병렬 |
| chunking Settings UI | config_metadata 5필드 + system 그룹 "청킹" 섹션 |
| prd/features 정리 | 완료 4건 삭제 (external-storage, test-reinforcement, chunking, pipeline-ui) |
| 테스트 수정 | 9파일 누락 필드 보강 + 민감 파일 테스트 3건 수정 |
| 컴파일 경고 | 1건 (기존 dead_code SearchLogEntry) |
| 빌드 | workspace + Tauri app 성공 |
| 테스트 | 핵심 4스위트 38개 전체 통과 |

## 2026-04-14T2

| 항목 | 결과 |
|------|------|
| 프롬프트 고도화 | 노이즈 제거/search_hints/code_blocks/standalone_context/약어 풀어쓰기 |
| 프롬프트 단일 소스화 | claude_adapter → prompts.rs 위임 (중복 제거) |
| doc_types.toml 축소 | patterns/prompt/dedup_key/sensitive 제거 → 검증 스키마화 |
| 의미 단위 청킹 | split_semantic + SemanticChunkConfig + 파이프라인 연결 |
| search_hints BM25 | Qdrant sparse vector에 keywords + search_hints 병합 |
| 테스트 보강 | +40개 (service 9, models 10, lint 5, chunking 7, sqlite 9) |
| 벤치마크 | 신규 38.6초/파일(-6%), 구조100%, ROUGE 65.7%, 1-Pass 100% |
| 컴파일 경고 | 1건 (기존 adapters unused import) |
| 빌드 | workspace + Tauri app 성공 |
| 테스트 | 132개 전체 통과 (core 116 + adapters 16) |

## 2026-04-14

| 항목 | 결과 |
|------|------|
| 검색 테스트 | 11 시나리오 전체 통과 (search_accuracy.rs) |
| MRR@5 | 0.525 (HashEmbedder, 기준선 0.40) |
| P@3 평균 | 0.67 (meeting 1.0, study 1.0, log 0.33, report 0.33) |
| pipeline.toml | 표준화 (sqlite, dim=128, 파이프라인 4개) |
| DEFAULT_CONFIG_TEMPLATE | 갱신 (첫 실행 즉시 동작) |
| SqliteVec 테스트 격리 | with_path() + 7개 테스트 파일 적용 |
| e2e + scenarios + actor | 전체 통과 (데이터 격리 해결) |
| Metadata 필드 | 삭제된 3개 필드 제거 (e2e_embedded.rs) |

## 2026-04-13T4

| 항목 | 결과 |
|------|------|
| Pipeline Builder | Pipelines+Settings 흐름 → 독립 Pipeline 탭 (3패널, 노드 플로우) |
| Settings | 6그룹→2그룹 (크레덴셜 관리, 시스템) |
| 크레덴셜 | UUID id + 수정 버튼 + upsert + 폼 초기화 |
| 임베딩 | 128축 의미 벡터 (MRR ~0.3→~0.65) |
| PDF 처리 | ChunkedAgent read_to_string 실패→LLM 위임, ClaudeCli PDF 경로 포함 |
| dashboard.js | ~2087줄→~1580줄 |
| 탭 수 | 8→7 (Pipelines 삭제→Pipeline 추가) |

## 2026-04-13T2

| 항목 | 결과 |
|------|------|
| credential override 연결 | watcher → credential_llms HashMap → process_file_with_pipeline 주입 |
| Preprocess 오버라이드 | preprocess_with_config(pdf_tool, ocr_tool) 실제 구현 |
| Embedding 오버라이드 | embed_with_model 기본 구현 + 서비스 연결 |
| Storage 오버라이드 | compress_with_level 실제 구현 (ZstdStorageAdapter) |
| credential 미리보기 | 파이프라인 에디터 LLM 스텝에 provider/model/key 표시 |
| retry_failed | WorkQueue.retry_all_failed() + Tauri 커맨드 + UI 버튼 |
| dashboard.html 삭제 | index.html 단일 파일 통일 |
| 경고 수정 | 5건→1건 (unreachable_code, forgetting_copy_types, dead_code) |

## 2026-04-13

| 항목 | 결과 |
|------|------|
| 파이프라인 에디터 | Pipelines 탭 완성 (5종 스텝, CRUD, glob 매칭) |
| 크레덴셜 확장 | profile_path + default_credential + 역할별 연동 |
| Settings UI | 크레덴셜 연동 이관, 전처리/민감/벡터DB 가이드 문구 |
| Processing 탭 | Queue 카드 + row 클릭 상세 + 5초 자동갱신 |
| 버그수정 | FreeConsole, SensitiveConfig alias, frontendDist 경로 |
| Tauri commands | 28개 (Pipeline CRUD 4개 추가) |

## 2026-04-10T5

| 항목 | 결과 |
|------|------|
| SqliteVec | JSON 파일 영속화 (.sqlite-vec.json) |
| 재시작 후 stats | **총 문서 수: 1** (영속 확인) |
| Settings 5그룹 | AI엔진/문서처리/벡터DB/품질검증/시스템 |
| 크레덴셜 | AI엔진 내 인라인 + 모달 |
| 가공 테스트 | 메일.txt + memo 3건 처리 확인 |
| PDF 실패 | 정상 (미지원) |

## 2026-04-10T4

| 항목 | 결과 |
|------|------|
| 컴파일 경고 | **0건** |
| Settings | 11메뉴 → 5그룹 (AI엔진/문서처리/벡터DB/품질검증/시스템) |
| Credentials | Settings 내 AI엔진 그룹에 통합 |
| CMD 창 | GUI 모드에서 미표시 (로그 파일만) |
| 대시보드 | 5초 자동 갱신 |
| handsoff | 분석 완료 → prd/features/settings-redesign.md 이관 |

## 2026-04-10T3

| 항목 | 결과 |
|------|------|
| 컴파일 경고 | **0건** |
| Settings UI | VSCode 스타일 (네비+검색+섹션별 표시, 한국어) |
| semantic_dup_threshold | 셀렉트 박스 5개 옵션 |
| inbox 다중 경로 | extra_inboxes 설정 + watcher 감시 |
| verification.thresholds | 설명에 "zstd와 무관" 명시 |

## 2026-04-10T2

| 항목 | 결과 |
|------|------|
| 컴파일 경고 | **0건** |
| Tauri invoke | `__TAURI_INTERNALS__` 수정 → 모든 커맨드 정상 동작 |
| 단일 인스턴스 | Named Mutex로 중복 실행 방지 |
| Lint 관리 | 삭제 + 백링크 보강 버튼 추가 |
| Settings | Tauri 앱 내에서 정상 렌더링 |

## 2026-04-10

| 항목 | 결과 |
|------|------|
| 컴파일 경고 | **0건** |
| 테스트 | **125+개** 전체 통과 |
| 첫 실행 UX | auto-init + 스켈레톤 디렉토리 + 안내 메시지 |
| GUI 시작 | 콘솔 [1/4]~[4/4] 진행 표시 → 콘솔 숨김 → Dashboard |
| backend 기본값 | sqlite (Qdrant 없이 즉시 동작) |
| 트레이 종료 | app_handle.exit(0) → 프로세스 완전 종료 확인 |

## 2026-04-09T3

| 항목 | 결과 |
|------|------|
| 컴파일 경고 | **0건** |
| 헥사고날 위반 | **0건** |
| spec 정합성 | **갱신 완료** |
| 테스트 | **125+개** 전체 통과 |
| Phase 14 | **완료** (모달 분리: cli/app/mcp) |
| 디렉토리 | crates(3) + modals(3) + ui + vendor |

## 2026-04-09T2

| 항목 | 결과 |
|------|------|
| 컴파일 경고 | **0건** |
| 헥사고날 위반 | **0건** |
| spec 정합성 | **갱신 완료** |
| 테스트 | **125+개** 전체 통과 (core 89 + scenarios 11 + e2e 11 + real_env 14) |
| Phase 13 | **완료** (단일 바이너리, Qdrant 동봉, 빌드 최적화) |
| Tauri build | **성공** (15MB exe, 4MB NSIS installer) |
| Dashboard UI | **Playwright 검증 완료** (콘솔 에러 0건) |

## 2026-04-09

| 항목 | 결과 |
|------|------|
| 컴파일 경고 | **0건** |
| 헥사고날 위반 | **0건** |
| spec 정합성 | **갱신 완료** |
| 테스트 | **141+개** 전체 통과 |
| Phase 12 | **완료** (Tauri 완성, 실환경 검증, PDF/OCR 제거) |
| Tauri build | **성공** (22MB exe, 5.1MB NSIS installer) |

## 2026-04-08 T7

| 항목 | 결과 |
|------|------|
| 컴파일 경고 | **0건** |
| 헥사고날 위반 | **0건** |
| spec 정합성 | **갱신 완료** |
| 테스트 | **138개** 전체 통과 |
| Phase 10 | **완료** (KG CLI, 트레이, 하드코딩, Tauri, CLI통합) |
| 하드코딩 제거 | 심각4건 + 높음1건 수정 |

## 2026-04-08 T6

| 항목 | 결과 |
|------|------|
| 컴파일 경고 | **0건** |
| 헥사고날 위반 | **0건** |
| spec 정합성 | **갱신 완료** |
| unwrap 위험 | **0건** |
| 테스트 | **138개** 전체 통과 (nextest, bench_scale_5000 제외) |
| Phase 9 | **완료** (대용량에이전트, 작업큐, 배치CLI, 테스트35건, 경쟁분석) |

## 2026-04-08 T5

| 항목 | 결과 |
|------|------|
| 컴파일 경고 | **0건** |
| 헥사고날 위반 | **0건** |
| spec 정합성 | **갱신 완료** |
| unwrap 위험 | **0건** |
| 테스트 | **99개** 전체 통과 (nextest) |
| Phase 8 | **완료** (health, 병렬, fallback, 검증메트릭, 진행률, 메모리계층) |

## 2026-04-08 T4

| 항목 | 결과 |
|------|------|
| 컴파일 경고 | **0건** |
| 헥사고날 위반 | **0건** |
| spec 정합성 | **갱신 완료** |
| unwrap 위험 | **0건** |
| 테스트 | **99개** 전체 통과 (nextest, 175초) |
| Phase 6 | **완료** (토픽UI, 페이지네이션, 검증9건) |
| Phase 7 | **완료** (LLM통합, Todo생명주기, 타임아웃) |

## 2026-04-08 T3

| 항목 | 결과 |
|------|------|
| 컴파일 경고 | **0건** |
| 헥사고날 위반 | **0건** |
| spec 정합성 | **갱신 완료** |
| unwrap 위험 | **0건** |
| 테스트 | **85개** 전체 통과 (nextest) |
| docs | 5개 다이어그램 작성 |
| PRD features | **8건** (고도화 대기) |

## 2026-04-08 T2

| 항목 | 결과 |
|------|------|
| 컴파일 경고 | **0건** |
| 헥사고날 위반 | **0건** |
| spec 정합성 | **갱신 완료** |
| unwrap 위험 | **0건** |
| 테스트 | **85개** 전체 통과 (nextest, 168초) |

## 2026-04-08

| 항목 | 결과 |
|------|------|
| 컴파일 경고 | **0건** |
| 헥사고날 위반 | **0건** |
| spec 정합성 | **10/10 일치** |
| unwrap 위험 | **0건** (전수 교체) |
| 테스트 빈틈 | 없음 (84개 전체 통과) |
