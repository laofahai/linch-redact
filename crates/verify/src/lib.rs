//! Post-processing verification checks.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyOptions {
    pub text_search: bool,
    pub ocr_sample: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    pub ok: bool,
    pub warnings: Vec<String>,
}

pub fn verify_output(_output_path: &str, _options: &VerifyOptions) -> VerifyResult {
    VerifyResult {
        ok: true,
        warnings: Vec::new(),
    }
}
