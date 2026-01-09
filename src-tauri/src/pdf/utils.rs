use lopdf::{Document, Object, Stream, content::Content};
use super::types::{Mask, MaskRect, PageContentType};

/// 获取页面的 MediaBox
pub fn get_media_box(doc: &Document, page_id: lopdf::ObjectId) -> (f32, f32, f32, f32) {
  if let Ok(Object::Dictionary(dict)) = doc.get_object(page_id) {
    if let Ok(Object::Array(arr)) = dict.get(b"MediaBox") {
      let values: Vec<f32> = arr.iter().filter_map(|o| {
        match o {
          Object::Integer(i) => Some(*i as f32),
          Object::Real(r) => Some(*r),
          _ => None,
        }
      }).collect();
      if values.len() == 4 {
        return (values[0], values[1], values[2], values[3]);
      }
    }
  }
  (0.0, 0.0, 612.0, 792.0) // 默认 Letter 尺寸
}

/// 将相对坐标的 mask 转换为 PDF 坐标系
pub fn convert_masks_to_pdf_coords(masks: &[Mask], media_box: (f32, f32, f32, f32)) -> Vec<MaskRect> {
  let page_width = media_box.2 - media_box.0;
  let page_height = media_box.3 - media_box.1;

  log::info!("[坐标转换] MediaBox: {:?}, page_size: {}x{}", media_box, page_width, page_height);

  masks.iter().enumerate().map(|(i, m)| {
    let rect = MaskRect {
      x: media_box.0 + (m.x as f32) * page_width,
      y: media_box.1 + (1.0 - m.y as f32 - m.height as f32) * page_height,
      width: (m.width as f32) * page_width,
      height: (m.height as f32) * page_height,
    };
    log::info!("[坐标转换] Mask {}: 相对坐标 ({:.4}, {:.4}, {:.4}, {:.4}) -> PDF坐标 ({:.2}, {:.2}, {:.2}, {:.2})",
      i, m.x, m.y, m.width, m.height, rect.x, rect.y, rect.width, rect.height);
    log::info!("[坐标转换] Mask {} 覆盖 PDF 区域: x=[{:.2}, {:.2}], y=[{:.2}, {:.2}]",
      i, rect.x, rect.x + rect.width, rect.y, rect.y + rect.height);
    rect
  }).collect()
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
    text_op_count, path_op_count, has_image_ops
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
