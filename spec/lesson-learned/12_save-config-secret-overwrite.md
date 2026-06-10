# 12. save_config에서 마스킹된 시크릿 덮어쓰기

## 상황
Settings에서 "기본으로 설정" 클릭 시 전체 크레덴셜이 삭제됨

## 문제
`get_config`가 시크릿 필드를 `"****"`로 마스킹 → JS `state.config`에 마스킹된 값 저장 → `setDefaultCredential`이 `API.updateConfig(state.config)` 호출 → `save_config`가 마스킹된 config를 그대로 저장 → API 키가 `"****"`로 덮어쓰기 + credentials 배열이 JS에 없어서 빈 배열로 교체

## 원인
`save_config`가 전체 config를 역직렬화하여 교체하는 구조에서, 마스킹된 시크릿을 복원하는 로직이 없었음

## 개선
- `restore_masked_secrets()` 함수 추가: `"****"` 값은 기존 config에서 복원
- `credentials` 배열은 항상 기존 값을 보존 (credentials는 save_credential/delete_credential로만 관리)
- **일반 규칙**: 마스킹된 데이터를 포함한 객체를 저장 API에 보내는 모든 경로에서 원본 복원 필요
