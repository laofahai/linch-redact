//! 脱敏规则系统
//!
//! 定义各种类型的脱敏规则，包括正则表达式、词典匹配和启发式算法。

use serde::{Deserialize, Serialize};

/// 启发式算法类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HeuristicType {
    /// 地址识别
    Address,
    /// 人名识别
    PersonName,
    /// 组织机构名称识别
    Organization,
}

/// 规则类型
///
/// 定义规则的匹配方式。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RuleType {
    /// 正则表达式匹配
    Regex(String),
    /// 词典匹配（关键词列表）
    Dictionary(Vec<String>),
    /// 启发式算法
    Heuristic(HeuristicType),
}

/// 脱敏规则
///
/// 系统中的每一条规则都是一个结构化的数据单元。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// 唯一标识符
    pub id: String,
    /// 用户可读的规则名称
    pub name: String,
    /// 是否启用
    pub enabled: bool,
    /// 是否为系统内置规则（不可删除）
    pub is_system: bool,
    /// 规则类型和匹配数据
    pub rule_type: RuleType,
}

/// 规则集合
///
/// 包含一组用于脱敏任务的规则。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleSet {
    pub rules: Vec<Rule>,
}

impl RuleSet {
    /// 创建空规则集
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// 添加规则
    pub fn add(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    /// 获取所有启用的规则
    pub fn enabled_rules(&self) -> Vec<&Rule> {
        self.rules.iter().filter(|r| r.enabled).collect()
    }
}
