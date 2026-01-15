//! PDF 文档处理器
//!
//! 实现 `Document` trait，提供 PDF 文件的加载、文本提取和脱敏功能。

use anyhow::{anyhow, Result};
use linch_core::document::{Document, Page};
use linch_core::rules::RuleSet;
use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ============================================================================
// 原有数据结构（保持兼容）
// ============================================================================

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

// ============================================================================
// 新架构：Document trait 实现
// ============================================================================

/// PDF 文档处理器
///
/// 存储文件路径，在需要时初始化 Pdfium 实例（因为 Pdfium 不是线程安全的）。
pub struct PdfDocument {
    path: PathBuf,
}

/// 初始化 Pdfium 库
///
/// 尝试从当前目录加载，失败则回退到系统库。
fn init_pdfium() -> Result<Pdfium> {
    let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
        .or_else(|_| Pdfium::bind_to_system_library())
        .map_err(|e| anyhow!("无法加载 Pdfium 库: {}", e))?;

    // Pdfium::new() 直接返回 Pdfium 实例，不是 Result
    Ok(Pdfium::new(bindings))
}

impl Document for PdfDocument {
    fn load(path: &Path) -> Result<Self>
    where
        Self: Sized,
    {
        if !path.exists() {
            return Err(anyhow!("文件不存在: {}", path.display()));
        }

        // 验证文件可以被 Pdfium 打开
        let pdfium = init_pdfium()?;
        pdfium
            .load_pdf_from_file(path, None)
            .map_err(|e| anyhow!("无法打开 PDF 文件: {}", e))?;

        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    fn get_pages(&self) -> Result<Vec<Page>> {
        let pdfium = init_pdfium()?;
        let doc = pdfium
            .load_pdf_from_file(&self.path, None)
            .map_err(|e| anyhow!("无法加载 PDF: {}", e))?;

        let mut pages = Vec::new();
        for (index, page) in doc.pages().iter().enumerate() {
            let page_number = (index + 1) as u32;
            let content = page.text().map(|t| t.all()).unwrap_or_default();

            pages.push(Page {
                page_number,
                content,
            });
        }

        Ok(pages)
    }

    fn redact(&self, _ruleset: &RuleSet) -> Result<Vec<u8>> {
        // TODO: 实现 PDF 脱敏逻辑
        Err(anyhow!("PDF 脱敏功能尚未在新架构中实现"))
    }

    fn get_supported_features(&self) -> Vec<String> {
        vec![
            "text_redact".to_string(),
            "metadata_clean".to_string(),
            "image_redact".to_string(),
        ]
    }
}
