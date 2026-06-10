# Lesson 39 — Phase 86 일괄: 위생 후속 + 트리거 인프라 선구현

## 상황

Phase 85 위생 마무리 + 측정 무관 항목 일괄 처리. 5K 코퍼스 측정 의존 항목과 분리해 측정 사이클 들어가기 전 단계 정리:

- A-3: lesson 36 종결 표시
- A-4: `spec/deprecated.md` 신규 (dead 자산 단일 진실원)
- A-5: architecture.md Phase 65~78 추가 아카이빙
- A-2: 표 마크다운 보존 청킹 (트리거 #8 인프라)
- A-1: HyDE 폴백 검색 (트리거 #6 인프라)

## 문제 / 발견

### 1. 트리거 인프라 vs 본구현 — lesson 30 패턴 재확인

트리거 #6(HyDE) / #8(표 청킹)은 "실사용 피드백" 조건에 묶여 있어 측정 없이 활성화하면 dead 위험(lesson 14). 그러나 코드만 있고 호출처가 없는 dead와 달리, 본 작업은 **디폴트 비활성 + configField + 호출처 연결 완성**이라 lesson 30 패턴(Ruflo A1/A2/B1) 그대로 적용. 트리거 도달 시 코드 변경 0건으로 켤 수 있는 형태.

**메타 룰 5 강화**: "트리거 대기 = 코드 변경 없이 켤 수 있는 형태"는 다음 3요소 모두 필요:
1. 토글 가능 config 필드 (디폴트 비활성)
2. 호출처 분기 완성 (조건문이 코드에 존재)
3. 디폴트 동작 보장 (no-op 또는 안전한 fallback)

HyDE의 경우: `LLMPort.generate_hypothetical` 디폴트가 query 자체 반환 → 어댑터 미오버라이드 시 자동 no-op. 활성화해도 효과 없을 뿐 회귀 없음. 어댑터 오버라이드 시점이 실제 활성 시점.

### 2. dead 자산 표면 가시화 — 코드 주석 vs spec 인덱스

Phase 85에서 `auto_link` 삭제 후 보류 마커를 코드 주석으로만 두었으나, 사용자 Q3에서 "표면 가시화도 검토 여지" 지적. 본 phase에서 `spec/deprecated.md` 신규로 단일 진실원 확보.

**판단 기준**: 
- 코드 주석 = 해당 코드를 보러 갈 때만 발견. 점검 트리거 약함
- spec 인덱스 = 월 1회 grep 검증 가능. lesson 14 누적 감시 적합

본 phase는 둘 다 유지 (코드 주석 + spec 인덱스). 누적 시 spec 우선 갱신.

### 3. lesson "잔존 N건" 표기의 stale 위험 — 강제 종결 규칙

lesson 36이 작성 시점에 "잔존 8건" 표기를 남겼는데 Phase 84/85에서 모두 해소되었음에도 본문 미갱신. 본 phase A-3에서 명시 종결 표시 추가.

**메타 룰 (신규 후보)**: lesson 본문의 "잔존 N건" / "트리거 대기" / "후속" 표기는 다음 phase 종결 시 해당 lesson 본문에 **(✅ Phase X에서 종결)** 표시 의무. INDEX.md만 갱신하면 본문 stale 잔존.

### 4. architecture.md 아카이빙 — 2단계 누적

Phase 85에서 Phase 64 이하(130줄) 분리. Phase 86에서 Phase 65~78(390줄) 추가 분리. 총 1876 → 1368 (-27%). archive는 153 → 527.

**패턴**: 큰 아카이빙은 한 번에 하지 말고 phase별 누적. 직전 분리 영향(매 세션 컨텍스트 로드) 관찰 후 다음 단계 진행이 안전. 이번 2단계 결정은 "Phase 65 이전" → 사용자 선택 후 "Phase 80 이전"으로 확장. 매 phase 단계적 분리가 lesson 23 "보수성" 원칙과 일치.

본문에 시기 요약(추천 시스템 / IA 재설계 / UI 정합성)을 짧게 남겨 archive를 모르더라도 본문에서 흐름 파악 가능하게 유지.

### 5. 표 청킹의 `in_table` 추적 한계

표 라인 감지(`is_table_line`)는 단순 시그니처: trim 후 `|`로 시작하고 끝남. 그러나 표 사이의 빈 줄·HR은 표 외부로 판정될 위험:
- 표는 일반적으로 연속 라인이라 사이에 빈 줄 없음
- 표 안에서 헤딩(`## `)이 나타날 가능성도 거의 없음 (마크다운 표 안에 헤딩 마크업은 비표준)

따라서 본 구현은 보수적으로 "현재 라인이 표 라인이면 분할 금지"만 적용. 표 직후 빈 줄에서 단락 분할이 가능하나 — 이건 표 자체를 한 단락으로 묶는 자연스러운 동작.

**알려진 한계**: 표 안의 셀 내용에 `|` 이스케이프(`\|`)가 있는 경우는 단순 검사로 잘못 판정 가능. 트리거 #8 실제 활성 시 실 데이터로 재확인 필요.

## 개선 / 적용

### 신규 메타 룰 후보

- **메타 룰 5 강화**: 트리거 대기 인프라 = configField + 호출처 분기 + 디폴트 no-op 3요소 모두
- **메타 룰 (신규)**: lesson 본문의 "잔존 / 트리거 대기 / 후속" 표기는 다음 phase 종결 시 본문 명시 종결 의무

### 코드 변경 요약

| 파일 | 변경 |
|------|------|
| `crates/core/src/ports/output.rs` | `LLMPort.generate_hypothetical` 디폴트 메서드 (no-op) |
| `crates/core/src/domain/chunking.rs` | `SemanticChunkConfig.preserve_tables` + `is_table_line` 헬퍼 + `split_by_headings_with_path` / `split_paragraphs` 갱신 + 단위 테스트 2건 |
| `crates/shared/src/config.rs` | `ChunkingConfig.preserve_tables` / `SearchConfig.hyde_enabled` / `SearchConfig.hyde_min_results` + config_metadata 노출 |
| `crates/shared/src/lib.rs` | SemanticChunkConfig 빌드에 `preserve_tables` 전달 + DEFAULT_CONFIG_TEMPLATE에 `preserve_tables = false` |
| `crates/shared/src/mcp_server.rs` | `McpState.hyde_enabled` / `hyde_min_results` + `handle_search`에 HyDE 분기 + 테스트 2건 |
| `crates/shared/src/cli.rs` + `modals/cli/src/main.rs` | McpState 생성에 hyde 필드 주입 |
| `spec/lesson-learned/36_*.md` | "잔존 8건" 표 → Phase 84/85 종결 표시 |
| `spec/deprecated.md` | **신규** — dead/보류/폐기 단일 인벤토리 |
| `spec/architecture.md` + `spec/architecture-archive.md` | Phase 65~78 추가 아카이빙 (1758→1368) |

### 회귀 기준선

- workspace lib **336** 통과 (Phase 85 332 + 4 신규: 표 청킹 2 + HyDE 2)
- workspace clippy `--all --tests` **0건** 유지
- workspace + Tauri `cargo check` ✅
- Tauri commands 70 / MCP tools 32 변동 없음

### 후속

- 5K 코퍼스 측정 — 본 phase 인프라 둘(표 청킹·HyDE) 모두 측정 사이클에 활성화 가능. 효과 측정 후 디폴트 변경 결정
- HyDE 어댑터 오버라이드 — 디폴트 no-op이라 실제 효과는 어댑터별 `generate_hypothetical` 구현 시점에 발생. prompts.toml에 `hyde` 키 추가 + 어댑터에서 LLM 호출하는 형태
- 표 청킹 — `\|` 이스케이프 케이스 실 데이터로 재확인
