//! 비텍스트 전처리 어댑터 — 확장자별 라우팅
//!
//! .txt/.md → 직접 읽기
//! .pdf → 외부 도구 (marker/pymupdf4llm) CLI 호출
//! .png/.jpg → OCR (tesseract) 또는 Claude Vision
//! .docx/.hwp → 외부 도구 CLI 호출

use std::io::Read as IoRead;
use std::path::Path;
use std::process::Command;

/// Windows에서 콘솔 창 없이 프로세스 실행
#[cfg(windows)]
fn hide_window(cmd: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000)
}

#[cfg(not(windows))]
fn hide_window(cmd: &mut Command) -> &mut Command { cmd }

use anyhow::{Context, Result};
use file_pipeline_core::domain::models::PreprocessResult;
use file_pipeline_core::ports::output::PreprocessPort;

/// 호스트에 설치된 변환 도구
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HostTool {
    Pandoc,
    PythonDocx,
    PythonOpenpyxl,
    LibreOffice,
}

impl HostTool {
    /// 캐시 직렬화용 키 (settings.db 저장)
    pub fn as_key(&self) -> &'static str {
        match self {
            HostTool::Pandoc => "pandoc",
            HostTool::PythonDocx => "python_docx",
            HostTool::PythonOpenpyxl => "python_openpyxl",
            HostTool::LibreOffice => "libreoffice",
        }
    }
    pub fn from_key(s: &str) -> Option<HostTool> {
        match s {
            "pandoc" => Some(HostTool::Pandoc),
            "python_docx" => Some(HostTool::PythonDocx),
            "python_openpyxl" => Some(HostTool::PythonOpenpyxl),
            "libreoffice" => Some(HostTool::LibreOffice),
            _ => None,
        }
    }
    pub fn install_hint(&self) -> &'static str {
        match self {
            HostTool::Pandoc => "https://pandoc.org/installing.html",
            HostTool::PythonDocx => "pip install python-docx",
            HostTool::PythonOpenpyxl => "pip install openpyxl",
            HostTool::LibreOffice => "https://libreoffice.org",
        }
    }
    pub fn all() -> [HostTool; 4] {
        [HostTool::Pandoc, HostTool::PythonDocx, HostTool::PythonOpenpyxl, HostTool::LibreOffice]
    }
}

/// 호스트 도구 감지기
pub struct HostToolDetector;

impl HostToolDetector {
    /// 호스트에 설치된 도구 목록 감지 (외부 프로세스 실행 — 느림)
    pub fn detect() -> Vec<(HostTool, String)> {
        let mut tools = vec![];

        // pandoc
        if let Ok(output) = hide_window(Command::new("pandoc").arg("--version")).output() {
            if output.status.success() {
                let ver = String::from_utf8_lossy(&output.stdout);
                let version = ver.lines().next().unwrap_or("pandoc").to_string();
                tools.push((HostTool::Pandoc, version));
            }
        }

        // python + python-docx
        if let Ok(output) = hide_window(Command::new("python").args(["-c", "import docx; print(docx.__version__)"])).output() {
            if output.status.success() {
                let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
                tools.push((HostTool::PythonDocx, format!("python-docx {}", ver)));
            }
        }

        // python + openpyxl
        if let Ok(output) = hide_window(Command::new("python").args(["-c", "import openpyxl; print(openpyxl.__version__)"])).output() {
            if output.status.success() {
                let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
                tools.push((HostTool::PythonOpenpyxl, format!("openpyxl {}", ver)));
            }
        }

        // libreoffice
        if let Ok(output) = hide_window(Command::new("soffice").arg("--version")).output() {
            if output.status.success() {
                let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
                tools.push((HostTool::LibreOffice, ver));
            }
        }

        tools
    }

    /// detect() 결과를 found + not_found 모두 포함하는 전체 캐시 행으로 변환.
    /// 설치된 도구는 not_found=false, 미설치는 not_found=true로 음성 캐시.
    pub fn detect_full() -> Vec<(HostTool, Option<String>)> {
        let found: std::collections::HashMap<_, _> = Self::detect().into_iter().collect();
        HostTool::all().iter().map(|t| {
            (t.clone(), found.get(t).cloned())
        }).collect()
    }

    /// 확장자별 최적 도구 선택
    pub fn best_tool_for(ext: &str, available: &[(HostTool, String)]) -> Option<HostTool> {
        let has = |t: &HostTool| available.iter().any(|(tool, _)| tool == t);
        match ext {
            "docx" => {
                if has(&HostTool::Pandoc) { Some(HostTool::Pandoc) }
                else if has(&HostTool::PythonDocx) { Some(HostTool::PythonDocx) }
                else if has(&HostTool::LibreOffice) { Some(HostTool::LibreOffice) }
                else { None }
            }
            "xlsx" | "xls" => {
                if has(&HostTool::PythonOpenpyxl) { Some(HostTool::PythonOpenpyxl) }
                else if has(&HostTool::Pandoc) { Some(HostTool::Pandoc) }
                else if has(&HostTool::LibreOffice) { Some(HostTool::LibreOffice) }
                else { None }
            }
            "pptx" => {
                if has(&HostTool::Pandoc) { Some(HostTool::Pandoc) }
                else if has(&HostTool::LibreOffice) { Some(HostTool::LibreOffice) }
                else { None }
            }
            "hwp" | "hwpx" => {
                if has(&HostTool::LibreOffice) { Some(HostTool::LibreOffice) }
                else { None }
            }
            _ => None,
        }
    }
}

/// 확장자별 라우팅 전처리기
pub struct CompositePreprocessor {
    pdf_tool: String,
    ocr_tool: String,
    /// 호스트에 감지된 도구 목록 (시작 시 1회 감지)
    host_tools: Vec<(HostTool, String)>,
}

impl CompositePreprocessor {
    pub fn new(pdf_tool: &str, ocr_tool: &str) -> Self {
        // Phase 81: 호출자(shared::build_service)가 캐시된 도구 목록을 with_tools로
        // 주입하는 것이 기본. 직접 new() 호출 시는 fallback으로 즉시 감지.
        let host_tools = HostToolDetector::detect();
        if !host_tools.is_empty() {
            tracing::info!("호스트 전처리 도구 감지(fallback, 비캐시): {}",
                host_tools.iter().map(|(_, v)| v.as_str()).collect::<Vec<_>>().join(", "));
        }
        Self {
            pdf_tool: pdf_tool.to_string(),
            ocr_tool: ocr_tool.to_string(),
            host_tools,
        }
    }

    /// Phase 81: 캐시된 host_tools 주입 (외부 프로세스 spawn 없음)
    pub fn with_tools(pdf_tool: &str, ocr_tool: &str, host_tools: Vec<(HostTool, String)>) -> Self {
        Self {
            pdf_tool: pdf_tool.to_string(),
            ocr_tool: ocr_tool.to_string(),
            host_tools,
        }
    }

    fn read_plain_text(path: &Path) -> Result<PreprocessResult> {
        let text = Self::read_text_with_encoding(path)
            .context(format!("텍스트 파일 읽기 실패: {:?}", path))?;
        Ok(PreprocessResult { text, images: vec![], tables: vec![] })
    }

    /// UTF-8 우선, 실패 시 인코딩 자동 감지 (EUC-KR, Shift-JIS 등)
    fn read_text_with_encoding(path: &Path) -> Result<String> {
        let bytes = std::fs::read(path)
            .context(format!("파일 읽기 실패: {:?}", path))?;

        // 1. BOM 포함 UTF-8
        if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            return Ok(String::from_utf8_lossy(&bytes[3..]).into_owned());
        }

        // 2. UTF-8 시도
        if let Ok(text) = String::from_utf8(bytes.clone()) {
            return Ok(text);
        }

        // 3. 인코딩 자동 감지
        let mut detector = chardetng::EncodingDetector::new();
        detector.feed(&bytes, true);
        let encoding = detector.guess(None, true);

        let (text, _, had_errors) = encoding.decode(&bytes);
        if had_errors {
            tracing::warn!("[encoding] {:?} → {} (일부 문자 변환 실패)", path, encoding.name());
        } else {
            tracing::info!("[encoding] {:?} → {} 자동 감지", path, encoding.name());
        }

        Ok(text.into_owned())
    }

    /// CSV/TSV: 헤더 + 통계 + 상위 레코드 추출
    fn process_csv(path: &Path) -> Result<PreprocessResult> {
        let content = Self::read_text_with_encoding(path)
            .context("CSV 파일 읽기 실패")?;
        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return Ok(PreprocessResult { text: String::new(), images: vec![], tables: vec![] });
        }

        let header = lines[0];
        let col_count = header.split([',', '\t']).count();
        let row_count = lines.len() - 1;

        let mut result = format!(
            "=== CSV 분석 ===\n파일: {}\n컬럼: {} 개\n행: {} 개\n\n헤더: {}\n",
            path.file_name().unwrap_or_default().to_string_lossy(),
            col_count, row_count, header,
        );

        // 상위 10행 샘플
        result.push_str("\n=== 샘플 (상위 10행) ===\n");
        for line in lines.iter().skip(1).take(10) {
            result.push_str(line);
            result.push('\n');
        }

        // 전체 내용 (LLM이 분석할 수 있도록)
        if lines.len() <= 100 {
            result.push_str("\n=== 전체 데이터 ===\n");
            result.push_str(&content);
        }

        Ok(PreprocessResult { text: result, images: vec![], tables: vec![header.to_string()] })
    }

    /// 로그 파일: 에러/경고 패턴만 추출
    fn process_log(path: &Path) -> Result<PreprocessResult> {
        let content = Self::read_text_with_encoding(path)
            .context("로그 파일 읽기 실패")?;
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // 에러/경고 패턴 필터
        let error_patterns = ["error", "ERROR", "Error", "FATAL", "fatal", "panic", "PANIC"];
        let warn_patterns = ["warn", "WARN", "Warning", "WARNING"];

        let errors: Vec<&str> = lines.iter()
            .filter(|l| error_patterns.iter().any(|p| l.contains(p)))
            .copied()
            .collect();
        let warnings: Vec<&str> = lines.iter()
            .filter(|l| warn_patterns.iter().any(|p| l.contains(p)))
            .copied()
            .collect();

        let mut result = format!(
            "=== 로그 분석 ===\n파일: {}\n총 줄: {}\n에러: {} 건\n경고: {} 건\n\n",
            path.file_name().unwrap_or_default().to_string_lossy(),
            total_lines, errors.len(), warnings.len(),
        );

        if !errors.is_empty() {
            result.push_str("=== 에러 ===\n");
            for (i, line) in errors.iter().take(50).enumerate() {
                result.push_str(&format!("{}. {}\n", i + 1, line));
            }
            if errors.len() > 50 {
                result.push_str(&format!("... 외 {} 건\n", errors.len() - 50));
            }
        }

        if !warnings.is_empty() {
            result.push_str("\n=== 경고 ===\n");
            for (i, line) in warnings.iter().take(20).enumerate() {
                result.push_str(&format!("{}. {}\n", i + 1, line));
            }
        }

        // 마지막 20줄 (최근 컨텍스트)
        result.push_str("\n=== 최근 로그 (마지막 20줄) ===\n");
        for line in lines.iter().rev().take(20).rev() {
            result.push_str(line);
            result.push('\n');
        }

        Ok(PreprocessResult { text: result, images: vec![], tables: vec![] })
    }

    fn process_pdf(&self, path: &Path) -> Result<PreprocessResult> {
        match self.pdf_tool.as_str() {
            "marker" => {
                let mut cmd = Command::new("marker_single");
                cmd.arg(path).args(["--output_format", "markdown"]);
                hide_window(&mut cmd);
                let output = cmd.output()
                    .context("marker 실행 실패 (pip install marker-pdf)")?;
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                if text.is_empty() {
                    // fallback: 단순 텍스트 추출 시도
                    return Self::read_plain_text(path);
                }
                Ok(PreprocessResult { text, images: vec![], tables: vec![] })
            }
            "pymupdf4llm" => {
                let python = std::env::var("PYMUPDF_PYTHON")
                    .unwrap_or_else(|_| "python".to_string());
                let script = format!(
                    "import sys; sys.stdout.reconfigure(encoding='utf-8'); \
                     import pymupdf4llm; print(pymupdf4llm.to_markdown(r'{}'))",
                    path.display()
                );
                let mut cmd = Command::new(&python);
                cmd.args(["-c", &script]);
                #[cfg(windows)]
                hide_window(&mut cmd);
                let output = cmd.output()
                    .context(format!("pymupdf4llm 실행 실패 (python={})", python))?;
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    tracing::warn!("pymupdf4llm 오류: {}", &stderr[..stderr.len().min(200)]);
                    return Self::read_plain_text(path);
                }
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                if text.trim().is_empty() {
                    tracing::warn!("pymupdf4llm: 빈 결과 — 텍스트 폴백: {:?}", path);
                    return Self::read_plain_text(path);
                }
                Ok(PreprocessResult { text, images: vec![], tables: vec![] })
            }
            _ => {
                tracing::warn!("PDF 도구 미설정 — 텍스트 추출 시도");
                Self::read_plain_text(path)
            }
        }
    }

    fn process_image(&self, path: &Path) -> Result<PreprocessResult> {
        match self.ocr_tool.as_str() {
            "tesseract" => {
                let mut cmd = Command::new("tesseract");
                cmd.arg(path).arg("stdout").args(["-l", "kor+eng"]);
                hide_window(&mut cmd);
                let output = cmd.output()
                    .context("tesseract 실행 실패 (apt install tesseract-ocr)")?;
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                Ok(PreprocessResult {
                    text,
                    images: vec![path.to_path_buf()],
                    tables: vec![],
                })
            }
            "claude_vision" => {
                let mut cmd = Command::new("claude");
                cmd.arg("-p")
                    .arg(format!(
                        "이 이미지의 모든 텍스트를 추출하고, 표가 있으면 마크다운 표로 변환하세요. 다이어그램은 텍스트로 설명하세요. 이미지 경로: {}",
                        path.display()
                    ))
                    .args(["--output-format", "text"]);
                #[cfg(windows)]
                hide_window(&mut cmd);
                let output = cmd.output()
                    .context("claude vision 실행 실패")?;
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                Ok(PreprocessResult {
                    text,
                    images: vec![path.to_path_buf()],
                    tables: vec![],
                })
            }
            _ => {
                // Claude CLI가 있으면 자동으로 Vision 사용
                let has_claude = {
                    let mut vc = Command::new("claude");
                    vc.arg("--version");
                    #[cfg(windows)]
                    hide_window(&mut vc);
                    vc.output().map(|o| o.status.success()).unwrap_or(false)
                };
                if has_claude {
                    tracing::info!("OCR 미설정 → Claude Vision 자동 감지: {:?}", path);
                    let mut cmd = Command::new("claude");
                    cmd.arg("-p")
                        .arg(format!(
                            "이 이미지의 모든 텍스트를 추출하고, 표가 있으면 마크다운 표로 변환하세요. 이미지 경로: {}",
                            path.display()
                        ))
                        .args(["--output-format", "text"]);
                    #[cfg(windows)]
                    hide_window(&mut cmd);
                    let output = cmd.output()
                        .context("claude vision 자동 실행 실패")?;
                    let text = String::from_utf8_lossy(&output.stdout).to_string();
                    if !text.trim().is_empty() {
                        return Ok(PreprocessResult { text, images: vec![path.to_path_buf()], tables: vec![] });
                    }
                }
                tracing::warn!("OCR 도구 미설정/Vision 실패 — placeholder: {:?}", path);
                Ok(PreprocessResult {
                    text: format!("[이미지: {}]", path.file_name().unwrap_or_default().to_string_lossy()),
                    images: vec![path.to_path_buf()],
                    tables: vec![],
                })
            }
        }
    }

    /// Rust 네이티브 DOCX 텍스트 추출 (zip + XML 파싱)
    fn native_docx(path: &Path) -> Result<PreprocessResult> {
        let file = std::fs::File::open(path)
            .context(format!("DOCX 파일 열기 실패: {:?}", path))?;
        let mut archive = zip::ZipArchive::new(file)
            .context("DOCX ZIP 아카이브 파싱 실패")?;

        let mut text = String::new();

        // word/document.xml에서 텍스트 추출
        if let Ok(mut entry) = archive.by_name("word/document.xml") {
            let mut xml = String::new();
            entry.read_to_string(&mut xml).context("document.xml 읽기 실패")?;
            text.push_str(&Self::extract_text_from_xml(&xml));
        }

        // word/header*.xml, word/footer*.xml에서도 텍스트 추출
        let names: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
            .filter(|n| (n.starts_with("word/header") || n.starts_with("word/footer")) && n.ends_with(".xml"))
            .collect();
        for name in names {
            if let Ok(mut entry) = archive.by_name(&name) {
                let mut xml = String::new();
                if entry.read_to_string(&mut xml).is_ok() {
                    let part_text = Self::extract_text_from_xml(&xml);
                    if !part_text.trim().is_empty() {
                        text.push('\n');
                        text.push_str(&part_text);
                    }
                }
            }
        }

        if text.trim().is_empty() {
            anyhow::bail!("DOCX에서 텍스트를 추출할 수 없습니다");
        }

        Ok(PreprocessResult { text, images: vec![], tables: vec![] })
    }

    /// XML에서 <w:t> 태그의 텍스트만 추출 (간단한 파서)
    fn extract_text_from_xml(xml: &str) -> String {
        let mut result = String::new();
        let mut in_paragraph = false;
        let mut paragraph_text = String::new();

        // <w:p> = 단락, <w:t> = 텍스트 런, <w:tab/> = 탭, <w:br/> = 줄바꿈
        let mut pos = 0;
        let bytes = xml.as_bytes();
        while pos < bytes.len() {
            if bytes[pos] == b'<' {
                // 태그 시작
                let tag_end = xml[pos..].find('>').map(|i| pos + i + 1).unwrap_or(bytes.len());
                let tag = &xml[pos..tag_end];

                if tag.starts_with("<w:p ") || tag == "<w:p>" {
                    in_paragraph = true;
                    paragraph_text.clear();
                } else if tag == "</w:p>" {
                    if in_paragraph && !paragraph_text.trim().is_empty() {
                        result.push_str(paragraph_text.trim());
                        result.push('\n');
                    }
                    in_paragraph = false;
                } else if tag.starts_with("<w:t") && in_paragraph {
                    // <w:t> 또는 <w:t xml:space="preserve">
                    let text_start = tag_end;
                    if let Some(end_offset) = xml[text_start..].find("</w:t>") {
                        let text_content = &xml[text_start..text_start + end_offset];
                        paragraph_text.push_str(text_content);
                        pos = text_start + end_offset + 6; // skip </w:t>
                        continue;
                    }
                } else if (tag == "<w:tab/>" || tag == "<w:tab />") && in_paragraph {
                    paragraph_text.push('\t');
                } else if (tag == "<w:br/>" || tag == "<w:br />") && in_paragraph {
                    paragraph_text.push('\n');
                }

                pos = tag_end;
            } else {
                pos += 1;
            }
        }

        result
    }

    /// Rust 네이티브 XLSX 텍스트 추출 (calamine)
    fn native_xlsx(path: &Path) -> Result<PreprocessResult> {
        use calamine::{Reader, open_workbook_auto};

        let mut workbook = open_workbook_auto(path)
            .context(format!("XLSX 파일 열기 실패: {:?}", path))?;

        let sheet_names: Vec<String> = workbook.sheet_names().to_vec();
        let mut text = String::new();
        let mut tables = Vec::new();

        for name in &sheet_names {
            if let Ok(range) = workbook.worksheet_range(name) {
                let rows: Vec<_> = range.rows().collect();
                if rows.is_empty() {
                    continue;
                }

                text.push_str(&format!("=== {} ===\n", name));

                // 헤더 기록
                if let Some(header_row) = rows.first() {
                    let header: String = header_row.iter()
                        .map(|c| format!("{}", c))
                        .collect::<Vec<_>>()
                        .join("\t");
                    tables.push(header.clone());
                    text.push_str(&header);
                    text.push('\n');
                }

                // 데이터 행
                for row in rows.iter().skip(1) {
                    let line: String = row.iter()
                        .map(|c| format!("{}", c))
                        .collect::<Vec<_>>()
                        .join("\t");
                    text.push_str(&line);
                    text.push('\n');
                }
                text.push('\n');
            }
        }

        if text.trim().is_empty() {
            anyhow::bail!("XLSX에서 데이터를 추출할 수 없습니다");
        }

        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        let header = format!(
            "=== XLSX 분석 ===\n파일: {}\n시트: {} 개\n\n",
            filename,
            sheet_names.len(),
        );

        Ok(PreprocessResult {
            text: format!("{}{}", header, text),
            images: vec![],
            tables,
        })
    }

    /// 오피스 문서 처리 — 호스트 도구 자동 감지 + subprocess
    fn process_office(&self, path: &Path, ext: &str) -> Result<PreprocessResult> {
        let tool = HostToolDetector::best_tool_for(ext, &self.host_tools);
        let _filename = path.file_name().unwrap_or_default().to_string_lossy();

        match tool {
            Some(HostTool::Pandoc) => {
                tracing::info!("전처리 [pandoc]: {:?}", path);
                let mut cmd = Command::new("pandoc");
                cmd.arg(path).args(["-t", "plain", "--wrap=none"]);
                #[cfg(windows)]
                hide_window(&mut cmd);
                let output = cmd.output()
                    .context(format!("pandoc 실행 실패: {:?}", path))?;
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    anyhow::bail!("pandoc 오류: {}", stderr);
                }
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                Ok(PreprocessResult { text, images: vec![], tables: vec![] })
            }

            Some(HostTool::PythonDocx) if ext == "docx" => {
                tracing::info!("전처리 [python-docx]: {:?}", path);
                let script = format!(
                    "from docx import Document\nd=Document(r'{}')\nfor p in d.paragraphs:\n    print(p.text)\nfor t in d.tables:\n    for row in t.rows:\n        print('|'.join(c.text for c in row.cells))",
                    path.display()
                );
                let mut cmd = Command::new("python");
                cmd.args(["-c", &script]);
                #[cfg(windows)]
                hide_window(&mut cmd);
                let output = cmd.output().context("python-docx 실행 실패")?;
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                if text.trim().is_empty() {
                    anyhow::bail!("python-docx: 텍스트 추출 결과 없음");
                }
                Ok(PreprocessResult { text, images: vec![], tables: vec![] })
            }

            Some(HostTool::PythonOpenpyxl) if ext == "xlsx" || ext == "xls" => {
                tracing::info!("전처리 [openpyxl]: {:?}", path);
                let script = format!(
                    "import openpyxl\nwb=openpyxl.load_workbook(r'{}')\nfor ws in wb.worksheets:\n    print(f'=== {{ws.title}} ===')\n    for row in ws.iter_rows(values_only=True):\n        print('\\t'.join(str(c) if c is not None else '' for c in row))",
                    path.display()
                );
                let mut cmd = Command::new("python");
                cmd.args(["-c", &script]);
                #[cfg(windows)]
                hide_window(&mut cmd);
                let output = cmd.output().context("openpyxl 실행 실패")?;
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                Ok(PreprocessResult { text, images: vec![], tables: vec![] })
            }

            Some(HostTool::LibreOffice) => {
                tracing::info!("전처리 [libreoffice]: {:?}", path);
                let tmp_dir = std::env::temp_dir().join("fp-preprocess");
                let _ = std::fs::create_dir_all(&tmp_dir);
                let mut cmd = Command::new("soffice");
                cmd.args(["--headless", "--convert-to", "txt:Text", "--outdir"])
                    .arg(&tmp_dir)
                    .arg(path);
                #[cfg(windows)]
                hide_window(&mut cmd);
                let output = cmd.output().context("libreoffice 실행 실패")?;
                if !output.status.success() {
                    anyhow::bail!("libreoffice 변환 실패: {}", String::from_utf8_lossy(&output.stderr));
                }
                // 변환된 .txt 파일 읽기
                let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                let txt_path = tmp_dir.join(format!("{}.txt", stem));
                let text = std::fs::read_to_string(&txt_path)
                    .context(format!("변환된 텍스트 파일 읽기 실패: {:?}", txt_path))?;
                let _ = std::fs::remove_file(&txt_path);
                Ok(PreprocessResult { text, images: vec![], tables: vec![] })
            }

            _ => {
                // Rust 네이티브 폴백: 외부 도구 없이 직접 변환
                match ext {
                    "docx" => {
                        tracing::info!("전처리 [네이티브 docx]: {:?}", path);
                        return Self::native_docx(path);
                    }
                    "xlsx" | "xls" => {
                        tracing::info!("전처리 [네이티브 calamine]: {:?}", path);
                        return Self::native_xlsx(path);
                    }
                    _ => {}
                }
                tracing::warn!("전처리 도구 없음: .{} — pandoc 또는 libreoffice를 설치하세요.", ext);
                anyhow::bail!(
                    "{} 파일을 처리할 도구가 없습니다.\n\
                     설치 방법:\n\
                     - pandoc: https://pandoc.org/installing.html (docx/xlsx/pptx/hwp 범용)\n\
                     - LibreOffice: https://www.libreoffice.org/ (모든 오피스 포맷)\n\
                     참고: .docx와 .xlsx는 외부 도구 없이 기본 텍스트 추출이 가능합니다.",
                    ext
                )
            }
        }
    }
}

impl PreprocessPort for CompositePreprocessor {
    fn preprocess(&self, file_path: &Path) -> Result<PreprocessResult> {
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "txt" | "md" | "json" | "toml" | "yaml" | "yml" => {
                Self::read_plain_text(file_path)
            }
            "csv" | "tsv" => Self::process_csv(file_path),
            "log" => Self::process_log(file_path),
            "pdf" => self.process_pdf(file_path),
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" => {
                self.process_image(file_path)
            }
            "docx" | "xlsx" | "xls" | "pptx" | "hwp" | "hwpx" => {
                self.process_office(file_path, &ext)
            }
            _ => Self::read_plain_text(file_path),
        }
    }

    fn preprocess_with_config(&self, file_path: &Path, pdf_tool: &str, ocr_tool: &str) -> Result<PreprocessResult> {
        // Phase 89 C-2: 캐시된 host_tools 재사용 — 매 호출마다 detect spawn 회피.
        // 기존 self.host_tools를 clone하여 새 인스턴스에 주입 (외부 프로세스 0건).
        let overridden = CompositePreprocessor::with_tools(pdf_tool, ocr_tool, self.host_tools.clone());
        overridden.preprocess(file_path)
    }

    fn supports(&self, extension: &str) -> bool {
        matches!(
            extension.to_lowercase().as_str(),
            "txt" | "md" | "csv" | "tsv" | "json" | "toml" | "yaml" | "yml"
                | "log"
                | "pdf" | "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp"
                | "docx" | "xlsx" | "xls" | "pptx" | "hwp" | "hwpx"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;
    use std::path::PathBuf;

    // ── DOCX 네이티브 테스트 ──

    /// 테스트용 최소 DOCX 파일 생성 (ZIP + word/document.xml)
    fn create_test_docx(dir: &Path, filename: &str, paragraphs: &[&str]) -> PathBuf {
        let path = dir.join(filename);
        let file = std::fs::File::create(&path).expect("docx 파일 생성 실패");
        let mut zip = zip::ZipWriter::new(file);

        // [Content_Types].xml (필수)
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("[Content_Types].xml", options).expect("zip entry");
        write!(zip, r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#).unwrap();

        // word/document.xml
        zip.start_file("word/document.xml", options).expect("zip entry");
        let mut body = String::from(r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#);
        for p in paragraphs {
            body.push_str(&format!(r#"<w:p><w:r><w:t>{}</w:t></w:r></w:p>"#, p));
        }
        body.push_str("</w:body></w:document>");
        write!(zip, "{}", body).unwrap();

        zip.finish().expect("zip finish");
        path
    }

    #[test]
    fn test_native_docx_basic() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = create_test_docx(dir.path(), "test.docx", &[
            "첫 번째 단락입니다.",
            "두 번째 단락.",
            "세 번째 단락 내용."
        ]);

        let result = CompositePreprocessor::native_docx(&path).unwrap();
        assert!(result.text.contains("첫 번째 단락입니다."));
        assert!(result.text.contains("두 번째 단락."));
        assert!(result.text.contains("세 번째 단락 내용."));
    }

    #[test]
    fn test_native_docx_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = create_test_docx(dir.path(), "empty.docx", &[]);

        let result = CompositePreprocessor::native_docx(&path);
        assert!(result.is_err(), "빈 DOCX는 에러 반환");
    }

    #[test]
    fn test_native_docx_corrupted() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("corrupted.docx");
        std::fs::write(&path, b"not a zip file").unwrap();

        let result = CompositePreprocessor::native_docx(&path);
        assert!(result.is_err(), "손상된 파일은 에러 반환");
    }

    #[test]
    fn test_native_docx_korean() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = create_test_docx(dir.path(), "korean.docx", &[
            "가나다라마바사",
            "한글 테스트 문서입니다.",
            "특수문자: @#$%"
        ]);

        let result = CompositePreprocessor::native_docx(&path).unwrap();
        assert!(result.text.contains("가나다라마바사"));
        assert!(result.text.contains("한글 테스트 문서입니다."));
    }

    // ── XLSX 네이티브 테스트 ──

    #[test]
    fn test_native_xlsx_missing_file() {
        let result = CompositePreprocessor::native_xlsx(Path::new("/nonexistent/test.xlsx"));
        assert!(result.is_err());
    }

    #[test]
    fn test_native_xlsx_corrupted() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("bad.xlsx");
        std::fs::write(&path, b"not an xlsx file").unwrap();

        let result = CompositePreprocessor::native_xlsx(&path);
        assert!(result.is_err());
    }

    // ── CSV 테스트 ──

    #[test]
    fn test_process_csv() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.csv");
        std::fs::write(&path, "이름,나이,도시\n김철수,30,서울\n이영희,25,부산\n박민수,35,대전").unwrap();

        let result = CompositePreprocessor::process_csv(&path).unwrap();
        assert!(result.text.contains("이름"));
        assert!(result.text.contains("김철수"));
        assert!(result.text.contains("컬럼: 3 개"));
        assert!(result.text.contains("행: 3 개"));
    }

    // ── 로그 파일 테스트 ──

    #[test]
    fn test_process_log_with_errors() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("app.log");
        std::fs::write(&path, "2026-04-16 INFO Starting\n2026-04-16 ERROR Connection failed\n2026-04-16 WARNING Retry\n2026-04-16 INFO Done").unwrap();

        let result = CompositePreprocessor::process_log(&path).unwrap();
        assert!(result.text.contains("에러: 1 건"));
        assert!(result.text.contains("경고: 1 건"));
        assert!(result.text.contains("Connection failed"));
    }

    // ── 확장자 라우팅 테스트 ──

    #[test]
    fn test_preprocess_txt() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("hello.txt");
        std::fs::write(&path, "Hello World").unwrap();

        let pp = CompositePreprocessor::new("none", "none");
        let result = pp.preprocess(&path).unwrap();
        assert_eq!(result.text, "Hello World");
    }

    #[test]
    fn test_preprocess_docx_native_fallback() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = create_test_docx(dir.path(), "doc.docx", &["네이티브 DOCX 테스트"]);

        // 호스트 도구 없이 CompositePreprocessor로 호출 → 네이티브 폴백
        let pp = CompositePreprocessor {
            pdf_tool: "none".to_string(),
            ocr_tool: "none".to_string(),
            host_tools: vec![], // 호스트 도구 없음
        };
        let result = pp.preprocess(&path).unwrap();
        assert!(result.text.contains("네이티브 DOCX 테스트"));
    }

    #[test]
    fn test_supports() {
        let pp = CompositePreprocessor::new("none", "none");
        assert!(pp.supports("txt"));
        assert!(pp.supports("docx"));
        assert!(pp.supports("xlsx"));
        assert!(pp.supports("csv"));
        assert!(pp.supports("pdf"));
        assert!(!pp.supports("exe"));
        assert!(!pp.supports("rs"));
    }

    // ── HostToolDetector 테스트 ──

    #[test]
    fn test_best_tool_for_docx() {
        let tools = vec![
            (HostTool::Pandoc, "pandoc 3.1".into()),
            (HostTool::PythonDocx, "python-docx 0.8".into()),
        ];
        assert_eq!(HostToolDetector::best_tool_for("docx", &tools), Some(HostTool::Pandoc));
    }

    #[test]
    fn test_best_tool_for_xlsx_openpyxl() {
        let tools = vec![
            (HostTool::PythonOpenpyxl, "openpyxl 3.1".into()),
        ];
        assert_eq!(HostToolDetector::best_tool_for("xlsx", &tools), Some(HostTool::PythonOpenpyxl));
    }

    #[test]
    fn test_best_tool_for_unknown_ext() {
        let tools = vec![(HostTool::Pandoc, "pandoc".into())];
        assert_eq!(HostToolDetector::best_tool_for("zip", &tools), None);
    }
}
