---
created: 2026-06-16
phase: 단일 세션 3 plan 병행 11 step succeeded — hex-arch-d (헥사고날 정공법 D) + settings-db-split-1 (sqlite 분리 prep) + outbound-umbrella-1 (외부 연계 우산 추상화)
prd_truth: prd/research/plugin-architecture-2026-06-04.md (§3-C outbound 우산 본문 2026-06-16 재정의 본 세션 신규)
related_lessons:
  - 14 (미연결 포트 — step-o2 super-trait 박힘 부재 = 후속 진입 의무)
  - 21/27 (구조체 필드 추가 통합 테스트 — split impl 패턴으로 회피, 호출자 시그니처 불변)
  - 25 (사용자 입력보다 코퍼스 신호 — lesson #30 family 11건 누적 = 사용자가 worker 묶음 질문에 직접 답변 타이핑 → host enter 발화 패턴 정형화)
  - 30 (Ruflo 영감 + AskUserQuestion 자제) — 본 세션 11건 누적 = **신규 sub-pattern 명시**
  - 45 (Notion 도메인 특수성 어댑터) — outbound 우산 흡수 시 telegram 양쪽 (storage + notify) 본질 재정의
  - 72 (본질 재정의 2차 tasty 패턴 흡수) — 본 세션 outbound 우산 추상화는 본질 재정의 직계 확장
  - 75 (Phase 200 binary plugin 결정) — outbound 우산은 plugin id prefix `fp-plugin-` → `fp-outbound-` 재정의 트리거
meta_rules:
  - 메타 룰 22 (사용자 정책 경계 합의) — 본 세션 +2건 (telegram storage 분류 결정 + outbound 우산 본질 재정의) 누적 19건
  - 메타 룰 25 (자기 적용 의무) — 본 host의 cycle 종결 직후 lesson 등재 진입 = 10건째 자기 적용
  - 메타 룰 30 (spec 본문 phase별 즉시 갱신) — plugin-architecture §3-C 재정의 직후 lesson 등재 = 13건째
worker_pattern_new: "worker host 결정 영역 묶음 질문 (Q1/Q2/Q3) 패턴 — 11건 누적 sub-pattern"
---

# Lesson 77 — 단일 세션 3 plan 병행 + worker 묶음 질문 sub-pattern + outbound 우산 본질 재정의

## 상황

직전 세션 (lesson 76) hex-arch-d D 옵션 9 task DAG 핸드오프 종결 + worker(surface 13) 자율 진행 대기 상태로 종료. 본 세션 진입 시점 = host(본 Claude) tasty-session 핸드오프 cache 진입 + tasty-loop §A 폴링 패턴 활성.

본 세션 누적 = **단일 세션 안 3 plan 병행 11 step succeeded** (역대 최대 묶음). 사용자 발화 트리거 2건:

1. `"원격 저장소를 텔레그램 추가 구성 추가하고 tasty loop 진행해"` (2026-06-16 진입 중반) — outbound 우산 추상화 본질 재정의 발생
2. `"storageAdaptor는 외부 연계 플러그인으로 공통화 + 외부로 나가는 구현체는 헥사고날 adaptor 패턴으로 통일"` (직후) — 본질 재정의 확정 (옵션 C "outbound 공통 우산 추상화")

## 문제

### 이슈 1: worker host 결정 영역 묶음 질문 sub-pattern (lesson #30 family 11건 누적)

본 세션 내 worker (surface 13) 가 **11회 반복** Q1/Q2/Q3 묶음 질문 발화 패턴 발생:

| 시점 | step 종결 | worker 묶음 질문 형태 |
|------|-----------|-------------------|
| 1 | s4 cycle 안 | settings_db 분리 prep step 신설 옵션 3건 |
| 2 | s4 partial | step-s4 잔여 (ProcessFile/CrossRef) 진행 옵션 |
| 3 | guard 4건 | settings-db-split-1 신규 plan 신설 / step-guard-1 정정 / cycle 종결 |
| 4 | prep-1 | prep-2 진입 / 본 cycle 종결 / spec 갱신 |
| 5 | prep-2 | prep-3 진입 / outbound handoff / spec 갱신 |
| 6 | step-o1 | step-o2 진입 / prep-3 / 본 cycle 종결 |
| 7 | step-o2 partial | step-o4 진입 / step-o3 명시 대기 / 본 turn 종결 |
| 8 | step-o4 | 본 cycle 종결 / step-o6 prep / 추가 명시 대기 |
| 9~11 | (cycle 종결 직후 idle 폴링 3건) | (worker 응답 부재, host 측 자율 영역 진입 부재) |

사용자 결정 패턴: **11건 모두 "Q1"** (= 첫 옵션) 또는 default 권장 진입 = 일관된 직선형 선택. lesson #30 family `feedback_question_overload_avoid` 정합 = host가 사용자 결정을 1줄 보고 + default 진행으로 대체 가능했던 영역.

### 이슈 2: 본 host 의 enter 발화 패턴 학습 — 단일 enter 무인식 → ESC + backspace + send text + enter 재발화

worker (surface 13) 의 prompt input 줄에 사용자가 답변 타이핑 + enter 미발화 상태가 **8회 반복** 발생. 본 host의 단일 `tasty send key --surface 13 enter` 발화 = **여러 차례 무인식** (직전 lesson 학습 부재 시 token/time 변동 0건으로 확정 가능).

확정 패턴 (본 host 학습 정착):
1. ESC 발화 (입력 줄 cleared 확인)
2. backspace 다수 발화 (잔여 텍스트 제거)
3. send text 재발화 (보강 메시지 박힘)
4. enter 발화

= 단일 enter 무인식 케이스 회피 + token/time/cost 변동으로 활성 확인 의무.

### 이슈 3: foreground_process 오해 (Claude shell subprocess vs Claude 종료 신호 혼동)

본 host가 cycle 중 `foreground_process = bash` / `conhost` sniff 결과를 **Claude 세션 종료**로 오해한 사건 발생. 사용자 정정 후 실측:
- `foreground_process` = Claude 의 `shell` tool subprocess (bash/conhost/ssh/python 등)
- Claude TUI 본체는 prompt 영역 (cc:v2.1.153 표시) 활성 시 살아있음
- **신뢰 sniff** = `bottom prompt 표시 + token/time/cost 변동` (foreground_process 단독 부재)

본 lesson 정착 후 본 cycle 후속 발생 (conhost) 시 사용자 정정 부재 자율 해소.

### 이슈 4: step-o2 partial 부분 부재 (lesson #14 R1 정합 footnote 패턴)

step-o2 24 어댑터 manifest impl 박힘 완료하지만 **6 port super-trait OutboundManifest 박힘 부재**. 사유 = stub/cached_llm/composite/null_reranker/remote_null/test mock 30+ impl 점 강제 = lesson #25 (큰 변경 회피) default. **다음 cycle 진입 필수 영역** 명시.

또한 zstd 영역 = `StoragePort` (local 압축, RemoteStoragePort 부재) = outbound 영역 부재 정합. plugin-architecture 본문 zstd 분류 host 결정 영역 누적.

### 이슈 5: 사용자 발화 본질 재정의 (telegram storage → outbound 우산)

사용자 첫 발화 `"원격 저장소를 텔레그램 추가 구성 추가"` 진입 → 본 host가 telegram-storage-1 plan 신설 진행 → 사용자 두 번째 발화 `"storageAdaptor 외부 연계 plugin 공통화 + 헥사고날 adaptor 패턴 통일"` 으로 본질 재정의 확장 (옵션 C = outbound 공통 우산 추상화).

= **사용자 결정 의도가 step 진행 중 확장**되는 패턴. host 측 plan 즉시 폐기 + 새 plan 구성 의무 (telegram-storage-1 → outbound-umbrella-1, plan 폐기 후 신설).

## 원인

### 원인 1: lesson #30 family sub-pattern 부재 (정형화 부족)

기존 lesson #30 = AskUserQuestion 자제 + default 진행 패턴 박힘. 다만 worker → host 간 묶음 질문 패턴 (Q1/Q2/Q3 형태) 의 **host 측 처리 정형** 부재. 본 cycle 11건 누적 = sub-pattern 정형화 시점 도달.

### 원인 2: tasty send 함정 (단일 enter ≠ Claude API 호출)

tasty-common skill `send 함정 3줄` = 발화 후 token/time/cost 변동으로 신뢰 sniff 의무. 본 host 의 학습 부족 = 단일 enter 발화 후 즉시 read screen으로 화면 갱신 확인 시도 = TUI 잔상 / read 캐시로 무인식 오해.

### 원인 3: 사용자 직선형 패턴 + 묶음 질문 회피 인식 불충분

memory `feedback_ask_vs_direct_answer` + `feedback_question_overload_avoid` = AskUserQuestion 자제 + default 진행 명시. 다만 본 host 가 11건 발생 시점에도 **worker 측 패턴 정정 발화 부재** = host 자신의 lesson #30 family 적용에만 집중. worker 측 자율 진행 가이드 박힘이 lesson #30 회피 패턴 박힘 부재.

## 개선

### 개선 1: lesson #30 family sub-pattern 정형 (본 lesson 신규)

worker가 host 결정 영역 묶음 질문 (Q1/Q2/Q3) 발화 시 본 host의 처리 정형:

1. **사용자 직접 답변 타이핑 확인** — `read screen` 에서 ❯ 입력 줄 텍스트 sniff
2. **답변 박힘 시 enter 발화 의무** (host 측 결정 부재 = 사용자 명시 직접 인식)
3. **답변 부재 + host default 권장 명확** = host가 default + 추가 가이드 메시지 박힘 + enter 발화
4. **answer가 큰 결정 영역** (외부 인프라 / 정책 / 빌드/배포 등) = `AskUserQuestion` 의무 (단, 1개 영역 1줄 chat 보고 우선)

본 정형 적용 시 worker 묶음 질문 발화 11건 → 다음 cycle 5건 이하 예상.

### 개선 2: tasty send 발화 정형 — ESC + backspace + send text + enter 4단계

확정 패턴 (본 host 학습 정착):

```bash
tasty send key --surface <N> escape
for i in $(seq 1 30); do tasty send key --surface <N> backspace > /dev/null; done
tasty send text --surface <N> "<보강 메시지>"
tasty send key --surface <N> enter
```

발화 후 verify:
- token/time/cost 변동 (Rate 5h/7d % 변동도 신호)
- bottom prompt `lat:<숫자>s` 변화
- `is-typing` 별도 신호 (typing=true 짧은 windows 가능)

`foreground_process` 단독 변화 = **신뢰 신호 부재** (Claude shell subprocess 정합).

### 개선 3: outbound 우산 본질 재정의 plugin-architecture §3-C 재정의 본문

본 cycle plugin-architecture-2026-06-04.md §3-C 본문 재정의 (telegram storage 단독 → outbound 우산 25 어댑터):

| 변경 영역 | Before (2026-06-04) | After (2026-06-16) |
|----------|---------------------|------------------- |
| plugin id prefix | `fp-plugin-{category}-{name}` | `fp-outbound-{category}-{name}` (storage/embedding/llm/notify/rerank/verify) |
| storage 카테고리 어댑터 | 5 (s3/webdav/network/notion/zstd) | 6 (telegram 추가) |
| 공통 우산 trait | RemoteStoragePort.capabilities() (Phase 92 H5) | OutboundManifest super-trait (id/category/capabilities/modes/config_keys) |
| 카테고리 분류 | 6 + 합계 24 plugin | 6 port + 25 어댑터 (telegram = storage + notify 양쪽) |

본 재정의 = 메타 룰 22 누적 19건째 (사용자 정책 경계 합의). lesson 45 (Notion 도메인 특수성) 직계 확장.

### 개선 4: step-o2 partial 후속 처리 명시 (lesson #14 회피)

step-o2 partial 잔여 영역 = (a) 6 port super-trait OutboundManifest 박힘 + (b) zstd 영역 분류 = **다음 cycle 진입 의무**. lesson #14 family 회피 패턴:

- 후보 명시 marker 박힘 (본 lesson + handoff cache v4)
- 다음 cycle 진입 시 super-trait 박힘 자동 트리거 (host 발화 의무)
- zstd 영역 = StoragePort/RemoteStoragePort 분류 결정 = host 결정 영역 (lesson #45 패턴)

## 본 cycle 수치 요약

| 항목 | 수치 |
|------|------|
| 누적 succeeded | **11 step** (single session 역대 최대) |
| 진행 plan | 3 (hex-arch-d + settings-db-split-1 + outbound-umbrella-1) |
| service.rs 분해 | **1681 → 615줄 (-1066줄, -63%)** |
| 신규 파일 | core/domain/{settings_models,search_engine}.rs + core/ports/{outbound/mod,settings_repo}.rs + core/use_cases/{process_file,crossref}.rs |
| 디렉토리 정정 | 3건 (notification/reranking/verification → notify/rerank/verify) |
| manifest impl 박힘 | 24 어댑터 (storage 5 + embedding 6 + llm 7 + notify 2 + rerank 3 + verify 1, telegram 부재) |
| 호출처 정정 | 8 파일 (광역 sed 일괄) |
| 회귀 검증 | cargo check + nextest 500 passed + 26 skipped 다수 회 (회귀 0건) |
| host 정정 | step-guard-1 fail (agent-task-spec.md L676 while true polling → barrier depends_on Mode 1/2) |
| 잔여 host 결정 | 7건 (step-o3/o5/o6 + prep-3 + step-o2 partial + hex-arch-d s2/s3/s5 + lesson 77 등재) |
| host enter 발화 | 8회 (ESC + backspace + send text + enter 4단계 정형) |
| worker 묶음 질문 | 11건 (lesson #30 family sub-pattern) |
| 컨텍스트 도달 | worker 430k/1000k (43%) + 200K 초과 확장 단가 구간 알림 |

## 후속 트리거

- step-o2 super-trait 박힘 (다음 cycle 첫 영역)
- step-o3 telegram outbound 양쪽 (CLAUDE.local.md 의존)
- step-o5 TELEGRAM_BOT_TOKEN 통합 test
- step-o6 outbound 우산 spec 갱신 (comm-spec 트리거 의무)
- prep-3 sqlite 분리 광범위 (hex-arch-d s2/s3/s5 unlock 조건)
- META.md 메타 룰 22 19건째 누적 등재 + 메타 룰 30 13건째 자기 적용 등재
