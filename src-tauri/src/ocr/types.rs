//! OCR 共享类型定义

use serde::{Deserialize, Serialize};

/// OCR 引擎类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OcrEngineType {
    /// Paddle OCR (PP-OCRv5 ONNX)
    #[default]
    Paddle,
    /// Tesseract OCR (CLI)
    Tesseract,
}

impl std::fmt::Display for OcrEngineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OcrEngineType::Paddle => write!(f, "paddle"),
            OcrEngineType::Tesseract => write!(f, "tesseract"),
        }
    }
}

/// OCR 识别结果（统一格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrTextResult {
    pub text: String,
    pub confidence: f32,
    pub bbox: BBox,
}

/// 边界框（相对坐标 0-1）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// 平台信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlatformInfo {
    pub os: String,
    pub arch: String,
}

/// Paddle 模型安装请求
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PaddleInstallRequest {
    /// 检测模型下载 URL
    pub det_url: String,
    /// 识别模型下载 URL
    pub rec_url: String,
    /// 模型版本
    pub model_version: Option<String>,
    /// 安装来源
    pub install_source: Option<String>,
}

/// Paddle 模型安装结果
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PaddleInstallResult {
    pub det_model_path: String,
    pub rec_model_path: String,
}

/// Tesseract 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TesseractConfig {
    /// Tesseract 可执行文件路径
    pub binary_path: Option<String>,
    /// tessdata 目录路径
    pub tessdata_path: Option<String>,
    /// 语言（如 "chi_sim+eng"）
    pub lang: Option<String>,
    /// 页面分割模式 (0-13)
    pub psm: Option<u8>,
    /// OCR 引擎模式 (0-3)
    pub oem: Option<u8>,
}

impl TesseractConfig {
    pub fn lang_or_default(&self) -> &str {
        self.lang.as_deref().unwrap_or("chi_sim+eng")
    }

    pub fn psm_or_default(&self) -> u8 {
        self.psm.unwrap_or(6)
    }

    pub fn oem_or_default(&self) -> u8 {
        self.oem.unwrap_or(1)
    }
}

/// Tesseract 安装状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TesseractStatus {
    /// 是否已安装
    pub installed: bool,
    /// 版本信息
    pub version: Option<String>,
    /// 可执行文件路径
    pub binary_path: Option<String>,
    /// tessdata 路径
    pub tessdata_path: Option<String>,
    /// 可用语言列表
    pub available_langs: Vec<String>,
    /// 错误信息
    pub error: Option<String>,
}

/// 下载进度事件
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub file_name: String,
    pub file_index: u32,
    pub total_files: u32,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percent: f32,
}

/// OCR 引擎状态（用于前端显示）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrEngineStatus {
    pub paddle: PaddleStatus,
    pub tesseract: TesseractStatus,
    pub current_engine: OcrEngineType,
}

/// Paddle 引擎状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaddleStatus {
    pub installed: bool,
    pub det_model_path: Option<String>,
    pub rec_model_path: Option<String>,
    pub model_version: Option<String>,
}

/// OCR 审计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrAuditInfo {
    /// 使用的引擎类型
    pub engine_type: OcrEngineType,
    /// 引擎版本
    pub engine_version: Option<String>,
    /// 引擎参数（JSON）
    pub engine_params: Option<String>,
    /// tessdata hash（仅 Tesseract）
    pub tessdata_hash: Option<String>,
}
