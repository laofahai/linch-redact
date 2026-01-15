mod config;
mod ocr;
mod pdf;

pub use config::{load_config, load_detection_rules, save_config, save_detection_rules};
pub use ocr::{
    check_tesseract_status,
    get_current_ocr_engine,
    get_current_platform,
    get_ocr_audit_info,
    get_ocr_engine_status,
    get_platform,
    get_tesseract_languages,
    // Paddle OCR
    init_paddle_ocr,
    // Tesseract OCR
    init_tesseract_ocr,
    install_paddle_ocr,
    install_tesseract_ocr,
    is_paddle_ocr_installed,
    // 识别
    ocr_recognize,
    save_tesseract_config,
    set_ocr_engine,
};
pub use pdf::{analyze_pdf, detect_sensitive_content, process_pdfs};

use linch_tech_desktop_core::{LinchConfig, LinchDesktopExt};

// ============================================================================
// 新架构：多文档格式支持
// ============================================================================

use linch_core::document::{Document, Page};
use std::path::Path;

/// 文档信息（返回给前端）
#[derive(Clone, serde::Serialize)]
pub struct DocumentInfo {
    pub path: String,
    pub name: String,
    pub file_type: String,
    pub pages: Vec<Page>,
    pub total_pages: usize,
    pub supported_features: Vec<String>,
}

/// 加载文档命令
///
/// 根据文件扩展名自动选择对应的处理器。
#[tauri::command]
async fn load_document(file_path: String) -> Result<DocumentInfo, String> {
    let path = Path::new(&file_path);
    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("")
        .to_lowercase();

    let result = match extension.as_str() {
        "pdf" => load_with_handler::<linch_pdf::PdfDocument>(path, "pdf"),
        "txt" => load_with_handler::<linch_text::TextDocument>(path, "txt"),
        "md" => load_with_handler::<linch_text::TextDocument>(path, "md"),
        _ => Err(anyhow::anyhow!("不支持的文件类型: {}", extension)),
    };

    result.map_err(|e| e.to_string())
}

/// 使用指定处理器加载文档
fn load_with_handler<D: Document>(path: &Path, file_type: &str) -> anyhow::Result<DocumentInfo> {
    let doc = D::load(path)?;
    let pages = doc.get_pages()?;
    let total_pages = pages.len();
    let supported_features = doc.get_supported_features();

    Ok(DocumentInfo {
        path: path.to_string_lossy().to_string(),
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        file_type: file_type.to_string(),
        pages,
        total_pages,
        supported_features,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let linch_config = LinchConfig::from_env();

    tauri::Builder::default()
        .with_linch_desktop(linch_config)
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
                .build(),
        )
        .setup(|app| {
            // 尝试初始化 OCR 引擎（忽略错误，让用户后续手动配置）
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                // 从配置加载当前引擎类型
                if let Ok(config) = crate::config::load_config(handle.clone()) {
                    if let Some(engine_type) = config.ocr_engine {
                        if let Err(e) = set_ocr_engine(handle.clone(), engine_type) {
                            log::warn!("[Startup] 设置 OCR 引擎失败: {}", e);
                        } else {
                            log::info!("[Startup] OCR 引擎设置为: {:?}", engine_type);
                        }
                    }
                }

                // 尝试初始化 Paddle OCR
                if let Err(e) = init_paddle_ocr(handle.clone()) {
                    log::info!("[Startup] Paddle OCR 初始化跳过: {}", e);
                }
                // 尝试初始化 Tesseract OCR
                if let Err(e) = init_tesseract_ocr(handle) {
                    log::info!("[Startup] Tesseract OCR 初始化跳过: {}", e);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 新架构：文档加载
            load_document,
            // 配置
            load_config,
            save_config,
            load_detection_rules,
            save_detection_rules,
            // OCR 通用
            get_platform,
            get_ocr_engine_status,
            set_ocr_engine,
            get_current_ocr_engine,
            ocr_recognize,
            get_ocr_audit_info,
            // Paddle OCR
            init_paddle_ocr,
            install_paddle_ocr,
            is_paddle_ocr_installed,
            // Tesseract OCR
            init_tesseract_ocr,
            check_tesseract_status,
            save_tesseract_config,
            get_tesseract_languages,
            get_current_platform,
            install_tesseract_ocr,
            // PDF 处理
            process_pdfs,
            analyze_pdf,
            detect_sensitive_content
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
