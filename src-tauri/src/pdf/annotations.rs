//! 注释处理模块
//!
//! 提供 PDF 注释（Annots）的脱敏和移除功能

use lopdf::{Document, Object, ObjectId};
use super::types::MaskRect;
use super::metadata::CleanResult;

/// 移除页面的所有注释
///
/// 删除指定页面的所有注释对象
pub fn remove_all_annotations(
  doc: &mut Document,
  page_id: ObjectId,
) -> Result<CleanResult, String> {
  let mut result = CleanResult::new();

  // 获取页面的 Annots 数组
  let annot_ids: Vec<ObjectId> =
    if let Ok(Object::Dictionary(page_dict)) = doc.get_object(page_id) {
      if let Ok(annots_ref) = page_dict.get(b"Annots") {
        get_annot_ids(doc, annots_ref)
      } else {
        Vec::new()
      }
    } else {
      return Err(format!("无法获取页面 {:?}", page_id));
    };

  if annot_ids.is_empty() {
    return Ok(result);
  }

  // 删除所有注释对象
  for annot_id in &annot_ids {
    // 获取注释类型用于日志
    let subtype = if let Ok(Object::Dictionary(annot_dict)) = doc.get_object(*annot_id) {
      if let Ok(Object::Name(st)) = annot_dict.get(b"Subtype") {
        String::from_utf8_lossy(st).to_string()
      } else {
        "Unknown".to_string()
      }
    } else {
      "Unknown".to_string()
    };

    doc.objects.remove(annot_id);
    result.add(format!("已移除注释 {:?} (类型: {})", annot_id, subtype));
  }

  // 从页面字典移除 Annots 引用
  if let Ok(Object::Dictionary(ref mut page_dict)) = doc.get_object_mut(page_id) {
    page_dict.remove(b"Annots");
    result.add(format!("已从页面 {:?} 移除 Annots 引用", page_id));
  }

  log::info!("页面 {:?} 注释移除完成: {:?}", page_id, result);
  Ok(result)
}

/// 移除文档所有页面的注释
pub fn remove_all_annotations_from_document(doc: &mut Document) -> Result<CleanResult, String> {
  let mut result = CleanResult::new();

  let page_ids: Vec<ObjectId> = doc.page_iter().collect();

  for page_id in page_ids {
    let page_result = remove_all_annotations(doc, page_id)?;
    result.merge(page_result);
  }

  log::info!("文档注释移除完成: {:?}", result);
  Ok(result)
}

/// 脱敏指定区域内的注释
///
/// 移除与 mask 区域相交的注释
pub fn redact_annotations(
  doc: &mut Document,
  page_id: ObjectId,
  masks: &[MaskRect],
) -> Result<CleanResult, String> {
  let mut result = CleanResult::new();

  if masks.is_empty() {
    return Ok(result);
  }

  // 获取页面的 Annots 数组
  let annot_ids: Vec<ObjectId> =
    if let Ok(Object::Dictionary(page_dict)) = doc.get_object(page_id) {
      if let Ok(annots_ref) = page_dict.get(b"Annots") {
        get_annot_ids(doc, annots_ref)
      } else {
        Vec::new()
      }
    } else {
      return Err(format!("无法获取页面 {:?}", page_id));
    };

  if annot_ids.is_empty() {
    return Ok(result);
  }

  // 找出需要移除的注释
  let mut annots_to_remove: Vec<ObjectId> = Vec::new();

  for annot_id in &annot_ids {
    if let Ok(Object::Dictionary(annot_dict)) = doc.get_object(*annot_id) {
      // 获取注释的 Rect
      if let Some(rect) = get_annot_rect(&annot_dict) {
        // 检查是否与任何 mask 相交
        for mask in masks {
          if rects_intersect(&rect, mask) {
            annots_to_remove.push(*annot_id);
            break;
          }
        }
      }
    }
  }

  if annots_to_remove.is_empty() {
    return Ok(result);
  }

  // 删除相交的注释
  for annot_id in &annots_to_remove {
    let subtype = if let Ok(Object::Dictionary(annot_dict)) = doc.get_object(*annot_id) {
      if let Ok(Object::Name(st)) = annot_dict.get(b"Subtype") {
        String::from_utf8_lossy(st).to_string()
      } else {
        "Unknown".to_string()
      }
    } else {
      "Unknown".to_string()
    };

    doc.objects.remove(annot_id);
    result.add(format!("已脱敏注释 {:?} (类型: {})", annot_id, subtype));
  }

  // 更新页面的 Annots 数组
  update_page_annots(doc, page_id, &annots_to_remove, &mut result);

  log::info!("页面 {:?} 注释脱敏完成: {:?}", page_id, result);
  Ok(result)
}

/// 移除指定类型的注释
///
/// 支持的类型: Text, FreeText, Markup (Highlight, Underline, StrikeOut, Squiggly),
/// Line, Square, Circle, Polygon, PolyLine, Stamp, Caret, Ink, FileAttachment, etc.
pub fn remove_annotations_by_type(
  doc: &mut Document,
  page_id: ObjectId,
  subtypes: &[&[u8]],
) -> Result<CleanResult, String> {
  let mut result = CleanResult::new();

  // 获取页面的 Annots 数组
  let annot_ids: Vec<ObjectId> =
    if let Ok(Object::Dictionary(page_dict)) = doc.get_object(page_id) {
      if let Ok(annots_ref) = page_dict.get(b"Annots") {
        get_annot_ids(doc, annots_ref)
      } else {
        Vec::new()
      }
    } else {
      return Err(format!("无法获取页面 {:?}", page_id));
    };

  // 找出需要移除的注释
  let mut annots_to_remove: Vec<ObjectId> = Vec::new();

  for annot_id in &annot_ids {
    if let Ok(Object::Dictionary(annot_dict)) = doc.get_object(*annot_id) {
      if let Ok(Object::Name(subtype)) = annot_dict.get(b"Subtype") {
        if subtypes.iter().any(|st| *st == subtype.as_slice()) {
          annots_to_remove.push(*annot_id);
        }
      }
    }
  }

  if annots_to_remove.is_empty() {
    return Ok(result);
  }

  // 删除注释
  for annot_id in &annots_to_remove {
    let subtype = if let Ok(Object::Dictionary(annot_dict)) = doc.get_object(*annot_id) {
      if let Ok(Object::Name(st)) = annot_dict.get(b"Subtype") {
        String::from_utf8_lossy(st).to_string()
      } else {
        "Unknown".to_string()
      }
    } else {
      "Unknown".to_string()
    };

    doc.objects.remove(annot_id);
    result.add(format!("已移除 {} 注释 {:?}", subtype, annot_id));
  }

  // 更新页面的 Annots 数组
  update_page_annots(doc, page_id, &annots_to_remove, &mut result);

  log::info!("页面 {:?} 指定类型注释移除完成: {:?}", page_id, result);
  Ok(result)
}

/// 清除注释的文本内容（保留注释但移除敏感内容）
pub fn clear_annotation_contents(
  doc: &mut Document,
  page_id: ObjectId,
  masks: &[MaskRect],
) -> Result<CleanResult, String> {
  let mut result = CleanResult::new();

  if masks.is_empty() {
    return Ok(result);
  }

  // 获取页面的 Annots 数组
  let annot_ids: Vec<ObjectId> =
    if let Ok(Object::Dictionary(page_dict)) = doc.get_object(page_id) {
      if let Ok(annots_ref) = page_dict.get(b"Annots") {
        get_annot_ids(doc, annots_ref)
      } else {
        Vec::new()
      }
    } else {
      return Err(format!("无法获取页面 {:?}", page_id));
    };

  for annot_id in annot_ids {
    let should_clear = if let Ok(Object::Dictionary(annot_dict)) = doc.get_object(annot_id) {
      if let Some(rect) = get_annot_rect(&annot_dict) {
        masks.iter().any(|mask| rects_intersect(&rect, mask))
      } else {
        false
      }
    } else {
      false
    };

    if should_clear {
      if let Ok(Object::Dictionary(ref mut annot_dict)) = doc.get_object_mut(annot_id) {
        // 清除 Contents（注释文本内容）
        if annot_dict.has(b"Contents") {
          annot_dict.set(b"Contents", Object::String(Vec::new(), lopdf::StringFormat::Literal));
          result.add(format!("已清除注释 {:?} 的内容", annot_id));
        }

        // 清除 RC（富文本内容）
        if annot_dict.has(b"RC") {
          annot_dict.remove(b"RC");
          result.add(format!("已清除注释 {:?} 的富文本", annot_id));
        }

        // 清除 T（标题/作者）
        if annot_dict.has(b"T") {
          annot_dict.remove(b"T");
          result.add(format!("已清除注释 {:?} 的标题", annot_id));
        }
      }
    }
  }

  log::info!("页面 {:?} 注释内容清除完成: {:?}", page_id, result);
  Ok(result)
}

// ============ 辅助函数 ============

/// 获取注释 ID 列表
fn get_annot_ids(doc: &Document, annots_ref: &Object) -> Vec<ObjectId> {
  match annots_ref {
    Object::Array(arr) => arr
      .iter()
      .filter_map(|o| {
        if let Object::Reference(id) = o {
          Some(*id)
        } else {
          None
        }
      })
      .collect(),
    Object::Reference(id) => {
      if let Ok(Object::Array(arr)) = doc.get_object(*id) {
        arr
          .iter()
          .filter_map(|o| {
            if let Object::Reference(id) = o {
              Some(*id)
            } else {
              None
            }
          })
          .collect()
      } else {
        Vec::new()
      }
    }
    _ => Vec::new(),
  }
}

/// 获取注释的矩形区域
fn get_annot_rect(annot_dict: &lopdf::Dictionary) -> Option<(f32, f32, f32, f32)> {
  if let Ok(Object::Array(rect)) = annot_dict.get(b"Rect") {
    if rect.len() == 4 {
      let values: Vec<f32> = rect
        .iter()
        .filter_map(|o| match o {
          Object::Integer(i) => Some(*i as f32),
          Object::Real(r) => Some(*r),
          _ => None,
        })
        .collect();
      if values.len() == 4 {
        return Some((values[0], values[1], values[2], values[3]));
      }
    }
  }
  None
}

/// 检查两个矩形是否相交
fn rects_intersect(annot_rect: &(f32, f32, f32, f32), mask: &MaskRect) -> bool {
  let (x1, y1, x2, y2) = *annot_rect;
  let annot_left = x1.min(x2);
  let annot_right = x1.max(x2);
  let annot_bottom = y1.min(y2);
  let annot_top = y1.max(y2);

  let mask_left = mask.x;
  let mask_right = mask.x + mask.width;
  let mask_bottom = mask.y;
  let mask_top = mask.y + mask.height;

  // 检查是否有重叠
  annot_left < mask_right
    && annot_right > mask_left
    && annot_bottom < mask_top
    && annot_top > mask_bottom
}

/// 更新页面的 Annots 数组
fn update_page_annots(
  doc: &mut Document,
  page_id: ObjectId,
  removed_ids: &[ObjectId],
  result: &mut CleanResult,
) {
  // 首先获取 annots 数据（避免借用冲突）
  let annots_data: Option<Vec<Object>> = if let Ok(Object::Dictionary(page_dict)) = doc.get_object(page_id) {
    if let Ok(annots_ref) = page_dict.get(b"Annots") {
      match annots_ref {
        Object::Array(arr) => Some(arr.clone()),
        Object::Reference(arr_id) => {
          if let Ok(Object::Array(arr)) = doc.get_object(*arr_id) {
            Some(arr.clone())
          } else {
            None
          }
        }
        _ => None,
      }
    } else {
      None
    }
  } else {
    None
  };

  // 如果有 annots 数据，过滤并更新
  if let Some(annots_arr) = annots_data {
    let new_annots: Vec<Object> = annots_arr
      .into_iter()
      .filter(|o| {
        if let Object::Reference(id) = o {
          !removed_ids.contains(id)
        } else {
          true
        }
      })
      .collect();

    // 现在可以安全地修改页面字典
    if let Ok(Object::Dictionary(ref mut page_dict)) = doc.get_object_mut(page_id) {
      if new_annots.is_empty() {
        page_dict.remove(b"Annots");
        result.add(format!("页面 {:?} 的 Annots 数组已清空并移除", page_id));
      } else {
        page_dict.set(b"Annots", Object::Array(new_annots));
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_rects_intersect() {
    let annot_rect = (100.0, 100.0, 200.0, 200.0);
    let mask = MaskRect {
      x: 150.0,
      y: 150.0,
      width: 100.0,
      height: 100.0,
    };
    assert!(rects_intersect(&annot_rect, &mask));

    let non_intersecting_mask = MaskRect {
      x: 300.0,
      y: 300.0,
      width: 50.0,
      height: 50.0,
    };
    assert!(!rects_intersect(&annot_rect, &non_intersecting_mask));
  }
}
