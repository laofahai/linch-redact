//! Core orchestration for redaction tasks.

pub mod document;
pub mod rules;

pub use document::{Document, Page};
pub use rules::{HeuristicType, Rule, RuleSet, RuleType};

use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("unsupported operation: {0}")]
    Unsupported(&'static str),
    #[error("invalid configuration: {0}")]
    InvalidConfig(&'static str),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub input_path: String,
    pub output_path: String,
    pub ocr_mode: OcrMode,
    pub clean: CleanOptions,
    pub verify: VerifyOptions,
    pub rule_pack: RulePackRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePackRef {
    pub name: String,
    pub version: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub output_path: Option<String>,
    pub audit_path: Option<String>,
    pub report_path: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OcrMode {
    Detect,
    Clear,
    Rebuild,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanOptions {
    pub metadata: bool,
    pub xmp: bool,
    pub annots: bool,
    pub forms: bool,
    pub attachments: bool,
    pub javascript: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyOptions {
    pub text_search: bool,
    pub ocr_sample: bool,
}

pub fn run_task(_config: TaskConfig) -> Result<TaskResult> {
    Err(CoreError::Unsupported("pipeline not wired yet"))
}
