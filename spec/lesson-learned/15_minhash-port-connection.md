---
date: 2026-04-28
phase: 59
---

# 미연결 MinHash 포트 → 사용처 연결

## 상황

- VectorDBPort에 `minhash_candidates(keywords) -> Vec<String>` + `minhash_enabled() -> bool` 두 메서드가 Phase 52에서 추가됨
- LocalVectorStore에 MinHashIndex 구현체도 들어가 있고 upsert 시 자동 등록됨
- 하지만 호출처가 **0건**. flush_crossref는 EmbeddingSnapshot을 받아서 N×M 행렬곱 cosine만 수행
- "5K+ 문서 도달 시 자동 활성화"라는 로드맵 문구가 있었지만 그 자동 활성화를 어디서 켜야 할지 코드에 명시되지 않음 (lesson-learned #14와 동일 패턴)

## 문제

- 트리거 대기 항목 (3K+ 문서 도달 시 활성)이지만 트리거가 와도 **자동 동작하지 않는 구조**였음
- minhash_enabled가 self.documents.len() >= 3000으로 하드코딩되어 있어 외부에서 끌 수도 강제로 켤 수도 없었음
- 강제 활성화/임계치 조정 옵션이 없어서 "벤치마크에서 효과만 측정하고 싶다"가 불가능

## 원인

1. 포트 메서드는 추가했지만 service.flush_crossref에서 호출하는 코드가 누락
2. minhash_enabled 시그니처가 인자를 받지 않아 설정 주입 불가
3. trait+impl만 만들어두고 cargo는 dead code로 잡지 못함 (#14와 동일)

## 개선

- 포트 시그니처를 `minhash_enabled_with(force: bool, min_docs: usize) -> bool`로 변경
- service에 `crossref_minhash_force` / `crossref_minhash_min_docs` 필드 추가, 설정에서 주입
- flush_crossref에서 활성 시 minhash_candidates로 후보 set 구성 → snapshot/flushed 양쪽 경로에서 사전 필터
- bench_crossref_variants 추가 — 변경의 효과를 즉시 측정할 수 있게 함

## 재발 방지

- 트리거 대기 항목을 만들 때는 **트리거가 도달해도 코드 수정 없이 켤 수 있는 형태**(설정 토글 + service 주입 + 호출처 연결)로 끝까지 작업한다. 포트 trait + impl만 만들고 호출처를 비워두면 #14와 같은 dead 자산이 된다.
- 호출처 0건 검증: 새 포트 메서드 추가 시 `grep -rn "메서드명" --include="*.rs"`로 호출처 확인. 1건 이상 없으면 PR 머지 보류.
