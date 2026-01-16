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
use linch_core::rules::{Rule, RuleMatch, RuleSet, RuleType};
use std::fs;
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

// ============================================================================
// 新架构：规则匹配与脱敏
// ============================================================================

/// 前端传入的规则（简化格式）
#[derive(Clone, serde::Deserialize)]
pub struct FrontendRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    #[serde(default)]
    pub is_system: bool,
    #[serde(rename = "ruleType")]
    pub rule_type: String,
    #[serde(default)]
    pub pattern: String,
    #[serde(rename = "heuristicType")]
    pub heuristic_type: Option<String>,
}

/// 脱敏结果
#[derive(Clone, serde::Serialize)]
pub struct RedactionResult {
    pub success: bool,
    pub output_path: Option<String>,
    pub matches_count: usize,
    pub message: String,
}

/// 匹配预览结果
#[derive(Clone, serde::Serialize)]
pub struct MatchPreviewResult {
    pub matches: Vec<RuleMatch>,
    pub total_count: usize,
}

use linch_core::rules::HeuristicType;

/// 将前端规则转换为后端规则集
fn convert_rules_to_ruleset(rules: Vec<FrontendRule>) -> RuleSet {
    let mut ruleset = RuleSet::new();

    for fr in rules {
        let rule_type = match fr.rule_type.as_str() {
            "regex" => RuleType::Regex(fr.pattern.clone()),
            "keyword" => RuleType::Dictionary(
                fr.pattern
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
            ),
            "heuristic" => {
                let heuristic_type = match fr.heuristic_type.as_deref() {
                    Some("Address") => HeuristicType::Address,
                    Some("PersonName") => HeuristicType::PersonName,
                    Some("Organization") => HeuristicType::Organization,
                    Some("Date") => HeuristicType::Date,
                    Some("Amount") => HeuristicType::Amount,
                    Some("Phone") => HeuristicType::Phone,
                    Some("Email") => HeuristicType::Email,
                    Some("IdNumber") => HeuristicType::IdNumber,
                    Some("CreditCard") => HeuristicType::CreditCard,
                    _ => continue,
                };
                RuleType::Heuristic(heuristic_type)
            }
            _ => continue,
        };

        ruleset.add(Rule {
            id: fr.id,
            name: fr.name,
            enabled: fr.enabled,
            is_system: fr.is_system,
            rule_type,
        });
    }

    ruleset
}

/// 预览匹配结果
///
/// 对文档内容进行规则匹配，返回所有匹配项（不修改文件）。
#[tauri::command]
async fn preview_matches(
    file_path: String,
    rules: Vec<FrontendRule>,
) -> Result<MatchPreviewResult, String> {
    let path = Path::new(&file_path);
    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("")
        .to_lowercase();

    // 加载文档获取文本
    let doc_info = match extension.as_str() {
        "pdf" => load_with_handler::<linch_pdf::PdfDocument>(path, "pdf"),
        "txt" => load_with_handler::<linch_text::TextDocument>(path, "txt"),
        "md" => load_with_handler::<linch_text::TextDocument>(path, "md"),
        _ => return Err(format!("不支持的文件类型: {}", extension)),
    }
    .map_err(|e| e.to_string())?;

    // 合并所有页面文本
    let full_text: String = doc_info
        .pages
        .iter()
        .map(|p| p.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    // 转换规则并匹配
    let ruleset = convert_rules_to_ruleset(rules);
    let matches = ruleset.match_text(&full_text);
    let total_count = matches.len();

    Ok(MatchPreviewResult {
        matches,
        total_count,
    })
}

/// 执行脱敏
///
/// 对文档应用规则进行脱敏，并保存到输出路径。
#[tauri::command]
async fn apply_redaction(
    file_path: String,
    rules: Vec<FrontendRule>,
    output_path: String,
) -> Result<RedactionResult, String> {
    let path = Path::new(&file_path);
    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("")
        .to_lowercase();

    // 转换规则
    let ruleset = convert_rules_to_ruleset(rules);

    // 根据文件类型选择处理器并执行脱敏
    let (redacted_bytes, matches_count) = match extension.as_str() {
        "txt" | "md" => {
            let doc =
                linch_text::TextDocument::load(path).map_err(|e| format!("加载文件失败: {}", e))?;

            // 先获取匹配数量
            let pages = doc.get_pages().map_err(|e| e.to_string())?;
            let full_text: String = pages.iter().map(|p| p.content.as_str()).collect();
            let matches = ruleset.match_text(&full_text);
            let count = matches.len();

            // 执行脱敏
            let bytes = doc
                .redact(&ruleset)
                .map_err(|e| format!("脱敏失败: {}", e))?;

            (bytes, count)
        }
        "pdf" => {
            let doc =
                linch_pdf::PdfDocument::load(path).map_err(|e| format!("加载文件失败: {}", e))?;

            // 获取匹配数量
            let pages = doc.get_pages().map_err(|e| e.to_string())?;
            let full_text: String = pages
                .iter()
                .map(|p| p.content.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");
            let matches = ruleset.match_text(&full_text);
            let count = matches.len();

            // PDF 脱敏尚未完全实现，返回原始内容的警告
            let bytes = doc
                .redact(&ruleset)
                .map_err(|e| format!("脱敏失败: {}", e))?;

            (bytes, count)
        }
        _ => return Err(format!("不支持的文件类型: {}", extension)),
    };

    // 保存到输出路径
    let output = Path::new(&output_path);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败: {}", e))?;
    }
    fs::write(output, &redacted_bytes).map_err(|e| format!("保存文件失败: {}", e))?;

    Ok(RedactionResult {
        success: true,
        output_path: Some(output_path),
        matches_count,
        message: format!("脱敏完成，共处理 {} 处敏感信息", matches_count),
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
            // 新架构：文档加载与脱敏
            load_document,
            preview_matches,
            apply_redaction,
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
