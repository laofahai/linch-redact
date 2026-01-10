use super::types::{Mask, MaskRect, PageContentType};
use lopdf::{content::Content, Document, Object, Stream};

/// 从数组对象中提取边界框坐标
fn extract_box_values(arr: &[Object]) -> Option<(f32, f32, f32, f32)> {
    let values: Vec<f32> = arr
        .iter()
        .filter_map(|o| match o {
            Object::Integer(i) => Some(*i as f32),
            Object::Real(r) => Some(*r),
            _ => None,
        })
        .collect();
    if values.len() == 4 {
        Some((values[0], values[1], values[2], values[3]))
    } else {
        None
    }
}

/// 获取页面的旋转角度
fn get_page_rotation(doc: &Document, page_id: lopdf::ObjectId) -> i32 {
    if let Ok(Object::Dictionary(dict)) = doc.get_object(page_id) {
        // 先检查页面自身的 Rotate 属性
        if let Ok(Object::Integer(rotate)) = dict.get(b"Rotate") {
            return *rotate as i32;
        }
        // 尝试从父页面继承
        if let Ok(Object::Reference(parent_ref)) = dict.get(b"Parent") {
            if let Ok(Object::Dictionary(parent_dict)) = doc.get_object(*parent_ref) {
                if let Ok(Object::Integer(rotate)) = parent_dict.get(b"Rotate") {
                    return *rotate as i32;
                }
            }
        }
    }
    0 // 默认无旋转
}

/// 获取页面的有效边界框（优先使用 CropBox，否则使用 MediaBox）
/// 返回 (llx, lly, urx, ury, rotation)
pub fn get_media_box(doc: &Document, page_id: lopdf::ObjectId) -> (f32, f32, f32, f32) {
    let rotation = get_page_rotation(doc, page_id);
    log::info!("[MediaBox] 页面旋转角度: {} 度", rotation);

    let raw_box = if let Ok(Object::Dictionary(dict)) = doc.get_object(page_id) {
        // 优先尝试 CropBox（实际可见区域）
        if let Ok(Object::Array(arr)) = dict.get(b"CropBox") {
            if let Some(values) = extract_box_values(arr) {
                log::info!("[MediaBox] 使用 CropBox: {:?}", values);
                Some(values)
            } else {
                None
            }
        // 其次尝试 MediaBox
        } else if let Ok(Object::Array(arr)) = dict.get(b"MediaBox") {
            if let Some(values) = extract_box_values(arr) {
                log::info!("[MediaBox] 使用 MediaBox: {:?}", values);
                Some(values)
            } else {
                None
            }
        // 尝试从父页面继承
        } else if let Ok(Object::Reference(parent_ref)) = dict.get(b"Parent") {
            if let Ok(Object::Dictionary(parent_dict)) = doc.get_object(*parent_ref) {
                if let Ok(Object::Array(arr)) = parent_dict.get(b"MediaBox") {
                    if let Some(values) = extract_box_values(arr) {
                        log::info!("[MediaBox] 从父页面继承 MediaBox: {:?}", values);
                        Some(values)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let (llx, lly, urx, ury) = raw_box.unwrap_or_else(|| {
        log::warn!("[MediaBox] 使用默认 Letter 尺寸");
        (0.0, 0.0, 612.0, 792.0) // 默认 Letter 尺寸
    });

    // 注意：MediaBox 的坐标是未旋转的
    // 但前端 pdf.js 显示时会自动应用旋转
    // 我们在这里返回原始的 MediaBox，旋转在坐标转换时处理
    (llx, lly, urx, ury)
}

/// 获取页面的有效边界框和旋转角度
pub fn get_media_box_with_rotation(
    doc: &Document,
    page_id: lopdf::ObjectId,
) -> (f32, f32, f32, f32, i32) {
    let rotation = get_page_rotation(doc, page_id);
    let (llx, lly, urx, ury) = get_media_box(doc, page_id);
    (llx, lly, urx, ury, rotation)
}

/// 将相对坐标的 mask 转换为 PDF 坐标系（不考虑旋转）
#[allow(dead_code)]
pub fn convert_masks_to_pdf_coords(
    masks: &[Mask],
    media_box: (f32, f32, f32, f32),
) -> Vec<MaskRect> {
    // 使用不带旋转的转换（默认行为）
    convert_masks_to_pdf_coords_with_rotation(masks, media_box, 0)
}

/// 将相对坐标的 mask 转换为 PDF 坐标系（考虑页面旋转）
///
/// 前端 pdf.js 显示时会自动应用旋转，所以：
/// - 旋转 90°：显示尺寸变为 (height, width)
/// - 旋转 180°：显示尺寸不变，但内容上下左右反转
/// - 旋转 270°：显示尺寸变为 (height, width)
///
/// 前端 mask 坐标是相对于显示后的页面的 (0-1) 范围
/// 我们需要将其转换回未旋转的 PDF 坐标系
pub fn convert_masks_to_pdf_coords_with_rotation(
    masks: &[Mask],
    media_box: (f32, f32, f32, f32),
    rotation: i32,
) -> Vec<MaskRect> {
    let page_width = media_box.2 - media_box.0;
    let page_height = media_box.3 - media_box.1;

    // 根据旋转角度确定显示尺寸
    // 90° 或 270° 旋转时，宽高互换
    let (display_width, display_height) = if rotation == 90 || rotation == 270 {
        (page_height, page_width)
    } else {
        (page_width, page_height)
    };

    log::info!(
        "[坐标转换] MediaBox: {:?}, 旋转: {}°, PDF尺寸: {}x{}, 显示尺寸: {}x{}",
        media_box,
        rotation,
        page_width,
        page_height,
        display_width,
        display_height
    );

    masks
        .iter()
        .enumerate()
        .map(|(i, m)| {
            // 前端坐标 (m.x, m.y) 是相对于显示尺寸的
            // 需要根据旋转角度转换回 PDF 坐标

            let (pdf_x, pdf_y, pdf_w, pdf_h) = match rotation {
                90 => {
                    // 90° 旋转：
                    // 显示的 X 对应 PDF 的 Y
                    // 显示的 Y 对应 PDF 的 (1-X)
                    let pdf_y = media_box.1 + (m.x as f32) * page_height;
                    let pdf_x = media_box.0 + (1.0 - m.y as f32 - m.height as f32) * page_width;
                    let pdf_h = (m.width as f32) * page_height;
                    let pdf_w = (m.height as f32) * page_width;
                    (pdf_x, pdf_y, pdf_w, pdf_h)
                }
                180 => {
                    // 180° 旋转：X 和 Y 都反向
                    let pdf_x =
                        media_box.0 + (1.0 - m.x as f32 - m.width as f32) * page_width;
                    let pdf_y = media_box.1 + (m.y as f32) * page_height;
                    let pdf_w = (m.width as f32) * page_width;
                    let pdf_h = (m.height as f32) * page_height;
                    (pdf_x, pdf_y, pdf_w, pdf_h)
                }
                270 => {
                    // 270° 旋转：
                    // 显示的 X 对应 PDF 的 (1-Y)
                    // 显示的 Y 对应 PDF 的 X
                    let pdf_y = media_box.1
                        + (1.0 - m.x as f32 - m.width as f32) * page_height;
                    let pdf_x = media_box.0 + (m.y as f32) * page_width;
                    let pdf_h = (m.width as f32) * page_height;
                    let pdf_w = (m.height as f32) * page_width;
                    (pdf_x, pdf_y, pdf_w, pdf_h)
                }
                _ => {
                    // 0° 或其他：标准转换
                    // PDF 坐标系 Y 轴从下往上，前端从上往下
                    let pdf_x = media_box.0 + (m.x as f32) * page_width;
                    let pdf_y =
                        media_box.1 + (1.0 - m.y as f32 - m.height as f32) * page_height;
                    let pdf_w = (m.width as f32) * page_width;
                    let pdf_h = (m.height as f32) * page_height;
                    (pdf_x, pdf_y, pdf_w, pdf_h)
                }
            };

            let rect = MaskRect {
                x: pdf_x,
                y: pdf_y,
                width: pdf_w,
                height: pdf_h,
            };

            log::info!(
                "[坐标转换] Mask {}: 相对坐标 ({:.4}, {:.4}, {:.4}, {:.4}) -> PDF坐标 ({:.2}, {:.2}, {:.2}, {:.2})",
                i, m.x, m.y, m.width, m.height, rect.x, rect.y, rect.width, rect.height
            );
            log::info!(
                "[坐标转换] Mask {} 覆盖 PDF 区域: x=[{:.2}, {:.2}], y=[{:.2}, {:.2}]",
                i,
                rect.x,
                rect.x + rect.width,
                rect.y,
                rect.y + rect.height
            );

            rect
        })
        .collect()
}

/// 从 Object 获取数值
pub fn get_number(obj: &Object) -> Option<f32> {
    match obj {
        Object::Integer(i) => Some(*i as f32),
        Object::Real(r) => Some(*r),
        _ => None,
    }
}

/// 获取流内容（支持压缩和未压缩的流）
pub fn get_stream_content(stream: &Stream) -> Result<Vec<u8>, String> {
    match stream.decompressed_content() {
        Ok(data) => Ok(data),
        Err(_) => Ok(stream.content.clone()),
    }
}

/// 获取页面的内容流数据
pub fn get_page_content(doc: &Document, page_id: lopdf::ObjectId) -> Result<Vec<u8>, String> {
    let page = doc.get_object(page_id).map_err(|e| e.to_string())?;

    if let Object::Dictionary(dict) = page {
        if let Ok(contents) = dict.get(b"Contents") {
            match contents {
                Object::Reference(ref_id) => {
                    if let Ok(Object::Stream(stream)) = doc.get_object(*ref_id) {
                        return get_stream_content(stream);
                    }
                }
                Object::Array(arr) => {
                    let mut all_content = Vec::new();
                    for item in arr {
                        if let Object::Reference(ref_id) = item {
                            if let Ok(Object::Stream(stream)) = doc.get_object(*ref_id) {
                                if let Ok(data) = get_stream_content(stream) {
                                    all_content.extend(data);
                                    all_content.push(b'\n');
                                }
                            }
                        }
                    }
                    return Ok(all_content);
                }
                Object::Stream(stream) => {
                    return get_stream_content(stream);
                }
                _ => {}
            }
        }
    }

    Err("无法获取页面内容".to_string())
}

/// 检测页面内容类型
pub fn detect_page_content_type(content_data: &[u8]) -> PageContentType {
    let content = match Content::decode(content_data) {
        Ok(c) => c,
        Err(_) => return PageContentType::Empty,
    };

    let mut has_text_ops = false;
    let mut has_path_ops = false;
    let mut has_image_ops = false;
    let mut path_op_count = 0;
    let mut text_op_count = 0;

    for op in &content.operations {
        match op.operator.as_str() {
            "Tj" | "TJ" | "'" | "\"" => {
                has_text_ops = true;
                text_op_count += 1;
            }
            "m" | "l" | "c" | "v" | "y" | "h" | "re" => {
                has_path_ops = true;
                path_op_count += 1;
            }
            "Do" => has_image_ops = true,
            _ => {}
        }
    }

    log::debug!(
        "[ContentType] text_ops={}, path_ops={}, image_ops={}",
        text_op_count,
        path_op_count,
        has_image_ops
    );

    // 判断逻辑：
    // 1. 纯图片页面（扫描件）
    if !has_text_ops && !has_path_ops && has_image_ops {
        return PageContentType::ImageBased;
    }

    // 2. 有文字操作 -> 优先使用 Text 模式
    //    即使有一些路径操作（表格边框等），只要路径操作不是特别多（<500），就认为是文字型
    if has_text_ops && path_op_count < 500 {
        return PageContentType::Text;
    }

    // 3. 纯路径绘制（没有文字，大量路径操作）
    if !has_text_ops && path_op_count > 500 {
        return PageContentType::PathDrawn;
    }

    // 4. 混合类型（有文字但路径操作非常多，或其他复杂情况）
    if has_text_ops || has_path_ops || has_image_ops {
        return PageContentType::Mixed;
    }

    PageContentType::Empty
}
