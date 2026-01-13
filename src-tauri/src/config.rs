use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

use crate::ocr::{OcrEngineType, TesseractConfig};
use crate::pdf::Rule;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct AppConfig {
    // ============ Paddle OCR 配置 ============
    /// 检测模型路径
    pub det_model_path: Option<String>,
    /// 识别模型路径
    pub rec_model_path: Option<String>,
    /// 模型版本
    pub model_version: Option<String>,
    /// 安装来源
    pub install_source: Option<String>,
    /// 是否使用镜像
    pub use_mirror: Option<bool>,

    // ============ OCR 引擎选择 ============
    /// 当前使用的 OCR 引擎
    pub ocr_engine: Option<OcrEngineType>,
    /// Tesseract 配置
    pub tesseract: Option<TesseractConfig>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("app data dir unavailable")]
    NoAppDataDir,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type ConfigResult<T> = Result<T, String>;

pub fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, ConfigError> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|_| ConfigError::NoAppDataDir)?;
    Ok(base.join("linch-redact").join("config.json"))
}

pub fn ocr_root(app: &tauri::AppHandle) -> Result<PathBuf, ConfigError> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|_| ConfigError::NoAppDataDir)?;
    Ok(base.join("linch-redact").join("ocr"))
}

pub fn models_dir(app: &tauri::AppHandle) -> Result<PathBuf, ConfigError> {
    Ok(ocr_root(app)?.join("models"))
}

#[tauri::command]
pub fn load_config(app: tauri::AppHandle) -> ConfigResult<AppConfig> {
    let path = config_path(&app).map_err(|err| err.to_string())?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&raw).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn save_config(app: tauri::AppHandle, config: AppConfig) -> ConfigResult<()> {
    let path = config_path(&app).map_err(|err| err.to_string())?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    }
    let raw = serde_json::to_string_pretty(&config).map_err(|err| err.to_string())?;
    fs::write(path, raw).map_err(|err| err.to_string())?;
    Ok(())
}

// ============ 检测规则存储 ============

fn rules_path(app: &tauri::AppHandle) -> Result<PathBuf, ConfigError> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|_| ConfigError::NoAppDataDir)?;
    Ok(base.join("linch-redact").join("detection-rules.json"))
}

#[tauri::command]
pub fn load_detection_rules(app: tauri::AppHandle) -> ConfigResult<Vec<Rule>> {
    let path = rules_path(&app).map_err(|err| err.to_string())?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&raw).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn save_detection_rules(app: tauri::AppHandle, rules: Vec<Rule>) -> ConfigResult<()> {
    let path = rules_path(&app).map_err(|err| err.to_string())?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    }
    let raw = serde_json::to_string_pretty(&rules).map_err(|err| err.to_string())?;
    fs::write(path, raw).map_err(|err| err.to_string())?;
    Ok(())
}
