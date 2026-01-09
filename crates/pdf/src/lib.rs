//! PDF cleaning and redaction utilities.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionHit {
    pub page: u32,
    pub bbox: BBox,
    pub label: String,
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

pub fn apply_redactions(
    _input_path: &str,
    _output_path: &str,
    _hits: &[RedactionHit],
    _clean: &CleanOptions,
) -> std::result::Result<(), String> {
    Err("not implemented".to_string())
}
