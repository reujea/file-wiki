---
created: 2026-05-22
purpose: TabPFN / Tabular Foundation Model 분석 단일 진실원
domain_match: 본질 도메인 불일치 (file-pipeline = 문서 가공·검색, TFM = 숫자 테이블 예측)
related: prd/research/external-analysis-2026-05-22.md, spec/lesson-learned/50_phase91-james-pattern-absorption.md
---

# TabPFN / Tabular Foundation Model 분석

## 1. 핵심 요약 (2026-05 시점)

| 차원 | 핵심 |
|------|------|
| 본질 | 표 형식 데이터(분류·회귀) 전용 파운데이션 모델. 학습 불필요 (in-context learning) |
| 경쟁 상대 | XGBoost / LightGBM / CatBoost / AutoML — LLM과 무관 |
| 현재 SOTA | TabPFN-3 / TabICLv2 / TabDPT / Mitra / LimiX / Orion-MSP |
| 라이선스 자유 | TabICLv2 (BSD) / Mitra (Apache-2.0) / TabDPT (Apache-2.0) |
| 통합 도구 | TabTune (Lexsi Labs, MIT) |
| 인프라 | H100 1장 또는 CPU (소규모 LimiX 2M) |

## 2. file-pipeline 도메인 정렬

### 본질 도메인 불일치
- file-pipeline = 자연어 문서 가공·검색·KG (텍스트 입력 → 가공본/검색 결과)
- TFM = 숫자/범주형 테이블 (숫자 행렬 → 클래스/연속값)
- → **본질 적용 불가**

### 부수 도메인 일치 영역
- `audit_trace` (Phase 91) / `decision_log` (Phase 84) / `processing_metrics` (Phase 82-prep) — **모두 표 형식**
- `LocalVectorStore` 메타 (doc_type/date/hierarchy/access_count) — 테이블화 가능
- 사용자 행동 로그 (search_mode / lint / verification 통계)

## 3. 적용 후보

### 🟢 적용 가능 (부수 도메인)

**G1 — `audit_trace` 이상 탐지** (Phase 91 A3 자연 확장)
- 입력: (stage, latency_ms, status, doc_type, hour_of_day, ...)
- 출력: 정상 / 이상
- 가치: 메타 룰 18 "추정 재검증" 인프라 — lesson 46 G-1 같은 사례 root cause 자동 검출

**G2 — 가공 ETA 예측** (Phase 82-prep 활용)
- 입력: (파일 크기, 확장자, doc_type, 시간대, fastembed 여부, ...)
- 출력: 예상 가공 시간 (회귀)
- 추천 모델: TabDPT (회귀 최강, Apache-2.0)

**G3 — TFM 인프라 선구현 (lesson 30 Ruflo 패턴)**
- 포트 trait + 디폴트 no-op 어댑터
- 토글 가능 config 필드 (`anomaly_detector_enabled: bool` 디폴트 false)

### 🔴 보류 (본질 도메인 불일치)
- 검색 리랭킹 TFM 대체 (Phase 62 Cross-Encoder 충분 + 라벨링 부재)
- doc_type 분류 LLM 대체 (자율 판단 + 새 유형 생성 불가)
- TabTune Python 라이브러리 직접 통합 (헥사고날 위반)

## 4. 추천 후보 (라이선스 + 도메인 적합도)

| 후보 | 모델 | 이유 |
|------|------|------|
| G1 이상 탐지 | TabICLv2 (BSD) | 가장 빠름 + 완전 오픈 |
| G2 ETA 예측 | TabDPT (Apache-2.0) | 회귀 최강 |
| 대안 | Mitra (Apache-2.0) | 소규모 강함 + CPU 지원 |

## 5. 진행 결정

**현재**: 본 분석은 분석 결과물. 즉시 코드 변경 권장 아님.

**트리거**:
- Phase 91 A3 trace_id 호출처 부착 완료 (메타 룰 13 2단계) 후 누적 데이터 1000건+ 도달 시 G1 활성화 검토
- `processing_metrics` 50파일+ 누적 후 G2 활성화 검토

**보류 (외부 협업/연계 보류 정책)**:
- TabTune Python 직접 통합

## 6. 메타 룰 적용

### 메타 룰 21 후보 강화 (lesson 50 후속)

"외부 도메인 도구 흡수 시 본질/부수 도메인 분리":
- TabPFN 사례 1건 추가
- Mirage와 함께 누적 2건 → META 정식 승격 임계 1건 추가
