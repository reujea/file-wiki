---
created: 2026-04-27
status: concluded — Phase 58에서 코드 삭제 확정
---

# GraphDB 교차참조 통합 — 전문가 자문 결론

## 결론

**방안 A(이중 저장) 보류. GraphDB 코드 삭제 (Phase 58). 실사용 전환 우선.**

**Phase 58 후속 조치**: `graph_db/` 디렉토리(json_graph.rs + neo4j_graph.rs) + `GraphDBPort` trait 완전 삭제. 포트 12→11개. 재활성화 시 git history 참조.

- GraphDB(JSON/_graph.json)는 실사용 가치 미검증 (0일)
- KgQueryEngine이 이미 LocalVectorStore.find_related 기반으로 동작
- GraphDB 코드는 삭제하지 않고 비활성 유지
- KG 시각화: 전체 N+1 → ego graph(1회 API 호출)로 전환
- 저장소 구조: 3-tier(LocalVectorStore+GraphDB+Storage) → 2-tier(LocalVectorStore+Storage)

## 적용 완료 사항

- config_metadata에서 graph_db 섹션 제거
- Settings UI 인프라 그룹에서 graph_db 제거 (외부저장소+리랭킹만 유지)
- Dashboard KG를 ego graph로 전환 (N+1 → 1회 API 호출)

## 재검토 트리거

| 조건 | 판단 |
|------|------|
| "이 문서를 작성한 사람이 다른 어떤 문서를 썼는지" | GraphDB 재활성화 |
| "K8s 관련 엔티티를 전부 보여줘" | GraphDB 재활성화 |
| "관련 문서 보여줘" | find_related로 충분 → GraphDB 불필요 |
| 2주 실사용 후 엔티티 쿼리 요구 없음 | GraphDB 제거 확정 |
