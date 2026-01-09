//! 敏感信息检测模块

use lopdf::{Document, Object};
use crate::pdf::types::{Rule, DetectionHit, DetectionBbox, PdfAnalysis, PageContentType, RedactionMode};
use crate::pdf::utils::{get_page_content, detect_page_content_type};
use crate::pdf::safe_render;
use std::collections::HashMap;
use std::time::Instant;

/// 分析 PDF 文件
pub fn analyze_pdf_file(pdf_path: &str) -> Result<PdfAnalysis, String> {
  let doc = Document::load(pdf_path).map_err(|e| format!("无法加载 PDF: {}", e))?;

  let page_ids: Vec<lopdf::ObjectId> = doc.page_iter().collect();

  // 检测每页的内容类型
  let mut page_types = Vec::new();
  for page_id in &page_ids {
    let content = get_page_content(&doc, *page_id).unwrap_or_default();
    let page_type = detect_page_content_type(&content);
    page_types.push(page_type);
  }

  let has_forms = check_has_forms(&doc);
  let has_annotations = check_has_annotations(&doc, &page_ids);
  let has_metadata = check_has_metadata(&doc);
  let has_attachments = check_has_attachments(&doc);
  let has_javascript = check_has_javascript(&doc);
  let recommended_mode = recommend_mode(&page_types);

  Ok(PdfAnalysis {
    page_types,
    has_forms,
    has_annotations,
    has_metadata,
    has_attachments,
    has_javascript,
    recommended_mode,
  })
}

/// 检测敏感内容（基于规则）
pub fn detect_sensitive_content_in_pdf(
  pdf_path: &str,
  rules: &[Rule],
  use_ocr: bool,
  page_indices: Option<&[usize]>,  // 可选：指定要扫描的页面索引
) -> Result<Vec<DetectionHit>, String> {
  let mut hits = Vec::new();

  // 过滤启用的规则
  let enabled_rules: Vec<&Rule> = rules.iter().filter(|r| r.enabled).collect();

  if enabled_rules.is_empty() {
    return Ok(hits);
  }

  // 编译正则表达式
  let compiled_rules: Vec<(&Rule, Option<regex::Regex>)> = enabled_rules
    .iter()
    .map(|r| {
      let regex = if r.rule_type == "regex" {
        regex::Regex::new(&r.pattern).ok()
      } else {
        None
      };
      (*r, regex)
    })
    .collect();

  // 将页面索引转换为 HashSet 以便快速查找
  let target_pages: Option<std::collections::HashSet<usize>> = page_indices
    .map(|indices| indices.iter().copied().collect());

  // 优先使用 pdfium 提取文本（更准确的编码处理）
  let mut page_texts = match safe_render::extract_text_from_pdf(pdf_path) {
    Ok(texts) => {
      log::info!("[Detection] 使用 pdfium 提取文本成功");
      // 如果指定了页面，只保留目标页面
      if let Some(ref targets) = target_pages {
        texts.into_iter().filter(|(idx, _)| targets.contains(idx)).collect()
      } else {
        texts
      }
    }
    Err(e) => {
      log::warn!("[Detection] pdfium 提取失败: {}，回退到 lopdf", e);
      // 回退到 lopdf 的原始提取方式
      let texts = extract_text_with_lopdf(pdf_path)?;
      if let Some(ref targets) = target_pages {
        texts.into_iter().filter(|(idx, _)| targets.contains(idx)).collect()
      } else {
        texts
      }
    }
  };

  let mut ocr_results_by_page: HashMap<usize, Vec<crate::ocr::OcrTextResult>> = HashMap::new();

  // 如果启用 OCR，对没有提取到文本的页面进行 OCR
  if use_ocr {
    log::info!("[Detection] OCR 已启用，检查是否需要 OCR 识别");

    // 分析 PDF 获取页面类型
    let analysis = analyze_pdf_file(pdf_path)?;

    // 找出图片型页面（或文本为空的页面）
    let text_page_indices: std::collections::HashSet<usize> =
      page_texts.iter().map(|(idx, _)| *idx).collect();

    let mut ocr_needed_pages: Vec<usize> = Vec::new();
    for (idx, page_type) in analysis.page_types.iter().enumerate() {
      // 如果指定了页面，检查是否在目标页面中
      if let Some(ref targets) = target_pages {
        if !targets.contains(&idx) {
          continue;
        }
      }
      if *page_type == PageContentType::ImageBased || !text_page_indices.contains(&idx) {
        ocr_needed_pages.push(idx);
      }
    }

    if !ocr_needed_pages.is_empty() {
      log::info!("[Detection] 需要 OCR 的页面: {:?}", ocr_needed_pages);

      // 对每个需要 OCR 的页面进行识别
      for page_idx in ocr_needed_pages {
        match ocr_page(pdf_path, page_idx) {
          Ok((text, results)) => {
            if !text.is_empty() {
              log::info!("[Detection] OCR 页面 {} 识别到 {} 个字符", page_idx, text.len());
              page_texts.push((page_idx, text));
            }
            if !results.is_empty() {
              ocr_results_by_page.insert(page_idx, results);
            }
          }
          Err(e) => {
            log::warn!("[Detection] OCR 页面 {} 失败: {}", page_idx, e);
          }
        }
      }
    }
  }

  // 遍历每页检测
  for (page_idx, text) in &page_texts {
    log::info!("[Detection] 页面 {} 提取到文本长度: {}", page_idx, text.len());
    if !text.is_empty() {
      if should_log_full_text() {
        log::info!("[Detection] 文本全文: {:?}", text);
      } else {
        // 打印前 200 个字符用于调试
        let preview: String = text.chars().take(200).collect();
        log::info!("[Detection] 文本预览: {:?}", preview);
      }
    }

    if let Some(ocr_results) = ocr_results_by_page.get(page_idx) {
      let mut added_positions: std::collections::HashSet<String> = std::collections::HashSet::new();

      for ocr_result in ocr_results {
        let ocr_text = ocr_result.text.trim();
        if ocr_text.is_empty() {
          continue;
        }

        for (rule, regex_opt) in &compiled_rules {
          let mut matched = text_matches_rule(ocr_text, rule, regex_opt.as_ref());
          if !matched && rule_uses_digits(rule) {
            let compact = compact_numeric_text(ocr_text);
            if !compact.is_empty() && compact != ocr_text {
              matched = text_matches_rule(&compact, rule, regex_opt.as_ref());
            }
          }

          if !matched {
            continue;
          }

          let bbox = &ocr_result.bbox;
          let pos_key = format!("{:.3},{:.3},{:.3},{:.3}", bbox.x, bbox.y, bbox.w, bbox.h);
          if added_positions.contains(&pos_key) {
            continue;
          }
          added_positions.insert(pos_key);

          let snippet = mask_snippet(ocr_text);
          hits.push(DetectionHit {
            page: *page_idx,
            bbox: DetectionBbox {
              x: bbox.x as f64,
              y: bbox.y as f64,
              width: bbox.w as f64,
              height: bbox.h as f64,
            },
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            snippet,
          });
        }
      }

      continue;
    }

    // 收集当前页面所有规则的匹配结果
    let mut rule_matches: Vec<(&Rule, &str)> = Vec::new();

    // 对每个规则进行匹配
    for (rule, regex_opt) in &compiled_rules {
      // 收集匹配的文本（包含位置信息用于边界检查）
      let matches_with_pos: Vec<(usize, &str)> = if let Some(regex) = regex_opt {
        regex
          .find_iter(text)
          .map(|m| (m.start(), m.as_str()))
          .collect()
      } else {
        // 关键词匹配
        text
          .match_indices(&rule.pattern)
          .collect()
      };

      // 边界检查：过滤掉前后有数字的匹配（避免匹配到更长数字的一部分）
      let filtered_matches: Vec<&str> = matches_with_pos
        .into_iter()
        .filter(|(start, matched)| {
          // start 是字节索引，需要检查前一个字节对应的字符
          // 检查前一个字符
          if *start > 0 {
            if let Some(prev_char) = text[..*start].chars().last() {
              if prev_char.is_ascii_digit() {
                return false;
              }
            }
          }
          // 检查后一个字符
          let end_pos = *start + matched.len();
          if let Some(next_char) = text[end_pos..].chars().next() {
            if next_char.is_ascii_digit() {
              return false;
            }
          }
          true
        })
        .map(|(_, s)| s)
        .collect();

      // 去重：相同的文本只搜索一次
      let unique_matches: std::collections::HashSet<&str> = filtered_matches.into_iter().collect();

      // 收集当前规则的所有匹配文本，稍后批量搜索
      for matched_text in unique_matches {
        rule_matches.push((rule, matched_text));
      }
    }

    // 批量搜索当前页面的所有匹配文本（性能优化：只打开 PDF 一次）
    if !rule_matches.is_empty() {
      let search_terms: Vec<&str> = rule_matches.iter().map(|(_, t)| *t).collect();

      // 用于追踪已添加的位置，避免重复
      let mut added_positions: std::collections::HashSet<String> = std::collections::HashSet::new();

      match safe_render::batch_search_text_in_page(pdf_path, *page_idx, &search_terms) {
        Ok(batch_results) => {
          // 建立搜索词到规则的映射
          let term_to_rule: std::collections::HashMap<&str, &Rule> = rule_matches
            .iter()
            .map(|(r, t)| (*t, *r))
            .collect();

          for (search_term, search_results) in batch_results {
            let rule = match term_to_rule.get(search_term.as_str()) {
              Some(r) => *r,
              None => continue,
            };

            for result in search_results {
              let pos_key = format!(
                "{:.3},{:.3},{:.3},{:.3}",
                result.x, result.y, result.width, result.height
              );

              if added_positions.contains(&pos_key) {
                continue;
              }
              added_positions.insert(pos_key);

              let snippet = mask_snippet(&search_term);
              hits.push(DetectionHit {
                page: *page_idx,
                bbox: DetectionBbox {
                  x: result.x,
                  y: result.y,
                  width: result.width,
                  height: result.height,
                },
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
                snippet,
              });
            }
          }
        }
        Err(e) => {
          log::warn!("[Detection] 批量搜索位置失败: {}，使用估算位置", e);
          // 回退到估算位置
          for (rule, matched_text) in &rule_matches {
            let snippet = mask_snippet(matched_text);
            let text_len = matched_text.chars().count() as f64;
            let width = (text_len * 0.015).min(0.4).max(0.08);

            hits.push(DetectionHit {
              page: *page_idx,
              bbox: DetectionBbox {
                x: 0.05,
                y: 0.5,
                width,
                height: 0.025,
              },
              rule_id: rule.id.clone(),
              rule_name: rule.name.clone(),
              snippet,
            });
          }
        }
      }
    }
  }

  Ok(hits)
}

/// 使用 lopdf 提取文本（回退方案）
fn extract_text_with_lopdf(pdf_path: &str) -> Result<Vec<(usize, String)>, String> {
  let doc = Document::load(pdf_path).map_err(|e| format!("无法加载 PDF: {}", e))?;
  let page_ids: Vec<lopdf::ObjectId> = doc.page_iter().collect();
  let mut results = Vec::new();

  for (page_idx, page_id) in page_ids.iter().enumerate() {
    let content_data = get_page_content(&doc, *page_id).unwrap_or_default();
    let text = extract_text_from_content(&content_data);
    if !text.is_empty() {
      results.push((page_idx, text));
    }
  }

  Ok(results)
}

// ============ PDF 分析辅助函数 ============

fn check_has_forms(doc: &Document) -> bool {
  if let Ok(Object::Reference(catalog_ref)) = doc.trailer.get(b"Root") {
    if let Ok(Object::Dictionary(catalog)) = doc.get_object(*catalog_ref) {
      return catalog.has(b"AcroForm");
    }
  }
  false
}

fn check_has_annotations(doc: &Document, page_ids: &[lopdf::ObjectId]) -> bool {
  for page_id in page_ids {
    if let Ok(Object::Dictionary(page_dict)) = doc.get_object(*page_id) {
      if page_dict.has(b"Annots") {
        return true;
      }
    }
  }
  false
}

fn check_has_metadata(doc: &Document) -> bool {
  // 检查 Info 字典
  if doc.trailer.has(b"Info") {
    return true;
  }
  // 检查 XMP Metadata
  if let Ok(Object::Reference(catalog_ref)) = doc.trailer.get(b"Root") {
    if let Ok(Object::Dictionary(catalog)) = doc.get_object(*catalog_ref) {
      return catalog.has(b"Metadata");
    }
  }
  false
}

fn check_has_attachments(doc: &Document) -> bool {
  if let Ok(Object::Reference(catalog_ref)) = doc.trailer.get(b"Root") {
    if let Ok(Object::Dictionary(catalog)) = doc.get_object(*catalog_ref) {
      if let Ok(Object::Reference(names_ref)) = catalog.get(b"Names") {
        if let Ok(Object::Dictionary(names)) = doc.get_object(*names_ref) {
          return names.has(b"EmbeddedFiles");
        }
      }
    }
  }
  false
}

fn check_has_javascript(doc: &Document) -> bool {
  if let Ok(Object::Reference(catalog_ref)) = doc.trailer.get(b"Root") {
    if let Ok(Object::Dictionary(catalog)) = doc.get_object(*catalog_ref) {
      if let Ok(Object::Reference(names_ref)) = catalog.get(b"Names") {
        if let Ok(Object::Dictionary(names)) = doc.get_object(*names_ref) {
          return names.has(b"JavaScript");
        }
      }
    }
  }
  false
}

fn recommend_mode(page_types: &[PageContentType]) -> RedactionMode {
  if page_types.is_empty() {
    return RedactionMode::Auto;
  }

  let has_image = page_types.iter().any(|t| *t == PageContentType::ImageBased);
  let has_path = page_types.iter().any(|t| *t == PageContentType::PathDrawn);
  let has_mixed = page_types.iter().any(|t| *t == PageContentType::Mixed);
  let all_text = page_types.iter().all(|t| *t == PageContentType::Text || *t == PageContentType::Empty);

  if all_text {
    RedactionMode::TextReplace
  } else if has_image {
    RedactionMode::ImageMode
  } else if has_path || has_mixed {
    RedactionMode::SafeRender
  } else {
    RedactionMode::Auto
  }
}

// ============ 文本提取函数 ============

/// 从内容流中提取纯文本
fn extract_text_from_content(content: &[u8]) -> String {
  let content_str = String::from_utf8_lossy(content);
  let mut text = String::new();

  // 状态机：提取括号字符串和尖括号十六进制字符串
  let mut in_literal = false;    // (...)
  let mut in_hex = false;        // <...>
  let mut escape_next = false;
  let mut current = String::new();
  let mut hex_buf = String::new();

  for ch in content_str.chars() {
    if escape_next {
      escape_next = false;
      if in_literal {
        current.push(ch);
      }
      continue;
    }

    match ch {
      '\\' if in_literal => {
        escape_next = true;
      }
      '(' if !in_literal && !in_hex => {
        in_literal = true;
        current.clear();
      }
      ')' if in_literal => {
        in_literal = false;
        // 尝试解码可能的编码文本
        let decoded = decode_pdf_string(&current);
        if !decoded.trim().is_empty() {
          text.push_str(&decoded);
          text.push(' ');
        }
      }
      '<' if !in_literal && !in_hex => {
        // 检查是否是 << (字典开始)
        in_hex = true;
        hex_buf.clear();
      }
      '>' if in_hex => {
        in_hex = false;
        // 解码十六进制字符串
        let decoded = decode_hex_string(&hex_buf);
        if !decoded.trim().is_empty() {
          text.push_str(&decoded);
          text.push(' ');
        }
      }
      _ if in_literal => {
        current.push(ch);
      }
      _ if in_hex => {
        if ch.is_ascii_hexdigit() {
          hex_buf.push(ch);
        }
      }
      _ => {}
    }
  }

  text
}

/// 解码 PDF 字符串（处理转义和编码）
fn decode_pdf_string(s: &str) -> String {
  let mut result = String::new();
  let mut chars = s.chars().peekable();

  while let Some(ch) = chars.next() {
    if ch == '\\' {
      if let Some(&next) = chars.peek() {
        chars.next();
        match next {
          'n' => result.push('\n'),
          'r' => result.push('\r'),
          't' => result.push('\t'),
          '\\' => result.push('\\'),
          '(' => result.push('('),
          ')' => result.push(')'),
          // 八进制转义 \nnn
          '0'..='7' => {
            let mut octal = String::new();
            octal.push(next);
            for _ in 0..2 {
              if let Some(&c) = chars.peek() {
                if c >= '0' && c <= '7' {
                  octal.push(chars.next().unwrap());
                } else {
                  break;
                }
              }
            }
            if let Ok(val) = u8::from_str_radix(&octal, 8) {
              if val < 128 {
                result.push(val as char);
              }
            }
          }
          _ => {
            result.push(next);
          }
        }
      }
    } else {
      result.push(ch);
    }
  }

  result
}

/// 解码十六进制字符串
fn decode_hex_string(hex: &str) -> String {
  let mut result = String::new();
  let hex_clean: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();

  // 尝试作为 UTF-16BE 解码（常见于 CID 字体）
  let bytes: Vec<u8> = (0..hex_clean.len())
    .step_by(2)
    .filter_map(|i| {
      if i + 2 <= hex_clean.len() {
        u8::from_str_radix(&hex_clean[i..i + 2], 16).ok()
      } else if i + 1 <= hex_clean.len() {
        // 奇数长度，补0
        u8::from_str_radix(&format!("{}0", &hex_clean[i..i + 1]), 16).ok()
      } else {
        None
      }
    })
    .collect();

  // 先尝试 UTF-16BE
  if bytes.len() >= 2 && bytes.len() % 2 == 0 {
    let u16_vec: Vec<u16> = bytes
      .chunks(2)
      .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
      .collect();

    if let Ok(decoded) = String::from_utf16(&u16_vec) {
      // 检查是否是有效的可打印文本
      if decoded.chars().all(|c| !c.is_control() || c == '\n' || c == '\r' || c == '\t') {
        return decoded;
      }
    }
  }

  // 回退到 Latin-1/ASCII
  for &b in &bytes {
    if b >= 32 && b < 127 {
      result.push(b as char);
    } else if b >= 128 {
      // Latin-1 扩展字符
      result.push(char::from_u32(b as u32).unwrap_or('?'));
    }
  }

  result
}

/// 脱敏显示 snippet
fn rule_uses_digits(rule: &Rule) -> bool {
  rule.pattern.contains("\\d") || rule.pattern.contains("[0-9]")
}

fn compact_numeric_text(text: &str) -> String {
  text.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

fn text_matches_rule(
  text: &str,
  rule: &Rule,
  regex_opt: Option<&regex::Regex>,
) -> bool {
  let matches_with_pos: Vec<(usize, &str)> = if let Some(regex) = regex_opt {
    regex
      .find_iter(text)
      .map(|m| (m.start(), m.as_str()))
      .collect()
  } else {
    text.match_indices(&rule.pattern).collect()
  };

  matches_with_pos.into_iter().any(|(start, matched)| {
    if start > 0 {
      if let Some(prev_char) = text[..start].chars().last() {
        if prev_char.is_ascii_digit() {
          return false;
        }
      }
    }
    let end_pos = start + matched.len();
    if let Some(next_char) = text[end_pos..].chars().next() {
      if next_char.is_ascii_digit() {
        return false;
      }
    }
    true
  })
}

fn mask_snippet(text: &str) -> String {
  let chars: Vec<char> = text.chars().collect();
  let len = chars.len();

  if len <= 4 {
    "*".repeat(len)
  } else {
    let visible = 4.min(len / 3);
    let prefix: String = chars[..visible].iter().collect();
    let suffix: String = chars[len - visible..].iter().collect();
    format!("{}****{}", prefix, suffix)
  }
}

/// 对 PDF 页面进行 OCR 识别
///
/// 1. 将 PDF 页面渲染为图片
/// 2. 调用当前配置的 OCR 引擎识别
/// 3. 返回识别出的文本
fn ocr_page(
  pdf_path: &str,
  page_index: usize,
) -> Result<(String, Vec<crate::ocr::OcrTextResult>), String> {
  // 创建临时文件路径
  let temp_dir = std::env::temp_dir();
  let temp_image_path = temp_dir.join(format!("ocr_page_{}_{}.png",
    std::process::id(), page_index));
  let temp_image_str = temp_image_path.to_string_lossy().to_string();

  let render_start = Instant::now();
  let dpi = std::env::var("LINCH_OCR_DPI")
    .ok()
    .and_then(|v| v.parse::<u32>().ok())
    .filter(|v| *v > 0)
    .unwrap_or(150);

  // 渲染 PDF 页面到图片（可通过 LINCH_OCR_DPI 覆盖）
  safe_render::render_page_to_image(pdf_path, page_index, &temp_image_str, dpi)?;
  log::info!(
    "[Detection] 页面 {} 渲染耗时: {} ms",
    page_index,
    render_start.elapsed().as_millis()
  );

  // 使用当前配置的 OCR 引擎识别
  let results = crate::ocr::recognize_with_current_engine(&temp_image_str)?;
  let text = results
    .iter()
    .map(|r| r.text.as_str())
    .collect::<Vec<_>>()
    .join(" ");

  // 删除临时文件
  if let Err(e) = std::fs::remove_file(&temp_image_path) {
    log::warn!("[OCR] 删除临时文件失败: {}", e);
  }

  Ok((text, results))
}

fn should_log_full_text() -> bool {
  match std::env::var("LINCH_LOG_FULL_TEXT") {
    Ok(val) => {
      let val = val.to_ascii_lowercase();
      val == "1" || val == "true" || val == "yes"
    }
    Err(_) => false,
  }
}
