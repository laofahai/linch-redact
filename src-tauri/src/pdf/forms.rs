//! 表单处理模块
//!
//! 提供 PDF AcroForm 表单字段的脱敏、展平和移除功能

use lopdf::{Document, Object, ObjectId};
use super::types::MaskRect;
use super::metadata::CleanResult;

/// 移除所有表单字段
///
/// 完全删除 AcroForm 及其所有字段
pub fn remove_all_forms(doc: &mut Document) -> Result<CleanResult, String> {
  let mut result = CleanResult::new();

  // 获取 Catalog
  let catalog_id = get_catalog_id(doc)?;

  // 查找 AcroForm 引用
  let acroform_ref = if let Ok(Object::Dictionary(catalog)) = doc.get_object(catalog_id) {
    match catalog.get(b"AcroForm") {
      Ok(Object::Reference(id)) => Some(*id),
      _ => None,
    }
  } else {
    None
  };

  if let Some(acroform_id) = acroform_ref {
    // 获取所有字段并删除
    if let Ok(Object::Dictionary(acroform)) = doc.get_object(acroform_id) {
      let field_ids = get_form_field_ids(doc, acroform);
      for field_id in field_ids {
        remove_form_field(doc, field_id, &mut result);
      }
    }

    // 删除 AcroForm 对象
    doc.objects.remove(&acroform_id);
    result.add("已移除 AcroForm 对象".to_string());

    // 从 Catalog 移除引用
    if let Ok(Object::Dictionary(ref mut catalog)) = doc.get_object_mut(catalog_id) {
      catalog.remove(b"AcroForm");
      result.add("已从 Catalog 移除 AcroForm 引用".to_string());
    }
  }

  // 移除页面中的表单字段注释（Widget annotations）
  let page_ids: Vec<ObjectId> = doc.page_iter().collect();
  for page_id in page_ids {
    remove_widget_annotations(doc, page_id, &mut result);
  }

  log::info!("表单移除完成: {:?}", result);
  Ok(result)
}

/// 脱敏指定区域内的表单字段
///
/// 清除与 mask 区域相交的表单字段值
pub fn redact_form_fields(
  doc: &mut Document,
  masks: &[MaskRect],
) -> Result<CleanResult, String> {
  let mut result = CleanResult::new();

  if masks.is_empty() {
    return Ok(result);
  }

  // 获取 Catalog
  let catalog_id = get_catalog_id(doc)?;

  // 查找 AcroForm
  let acroform_ref = if let Ok(Object::Dictionary(catalog)) = doc.get_object(catalog_id) {
    match catalog.get(b"AcroForm") {
      Ok(Object::Reference(id)) => Some(*id),
      _ => None,
    }
  } else {
    None
  };

  let acroform_id = match acroform_ref {
    Some(id) => id,
    None => return Ok(result), // 没有表单
  };

  // 获取所有字段
  let field_ids = if let Ok(Object::Dictionary(acroform)) = doc.get_object(acroform_id) {
    get_form_field_ids(doc, acroform)
  } else {
    return Ok(result);
  };

  // 检查每个字段是否与 mask 相交
  for field_id in field_ids {
    if let Ok(Object::Dictionary(field_dict)) = doc.get_object(field_id) {
      // 获取字段的 Rect
      if let Some(rect) = get_field_rect(&field_dict) {
        // 检查是否与任何 mask 相交
        for mask in masks {
          if rects_intersect(&rect, mask) {
            // 清除字段值
            redact_field_value(doc, field_id, &mut result);
            break;
          }
        }
      }
    }
  }

  log::info!("表单字段脱敏完成: {:?}", result);
  Ok(result)
}

/// 展平表单（将表单外观渲染到页面内容）
///
/// 注意：完整的展平需要复杂的外观流处理，这里提供简化版本
pub fn flatten_forms(doc: &mut Document) -> Result<CleanResult, String> {
  let mut result = CleanResult::new();

  // 获取 Catalog
  let catalog_id = get_catalog_id(doc)?;

  // 查找 AcroForm
  let acroform_ref = if let Ok(Object::Dictionary(catalog)) = doc.get_object(catalog_id) {
    match catalog.get(b"AcroForm") {
      Ok(Object::Reference(id)) => Some(*id),
      _ => None,
    }
  } else {
    None
  };

  let acroform_id = match acroform_ref {
    Some(id) => id,
    None => return Ok(result),
  };

  // 设置 NeedAppearances 为 false
  if let Ok(Object::Dictionary(ref mut acroform)) = doc.get_object_mut(acroform_id) {
    acroform.set(b"NeedAppearances", Object::Boolean(false));
  }

  // 获取所有字段
  let field_ids = if let Ok(Object::Dictionary(acroform)) = doc.get_object(acroform_id) {
    get_form_field_ids(doc, acroform)
  } else {
    return Ok(result);
  };

  // 将每个字段标记为只读并设置为打印
  for field_id in &field_ids {
    if let Ok(Object::Dictionary(ref mut field_dict)) = doc.get_object_mut(*field_id) {
      // 设置只读标志 (Ff 字段的 bit 1)
      let current_ff = match field_dict.get(b"Ff") {
        Ok(Object::Integer(ff)) => *ff,
        _ => 0,
      };
      field_dict.set(b"Ff", Object::Integer(current_ff | 1)); // ReadOnly flag

      result.add(format!("已展平字段 {:?}", field_id));
    }
  }

  log::info!("表单展平完成: {:?}", result);
  Ok(result)
}

// ============ 辅助函数 ============

/// 获取文档 Catalog 的 ObjectId
fn get_catalog_id(doc: &Document) -> Result<ObjectId, String> {
  match doc.trailer.get(b"Root") {
    Ok(Object::Reference(id)) => Ok(*id),
    _ => Err("无法获取文档 Catalog".to_string()),
  }
}

/// 获取 AcroForm 中所有字段的 ID
fn get_form_field_ids(doc: &Document, acroform: &lopdf::Dictionary) -> Vec<ObjectId> {
  let mut field_ids = Vec::new();

  // 获取 Fields 数组
  let fields = match acroform.get(b"Fields") {
    Ok(Object::Reference(id)) => {
      if let Ok(Object::Array(arr)) = doc.get_object(*id) {
        arr.clone()
      } else {
        return field_ids;
      }
    }
    Ok(Object::Array(arr)) => arr.clone(),
    _ => return field_ids,
  };

  // 递归收集所有字段
  for field_ref in fields {
    if let Object::Reference(field_id) = field_ref {
      collect_field_ids_recursive(doc, field_id, &mut field_ids);
    }
  }

  field_ids
}

/// 递归收集字段 ID（包括子字段）
fn collect_field_ids_recursive(doc: &Document, field_id: ObjectId, ids: &mut Vec<ObjectId>) {
  ids.push(field_id);

  // 检查是否有 Kids（子字段）
  if let Ok(Object::Dictionary(field_dict)) = doc.get_object(field_id) {
    if let Ok(kids) = field_dict.get(b"Kids") {
      let kid_refs = match kids {
        Object::Array(arr) => arr.clone(),
        Object::Reference(id) => {
          if let Ok(Object::Array(arr)) = doc.get_object(*id) {
            arr.clone()
          } else {
            return;
          }
        }
        _ => return,
      };

      for kid_ref in kid_refs {
        if let Object::Reference(kid_id) = kid_ref {
          collect_field_ids_recursive(doc, kid_id, ids);
        }
      }
    }
  }
}

/// 移除单个表单字段
fn remove_form_field(doc: &mut Document, field_id: ObjectId, result: &mut CleanResult) {
  // 首先递归移除子字段
  if let Ok(Object::Dictionary(field_dict)) = doc.get_object(field_id) {
    if let Ok(kids) = field_dict.get(b"Kids") {
      let kid_ids: Vec<ObjectId> = match kids {
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
      };

      for kid_id in kid_ids {
        remove_form_field(doc, kid_id, result);
      }
    }
  }

  // 移除字段对象
  doc.objects.remove(&field_id);
  result.add(format!("已移除表单字段 {:?}", field_id));
}

/// 移除页面中的 Widget 注释
fn remove_widget_annotations(doc: &mut Document, page_id: ObjectId, result: &mut CleanResult) {
  // 获取页面的 Annots 数组
  let annots_to_remove: Vec<ObjectId> = if let Ok(Object::Dictionary(page_dict)) = doc.get_object(page_id) {
    if let Ok(annots_ref) = page_dict.get(b"Annots") {
      let annot_ids = get_annot_ids(doc, annots_ref);
      annot_ids
        .into_iter()
        .filter(|id| {
          if let Ok(Object::Dictionary(annot_dict)) = doc.get_object(*id) {
            matches!(annot_dict.get(b"Subtype"), Ok(Object::Name(n)) if n == b"Widget")
          } else {
            false
          }
        })
        .collect()
    } else {
      Vec::new()
    }
  } else {
    Vec::new()
  };

  if annots_to_remove.is_empty() {
    return;
  }

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

  // 过滤并更新
  if let Some(annots_arr) = annots_data {
    let new_annots: Vec<Object> = annots_arr
      .into_iter()
      .filter(|o| {
        if let Object::Reference(id) = o {
          !annots_to_remove.contains(id)
        } else {
          true
        }
      })
      .collect();

    if let Ok(Object::Dictionary(ref mut page_dict)) = doc.get_object_mut(page_id) {
      if new_annots.is_empty() {
        page_dict.remove(b"Annots");
      } else {
        page_dict.set(b"Annots", Object::Array(new_annots));
      }
    }
  }

  for annot_id in annots_to_remove {
    doc.objects.remove(&annot_id);
    result.add(format!("已移除 Widget 注释 {:?}", annot_id));
  }
}

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

/// 获取字段的矩形区域
fn get_field_rect(field_dict: &lopdf::Dictionary) -> Option<(f32, f32, f32, f32)> {
  if let Ok(Object::Array(rect)) = field_dict.get(b"Rect") {
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
fn rects_intersect(field_rect: &(f32, f32, f32, f32), mask: &MaskRect) -> bool {
  let (x1, y1, x2, y2) = *field_rect;
  let field_left = x1.min(x2);
  let field_right = x1.max(x2);
  let field_bottom = y1.min(y2);
  let field_top = y1.max(y2);

  let mask_left = mask.x;
  let mask_right = mask.x + mask.width;
  let mask_bottom = mask.y;
  let mask_top = mask.y + mask.height;

  // 检查是否有重叠
  field_left < mask_right
    && field_right > mask_left
    && field_bottom < mask_top
    && field_top > mask_bottom
}

/// 清除字段值
fn redact_field_value(doc: &mut Document, field_id: ObjectId, result: &mut CleanResult) {
  if let Ok(Object::Dictionary(ref mut field_dict)) = doc.get_object_mut(field_id) {
    // 清除 V（值）字段
    if field_dict.has(b"V") {
      field_dict.remove(b"V");
      result.add(format!("已清除字段 {:?} 的值", field_id));
    }

    // 清除 DV（默认值）字段
    if field_dict.has(b"DV") {
      field_dict.remove(b"DV");
      result.add(format!("已清除字段 {:?} 的默认值", field_id));
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_rects_intersect() {
    let field_rect = (100.0, 100.0, 200.0, 200.0);
    let mask = MaskRect {
      x: 150.0,
      y: 150.0,
      width: 100.0,
      height: 100.0,
    };
    assert!(rects_intersect(&field_rect, &mask));

    let non_intersecting_mask = MaskRect {
      x: 300.0,
      y: 300.0,
      width: 50.0,
      height: 50.0,
    };
    assert!(!rects_intersect(&field_rect, &non_intersecting_mask));
  }
}
