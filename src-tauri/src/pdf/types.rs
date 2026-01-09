use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 脱敏模式
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RedactionMode {
  #[default]
  Auto,           // 自动检测 PDF 类型并选择最佳方案
  TextReplace,    // 文字替换为 ****（仅适用于文字型 PDF）
  BlackOverlay,   // 黑框覆盖（适用于路径绘制型，不安全）
  ImageMode,      // 转图片模式（最安全，适用于所有类型）
  SafeRender,     // 安全渲染模式（使用 pdfium 渲染后脱敏，适用于路径绘制型）
}

/// 清理选项
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CleaningOptions {
  /// 清理文档信息（Info 字典）
  #[serde(default)]
  pub document_info: bool,
  /// 清理 XMP 元数据
  #[serde(default)]
  pub xmp_metadata: bool,
  /// 清理隐藏数据（PieceInfo、LastModified 等）
  #[serde(default)]
  pub hidden_data: bool,
  /// 清理注释
  #[serde(default)]
  pub annotations: bool,
  /// 清理表单字段
  #[serde(default)]
  pub forms: bool,
  /// 清理附件
  #[serde(default)]
  pub attachments: bool,
  /// 清理 JavaScript
  #[serde(default)]
  pub javascript: bool,
}

/// 页面内容类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageContentType {
  Text,           // 包含文字操作符 (Tj/TJ)
  PathDrawn,      // 主要是路径绘制
  ImageBased,     // 主要是图片（扫描件）
  Mixed,          // 混合类型
  Empty,          // 空页面
}

/// PDF 分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfAnalysis {
  pub page_types: Vec<PageContentType>,
  pub has_forms: bool,
  pub has_annotations: bool,
  pub has_metadata: bool,
  pub has_attachments: bool,
  pub has_javascript: bool,
  pub recommended_mode: RedactionMode,
}

/// 检测规则
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
  pub id: String,
  pub name: String,
  pub rule_type: String, // "keyword" | "regex"
  pub pattern: String,
  pub enabled: bool,
}

/// 检测命中结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionHit {
  pub page: usize,
  pub bbox: DetectionBbox,
  pub rule_id: String,
  pub rule_name: String,
  pub snippet: String,
}

/// 检测边界框
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionBbox {
  pub x: f64,
  pub y: f64,
  pub width: f64,
  pub height: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Mask {
  pub x: f64,      // 0-1 相对坐标
  pub y: f64,
  pub width: f64,
  pub height: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct MaskRect {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
}

impl MaskRect {
  /// 检查文字边界框是否与 mask 区域相交
  pub fn intersects_text_bbox(&self, text_x: f32, text_y: f32, text_width: f32, text_height: f32) -> bool {
    let text_left = text_x;
    let text_right = text_x + text_width;
    let text_bottom = text_y;
    let text_top = text_y + text_height;

    let margin: f32 = 5.0;
    let mask_left = self.x - margin;
    let mask_right = self.x + self.width + margin;
    let mask_bottom = self.y - margin;
    let mask_top = self.y + self.height + margin;

    let x_overlap = text_left < mask_right && text_right > mask_left;
    let y_overlap = text_bottom < mask_top && text_top > mask_bottom;

    x_overlap && y_overlap
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PageAction {
  pub index: usize,
  pub action: String, // "keep", "redact", "delete"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileProcessRequest {
  pub path: String,
  pub pages: Vec<PageAction>,
  pub masks_by_page: BTreeMap<usize, Vec<Mask>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessRequest {
  pub files: Vec<FileProcessRequest>,
  pub output_directory: String,
  pub suffix: String,
  #[serde(default)]
  pub mode: RedactionMode,
  #[serde(default)]
  pub cleaning: CleaningOptions,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessResult {
  pub success: bool,
  pub processed_files: Vec<String>,
  pub errors: Vec<String>,
}
