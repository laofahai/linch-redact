//! 安全渲染脱敏模块
//!
//! 使用 pdfium-render 将页面渲染为图片，在图片上绘制黑框，
//! 然后用图片替换原页面内容。这样底层文字完全被销毁，无法复制。

use image::{DynamicImage, Rgba, RgbaImage};
use imageproc::drawing::draw_filled_rect_mut;
use imageproc::rect::Rect;
use pdfium_render::prelude::*;
use std::path::PathBuf;

use super::types::Mask;

/// 获取 pdfium 库的搜索路径
fn get_pdfium_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // 1. Linux/Windows: 可执行文件同级的 libs 目录
            paths.push(exe_dir.join("libs"));

            // 2. Linux/Windows: 可执行文件同级目录
            paths.push(exe_dir.to_path_buf());

            // 3. macOS: app bundle 内的 Resources 目录
            #[cfg(target_os = "macos")]
            {
                // .app/Contents/MacOS/app -> .app/Contents/Resources/libs
                if let Some(contents_dir) = exe_dir.parent() {
                    paths.push(contents_dir.join("Resources").join("libs"));
                    paths.push(contents_dir.join("Resources"));
                }
            }

            // 4. Linux AppImage: 检查 APPDIR 环境变量
            #[cfg(target_os = "linux")]
            {
                if let Ok(appdir) = std::env::var("APPDIR") {
                    let appdir_path = PathBuf::from(appdir);
                    paths.push(appdir_path.join("usr").join("lib").join("libs"));
                    paths.push(appdir_path.join("usr").join("lib"));
                }
            }
        }
    }

    // 5. 本地开发: src-tauri/libs 目录
    paths.push(PathBuf::from("libs"));
    paths.push(PathBuf::from("src-tauri/libs"));

    // 6. 当前目录
    paths.push(PathBuf::from("./"));

    paths
}

/// 尝试绑定 pdfium 库
fn bind_pdfium() -> Result<Pdfium, String> {
    let search_paths = get_pdfium_search_paths();

    // 尝试从各个路径加载
    for path in &search_paths {
        let lib_path = Pdfium::pdfium_platform_library_name_at_path(path);
        log::debug!("[SafeRender] 尝试加载 pdfium: {:?}", lib_path);

        if let Ok(bindings) = Pdfium::bind_to_library(&lib_path) {
            log::info!("[SafeRender] 成功从 {:?} 加载 pdfium", path);
            return Ok(Pdfium::new(bindings));
        }
    }

    // 最后尝试系统库
    log::debug!("[SafeRender] 尝试加载系统 pdfium 库");
    Pdfium::bind_to_system_library()
        .map(Pdfium::new)
        .map_err(|e| {
            format!(
                "Pdfium 库不可用: {}。\n请运行 ./scripts/setup-pdfium.sh 下载 pdfium 库。",
                e
            )
        })
}

/// 渲染配置
pub struct RenderConfig {
    /// DPI（每英寸点数），默认 150
    pub dpi: u32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self { dpi: 150 }
    }
}

/// 对单个页面进行安全脱敏
///
/// 1. 使用 pdfium 渲染页面为图片
/// 2. 在图片上绘制黑色矩形覆盖 mask 区域
/// 3. 返回脱敏后的图片
pub fn render_and_redact_page(
    pdfium: &Pdfium,
    pdf_path: &str,
    page_index: usize,
    masks: &[Mask],
    config: &RenderConfig,
) -> Result<DynamicImage, String> {
    let document = pdfium
        .load_pdf_from_file(pdf_path, None)
        .map_err(|e| format!("加载 PDF 失败: {}", e))?;

    let page = document
        .pages()
        .get(page_index as u16)
        .map_err(|e| format!("获取页面 {} 失败: {}", page_index, e))?;

    // 计算渲染尺寸
    let page_width = page.width().value;
    let page_height = page.height().value;

    // PDF 默认 72 DPI，计算目标像素尺寸
    let scale = config.dpi as f32 / 72.0;
    let target_width = (page_width * scale) as i32;
    let target_height = (page_height * scale) as i32;

    log::info!(
        "[SafeRender] 页面 {}: {}x{} pt -> {}x{} px (DPI: {})",
        page_index,
        page_width,
        page_height,
        target_width,
        target_height,
        config.dpi
    );

    // 渲染页面为图片
    let render_config = PdfRenderConfig::new()
        .set_target_width(target_width)
        .set_target_height(target_height);

    let bitmap = page
        .render_with_config(&render_config)
        .map_err(|e| format!("渲染页面失败: {}", e))?;

    let mut image: RgbaImage = bitmap
        .as_image()
        .as_rgba8()
        .ok_or("转换图片格式失败")?
        .clone();

    // 在图片上绘制黑色矩形
    let black = Rgba([0u8, 0u8, 0u8, 255u8]);

    for mask in masks {
        // mask 坐标是相对坐标 (0-1)，转换为像素坐标
        let x = (mask.x * target_width as f64) as i32;
        let y = (mask.y * target_height as f64) as i32;
        let w = (mask.width * target_width as f64) as u32;
        let h = (mask.height * target_height as f64) as u32;

        // 确保坐标在有效范围内
        let x = x.max(0) as u32;
        let y = y.max(0) as u32;
        let w = w.min(target_width as u32 - x);
        let h = h.min(target_height as u32 - y);

        if w > 0 && h > 0 {
            let rect = Rect::at(x as i32, y as i32).of_size(w, h);
            draw_filled_rect_mut(&mut image, rect, black);
            log::info!("[SafeRender] 绘制黑框: ({}, {}, {}, {})", x, y, w, h);
        }
    }

    Ok(DynamicImage::ImageRgba8(image))
}

/// 对整个 PDF 进行安全脱敏，生成新的 PDF 文件
///
/// 将每个需要脱敏的页面渲染为图片，绘制黑框后保存为新 PDF
pub fn safe_redact_pdf(
    input_path: &str,
    output_path: &str,
    masks_by_page: &std::collections::BTreeMap<usize, Vec<Mask>>,
    config: &RenderConfig,
) -> Result<(), String> {
    // 尝试初始化 pdfium，如果失败则返回错误让调用方回退
    let pdfium = bind_pdfium()?;

    let document = pdfium
        .load_pdf_from_file(input_path, None)
        .map_err(|e| format!("加载 PDF 失败: {}", e))?;

    let page_count = document.pages().len();

    // 创建新文档
    let mut new_doc = pdfium
        .create_new_pdf()
        .map_err(|e| format!("创建新 PDF 失败: {}", e))?;

    for page_idx in 0..page_count {
        let page = document
            .pages()
            .get(page_idx)
            .map_err(|e| format!("获取页面 {} 失败: {}", page_idx, e))?;

        let page_width = page.width();
        let page_height = page.height();

        // 检查此页是否需要脱敏
        let masks = masks_by_page.get(&(page_idx as usize));

        if let Some(masks) = masks {
            if !masks.is_empty() {
                // 需要脱敏：渲染为图片并添加黑框
                let redacted_image =
                    render_and_redact_page(&pdfium, input_path, page_idx as usize, masks, config)?;

                // 将图片添加到新 PDF
                let mut new_page = new_doc
                    .pages_mut()
                    .create_page_at_end(PdfPagePaperSize::Custom(page_width, page_height))
                    .map_err(|e| format!("创建页面失败: {}", e))?;

                // 保存图片到临时文件
                let temp_path = format!("/tmp/redact_page_{}.jpg", page_idx);
                redacted_image
                    .to_rgb8()
                    .save_with_format(&temp_path, image::ImageFormat::Jpeg)
                    .map_err(|e| format!("保存临时图片失败: {}", e))?;

                // 创建图片对象并添加到页面
                let mut image_obj = PdfPageImageObject::new_from_jpeg_file(&new_doc, &temp_path)
                    .map_err(|e| format!("创建图片对象失败: {}", e))?;

                // 设置图片尺寸和位置（覆盖整个页面）
                image_obj
                    .scale(page_width.value, page_height.value)
                    .map_err(|e| format!("缩放图片失败: {}", e))?;

                new_page
                    .objects_mut()
                    .add_image_object(image_obj)
                    .map_err(|e| format!("添加图片到页面失败: {}", e))?;

                // 删除临时文件
                let _ = std::fs::remove_file(&temp_path);

                log::info!("[SafeRender] 页面 {} 脱敏完成", page_idx);
            } else {
                // 没有 mask，复制原页面
                copy_page(&pdfium, &document, &mut new_doc, page_idx)?;
            }
        } else {
            // 没有 mask，复制原页面
            copy_page(&pdfium, &document, &mut new_doc, page_idx)?;
        }
    }

    // 保存新 PDF
    new_doc
        .save_to_file(output_path)
        .map_err(|e| format!("保存 PDF 失败: {}", e))?;

    log::info!("[SafeRender] PDF 保存到: {}", output_path);
    Ok(())
}

/// 复制页面到新文档（对于不需要脱敏的页面）
fn copy_page(
    _pdfium: &Pdfium,
    source_doc: &PdfDocument,
    dest_doc: &mut PdfDocument,
    page_idx: u16,
) -> Result<(), String> {
    let source_page = source_doc
        .pages()
        .get(page_idx)
        .map_err(|e| format!("获取源页面 {} 失败: {}", page_idx, e))?;

    let page_width = source_page.width();
    let page_height = source_page.height();

    // 渲染源页面为图片（高清晰度）
    let render_config = PdfRenderConfig::new()
        .set_target_width((page_width.value * 2.0) as i32)
        .set_target_height((page_height.value * 2.0) as i32);

    let bitmap = source_page
        .render_with_config(&render_config)
        .map_err(|e| format!("渲染页面失败: {}", e))?;

    let image = bitmap.as_image();

    // 保存到临时文件
    let temp_path = format!("/tmp/copy_page_{}.jpg", page_idx);
    image
        .to_rgb8()
        .save_with_format(&temp_path, image::ImageFormat::Jpeg)
        .map_err(|e| format!("保存临时图片失败: {}", e))?;

    // 创建新页面
    let mut new_page = dest_doc
        .pages_mut()
        .create_page_at_end(PdfPagePaperSize::Custom(page_width, page_height))
        .map_err(|e| format!("创建页面失败: {}", e))?;

    // 添加图片
    let mut image_obj = PdfPageImageObject::new_from_jpeg_file(dest_doc, &temp_path)
        .map_err(|e| format!("创建图片对象失败: {}", e))?;

    image_obj
        .scale(page_width.value, page_height.value)
        .map_err(|e| format!("缩放图片失败: {}", e))?;

    new_page
        .objects_mut()
        .add_image_object(image_obj)
        .map_err(|e| format!("添加图片到页面失败: {}", e))?;

    // 删除临时文件
    let _ = std::fs::remove_file(&temp_path);

    Ok(())
}

/// 使用 pdfium 提取 PDF 中的所有文本
///
/// 返回 Vec<(page_index, text)>
pub fn extract_text_from_pdf(pdf_path: &str) -> Result<Vec<(usize, String)>, String> {
    let pdfium = bind_pdfium()?;

    let document = pdfium
        .load_pdf_from_file(pdf_path, None)
        .map_err(|e| format!("加载 PDF 失败: {}", e))?;

    let page_count = document.pages().len();
    let mut results = Vec::new();

    for page_idx in 0..page_count {
        let page = document
            .pages()
            .get(page_idx)
            .map_err(|e| format!("获取页面 {} 失败: {}", page_idx, e))?;

        // 提取页面文本
        let text = page.text().map_err(|e| format!("提取文本失败: {}", e))?;
        let page_text = text.all();

        if !page_text.is_empty() {
            results.push((page_idx as usize, page_text));
        }
    }

    Ok(results)
}

/// 文本搜索结果
#[derive(Debug, Clone)]
pub struct TextSearchResult {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// 在 PDF 页面中搜索文本并返回精确位置
#[allow(dead_code)]
pub fn search_text_in_page(
    pdf_path: &str,
    page_index: usize,
    search_term: &str,
) -> Result<Vec<TextSearchResult>, String> {
    let results = batch_search_text_in_page(pdf_path, page_index, &[search_term])?;
    Ok(results.into_iter().flat_map(|(_, v)| v).collect())
}

/// 批量在 PDF 页面中搜索多个文本（性能优化：只打开 PDF 一次）
pub fn batch_search_text_in_page(
    pdf_path: &str,
    page_index: usize,
    search_terms: &[&str],
) -> Result<Vec<(String, Vec<TextSearchResult>)>, String> {
    let pdfium = bind_pdfium()?;

    let document = pdfium
        .load_pdf_from_file(pdf_path, None)
        .map_err(|e| format!("加载 PDF 失败: {}", e))?;

    let page = document
        .pages()
        .get(page_index as u16)
        .map_err(|e| format!("获取页面 {} 失败: {}", page_index, e))?;

    let page_width = page.width().value as f64;
    let page_height = page.height().value as f64;

    let text = page.text().map_err(|e| format!("提取文本失败: {}", e))?;
    let search_options = PdfSearchOptions::new();

    let mut all_results = Vec::new();

    for search_term in search_terms {
        let search = match text.search(search_term, &search_options) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let mut results = Vec::new();

        for segments in search.iter(PdfSearchDirection::SearchForward) {
            for segment in segments.iter() {
                let bounds = segment.bounds();

                let pdf_left = bounds.left().value as f64;
                let pdf_bottom = bounds.bottom().value as f64;
                let pdf_right = bounds.right().value as f64;
                let pdf_top = bounds.top().value as f64;

                let x = pdf_left / page_width;
                let y = 1.0 - (pdf_top / page_height);
                let width = (pdf_right - pdf_left) / page_width;
                let height = (pdf_top - pdf_bottom) / page_height;

                let padding = 0.003;
                results.push(TextSearchResult {
                    x: (x - padding).max(0.0),
                    y: (y - padding).max(0.0),
                    width: (width + padding * 2.0).min(1.0),
                    height: (height + padding * 2.0).min(1.0),
                });
            }
        }

        if !results.is_empty() {
            all_results.push((search_term.to_string(), results));
        }
    }

    Ok(all_results)
}

/// 渲染 PDF 页面到图片文件（用于 OCR）
///
/// 返回保存的图片文件路径
pub fn render_page_to_image(
    pdf_path: &str,
    page_index: usize,
    output_path: &str,
    dpi: u32,
) -> Result<(), String> {
    let pdfium = bind_pdfium()?;

    let document = pdfium
        .load_pdf_from_file(pdf_path, None)
        .map_err(|e| format!("加载 PDF 失败: {}", e))?;

    let page = document
        .pages()
        .get(page_index as u16)
        .map_err(|e| format!("获取页面 {} 失败: {}", page_index, e))?;

    // 计算渲染尺寸
    let page_width = page.width().value;
    let page_height = page.height().value;

    // PDF 默认 72 DPI，计算目标像素尺寸
    let scale = dpi as f32 / 72.0;
    let target_width = (page_width * scale) as i32;
    let target_height = (page_height * scale) as i32;

    log::info!(
        "[RenderPage] 页面 {}: {}x{} pt -> {}x{} px (DPI: {})",
        page_index,
        page_width,
        page_height,
        target_width,
        target_height,
        dpi
    );

    // 渲染页面为图片
    let render_config = PdfRenderConfig::new()
        .set_target_width(target_width)
        .set_target_height(target_height);

    let bitmap = page
        .render_with_config(&render_config)
        .map_err(|e| format!("渲染页面失败: {}", e))?;

    let image = bitmap.as_image();

    // 保存到文件
    image
        .to_rgb8()
        .save(output_path)
        .map_err(|e| format!("保存图片失败: {}", e))?;

    log::info!("[RenderPage] 页面 {} 已保存到: {}", page_index, output_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_config_default() {
        let config = RenderConfig::default();
        assert_eq!(config.dpi, 150);
    }
}
