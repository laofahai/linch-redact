#[allow(dead_code)]
mod types;
mod utils;
mod text;
mod image;
mod metadata;
#[allow(dead_code)]
mod forms;
#[allow(dead_code)]
mod annotations;
mod safe_render;
mod detection;

pub use types::{
  RedactionMode, PageContentType, Mask,
  FileProcessRequest, ProcessRequest, ProcessResult,
  PdfAnalysis, Rule, DetectionHit,
};

use std::fs;
use std::path::Path;
use lopdf::{Document, Object, Stream};

use utils::{get_media_box, convert_masks_to_pdf_coords, get_page_content, detect_page_content_type};
use text::{process_content_stream, add_black_overlay};
use image::redact_page_images;
use detection::{analyze_pdf_file, detect_sensitive_content_in_pdf};

/// 应用清理选项
fn apply_cleaning(doc: &mut Document, cleaning: &types::CleaningOptions) -> Result<(), String> {
  if cleaning.document_info {
    if let Err(e) = metadata::clean_info_dict(doc) {
      log::warn!("清理文档信息失败: {}", e);
    }
  }

  if cleaning.xmp_metadata {
    if let Err(e) = metadata::clean_xmp_metadata(doc) {
      log::warn!("清理 XMP 元数据失败: {}", e);
    }
  }

  if cleaning.hidden_data {
    if let Err(e) = metadata::remove_hidden_data(doc) {
      log::warn!("清理隐藏数据失败: {}", e);
    }
  }

  if cleaning.javascript {
    if let Err(e) = metadata::remove_javascript(doc) {
      log::warn!("清理 JavaScript 失败: {}", e);
    }
  }

  if cleaning.attachments {
    if let Err(e) = metadata::remove_attachments(doc) {
      log::warn!("清理附件失败: {}", e);
    }
  }

  if cleaning.forms {
    if let Err(e) = forms::remove_all_forms(doc) {
      log::warn!("清理表单失败: {}", e);
    }
  }

  if cleaning.annotations {
    if let Err(e) = annotations::remove_all_annotations_from_document(doc) {
      log::warn!("清理注释失败: {}", e);
    }
  }

  Ok(())
}

/// 对页面进行脱敏处理
fn redact_page(
  doc: &mut Document,
  page_id: lopdf::ObjectId,
  masks: &[Mask],
  mode: &RedactionMode,
) -> Result<(), String> {
  let media_box = get_media_box(doc, page_id);
  log::info!("MediaBox: {:?}", media_box);
  log::info!("Input masks: {:?}", masks);
  let mask_rects = convert_masks_to_pdf_coords(masks, media_box);
  log::info!("Converted mask_rects: {:?}", mask_rects);

  let content_data = get_page_content(doc, page_id)?;

  let page_type = detect_page_content_type(&content_data);
  log::info!("页面类型检测: {:?}", page_type);

  let effective_mode = match mode {
    RedactionMode::Auto => {
      match page_type {
        PageContentType::Text => RedactionMode::TextReplace,
        PageContentType::PathDrawn => RedactionMode::SafeRender,
        PageContentType::ImageBased => RedactionMode::ImageMode,
        // Mixed 类型（表格等）使用 SafeRender，将页面渲染为图片后脱敏
        PageContentType::Mixed => RedactionMode::SafeRender,
        PageContentType::Empty => RedactionMode::BlackOverlay,
      }
    }
    _ => mode.clone(),
  };

  log::info!("使用脱敏模式: {:?}", effective_mode);

  match effective_mode {
    RedactionMode::TextReplace | RedactionMode::Auto => {
      // 先进行文字替换
      let processed_data = process_content_stream(&content_data, &mask_rects)?;
      // 然后添加黑框覆盖作为保险（处理编码字体等问题）
      let final_data = add_black_overlay(&processed_data, &mask_rects)?;
      let stream = Stream::new(lopdf::Dictionary::new(), final_data);
      let stream_id = doc.add_object(stream);
      if let Ok(Object::Dictionary(ref mut dict)) = doc.get_object_mut(page_id) {
        dict.set(b"Contents", Object::Reference(stream_id));
      }
    }
    RedactionMode::BlackOverlay => {
      let processed_data = add_black_overlay(&content_data, &mask_rects)?;
      let stream = Stream::new(lopdf::Dictionary::new(), processed_data);
      let stream_id = doc.add_object(stream);
      if let Ok(Object::Dictionary(ref mut dict)) = doc.get_object_mut(page_id) {
        dict.set(b"Contents", Object::Reference(stream_id));
      }
    }
    RedactionMode::ImageMode => {
      let processed = redact_page_images(doc, page_id, masks)?;
      if !processed {
        log::warn!("未找到可处理的图片，回退到黑框覆盖模式");
        let processed_data = add_black_overlay(&content_data, &mask_rects)?;
        let stream = Stream::new(lopdf::Dictionary::new(), processed_data);
        let stream_id = doc.add_object(stream);
        if let Ok(Object::Dictionary(ref mut dict)) = doc.get_object_mut(page_id) {
          dict.set(b"Contents", Object::Reference(stream_id));
        }
      }
    }
    RedactionMode::SafeRender => {
      // SafeRender 模式需要 pdfium-render，暂时回退到黑框覆盖
      log::warn!("SafeRender 模式尚未完全实现，回退到黑框覆盖模式");
      let processed_data = add_black_overlay(&content_data, &mask_rects)?;
      let stream = Stream::new(lopdf::Dictionary::new(), processed_data);
      let stream_id = doc.add_object(stream);
      if let Ok(Object::Dictionary(ref mut dict)) = doc.get_object_mut(page_id) {
        dict.set(b"Contents", Object::Reference(stream_id));
      }
    }
  }

  Ok(())
}

fn process_pdf_file(
  file_req: &FileProcessRequest,
  output_dir: &str,
  suffix: &str,
  mode: &RedactionMode,
  cleaning: &types::CleaningOptions,
) -> Result<String, String> {
  let input_path = Path::new(&file_req.path);
  let stem = input_path.file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or("output");
  let output_filename = format!("{}{}.pdf", stem, suffix);
  let output_path = Path::new(output_dir).join(&output_filename);

  // 检测是否需要使用 SafeRender 模式
  let use_safe_render = should_use_safe_render(&file_req.path, &file_req.masks_by_page, mode)?;

  // 用于回退的模式
  let mut fallback_mode = mode.clone();

  if use_safe_render {
    // 尝试使用 SafeRender 模式
    log::info!("尝试使用 SafeRender 模式处理文件: {}", file_req.path);
    let config = safe_render::RenderConfig::default();
    match safe_render::safe_redact_pdf(
      &file_req.path,
      output_path.to_str().ok_or("无效输出路径")?,
      &file_req.masks_by_page,
      &config,
    ) {
      Ok(()) => {
        return Ok(output_path.to_string_lossy().to_string());
      }
      Err(e) => {
        // SafeRender 失败，回退到 BlackOverlay 模式
        log::warn!("SafeRender 失败: {}，回退到 BlackOverlay 模式", e);
        fallback_mode = RedactionMode::BlackOverlay;
      }
    }
  }

  // 使用传统 lopdf 模式
  let effective_mode = &fallback_mode;
  let mut doc = Document::load(&file_req.path).map_err(|e| format!("无法加载 PDF: {}", e))?;

  let page_ids: Vec<lopdf::ObjectId> = doc.page_iter().collect();
  let total_pages = page_ids.len();

  let mut pages_to_delete: Vec<usize> = file_req.pages
    .iter()
    .filter(|p| p.action == "delete" && p.index < total_pages)
    .map(|p| p.index)
    .collect();
  pages_to_delete.sort_by(|a, b| b.cmp(a));

  log::info!("处理文件: {}, masks_by_page 数量: {}, 模式: {:?}", file_req.path, file_req.masks_by_page.len(), mode);
  for (page_idx, masks) in &file_req.masks_by_page {
    log::info!("页面 {}: {} 个 masks", page_idx, masks.len());
    for (i, m) in masks.iter().enumerate() {
      log::info!("  Mask {}: x={}, y={}, w={}, h={}", i, m.x, m.y, m.width, m.height);
    }
    if *page_idx < page_ids.len() && !masks.is_empty() {
      let page_id = page_ids[*page_idx];
      log::info!("正在处理页面 {} (page_id: {:?})", page_idx, page_id);
      if let Err(e) = redact_page(&mut doc, page_id, masks, effective_mode) {
        log::warn!("脱敏处理失败 (页 {}): {}", page_idx + 1, e);
      } else {
        log::info!("页面 {} 处理成功", page_idx);
      }
    }
  }

  for page_idx in pages_to_delete {
    let page_num = (page_idx + 1) as u32;
    doc.delete_pages(&[page_num]);
  }

  // 执行清理操作
  apply_cleaning(&mut doc, cleaning)?;

  // 设置脱敏工具元信息
  metadata::set_redaction_metadata(&mut doc)?;

  doc.compress();

  let mut file = fs::File::create(&output_path).map_err(|e| format!("创建文件失败: {}", e))?;
  doc.save_to(&mut file).map_err(|e| format!("保存失败: {}", e))?;

  Ok(output_path.to_string_lossy().to_string())
}

/// 判断是否应该使用 SafeRender 模式
fn should_use_safe_render(
  pdf_path: &str,
  masks_by_page: &std::collections::BTreeMap<usize, Vec<Mask>>,
  mode: &RedactionMode,
) -> Result<bool, String> {
  // 如果用户明确选择 SafeRender，直接使用
  if *mode == RedactionMode::SafeRender {
    return Ok(true);
  }

  // 如果用户明确选择其他模式（非 Auto），不使用 SafeRender
  if *mode != RedactionMode::Auto {
    return Ok(false);
  }

  // Auto 模式：检测页面类型
  let doc = Document::load(pdf_path).map_err(|e| format!("无法加载 PDF: {}", e))?;
  let page_ids: Vec<lopdf::ObjectId> = doc.page_iter().collect();

  for (page_idx, masks) in masks_by_page {
    if masks.is_empty() {
      continue;
    }
    if *page_idx >= page_ids.len() {
      continue;
    }

    let page_id = page_ids[*page_idx];
    let content_data = get_page_content(&doc, page_id).unwrap_or_default();
    let page_type = detect_page_content_type(&content_data);

    // Mixed 或 PathDrawn 类型使用 SafeRender
    if page_type == PageContentType::Mixed || page_type == PageContentType::PathDrawn {
      log::info!("页面 {} 类型为 {:?}，使用 SafeRender 模式", page_idx, page_type);
      return Ok(true);
    }
  }

  Ok(false)
}

#[tauri::command]
pub async fn process_pdfs(request: ProcessRequest) -> Result<ProcessResult, String> {
  let mut processed_files = Vec::new();
  let mut errors = Vec::new();

  fs::create_dir_all(&request.output_directory)
    .map_err(|e| format!("无法创建输出目录: {}", e))?;

  for file_req in &request.files {
    match process_pdf_file(file_req, &request.output_directory, &request.suffix, &request.mode, &request.cleaning) {
      Ok(output_path) => {
        processed_files.push(output_path);
      }
      Err(e) => {
        let filename = Path::new(&file_req.path)
          .file_name()
          .and_then(|s| s.to_str())
          .unwrap_or(&file_req.path);
        errors.push(format!("{}: {}", filename, e));
      }
    }
  }

  Ok(ProcessResult {
    success: errors.is_empty(),
    processed_files,
    errors,
  })
}

/// 分析 PDF 文件内容
#[tauri::command]
pub async fn analyze_pdf(pdf_path: String) -> Result<PdfAnalysis, String> {
  analyze_pdf_file(&pdf_path)
}

/// 检测敏感内容（基于规则）
#[tauri::command]
pub async fn detect_sensitive_content(
  pdf_path: String,
  rules: Vec<Rule>,
  use_ocr: Option<bool>,
  page_indices: Option<Vec<usize>>,  // 可选：指定要扫描的页面索引
) -> Result<Vec<DetectionHit>, String> {
  detect_sensitive_content_in_pdf(&pdf_path, &rules, use_ocr.unwrap_or(false), page_indices.as_deref())
}

