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
