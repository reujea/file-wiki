# 10: 컬럼 rename 시 인덱스 참조 불일치

## 상황
Phase 53 todos 테이블에서 doc_id → doc_ids 컬럼 rename 시 인덱스가 기존 컬럼명(doc_id)을 참조하여 스키마 생성 실패.

## 문제
settings_db 테스트 19/34 실패. CREATE INDEX ON todos(doc_id) → 존재하지 않는 컬럼 참조로 테이블 생성 자체 실패.

## 원인
컬럼명 변경 시 CREATE TABLE은 수정했으나 CREATE INDEX는 이전 컬럼명을 참조.

## 개선
컬럼 rename 시 해당 컬럼을 참조하는 모든 인덱스/쿼리를 함께 변경.
grep으로 이전 컬럼명의 모든 참조를 검색한 후 일괄 수정.

## 교훈
DB 스키마 변경 시 테이블 DDL + 인덱스 DDL + SELECT/INSERT/UPDATE 쿼리를 모두 확인해야 함.
