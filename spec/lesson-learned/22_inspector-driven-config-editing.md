## 상황 (Phase 67, 2026-05-04)

Phase 66에서 Pipeline 가운데 영역에 [가공] [검색] [배치] 3탭 + 우측 인스펙터를 도입했지만, 가공 파이프라인의 [가공] 탭에서 4서브탭(데이터 가공/외부 저장소/청킹/보존&Purge)이 그대로 살아있어 정보 중복 발생. 같은 config 필드가 4서브탭 form + 인스펙터(readonly) 양쪽에 표시.

또한 가공 파이프라인이 17노드로 누락 항목(Chunking sub-process / Todo 병합 / 토픽 자동 병합) 발견. 검색 파이프라인도 MMR 다양성 노드 누락.

## 문제

1. **편집 진입점 분산**: 4서브탭에서도 편집 가능, 인스펙터에도 표시 → 어디서 편집해야 할지 불명확.
2. **인스펙터 readonly 한계**: Phase 66의 인스펙터가 "설명 + readonly 표시"만 제공 → 실질 편집은 4서브탭으로 다시 가야 함. 인스펙터 가치 약화.
3. **노드 누락**: 사용자 spec(가공·검색 파이프라인 정의)과 코드 PB_NODES 사이 4 노드 차이.
4. **노드 ↔ config 섹션 매핑 부재**: 노드를 클릭해도 어떤 config 섹션을 보여줄지 코드 곳곳에 산재.

## 원인

1. Phase 66은 IA 원복이 주 목적이었고 Pipeline 재구성은 "구조 추가"였지 "구조 통합"이 아니었음 → 4서브탭과 인스펙터가 양립.
2. PB_NODES 정의가 Phase 22 시점에 굳어진 후 Phase 61(청킹) / Phase 53(Todo) / Phase 35(토픽) 추가에도 노드 매핑이 갱신되지 않음.
3. 인스펙터 데이터 모델이 nodeDef.fields(노드 자체 옵션)와 config 섹션을 분리해서 다루는 구조가 없었음.

## 개선

### 패턴 A — `configSections` 메타로 노드↔config 매핑

```javascript
PB_NODES: {
  preprocess: { ..., configSections:['preprocessing'] },
  verify: { ..., configSections:['verification', 'verification.thresholds'] },
  notify: { ..., configSections:['notification'] },
  // ...
}
```

인스펙터 렌더 함수가 이 메타를 읽고 자동으로 `_renderField`를 호출해 form 생성. 새 config 섹션이 추가되면 노드 정의에 한 줄만 추가하면 됨.

### 패턴 B — 4서브탭 폐기 후 가운데를 시각화 단일 역할로

```
가운데:  21노드 플로우 (시각화 + 클릭 진입점) — 정보 표시 0
우측:    인스펙터 (설명 + 편집 form + 저장 버튼) — 편집 100
```

좌→가운데→우의 시선 흐름이 명확. 사용자가 "노드를 본다 → 클릭한다 → 편집한다 → 저장한다"의 단순 동선.

### 패턴 C — 인스펙터 너비 320 → 480px

검증 임계 7개, 교차참조 13 필드, 원격 저장소 인증 5+ 필드 등 긴 form 처리에 320px 부족. 480px로 확장 시 한 줄에 라벨 + 입력 + help 텍스트가 잘 들어감.

### 패턴 D — 누락 노드 보완 시 사용자 spec 기준 정렬

PB_NODES 정의는 코드 진화 과정에서 자연스럽게 굳어지지만, 사용자가 명시한 spec 순서(예: "Preprocess → Chunking → LLM")를 기준으로 재정렬하면 멘탈 모델 일치. Phase 67에서:
- Chunking을 Preprocess와 LLM 사이로 이동
- Todo / Topic 후처리 노드 신규
- MMR 검색 후처리 노드 신규

### 패턴 E — 노드 옵션과 config 섹션의 분리

```
노드 옵션 (nodeDef.fields):
  - credential, prompt 같이 노드 단위 선택자
  - PipelineStep enum의 스텝별 오버라이드와 1:1
  - state.pb.nodeValues에 저장

config 섹션 (nodeDef.configSections):
  - 글로벌 설정 (sensitive.keywords 등)
  - state.config에 저장
  - 모든 노드/실행에 동일 적용
```

같은 인스펙터 안에서 두 영역을 구분 표시 ("노드 옵션" vs "설정") → 사용자가 "이 변경이 이 노드에만 적용되는가, 글로벌인가"를 즉시 인지.

## 적용 범위

- 향후 노드 추가 시 `configSections` + `fields` 두 메타로 노드 정의 완결
- 다른 영역(Topics 편집, KG 시각화)도 인스펙터 패턴 재사용 가치
- spec 변경 시 PB_NODES 누락 점검 — 기능 PR 체크리스트에 "PB_NODES 동기화 + configSections 매핑" 추가

## 사후 점검

- 사용자가 인스펙터에서 모든 config 편집 가능한지 (Settings 탭 가지 않고)
- 자동 저장(또는 명시 저장) 후 노드 카드의 시각적 변화(예: 활성/비활성 마커)가 자연스러운지
- 21노드 + 18노드 + 3섹션 매핑이 향후 추가될 트리거 노드(HyDE / Parent / Sparse)와 충돌 없이 확장 가능한지

## 참고
- lesson 20: Phase 65 IA 재설계 5패턴
- lesson 21: Phase 66 IA 원복 + 재구성 5패턴
- lesson 13: UI 기능 제거 시 dead code 누적 — 4서브탭 폐기 시 PB_SUBTABS / _renderPBSubtabs 같은 잔존 함수 점검 필요 (Phase 67에서 호출만 제거, 함수는 dead로 남음)
