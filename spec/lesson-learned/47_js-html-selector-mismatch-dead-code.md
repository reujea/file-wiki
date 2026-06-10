---
phase: G-4 (Phase 90+ 사이드)
date: 2026-05-20
topics: JS render 함수 ↔ HTML 엘리먼트 ID 불일치 / IA 전환 시 dead UI 코드 잔존 / lesson 19 10단계 변형
related_lessons: 13, 19, 28, 38, 46
related_meta_rules: 1
---

# 47. JS render 함수 ↔ HTML 엘리먼트 ID 불일치로 인한 dead UI 코드

## 상황

Phase 90+ GUI 전수 검증 세션 (lesson 46) 사이드 발견 G-4 "Pipeline 서브탭 invoke 의존 미렌더"의 재진단 결과. browser-automation MCP v0.2 전환 후 `extract_structured`로 정밀 검증한 결과, **invoke 의존이 아닌 HTML 엘리먼트 부재**가 진짜 원인으로 식별됨.

## 문제

`ui/dashboard.js`의 다음 6 함수가 7+ phase 동안 dead 상태로 잔존:
- `_renderPBSubtabs` — `getElementById('pb-subtabs')` 호출
- `_renderPBSubtabContent` — `getElementById('pb-subtab-content')` 호출
- `_renderSubtabProcessing` / `_renderSubtabRemote` / `_renderSubtabChunking` / `_renderSubtabRetention`

그런데 `ui/index.html`에는 `#pb-subtabs` / `#pb-subtab-content` 엘리먼트가 **없음**. Phase 67(인스펙터 480px) IA 전환 시 HTML만 정리되고 JS/CSS는 잔존.

호출처 6건이 무해한 NoOp으로 동작:
- `case 'pb-subtab'` 핸들러 (`data-action="pb-subtab"`는 어떤 HTML에도 없음)
- 단독 호출 5건 (데이터 변경 → UI 갱신 트리거이지만 entry point가 null이라 갱신 안 됨)

또한 `PB_SUBTABS` 배열은 **선언조차 없음** — 함수가 실행되면 ReferenceError지만 `getElementById`가 null로 일찍 return하여 도달 불가.

## 원인

직접 원인:
- IA 전환(Phase 56/67) 시 HTML 엘리먼트만 제거되고 대응 JS/CSS는 보존
- 컴파일러 검증 없음 — `getElementById('xxx')` 결과가 null일 가능성은 정적 검사 불가
- 자기진단 메커니즘 부재 — `_render*` 함수가 null entry point에 대해 silent하게 return

구조적 원인:
- lesson 19 10단계 체크리스트의 "JS render 함수 + DOM 셀렉터 정합성 검증" 단계 적용 시점에 본 사례 누락
- `getElementById` 결과를 검사하는 패턴(`const el = ...; if (!el) return;`)이 **방어적이지만 동시에 dead 식별을 숨김** — 좋은 의도가 dead-code 발견을 지연시킴
- HTTP 모드 Playwright 검증이 invoke 미동작 → 빈 상태로 표시하여 진짜 원인이 invoke로 오인됨 (lesson 46의 추정 오류 사례)

## 개선 (완료)

- ✅ **dashboard.js 6 함수 + 6 호출처 삭제**: -271줄 (4915→4644, -5.5%)
- ✅ **dashboard.css 5 rule + 주석 삭제**: `.pb-subtabs` / `.pb-subtab` / `.pb-subtab:hover` / `.pb-subtab.active` / `.pb-subtab-content`
- ✅ **spec/deprecated.md 항목 추가**: 단일 진실원 누적
- ✅ **lesson 19 10단계 검증**: Tauri commands / settings.db / 통합 테스트 모두 0건 (UI-only dead-code)
- ✅ **회귀 검증**: browser-automation으로 HTTP 모드 7탭 전환 + Pipeline 영역 정상 렌더 재확인

## 메타 룰 1 추가 사례

기존 META.md 메타 룰 1 "다중 위치 동기화 누락"의 14번째 누적 사례. 이번 사례의 특징:

| 차원 | 본 사례 |
|------|---------|
| 정의 위치 | JS `_render*` 함수 + `getElementById('id')` |
| 동기화 대상 | HTML `id="..."` 엘리먼트 |
| 컴파일 검증 | 불가능 (JS 런타임) |
| 발견 방법 | HTTP 모드 정적 자원 검증 + grep |
| 자기진단 패턴 | `if (!el) return;` (방어 패턴이 dead 가리기) |

### 신규 작업 시 사전 체크리스트 (메타 룰 1 추가)

#### JS render 함수 추가/제거 시

- [ ] 신규 `_render*` 함수: `grep -n 'getElementById' ui/dashboard.js`로 ID 추출 → `grep -n 'id="{id}"' ui/index.html` 존재 확인
- [ ] HTML 엘리먼트 제거 시: `grep -rn '{element-id}' ui/` → JS/CSS 동시 정리
- [ ] IA 재설계 phase 종결 시: dashboard.js의 모든 `getElementById(` 호출에 대해 ID 존재 확인 의무 (lesson 47 패턴 차단)

#### 정적 진단 도구 (제안)

```bash
# 모든 getElementById 호출의 ID 추출 후 HTML 매칭 검사
grep -oE "getElementById\('[^']+'\)" ui/dashboard.js | sort -u | \
  while read m; do
    id=$(echo "$m" | sed -E "s/.*'([^']+)'.*/\1/")
    grep -q "id=\"$id\"" ui/index.html || echo "DEAD: $id"
  done
```

본 스크립트를 `spec/benchmarks/scripts/dead-selector-scan.sh`로 정형화 후 phase 종결 체크리스트에 추가 검토.

## 잘한 것 (재사용 가능)

1. **browser-automation MCP v0.2 `extract_structured` 정밀 검증**: 단순 HTTP grep으론 식별 불가했던 "엘리먼트는 없지만 함수는 있는" 패턴 발견. 10 필드 동시 추출로 1회 호출로 진단 완료
2. **호출처 NoOp 안전성 확인**: 6 호출처가 모두 NoOp이라 동작 변경 없이 삭제 가능 — `getElementById` null check가 방어 역할은 했음. 다만 dead 식별을 지연시킨 양면성 기록
3. **lesson 46 추정 오류 즉시 수정**: G-4를 "invoke 의존"으로 기록했으나 재진단으로 "dead-code" 패턴으로 재분류. lesson의 추정 사항은 다음 phase에서 재검증 의무 (메타 룰 12 "잔존 N건 종결 의무"의 변형 — "추정 N건 검증 의무")

## 다음 세션 플래그

- **G-4 (b) invoke-no-fallback**: ✅ 종결 (2026-05-20) — Verification 카드 빈 객체 가드 강화
- **dead-selector-scan.sh 정형화**: ✅ 종결 (2026-05-20, G-5) — spec/benchmarks/scripts/dead_selector_scan.sh
- **release 재빌드 (선택)**: dashboard.js/css 정리는 UI 정적 자원이라 workspace 영향 없음. Tauri 재빌드만 하면 GUI 반영. 다만 다음 phase 변경 시 함께 묶어도 무방 (메타 룰 17 후보 — release 빌드 시점 의무화의 예외 사례)

## G-6 후속 정리 결과 (2026-05-20)

본 lesson 작성 직후 G-5 `dead_selector_scan.sh` 첫 실행에서 추가 dead 후보 14건 발견. 분류:

| 분류 | 건수 | 처리 |
|------|------|------|
| (a) 진짜 dead | 13건 | ✅ G-6 일괄 삭제 |
| (b) 동적 fallback (false positive) | 1건 (settings-no-results) | 스크립트 whitelist 강화 |

### 누적 dashboard.js 감소량 (2026-05-20)

| Phase | 변경 | 라인 수 |
|-------|------|--------|
| G-4 (a) | pb-subtabs 6 함수 삭제 | 4915 → 4644 (-271) |
| G-4 (b) | Verification 가드 추가 | 4644 → 4645 (+1) |
| G-6 | dead 13건 일괄 정리 (10 함수 + 7 case + 5 if action + 11 API) | 4645 → 4234 (-411) |
| **합계** | | **4915 → 4234 (-681, -13.9%)** |

### 메타 룰 1 누적 — IA 재설계 phase는 dead UI 코드를 동반

본 lesson에서 식별된 패턴이 **단일 사례가 아닌 7+ phase 누적 잔재**로 확인:
- Phase 56/67 IA 전환: pb-subtabs 4서브탭 (lesson 47 본문 사례)
- Phase 56/67 IA 전환 후속: search-sim / sys-credentials / migration / mcp-tools / host-tools / pb-purge / pb-doctypes / pb-preprocess / test-preprocess (G-6)

→ IA 재설계 phase는 항상 JS render 함수 잔재가 동반. lesson 47 §개선의 grep 패턴이 향후 IA 변경 시 의무 적용.

### dead_selector_scan.sh whitelist 강화 (G-6 후속)

스크립트에 추가된 4번째 스킵 규칙:

```bash
# 4. createElement + .id 할당 패턴 스킵 (dynamic fallback)
if grep -qE "\.id\s*=\s*[\"']${id}[\"']" "$UI_DIR"/dashboard.js; then
  continue
fi
```

`settings-no-results` 같은 동적 createElement 패턴 자동 제외. false positive 0건.

### 백엔드 dead 후보 (Tauri commands, 별도 phase)

frontend API 정의 11건 삭제로 다음 Tauri commands는 dead 가능성:
- `refresh_host_tools` / `list_doc_types` / `save_doc_type` / `delete_doc_type`
- `test_preprocess` / `purge_dry_run` / `purge_execute`
- `mcp_tools_list` / `mcp_tool_set_enabled` / `search_with_trace`

lesson 19 10단계 중 단계 2~3 (Tauri commands 함수 + invoke_handler 등록) 별도 정리 트리거.
