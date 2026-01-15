//! 统一文档接口定义
//!
//! 所有文件处理器都必须实现 `Document` trait，以确保统一的处理流程。

use crate::rules::RuleSet;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 页面数据结构
///
/// 每个页面包含页码和纯文本内容。对于无分页概念的文件（如 .txt），
/// 整个文件内容作为页码为 1 的唯一页面。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// 页码，从 1 开始
    pub page_number: u32,
    /// 页面的纯文本内容
    pub content: String,
}

/// 统一文档接口
///
/// 这是所有文件处理器的"资格认证标准"。任何处理器都必须实现这些方法。
pub trait Document: Send + Sync {
    /// 加载文档
    ///
    /// 将文件路径转换为可操作的文档对象。
    ///
    /// # 参数
    /// - `path`: 文件的完整磁盘路径
    ///
    /// # 返回
    /// - 成功：已加载的文档实例
    /// - 失败：明确的错误信息（文件不存在、格式损坏等）
    fn load(path: &std::path::Path) -> Result<Self>
    where
        Self: Sized;

    /// 提取页面文本
    ///
    /// 从文档中抽取所有可供脱敏的纯文本，按页面组织。
    ///
    /// # 返回
    /// 页面列表，每个页面包含页码和文本内容
    fn get_pages(&self) -> Result<Vec<Page>>;

    /// 执行脱敏
    ///
    /// 根据规则集对文档进行脱敏处理。
    ///
    /// # 参数
    /// - `ruleset`: 包含所有匹配规则的集合
    ///
    /// # 返回
    /// 已脱敏文档的二进制数据
    fn redact(&self, ruleset: &RuleSet) -> Result<Vec<u8>>;

    /// 声明支持的功能
    ///
    /// 返回此处理器支持的功能列表，用于前端动态 UI。
    ///
    /// # 常见功能标识
    /// - `text_redact`: 文本脱敏
    /// - `metadata_clean`: 元数据清理
    /// - `image_redact`: 图片脱敏
    fn get_supported_features(&self) -> Vec<String>;
}
