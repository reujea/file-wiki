//! 증분 컴파일 엔진 — 변경된 파일만 재가공하여 토큰 비용 절감
//!
//! llm-wiki-compiler 벤치마크:
//!   첫 컴파일: 383 파일 → 880K 토큰 (~$2.60 Sonnet)
//!   증분 컴파일: 변경분만 → ~100K 토큰 (~$0.30 Sonnet)
//!   세션 로드: 47K → 7.7K 토큰 (84% 절감)

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 파일별 컴파일 상태 추적
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileState {
    /// 파일 경로 → (SHA-256 해시, 마지막 컴파일 시각)
    pub entries: HashMap<String, FileState>,
    /// 총 컴파일 통계
    pub stats: CompileStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    pub hash: String,
    pub compiled_at: String,
    pub input_chars: u64,
    pub output_chars: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompileStats {
    pub total_files: u64,
    pub total_input_chars: u64,
    pub total_output_chars: u64,
    pub estimated_input_tokens: u64,
    pub estimated_output_tokens: u64,
    pub compression_ratio: f64,
}

/// 벤치마크 보고서 (단일 실행)
#[derive(Debug, Clone, Default)]
pub struct BenchmarkReport {
    pub files_processed: u64,
    pub files_skipped: u64,
    pub input_chars: u64,
    pub output_chars: u64,
    pub estimated_input_tokens: u64,
    pub estimated_output_tokens: u64,
    pub compression_ratio: f64,
    pub estimated_cost_usd: f64,
    pub is_incremental: bool,
}

impl BenchmarkReport {
    /// 토큰 추정: 한국어 1글자 ≈ 2~3 토큰, 영어 1단어 ≈ 1.3 토큰
    /// 보수적으로 chars / 2 로 추정 (한국어+영어 혼합)
    pub fn estimate_tokens(chars: u64) -> u64 {
        chars / 2
    }

    /// Sonnet 비용 추정: input $3/M, output $15/M
    pub fn estimate_cost(input_tokens: u64, output_tokens: u64) -> f64 {
        (input_tokens as f64 * 3.0 / 1_000_000.0)
            + (output_tokens as f64 * 15.0 / 1_000_000.0)
    }

    pub fn summary(&self) -> String {
        let mode = if self.is_incremental { "증분" } else { "전체" };
        format!(
            "=== {} 컴파일 벤치마크 ===\n\
             처리: {} 파일, 스킵: {} 파일\n\
             입력: {} chars → ~{} tokens\n\
             출력: {} chars → ~{} tokens\n\
             압축률: {:.1}x\n\
             추정 비용: ${:.4} (Sonnet)",
            mode,
            self.files_processed,
            self.files_skipped,
            self.input_chars,
            self.estimated_input_tokens,
            self.output_chars,
            self.estimated_output_tokens,
            if self.compression_ratio > 0.0 {
                self.compression_ratio
            } else {
                1.0
            },
            self.estimated_cost_usd,
        )
    }
}

impl CompileState {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            stats: CompileStats::default(),
        }
    }

    /// 상태 파일 로드
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::new())
        }
    }

    /// 상태 파일 저장
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 파일이 변경되었는지 확인 (해시 비교)
    pub fn is_changed(&self, file_path: &str, current_hash: &str) -> bool {
        match self.entries.get(file_path) {
            Some(state) => state.hash != current_hash,
            None => true, // 새 파일
        }
    }

    /// 컴파일 완료 기록
    pub fn record_compile(
        &mut self,
        file_path: &str,
        hash: &str,
        input_chars: u64,
        output_chars: u64,
    ) {
        self.entries.insert(
            file_path.to_string(),
            FileState {
                hash: hash.to_string(),
                compiled_at: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
                input_chars,
                output_chars,
            },
        );

        // 통계 재계산
        self.recalc_stats();
    }

    fn recalc_stats(&mut self) {
        let mut total_in = 0u64;
        let mut total_out = 0u64;
        for entry in self.entries.values() {
            total_in += entry.input_chars;
            total_out += entry.output_chars;
        }
        self.stats = CompileStats {
            total_files: self.entries.len() as u64,
            total_input_chars: total_in,
            total_output_chars: total_out,
            estimated_input_tokens: BenchmarkReport::estimate_tokens(total_in),
            estimated_output_tokens: BenchmarkReport::estimate_tokens(total_out),
            compression_ratio: if total_out > 0 {
                total_in as f64 / total_out as f64
            } else {
                0.0
            },
        };
    }
}

impl Default for CompileState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_state_change_detection() {
        let mut state = CompileState::new();
        assert!(state.is_changed("file.txt", "abc123"));

        state.record_compile("file.txt", "abc123", 1000, 300);
        assert!(!state.is_changed("file.txt", "abc123"));
        assert!(state.is_changed("file.txt", "def456"));
    }

    #[test]
    fn test_benchmark_report() {
        let report = BenchmarkReport {
            files_processed: 10,
            files_skipped: 5,
            input_chars: 100_000,
            output_chars: 20_000,
            estimated_input_tokens: 50_000,
            estimated_output_tokens: 10_000,
            compression_ratio: 5.0,
            estimated_cost_usd: BenchmarkReport::estimate_cost(50_000, 10_000),
            is_incremental: false,
        };
        assert!(report.estimated_cost_usd > 0.0);
        assert!(report.summary().contains("전체 컴파일"));
    }

    #[test]
    fn test_token_estimation() {
        assert_eq!(BenchmarkReport::estimate_tokens(1000), 500);
    }

    #[test]
    fn test_cost_estimation() {
        // 1M input tokens = $3, 1M output tokens = $15
        let cost = BenchmarkReport::estimate_cost(1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 0.01);
    }
}
