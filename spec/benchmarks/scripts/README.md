---
updated: 2026-06-05 (메타 룰 자동화 4종 누적 — release_rebuild_required (Phase 97) + audit_stage_check (Phase 97) + **release_redeploy (2026-06-05 신규, 메타 룰 17 강화 정식 §자동화)** + **single_source_check (2026-06-05 신규, 메타 룰 19/30 자동화)** = G-5 5종 + 4종 = **9종**)
purpose: phase 종결 시 GUI 회귀 자동 검증 + 메타 룰 자동화 + Claude가 호출 가능한 헬퍼
---

# spec/benchmarks/scripts

GUI 회귀 자동화 + 코퍼스 생성 스크립트. **phase 종결 시 5종 GUI 스크립트 호출 의무**.

## GUI 회귀 자동화 5종 (G-5)

| 스크립트 | 검증 대상 | 회귀 게이트 | 외부 의존 |
|----------|----------|-----------|----------|
| `action_catalog.sh` | data-action 카탈로그 수치 (lesson 46 1) | `--diff <baseline>`으로 사용 | 없음 |
| `dead_selector_scan.sh` | JS `getElementById` ↔ HTML id 매칭 (lesson 47) | 1+건이면 1 exit | 없음 |
| `empty_state_audit.sh` | 빈 객체 truthy 가드 후보 (G-4 (b)) | 후보 출력만 (게이트 아님) | 없음 |
| `data_flow_trace.sh` | frontend → invoke → commands → service → DB 6단계 (lesson 46 3) | 추적 도구 (게이트 아님) | 없음 |
| `gui_http_smoke.sh` | http.server + 정적 응답 검증 | 5/5 통과 의무 | python3, curl |

### 빠른 실행 (전체 5종)

```bash
cd <repo-root>
bash spec/benchmarks/scripts/action_catalog.sh --count
bash spec/benchmarks/scripts/dead_selector_scan.sh
bash spec/benchmarks/scripts/empty_state_audit.sh
bash spec/benchmarks/scripts/gui_http_smoke.sh
# data_flow_trace.sh는 대상 action을 인자로 받음
bash spec/benchmarks/scripts/data_flow_trace.sh search
```

### action_catalog.sh

```bash
# 전체 카탈로그 출력 (sort -u)
bash spec/benchmarks/scripts/action_catalog.sh

# 카운트만
bash spec/benchmarks/scripts/action_catalog.sh --count

# 기준값(예: 77) 대비 변동 검사 (회귀 게이트)
bash spec/benchmarks/scripts/action_catalog.sh --diff 77
```

phase 종결 시 카운트를 architecture.md "GUI 액션 카탈로그" 수치와 동기화.

### dead_selector_scan.sh (grep 기반, v1)

```bash
bash spec/benchmarks/scripts/dead_selector_scan.sh
```

`dashboard.js`의 모든 `getElementById('xxx')` 호출의 ID가 `index.html`에 존재하는지 확인. 0건이면 PASS, 발견 시 dead 코드 위치 출력.

### dead_selector_scan_v2.js (AST 기반, #8)

```bash
node spec/benchmarks/scripts/dead_selector_scan_v2.js
```

acorn AST 파서 기반 정밀화 (#8, 2026-05-20). v1의 grep 한계 해소:
- 템플릿 보간 `getElementById(`row-${id}`)` 같은 동적 ID 정확 처리 (정적 부분만 매칭)
- innerHTML 문자열 안의 정적 `id="..."` 패턴 AST로 정확 추출
- `createElement` + `.id` 할당 패턴 AssignmentExpression으로 인식
- 동적 ID 호출 건수 별도 보고 (잠재 위험 가시화)

의존: Node 20+ / acorn (자동 설치, ~30초 첫 실행).

**v1과 v2의 차이**: v1은 grep 휴리스틱 (빠르고 외부 의존 0, false positive 가능). v2는 AST 기반 (정확하나 npm install 비용). phase 종결 시 v1 우선, 의심 시 v2로 재확인 권장.

### empty_state_audit.sh

```bash
bash spec/benchmarks/scripts/empty_state_audit.sh
```

`await API.X()` 응답을 state에 직접 할당한 뒤 단순 `if (!X)` 가드만 적용한 곳을 검출. 빈 객체 `{}` truthy 가드 후보로 수동 검토 권고.

### data_flow_trace.sh

```bash
bash spec/benchmarks/scripts/data_flow_trace.sh <action_or_cmd>

# 예시:
bash spec/benchmarks/scripts/data_flow_trace.sh search
bash spec/benchmarks/scripts/data_flow_trace.sh pii-add
bash spec/benchmarks/scripts/data_flow_trace.sh save_config
```

6단계 grep 추적:
1. `data-action` 정의 (HTML/JS)
2. action 핸들러 (dashboard.js)
3. `API.X()` 또는 `invoke()` 호출
4. Tauri command (modals/app/src/commands.rs)
5. core/service 또는 adapters 호출
6. 영속화 (settings.db / 파일)

### gui_http_smoke.sh

```bash
bash spec/benchmarks/scripts/gui_http_smoke.sh
# 포트 변경 시
bash spec/benchmarks/scripts/gui_http_smoke.sh --port 9000
```

5종 정적 검증: index.html / dashboard.js / dashboard.css 200 응답 + "undefined" 문자열 0건 + 7탭 존재. 5/5 PASS 시 0 exit, 실패 시 1 exit.

## 메타 룰 자동화 4종 (Phase 97 + 2026-06-05)

| 스크립트 | 메타 룰 | 분류 (메타 룰 27) | 게이트 의무 |
|---------|---------|-----------------|-----------|
| `release_rebuild_required.sh` | 17 (release 재빌드) | 게이트 | git diff/marker로 결정적 — exit 1 시 빌드 의무 |
| `audit_stage_check.sh` | 24 (stage 명명) | 게이트 | ALLOWED 허용 목록 명확 — exit 0 의무 |
| `release_redeploy.sh` (2026-06-05 신규) | 17 강화 (정식 §2단계) | 게이트 | sha256 결정적 — exit 1 시 `--apply` 권고 |
| `single_source_check.sh` (2026-06-05 신규) | 19 / 30 / sub-rule 1g | 점검 | grep 휴리스틱 — 후보 출력만, exit 0 |

### release_rebuild_required.sh

```bash
bash spec/benchmarks/scripts/release_rebuild_required.sh
```

`.rs` / `ui/*.{js,css,html}` 변경 자동 감지. git 모드 + marker 모드 자동 선택. exit 1 시 빌드 명령 안내. 메타 룰 17 §1단계 자동화 (lesson 46 G-3 회귀 차단).

### audit_stage_check.sh

```bash
bash spec/benchmarks/scripts/audit_stage_check.sh
```

audit.record stage가 `{영역}.{도구명}[.{sub}]` 규칙 준수 여부. ALLOWED prefix (llm / mcp / tauri / remote / verify / service) 외 사용 시 exit 1. 메타 룰 24 자동화 (Phase 97 lesson 56).

### release_redeploy.sh (2026-06-05 신규, 메타 룰 17 강화 §2단계)

```bash
bash spec/benchmarks/scripts/release_redeploy.sh           # 검사만 (--check)
bash spec/benchmarks/scripts/release_redeploy.sh --apply   # 종료 + 배포 + 검증
```

D:\file-test 잔류 binary 감지 + 종료 + cp + sha256 일치 검증. Windows (tasklist/taskkill) + Linux (ps/kill) 분기. 안전 디폴트: `--check` 모드는 종료 안 함. `--apply` 명시적 합의 시에만 종료.

- 잔류 binary 감지 시 exit 1 + `--apply` 권고
- 미감지 + sha256 일치 시 exit 0
- D:\file-test 미존재 시 환경 의존 WARN (게이트 PASS, 메타 룰 27 false positive 회피)

관련: lesson 65 Phase 106 (1차 빌드 후 재배포 누락) + lesson 71 Linux cross-build (sha256 일치 검증) + META.md §메타 룰 17 강화 정식 (2026-06-05 승격).

### single_source_check.sh (2026-06-05 신규, 메타 룰 19 / 30 자동화)

```bash
bash spec/benchmarks/scripts/single_source_check.sh
bash spec/benchmarks/scripts/single_source_check.sh --verbose
```

spec 본문 5종(architecture / domain-map / webapp-design / deprecated / scenarios)에서 "삭제/폐기/제거" 줄을 grep + 단일 진실원 위임 표시(`deprecated.md` / "단일 진실원" / "→ 참조" / "위임") 누락 후보 출력. 점검 도구 분류(메타 룰 27, false positive 가능). lesson 49 옵션 A(시간축 Why / 상태축 What 분리) + META.md sub-rule 1g 자동화.

- PASS: 누락 0건 → exit 0
- WARN: 누락 의심 N건 → 후보 출력 + exit 0 (게이트 아님)
- `architecture-archive.md` 는 시간축 보존 영역이므로 검사 대상 제외

## Phase 종결 체크리스트 (META.md 메타 룰 1과 결합)

각 phase 종결 시 (코드 변경 phase에 한정):

```bash
# 1. 정적 회귀 게이트 (필수)
bash spec/benchmarks/scripts/dead_selector_scan.sh        # exit 0 의무 (lesson 47)
bash spec/benchmarks/scripts/gui_http_smoke.sh             # 5/5 통과 의무
bash spec/benchmarks/scripts/audit_stage_check.sh          # exit 0 의무 (메타 룰 24)

# 2. 메타 룰 17 자동화 (release 재빌드 + 배포)
bash spec/benchmarks/scripts/release_rebuild_required.sh   # exit 1 시 재빌드 (메타 룰 17)
# 빌드 완료 후
bash spec/benchmarks/scripts/release_redeploy.sh           # exit 1 시 --apply (메타 룰 17 강화)

# 3. 카탈로그 변동 확인 (architecture.md 수치 동기화)
bash spec/benchmarks/scripts/action_catalog.sh --count
# → architecture.md "GUI 액션 카탈로그" 수치와 비교

# 4. 후보 점검 (게이트 아님, 수동 검토)
bash spec/benchmarks/scripts/empty_state_audit.sh
bash spec/benchmarks/scripts/single_source_check.sh        # spec 단일 진실원 위임 위반 후보
```

## Git pre-push hook (2026-05-20 등록)

`src/.git/hooks/pre-push`에 정적 회귀 게이트 2종이 자동 실행됩니다:
- `dead_selector_scan.sh`
- `gui_http_smoke.sh`

게이트 실패 시 push 차단. 우회 (긴급용):
```bash
PIPELINE_SKIP_GUI_GATE=1 git push
```

hook 본문은 `src/.git/hooks/pre-push` 참조. 새 clone 시 hook은 자동 복사되지 않으므로 본 README의 hook 내용을 수동 복사 필요.

## 코퍼스 생성

| 스크립트 | 역할 |
|---------|------|
| `gen_synthetic_corpus.ps1` | 5K 합성 코퍼스 생성 (PowerShell, lesson 32 #2/#4/A2/B1 측정 의존성 해소용) |

## 관련 문서

- lesson 46 §"잘한 것" — 본 5종 스크립트의 원본 grep 패턴
- lesson 47 — dead_selector_scan.sh의 원본 사례
- META.md 메타 룰 1 12번째 단계 — phase 종결 시 본 스크립트 호출 의무
- spec/architecture.md "GUI 액션 카탈로그 (80개)" — action_catalog.sh의 기준값
