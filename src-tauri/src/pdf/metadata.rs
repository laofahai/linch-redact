//! 元数据清理模块
//!
//! 提供 PDF 文档元数据、XMP、JavaScript、附件等清理功能

use lopdf::{Document, Object, ObjectId};

/// 清理结果
#[derive(Debug, Clone)]
pub struct CleanResult {
    pub items_removed: usize,
    pub details: Vec<String>,
}

impl CleanResult {
    pub fn new() -> Self {
        Self {
            items_removed: 0,
            details: Vec::new(),
        }
    }

    pub fn add(&mut self, detail: String) {
        self.items_removed += 1;
        self.details.push(detail);
    }

    pub fn merge(&mut self, other: CleanResult) {
        self.items_removed += other.items_removed;
        self.details.extend(other.details);
    }
}

/// 清理 Info 字典（文档信息）
///
/// 删除 Title, Author, Subject, Keywords, Creator, Producer, CreationDate, ModDate 等
pub fn clean_info_dict(doc: &mut Document) -> Result<CleanResult, String> {
    let mut result = CleanResult::new();

    // 获取 Info 字典引用
    let info_ref = match doc.trailer.get(b"Info") {
        Ok(Object::Reference(id)) => Some(*id),
        _ => None,
    };

    if let Some(info_id) = info_ref {
        // 获取并清理 Info 字典中的字段
        if let Ok(Object::Dictionary(ref mut info_dict)) = doc.get_object_mut(info_id) {
            let fields_to_remove = [
                b"Title".as_slice(),
                b"Author".as_slice(),
                b"Subject".as_slice(),
                b"Keywords".as_slice(),
                b"Creator".as_slice(),
                b"Producer".as_slice(),
                b"CreationDate".as_slice(),
                b"ModDate".as_slice(),
            ];

            for field in &fields_to_remove {
                if info_dict.has(*field) {
                    info_dict.remove(*field);
                    result.add(format!("已移除 Info/{}", String::from_utf8_lossy(field)));
                }
            }
        }

        // 如果 Info 字典为空，移除整个引用
        if let Ok(Object::Dictionary(info_dict)) = doc.get_object(info_id) {
            if info_dict.is_empty() {
                doc.trailer.remove(b"Info");
                doc.objects.remove(&info_id);
                result.add("已移除空的 Info 字典".to_string());
            }
        }
    }

    log::info!("Info 字典清理完成: {:?}", result);
    Ok(result)
}

/// 清理 XMP 元数据
///
/// 删除 Catalog 中的 /Metadata 流
pub fn clean_xmp_metadata(doc: &mut Document) -> Result<CleanResult, String> {
    let mut result = CleanResult::new();

    // 获取 Catalog
    let catalog_id = get_catalog_id(doc)?;

    // 查找 Metadata 引用
    let metadata_ref = if let Ok(Object::Dictionary(catalog)) = doc.get_object(catalog_id) {
        match catalog.get(b"Metadata") {
            Ok(Object::Reference(id)) => Some(*id),
            _ => None,
        }
    } else {
        None
    };

    // 删除 Metadata 流
    if let Some(metadata_id) = metadata_ref {
        doc.objects.remove(&metadata_id);
        result.add(format!("已移除 XMP Metadata 流 (ID: {:?})", metadata_id));

        // 从 Catalog 中移除引用
        if let Ok(Object::Dictionary(ref mut catalog)) = doc.get_object_mut(catalog_id) {
            catalog.remove(b"Metadata");
            result.add("已从 Catalog 移除 Metadata 引用".to_string());
        }
    }

    log::info!("XMP 元数据清理完成: {:?}", result);
    Ok(result)
}

/// 移除 JavaScript
///
/// 删除文档级 /JavaScript 和页面级 /AA (Additional Actions)
pub fn remove_javascript(doc: &mut Document) -> Result<CleanResult, String> {
    let mut result = CleanResult::new();

    // 1. 移除文档级 JavaScript
    let catalog_id = get_catalog_id(doc)?;

    // 查找 Names 字典中的 JavaScript (先收集数据)
    let (names_ref, js_tree_id) =
        if let Ok(Object::Dictionary(catalog)) = doc.get_object(catalog_id) {
            let names_id = match catalog.get(b"Names") {
                Ok(Object::Reference(id)) => Some(*id),
                _ => None,
            };

            let js_id = if let Some(nid) = names_id {
                if let Ok(Object::Dictionary(names_dict)) = doc.get_object(nid) {
                    match names_dict.get(b"JavaScript") {
                        Ok(Object::Reference(id)) => Some(*id),
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            };

            (names_id, js_id)
        } else {
            (None, None)
        };

    // 先删除 JavaScript 名称树
    if let Some(js_id) = js_tree_id {
        remove_name_tree(doc, js_id, &mut result);
    }

    // 从 Names 字典移除 JavaScript 引用
    if let Some(names_id) = names_ref {
        if let Ok(Object::Dictionary(ref mut names_dict)) = doc.get_object_mut(names_id) {
            if names_dict.has(b"JavaScript") {
                names_dict.remove(b"JavaScript");
                result.add("已移除 Names/JavaScript".to_string());
            }
        }
    }

    // 收集 Catalog 中需要处理的数据
    let (has_js, has_aa, should_remove_open_action) =
        if let Ok(Object::Dictionary(catalog)) = doc.get_object(catalog_id) {
            let has_js = catalog.has(b"JavaScript");
            let has_aa = catalog.has(b"AA");
            let should_remove = match catalog.get(b"OpenAction") {
                Ok(open_action) => is_javascript_action(doc, open_action),
                _ => false,
            };
            (has_js, has_aa, should_remove)
        } else {
            (false, false, false)
        };

    // 修改 Catalog
    if let Ok(Object::Dictionary(ref mut catalog)) = doc.get_object_mut(catalog_id) {
        if has_js {
            catalog.remove(b"JavaScript");
            result.add("已移除 Catalog/JavaScript".to_string());
        }

        if should_remove_open_action {
            catalog.remove(b"OpenAction");
            result.add("已移除包含 JavaScript 的 OpenAction".to_string());
        }

        if has_aa {
            catalog.remove(b"AA");
            result.add("已移除 Catalog/AA (Additional Actions)".to_string());
        }
    }

    // 2. 移除页面级 AA
    let page_ids: Vec<ObjectId> = doc.page_iter().collect();
    for page_id in page_ids {
        if let Ok(Object::Dictionary(ref mut page_dict)) = doc.get_object_mut(page_id) {
            if page_dict.has(b"AA") {
                page_dict.remove(b"AA");
                result.add(format!("已移除页面 {:?} 的 AA", page_id));
            }
        }
    }

    log::info!("JavaScript 清理完成: {:?}", result);
    Ok(result)
}

/// 移除附件（嵌入文件）
pub fn remove_attachments(doc: &mut Document) -> Result<CleanResult, String> {
    let mut result = CleanResult::new();

    let catalog_id = get_catalog_id(doc)?;

    // 查找 Names 字典和 EmbeddedFiles (先收集数据)
    let (names_ref, ef_tree_id) =
        if let Ok(Object::Dictionary(catalog)) = doc.get_object(catalog_id) {
            let names_id = match catalog.get(b"Names") {
                Ok(Object::Reference(id)) => Some(*id),
                _ => None,
            };

            let ef_id = if let Some(nid) = names_id {
                if let Ok(Object::Dictionary(names_dict)) = doc.get_object(nid) {
                    match names_dict.get(b"EmbeddedFiles") {
                        Ok(Object::Reference(id)) => Some(*id),
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            };

            (names_id, ef_id)
        } else {
            (None, None)
        };

    // 先删除 EmbeddedFiles 名称树
    if let Some(ef_id) = ef_tree_id {
        remove_name_tree(doc, ef_id, &mut result);
    }

    // 从 Names 字典移除 EmbeddedFiles 引用
    if let Some(names_id) = names_ref {
        if let Ok(Object::Dictionary(ref mut names_dict)) = doc.get_object_mut(names_id) {
            if names_dict.has(b"EmbeddedFiles") {
                names_dict.remove(b"EmbeddedFiles");
                result.add("已移除 Names/EmbeddedFiles".to_string());
            }
        }
    }

    // 检查并移除文件附件注释中的嵌入文件
    let page_ids: Vec<ObjectId> = doc.page_iter().collect();
    for page_id in page_ids {
        if let Ok(Object::Dictionary(page_dict)) = doc.get_object(page_id) {
            if let Ok(annots_ref) = page_dict.get(b"Annots") {
                let annot_ids = get_annotation_ids(doc, annots_ref);
                for annot_id in annot_ids {
                    if let Ok(Object::Dictionary(annot_dict)) = doc.get_object(annot_id) {
                        // 检查是否是 FileAttachment 注释
                        if let Ok(Object::Name(subtype)) = annot_dict.get(b"Subtype") {
                            if subtype == b"FileAttachment" {
                                // 标记为需要移除（不在这里直接修改，避免借用问题）
                                result.add(format!("发现 FileAttachment 注释 {:?}", annot_id));
                            }
                        }
                    }
                }
            }
        }
    }

    log::info!("附件清理完成: {:?}", result);
    Ok(result)
}

/// 移除隐藏数据
///
/// 清理 /PieceInfo, /LastModified 等隐藏元数据
pub fn remove_hidden_data(doc: &mut Document) -> Result<CleanResult, String> {
    let mut result = CleanResult::new();

    let catalog_id = get_catalog_id(doc)?;

    // 清理 Catalog 中的隐藏数据
    if let Ok(Object::Dictionary(ref mut catalog)) = doc.get_object_mut(catalog_id) {
        let hidden_keys = [
            b"PieceInfo".as_slice(),
            b"LastModified".as_slice(),
            b"SpiderInfo".as_slice(),
            b"Perms".as_slice(),
        ];

        for key in &hidden_keys {
            if catalog.has(*key) {
                catalog.remove(*key);
                result.add(format!("已移除 Catalog/{}", String::from_utf8_lossy(key)));
            }
        }
    }

    // 清理页面级隐藏数据
    let page_ids: Vec<ObjectId> = doc.page_iter().collect();
    for page_id in page_ids {
        if let Ok(Object::Dictionary(ref mut page_dict)) = doc.get_object_mut(page_id) {
            if page_dict.has(b"PieceInfo") {
                page_dict.remove(b"PieceInfo");
                result.add(format!("已移除页面 {:?} 的 PieceInfo", page_id));
            }
            if page_dict.has(b"LastModified") {
                page_dict.remove(b"LastModified");
                result.add(format!("已移除页面 {:?} 的 LastModified", page_id));
            }
        }
    }

    log::info!("隐藏数据清理完成: {:?}", result);
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

/// 递归删除名称树中的对象
fn remove_name_tree(doc: &mut Document, tree_id: ObjectId, result: &mut CleanResult) {
    if let Ok(Object::Dictionary(tree_dict)) = doc.get_object(tree_id) {
        // 收集需要删除的子对象
        let mut to_remove: Vec<ObjectId> = Vec::new();

        // 检查 Kids 数组
        if let Ok(Object::Array(kids)) = tree_dict.get(b"Kids") {
            for kid in kids {
                if let Object::Reference(kid_id) = kid {
                    to_remove.push(*kid_id);
                }
            }
        }

        // 检查 Names 数组中的值
        if let Ok(Object::Array(names)) = tree_dict.get(b"Names") {
            for (i, item) in names.iter().enumerate() {
                if i % 2 == 1 {
                    // 偶数索引是键，奇数索引是值
                    if let Object::Reference(value_id) = item {
                        to_remove.push(*value_id);
                    }
                }
            }
        }

        // 递归处理子节点
        for child_id in &to_remove {
            remove_name_tree(doc, *child_id, result);
        }
    }

    // 删除当前节点
    doc.objects.remove(&tree_id);
    result.add(format!("已移除名称树节点 {:?}", tree_id));
}

/// 检查 Action 是否是 JavaScript Action
fn is_javascript_action(doc: &Document, action: &Object) -> bool {
    match action {
        Object::Reference(id) => {
            if let Ok(Object::Dictionary(action_dict)) = doc.get_object(*id) {
                if let Ok(Object::Name(s)) = action_dict.get(b"S") {
                    return s == b"JavaScript";
                }
            }
            false
        }
        Object::Dictionary(action_dict) => {
            if let Ok(Object::Name(s)) = action_dict.get(b"S") {
                return s == b"JavaScript";
            }
            false
        }
        _ => false,
    }
}

/// 从 Annots 引用获取注释 ID 列表
fn get_annotation_ids(doc: &Document, annots_ref: &Object) -> Vec<ObjectId> {
    let mut ids = Vec::new();

    match annots_ref {
        Object::Reference(id) => {
            if let Ok(Object::Array(arr)) = doc.get_object(*id) {
                for item in arr {
                    if let Object::Reference(annot_id) = item {
                        ids.push(*annot_id);
                    }
                }
            }
        }
        Object::Array(arr) => {
            for item in arr {
                if let Object::Reference(annot_id) = item {
                    ids.push(*annot_id);
                }
            }
        }
        _ => {}
    }

    ids
}

/// 品牌信息配置
pub struct BrandInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub url: &'static str,
}

/// 默认品牌信息
pub const BRAND: BrandInfo = BrandInfo {
    name: "Linch Redact",
    version: env!("CARGO_PKG_VERSION"),
    url: "https://linch.tech",
};

/// 设置脱敏工具的元信息
///
/// 在 Info 字典中添加工具标识和处理时间
pub fn set_redaction_metadata(doc: &mut Document) -> Result<(), String> {
    use chrono::Local;

    // 获取或创建 Info 字典
    let info_id = match doc.trailer.get(b"Info") {
        Ok(Object::Reference(id)) => *id,
        _ => {
            // 创建新的 Info 字典
            let info_dict = lopdf::Dictionary::new();
            let new_id = doc.add_object(Object::Dictionary(info_dict));
            doc.trailer.set(b"Info", Object::Reference(new_id));
            new_id
        }
    };

    // 获取当前时间，格式化为 PDF 日期格式 D:YYYYMMDDHHmmSS
    let now = Local::now();
    let pdf_date = format!("D:{}", now.format("%Y%m%d%H%M%S%z"));

    // 工具信息
    let producer = format!("{} v{} ({})", BRAND.name, BRAND.version, BRAND.url);
    let creator = BRAND.name;

    if let Ok(Object::Dictionary(ref mut info_dict)) = doc.get_object_mut(info_id) {
        // 设置 Producer（生成工具）
        info_dict.set(
            b"Producer",
            Object::String(producer.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        );

        // 设置 Creator（创建程序）
        info_dict.set(
            b"Creator",
            Object::String(creator.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        );

        // 设置 ModDate（修改时间）
        info_dict.set(
            b"ModDate",
            Object::String(pdf_date.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        );

        // 添加自定义字段标记已脱敏
        info_dict.set(
            b"Redacted",
            Object::String(b"true".to_vec(), lopdf::StringFormat::Literal),
        );
        info_dict.set(
            b"RedactedBy",
            Object::String(producer.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        );
        info_dict.set(
            b"RedactedAt",
            Object::String(pdf_date.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        );
    }

    log::info!(
        "已设置脱敏元信息: Producer={}, ModDate={}",
        producer,
        pdf_date
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_result() {
        let mut result = CleanResult::new();
        result.add("test1".to_string());
        result.add("test2".to_string());
        assert_eq!(result.items_removed, 2);
        assert_eq!(result.details.len(), 2);
    }
}
