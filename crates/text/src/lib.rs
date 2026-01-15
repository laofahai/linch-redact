//! 纯文本文档处理器
//!
//! 实现 `Document` trait，支持 .txt 和 .md 文件的加载、文本提取和脱敏。

use anyhow::{anyhow, Result};
use linch_core::document::{Document, Page};
use linch_core::rules::RuleSet;
use std::fs;
use std::path::{Path, PathBuf};

/// 纯文本文档处理器
///
/// 支持 .txt 和 .md 文件。整个文件内容作为单页处理。
pub struct TextDocument {
    path: PathBuf,
    content: String,
}

impl Document for TextDocument {
    fn load(path: &Path) -> Result<Self>
    where
        Self: Sized,
    {
        if !path.exists() {
            return Err(anyhow!("文件不存在: {}", path.display()));
        }

        let content = fs::read_to_string(path).map_err(|e| anyhow!("无法读取文件: {}", e))?;

        Ok(Self {
            path: path.to_path_buf(),
            content,
        })
    }

    fn get_pages(&self) -> Result<Vec<Page>> {
        // 纯文本文件作为单页处理
        Ok(vec![Page {
            page_number: 1,
            content: self.content.clone(),
        }])
    }

    fn redact(&self, ruleset: &RuleSet) -> Result<Vec<u8>> {
        let mut result = self.content.clone();

        // 应用所有启用的规则
        for rule in ruleset.enabled_rules() {
            match &rule.rule_type {
                linch_core::rules::RuleType::Regex(pattern) => {
                    if let Ok(re) = regex::Regex::new(pattern) {
                        result = re.replace_all(&result, "██████").to_string();
                    }
                }
                linch_core::rules::RuleType::Dictionary(words) => {
                    for word in words {
                        result = result.replace(word, "██████");
                    }
                }
                linch_core::rules::RuleType::Heuristic(_) => {
                    // TODO: 实现启发式算法
                }
            }
        }

        Ok(result.into_bytes())
    }

    fn get_supported_features(&self) -> Vec<String> {
        vec!["text_redact".to_string()]
    }
}

/// 获取文件路径
impl TextDocument {
    pub fn path(&self) -> &Path {
        &self.path
    }
}
