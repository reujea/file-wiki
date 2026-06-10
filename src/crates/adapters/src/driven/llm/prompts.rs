//! LLM 프롬프트 — file-pipeline 도메인 콘텐츠 + 빌더.
//!
//! 한국어 프롬프트 콘텐츠와 `DocTypeRegistry` 의존 빌더는 file-pipeline에 잔류.
//! TOML 핫 리로드 + 변수 치환 generic 엔진은 [`module_llm_prompts::TemplateEngine`] 위임.

use std::path::PathBuf;
use std::sync::OnceLock;

use file_pipeline_core::domain::models::DocTypeRegistry;
use module_llm_prompts::{SectionSpec, TemplateEngine};

pub const CHUNK_SIZE: usize = 40_000;

const NAME_CLASSIFY: &str = "classify";
const NAME_REPROCESS: &str = "reprocess";
const NAME_SUMMARIZE: &str = "summarize_text";
const NAME_HYDE: &str = "hyde";

const DEFAULT_CLASSIFY: &str = r#"당신은 문서 분류 및 가공 전문가입니다.

아래 문서를 분석하여 JSON 형식으로 응답하세요.

## 작업

1단계: 노이즈 제거 — 네비게이션, 광고, 중복 헤더/푸터, UI 잔여물("Edit this page" 등)을 제거하세요.
2단계: 문서 유형을 판단하세요 (복수 가능). 아래 알려진 유형에서 선택하거나, 적합한 유형이 없으면 새 id를 만드세요.
3단계: 각 유형의 섹션 형식에 맞춰 가공하세요. 각 섹션이 다른 섹션 없이도 독립적으로 이해 가능하도록 맥락을 포함하세요.
4단계: 메타데이터를 추출하세요.

{type_hints}

## 출력 형식 (반드시 아래 JSON만 출력)

```json
{{
  "doc_types": ["meeting", "todo"],
  "rationale": "회의록이면서 할일 목록 포함",
  "date": "2026-04-05",
  "summary": "2~3문장 핵심 요약",
  "keywords": ["키워드1", "키워드2", "...최대15개"],
  "search_hints": ["이 문서가 검색될 만한 질문이나 키워드 3~5개"],
  "sections": {{
    "결정사항": ["MyDocSearch 통합 불필요", "Qdrant 유지 결정"],
    "액션아이템": ["이개발: .vec 구현", "박기획: 와이어프레임"]
  }},
  "entities": [
    {{"name": "김철수", "type": "person"}},
    {{"name": "마케팅팀", "type": "organization"}},
    {{"name": "Next.js", "type": "technology"}},
    {{"name": "500만원", "type": "amount"}},
    {{"name": "프로젝트 A", "type": "project"}}
  ],
  "code_blocks": [
    {{"language": "yaml", "description": "배포 설정 예시", "code": "apiVersion: v1\nkind: Pod"}}
  ],
  "needs_verification": [
    "Redis 클러스터 자동 페일오버 설정 — 원문에 명시 없음, 운영 정책 확인 필요"
  ],
  "open_questions": [
    "롤백 시 트랜잭션 로그 보존 기간은?",
    "스테이징과 프로덕션의 인덱스 동기화 주기?"
  ],
  "content": "=== meeting ===\n결정사항...\n=== todo ===\n[ ] 항목..."
}}
```

## 규칙
- doc_types: 1개 이상, 가장 적합한 유형부터 나열
- date: 문서에서 추출, 없으면 오늘 날짜
- keywords: 10~15개, 원본에 실제 등장하는 단어
- search_hints: 사용자가 이 문서를 찾을 때 입력할 만한 질문/키워드 3~5개
- sections: 유형의 필수 섹션을 키로, 각 섹션의 핵심 항목을 문자열 배열로. 반드시 포함
- entities: 문서에서 추출한 개체 목록. type은 person/organization/place/technology/amount/project/concept 중 선택. 중요한 개체만 추출 (최대 20개)
- code_blocks: 원본의 코드블록을 구조화. 없으면 빈 배열. language와 description을 명시
- needs_verification: 원문에 명시되지 않거나 추가 검증/확인이 필요한 사항 목록 (0~3개). 없으면 빈 배열
- open_questions: 원문으로 답할 수 없지만 향후 답이 필요한 질문 목록 (0~3개). 없으면 빈 배열
- content: 유형별 === 섹션명 === 구분자 사용, 원본 핵심 정보 보존
- 약어는 첫 등장 시 풀어쓰기 (예: K8s → K8s(Kubernetes))
- 숫자, 날짜, 고유명사, 코드블록은 반드시 보존
- JSON 외 다른 텍스트 출력 금지

중요: 반드시 유효한 JSON만 출력하세요. JSON 외 텍스트(설명, 코멘트)를 포함하지 마세요.

## 문서

파일명: {filename}

{content}"#;

const DEFAULT_REPROCESS_SUFFIX: &str = r#"

## 피드백 (이전 가공이 실패한 이유)

{feedback}

위 피드백을 반영하여 다시 가공하세요."#;

const DEFAULT_MERGE_TODO: &str = r#"기존 할일 목록과 새 할일을 병합하세요.
중복은 제거하고, 아이젠하워 매트릭스(긴급+중요 / 중요+여유 / 위임 / 나중에)로 정리하세요.
형식: [ ] 내용 | 기한 | 태그

## 기존 할일

{existing}

## 새 할일

{new_content}

병합 결과만 출력하세요."#;

/// Phase 89 #6 HyDE 폴백 프롬프트 — query에 대한 가상 답변을 한 문단(2~4문장)으로 생성.
const DEFAULT_HYDE: &str = r#"당신은 사용자의 검색 의도를 이해하고, 그 의도에 맞는 가상의 문서 한 문단을 작성하는 전문가입니다.

사용자가 다음과 같이 검색했습니다:

{query}

이 검색에 가장 잘 부합하는 가상의 문서 한 문단(2~4문장)을 한국어로 작성하세요. 문체는 기술 문서/매뉴얼/메모 스타일을 사용하고, 사실로 보일 만한 구체적인 용어와 키워드를 포함하세요. 단, 출처가 없는 수치는 일반적인 표현으로 대체하세요. 설명·해설·메타 코멘트 없이 본문만 출력하세요."#;

const SECTIONS: &[SectionSpec] = &[
    SectionSpec { name: NAME_CLASSIFY, toml_key: "classify.template", default: DEFAULT_CLASSIFY },
    SectionSpec { name: NAME_REPROCESS, toml_key: "reprocess.suffix", default: DEFAULT_REPROCESS_SUFFIX },
    SectionSpec { name: NAME_SUMMARIZE, toml_key: "summarize_text.template", default: DEFAULT_MERGE_TODO },
    SectionSpec { name: NAME_HYDE, toml_key: "hyde.template", default: DEFAULT_HYDE },
];

fn engine() -> &'static TemplateEngine {
    static ENGINE: OnceLock<TemplateEngine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let e = TemplateEngine::new(SECTIONS);
        if let Some(path) = find_prompts_toml() {
            match e.load_from_toml(&path) {
                Ok(_) => tracing::info!("프롬프트 외부 파일 로드: {}", path.display()),
                Err(err) => tracing::warn!("prompts.toml 파싱 실패, 기본값 사용: {err}"),
            }
        }
        e
    })
}

/// prompts.toml 탐색 경로: cwd → exe 디렉토리
fn find_prompts_toml() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("prompts.toml"),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("prompts.toml")))
            .unwrap_or_default(),
    ];
    TemplateEngine::find_path(&candidates)
}

/// prompts.toml 재로드 (핫 리로드)
pub fn reload_prompts() {
    if let Err(e) = engine().reload() {
        tracing::warn!("프롬프트 재로드 실패: {}", e);
    }
}

/// 외부에서 프롬프트를 직접 주입 (SettingsDb 연동용)
pub fn inject_prompts(classify: Option<&str>, reprocess_suffix: Option<&str>, summarize_text: Option<&str>) {
    engine().inject(&[
        (NAME_CLASSIFY, classify),
        (NAME_REPROCESS, reprocess_suffix),
        (NAME_SUMMARIZE, summarize_text),
    ]);
    tracing::info!("프롬프트 주입 완료");
}

/// Phase 89 #6 HyDE 폴백 프롬프트 빌더.
pub fn build_hyde_prompt(query: &str) -> String {
    engine().render(NAME_HYDE, &[("query", query)])
}

/// 현재 프롬프트 내용을 TOML 형식 문자열로 반환 (UI 편집용)
pub fn get_prompts_content() -> String {
    engine().to_toml_string()
}

/// prompts.toml 저장 (UI 편집 후) + 핫 리로드
pub fn save_prompts(content: &str) -> Result<(), String> {
    let path = find_prompts_toml().unwrap_or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("prompts.toml")))
            .unwrap_or_else(|| PathBuf::from("prompts.toml"))
    });
    engine().save(content, &path).map_err(|e| e.to_string())?;
    tracing::info!("prompts.toml 저장 + 핫 리로드: {}", path.display());
    Ok(())
}

// ── 도메인 빌더 ──

pub fn build_type_hints(registry: &DocTypeRegistry) -> String {
    if registry.all().is_empty() {
        return String::from("## 문서 유형\n\n유형은 자율 판단하세요.\n");
    }
    let mut hints = String::from("## 알려진 문서 유형 (검증 스키마)\n\n");
    for def in registry.all() {
        if def.sections.is_empty() {
            hints.push_str(&format!("- **{}** ({})\n", def.id, def.label_ko));
        } else {
            hints.push_str(&format!(
                "- **{}** ({}): 섹션=[{}]\n",
                def.id, def.label_ko, def.sections.join(", "),
            ));
        }
    }
    hints.push_str("\n위 유형에 해당하지 않으면 적합한 id를 새로 만드세요.\n");
    hints
}

pub fn build_classify_prompt(filename: &str, content: &str, type_hints: &str) -> String {
    engine().render(NAME_CLASSIFY, &[
        ("type_hints", type_hints),
        ("filename", filename),
        ("content", content),
    ])
}

pub fn build_summarize_text_prompt(new_content: &str, existing: &str) -> String {
    engine().render(NAME_SUMMARIZE, &[
        ("existing", existing),
        ("new_content", new_content),
    ])
}

pub fn build_reprocess_prompt(
    filename: &str,
    content: &str,
    type_hints: &str,
    feedback: &str,
) -> String {
    let base = build_classify_prompt(filename, content, type_hints);
    let suffix = engine().render(NAME_REPROCESS, &[("feedback", feedback)]);
    format!("{base}{suffix}")
}

pub fn truncate_content(content: &str) -> &str {
    if content.len() > CHUNK_SIZE {
        &content[..CHUNK_SIZE]
    } else {
        content
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_classify_contains_filename_and_hints() {
        let prompt = build_classify_prompt("test.txt", "내용", "## 유형 힌트");
        assert!(prompt.contains("test.txt"));
        assert!(prompt.contains("내용"));
        assert!(prompt.contains("## 유형 힌트"));
    }

    #[test]
    fn build_reprocess_includes_feedback() {
        let prompt = build_reprocess_prompt("f.txt", "content", "hints", "구조 누락");
        assert!(prompt.contains("구조 누락"));
        assert!(prompt.contains("피드백"));
    }

    #[test]
    fn default_templates_have_required_vars() {
        assert!(DEFAULT_CLASSIFY.contains("{type_hints}"));
        assert!(DEFAULT_CLASSIFY.contains("{filename}"));
        assert!(DEFAULT_CLASSIFY.contains("{content}"));
        assert!(DEFAULT_REPROCESS_SUFFIX.contains("{feedback}"));
        assert!(DEFAULT_MERGE_TODO.contains("{existing}"));
        assert!(DEFAULT_MERGE_TODO.contains("{new_content}"));
    }

    #[test]
    fn get_prompts_content_lists_sections() {
        let content = get_prompts_content();
        assert!(content.contains("[classify]"));
        assert!(content.contains("[summarize_text]"));
        assert!(content.contains("[reprocess]"));
    }

    #[test]
    fn save_invalid_toml_returns_error() {
        let result = save_prompts("{{{{invalid toml");
        assert!(result.is_err());
    }
}
