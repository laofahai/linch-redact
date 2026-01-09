//! OCR 引擎 trait 定义

use crate::ocr::types::{OcrAuditInfo, OcrTextResult};
use image::DynamicImage;

/// OCR 引擎统一 trait
pub trait OcrEngine: Send {
    /// 识别图片中的文字
    fn recognize_image(&mut self, img: &DynamicImage) -> Result<Vec<OcrTextResult>, String>;

    /// 识别图片文件
    fn recognize_file(&mut self, image_path: &str) -> Result<Vec<OcrTextResult>, String> {
        let img = image::open(image_path).map_err(|e| format!("打开图片失败: {}", e))?;
        self.recognize_image(&img)
    }

    /// 提取纯文本
    #[allow(dead_code)]
    fn extract_text(&mut self, image_path: &str) -> Result<String, String> {
        let results = self.recognize_file(image_path)?;
        let text: String = results
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        Ok(text)
    }

    /// 获取审计信息
    fn audit_info(&self) -> OcrAuditInfo;
}
