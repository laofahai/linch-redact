//! PDF rendering for preview and OCR.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderOptions {
    pub dpi: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageImage {
    pub page: u32,
    pub bytes: Vec<u8>,
}

pub fn render_pdf(_input_path: &str, _options: &RenderOptions) -> Vec<PageImage> {
    Vec::new()
}
