//! OCR 模块
//!
//! 提供多引擎 OCR 支持：
//! - Paddle OCR (PP-OCRv5 ONNX)
//! - Tesseract OCR (CLI)

mod engine;
mod paddle;
mod tesseract;
mod types;

pub use engine::OcrEngine;
pub use paddle::{get_paddle_status, init_paddle_engine, install_paddle_models, is_paddle_installed};
pub use tesseract::{detect_tesseract_status, get_tesseract_langs, install_tesseract, Platform, TesseractEngine};
pub use types::*;

use crate::config::{load_config, save_config, ConfigResult};
use std::sync::Mutex;

/// 当前活动的 OCR 引擎类型
static CURRENT_ENGINE: Mutex<OcrEngineType> = Mutex::new(OcrEngineType::Paddle);

/// Tesseract 引擎实例
static TESSERACT_ENGINE: Mutex<Option<TesseractEngine>> = Mutex::new(None);

// ============ Tauri Commands ============

/// 获取平台信息
#[tauri::command]
pub fn get_platform() -> PlatformInfo {
    PlatformInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

/// 获取所有 OCR 引擎状态
#[tauri::command]
pub fn get_ocr_engine_status(app: tauri::AppHandle) -> ConfigResult<OcrEngineStatus> {
    let config = load_config(app.clone())?;

    let paddle_status = get_paddle_status(&app);

    let tesseract_config = config.tesseract.unwrap_or_default();
    let tesseract_status = detect_tesseract_status(&tesseract_config);

    let current_engine = config.ocr_engine.unwrap_or_default();

    Ok(OcrEngineStatus {
        paddle: paddle_status,
        tesseract: tesseract_status,
        current_engine,
    })
}

/// 设置当前 OCR 引擎
#[tauri::command]
pub fn set_ocr_engine(app: tauri::AppHandle, engine_type: OcrEngineType) -> ConfigResult<()> {
    // 更新内存中的引擎类型
    {
        let mut guard = CURRENT_ENGINE.lock().map_err(|e| e.to_string())?;
        *guard = engine_type;
    }

    // 保存到配置
    let mut config = load_config(app.clone())?;
    config.ocr_engine = Some(engine_type);
    save_config(app, config)?;

    log::info!("[OCR] 切换引擎为: {}", engine_type);
    Ok(())
}

/// 获取当前 OCR 引擎类型
#[tauri::command]
pub fn get_current_ocr_engine() -> ConfigResult<OcrEngineType> {
    let guard = CURRENT_ENGINE.lock().map_err(|e| e.to_string())?;
    Ok(*guard)
}

/// 初始化 Paddle OCR 引擎
#[tauri::command]
pub fn init_paddle_ocr(app: tauri::AppHandle) -> ConfigResult<()> {
    init_paddle_engine(&app)
}

/// 安装 Paddle OCR 模型
#[tauri::command]
pub async fn install_paddle_ocr(
    app: tauri::AppHandle,
    request: PaddleInstallRequest,
) -> ConfigResult<PaddleInstallResult> {
    install_paddle_models(app, request).await
}

/// 检查 Paddle OCR 是否已安装
#[tauri::command]
pub fn is_paddle_ocr_installed(app: tauri::AppHandle) -> ConfigResult<bool> {
    is_paddle_installed(&app)
}

/// 初始化 Tesseract 引擎
#[tauri::command]
pub fn init_tesseract_ocr(app: tauri::AppHandle) -> ConfigResult<()> {
    let config = load_config(app)?;
    let tesseract_config = config.tesseract.unwrap_or_default();

    let engine = TesseractEngine::new(tesseract_config)
        .map_err(|e| format!("初始化 Tesseract 引擎失败: {}", e))?;

    let mut guard = TESSERACT_ENGINE.lock().map_err(|e| e.to_string())?;
    *guard = Some(engine);

    log::info!("[Tesseract] 引擎初始化成功");
    Ok(())
}

/// 检测 Tesseract 安装状态
#[tauri::command]
pub fn check_tesseract_status(app: tauri::AppHandle) -> ConfigResult<TesseractStatus> {
    let config = load_config(app)?;
    let tesseract_config = config.tesseract.unwrap_or_default();
    Ok(detect_tesseract_status(&tesseract_config))
}

/// 保存 Tesseract 配置
#[tauri::command]
pub fn save_tesseract_config(
    app: tauri::AppHandle,
    tesseract_config: TesseractConfig,
) -> ConfigResult<()> {
    let mut config = load_config(app.clone())?;
    config.tesseract = Some(tesseract_config);
    save_config(app, config)?;
    log::info!("[Tesseract] 配置已保存");
    Ok(())
}

/// 获取 Tesseract 可用语言列表
#[tauri::command]
pub fn get_tesseract_languages(app: tauri::AppHandle) -> ConfigResult<Vec<String>> {
    let config = load_config(app)?;
    let tesseract_config = config.tesseract.unwrap_or_default();

    let binary = tesseract_config
        .binary_path
        .as_deref()
        .unwrap_or("tesseract");
    let langs = get_tesseract_langs(binary, tesseract_config.tessdata_path.as_deref())
        .map_err(|e| e.to_string())?;

    Ok(langs)
}

/// 获取当前平台信息
#[tauri::command]
pub fn get_current_platform() -> String {
    format!("{:?}", Platform::current()).to_lowercase()
}

/// 安装 Tesseract（自动根据平台选择安装方式）
#[tauri::command]
pub async fn install_tesseract_ocr(app: tauri::AppHandle) -> ConfigResult<()> {
    use tauri::Emitter;

    let app_clone = app.clone();

    install_tesseract(move |progress| {
        let _ = app_clone.emit("tesseract-install-progress", &progress);
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// 使用当前引擎识别图片
#[tauri::command]
pub fn ocr_recognize(image_path: String) -> ConfigResult<Vec<OcrTextResult>> {
    recognize_with_current_engine(&image_path)
}

/// 获取 OCR 审计信息
#[tauri::command]
pub fn get_ocr_audit_info() -> ConfigResult<OcrAuditInfo> {
    Ok(get_current_audit_info())
}

// ============ 内部辅助函数 ============

/// 使用当前引擎识别图片（供 detection 模块使用）
pub fn recognize_with_current_engine(image_path: &str) -> Result<Vec<OcrTextResult>, String> {
    let engine_type = *CURRENT_ENGINE.lock().map_err(|e| e.to_string())?;

    match engine_type {
        OcrEngineType::Paddle => paddle::paddle_recognize(image_path),
        OcrEngineType::Tesseract => {
            let mut guard = TESSERACT_ENGINE.lock().map_err(|e| e.to_string())?;

            // 如果引擎未初始化，尝试自动初始化
            if guard.is_none() {
                log::info!("[Tesseract] 引擎未初始化，尝试自动初始化...");
                let config = TesseractConfig::default();
                match TesseractEngine::new(config) {
                    Ok(engine) => {
                        log::info!("[Tesseract] 自动初始化成功");
                        *guard = Some(engine);
                    }
                    Err(e) => {
                        return Err(format!("Tesseract 引擎初始化失败: {}", e));
                    }
                }
            }

            let engine = guard.as_mut().ok_or("Tesseract 引擎未初始化")?;
            engine.recognize_file(image_path)
        }
    }
}

/// 获取当前引擎审计信息（供 detection 模块使用）
pub fn get_current_audit_info() -> OcrAuditInfo {
    let engine_type = CURRENT_ENGINE.lock().map(|g| *g).unwrap_or_default();

    match engine_type {
        OcrEngineType::Paddle => OcrAuditInfo {
            engine_type: OcrEngineType::Paddle,
            engine_version: Some("PP-OCRv5".to_string()),
            engine_params: None,
            tessdata_hash: None,
        },
        OcrEngineType::Tesseract => TESSERACT_ENGINE
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|e| e.audit_info()))
            .unwrap_or(OcrAuditInfo {
                engine_type: OcrEngineType::Tesseract,
                engine_version: None,
                engine_params: None,
                tessdata_hash: None,
            }),
    }
}
