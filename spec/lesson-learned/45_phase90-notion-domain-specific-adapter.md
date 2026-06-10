---
phase: 90
date: 2026-05-19
topics: 도메인 특수 외부 솔루션 통합 / module-storage 위임 vs 직접 구현 / Notion API 제약
related_lessons: 17, 30, 38
related_meta_rules: 14, 16후보
---

# Lesson Learned: Phase 90 — 도메인 특수 외부 솔루션 통합 시 헥사고날 경계 결정

## 상황

Phase 89 외부 신호 대기 단계에서 사용자가 첫 요청: "원격 저장소에 Notion 추가". 기존 S3/WebDAV/Network 어댑터는 `module-storage` (형제 프로젝트 공유 모듈)의 raw 어댑터를 thin wrap. Notion도 같은 패턴이려면 `module-storage::NotionRemoteStorage` 추가가 자연스러우나, Notion은 **페이지/블록 기반 콘텐츠 플랫폼**이라 form-agnostic한 module-storage 인터페이스와 정합이 어긋남.

## 문제

1. **module-storage 위임 vs 어댑터 직접 구현**: module-storage는 "파일 → 원격 키" 추상화. Notion은 "블록 배열 → 페이지" 추상화. 강제로 위임하면 `module-storage`에 Notion 도메인이 침투 (헥사고날 위반)
2. **Notion API 제약**: 공식 API v1 (2022-06-28)이 zst 직접 업로드 미지원. file_upload v2024 별도 API + 복잡도 큼. RemoteStoragePort 4 메서드(upload/download/list/delete)와 자연스럽게 매핑되지 않음
3. **사용자 요청 vs 기술 제약 충돌**: "두 패턴 모두 지원 (mode 설정)" 답변 받았으나 `attach` 모드는 Notion API 제약상 실용 불가

## 원인

직접 원인:
- 기존 원격 저장소 어댑터 4종(Network/WebDAV/S3/Null)이 모두 파일 시스템 추상화에 맞춰 설계됨. Notion 도메인 특수성 사전 검토 없이 같은 인터페이스 강제하면 어색
- Notion API 문서의 file_upload v2024와 v1 API의 분리를 사전 조사 안 함 (메타 룰 3 외부 크레이트 소스 사전 읽기 누락)

구조적 원인:
- `RemoteStoragePort` trait이 "파일 시스템 기반 백업" 가정으로 설계됨 (upload/download/list/delete). 콘텐츠 플랫폼(Notion/Confluence/Wiki)에는 다른 추상화 필요
- module-storage 분리(Phase 60) 시 "원격 저장소"를 단일 카테고리로 묶었으나, 실제로는 "백업 저장소" vs "콘텐츠 플랫폼"으로 분기 가능

## 개선

### 헥사고날 경계 결정 패턴 (메타 룰 14 변형)

외부 솔루션 통합 시 사전 체크:

- [ ] **추상화 매칭 검증**: 솔루션의 핵심 단위(파일/페이지/블록/문서/메시지)가 기존 포트 추상화(upload/download)와 자연스럽게 매핑되는가?
- [ ] **module 위임 가능 여부**: form-agnostic 모듈에 추가 시 도메인 누수 없는가?
- [ ] **API 직접 호출 정당화**: 매핑 안 되면 module 외부에 직접 구현 + lesson에 결정 근거 명시
- [ ] **mode 분기로 한 포트에 압축**: 다른 추상화를 같은 포트로 노출해야 하면 mode 필드 + 일부 모드 명시적 미지원 (bail!)

본 phase 적용: Notion `page` 모드만 의미 있게 구현, `attach` 모드는 명시적 미지원 + S3/WebDAV 권장 안내.

### 외부 API 제약 사전 조사 의무 (메타 룰 3 사례)

새 외부 솔루션 통합 전 다음 항목 확인:

- [ ] 공식 API 버전 + 분기 (예: Notion v1 (2022-06-28) vs file_upload v2024)
- [ ] 파일 크기 / rate limit / 블록 크기 / 페이지 크기 제약
- [ ] 인증 방식 + 권한 부여 흐름 (예: Internal Integration token + 페이지 Connect to integration)
- [ ] hard delete vs archive (Notion은 archive만)

Notion 제약 (본 어댑터 주석에 명시):
- 평균 3 req/s rate limit
- 블록 children 한 번에 최대 100개
- rich_text content 2000자 제한
- archived=true만 (hard delete 없음)

### 도메인 특수 어댑터 분리 후보

`module-storage`는 파일 시스템 백업용. 콘텐츠 플랫폼(Notion/Confluence/Slack)은 별도 모듈 검토:

- `module-notion-api` (필요 시 분리)
- `module-confluence-api`
- `module-content-publish-api` (공통 추상화)

본 phase에서는 단일 솔루션(Notion)이라 file-pipeline-adapters에 직접 구현. 형제 프로젝트에서 같은 솔루션 활용 신호 도달 시 모듈 분리.

### attach 모드 후속

진짜 attach 구현 트리거 도달 시:
1. Notion file_upload v2024 API (`POST /v1/file_uploads`)
2. multi-part 업로드 (5MB 청크)
3. 페이지에 file block으로 참조

본 phase는 인프라(mode 필드 + UI 옵션 + 명시적 미지원 에러)만 완성. lesson 30 패턴 (인프라 + 디폴트 비활성).

## 잘한 것 (재사용 가능 성공 패턴)

1. **mode 분기로 다중 추상화 한 포트에 압축**: page/attach mode로 같은 RemoteStoragePort 인터페이스를 도메인 특수성에 맞게 분기. 사용자 답변("두 패턴 모두 지원")을 기술 제약 안에서 실현
2. **명시적 미지원 + 안내 메시지**: `bail!` + 대안 권장 (S3/WebDAV). dead 자산 회귀 0 + 사용자 혼란 0
3. **단위 테스트 6건 사전 작성**: API 호출 없이 검증 가능한 항목(key_to_title / text_to_blocks 분할 / mode 파싱)부터 테스트. 실제 API 호출 테스트는 통합 테스트 단계 (트리거 대기)

## 다음 세션 플래그

- 형제 프로젝트에서 Notion 요구 도달 시 `module-notion-api` 분리 검토
- file_upload v2024 attach 모드 구현 트리거: 사용자가 "Notion에 zst 직접 백업" 명시
- rate limit 자동 backoff 트리거: 실 가공에서 429 응답 도달

## 메타 룰 16 (사전 분류) — ✅ Phase 90에서 META.md 정식 승격

본 phase는 lesson 44 메타 룰 16(자동 측정 가능성, 차원 A)의 변형 — **외부 솔루션 통합 전 분류, 차원 B**. 두 차원을 결합하여 META.md 메타 룰 16으로 정식 승격:

- 🟢 **추상화 매칭 + module 위임 가능**: S3/WebDAV/Network — 즉시 통합
- 🟡 **추상화 부분 매칭 + 직접 구현**: Notion (본 phase) — mode 분기 또는 일부 미지원
- 🔴 **추상화 불일치**: Slack 알림 / Discord webhook — 다른 포트(NotificationPort)에 매핑 검토

새 외부 솔루션 통합 전 사전 분류 후 진입.
