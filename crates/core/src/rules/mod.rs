//! 脱敏规则系统
//!
//! 定义各种类型的脱敏规则，包括正则表达式、词典匹配和启发式算法。

use regex::Regex;
use serde::{Deserialize, Serialize};

mod heuristics;
use heuristics::HeuristicMatcher;

/// 启发式算法类型
///
/// 分为两类：
/// - 语言相关：Address, PersonName, Organization（自动检测语言）
/// - 通用：Date, Amount, Phone, Email, IdNumber, CreditCard（跨语言）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HeuristicType {
    // ===== 语言相关 =====
    /// 地址识别（支持中英文）
    Address,
    /// 人名识别（支持中英文）
    PersonName,
    /// 组织机构名称识别（支持中英文）
    Organization,

    // ===== 通用（跨语言）=====
    /// 日期识别
    Date,
    /// 金额识别
    Amount,
    /// 电话号码识别
    Phone,
    /// 邮箱地址识别
    Email,
    /// 身份证号/社会安全号识别
    IdNumber,
    /// 信用卡号识别
    CreditCard,
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

/// 规则匹配结果
///
/// 记录一次匹配的详细信息，包括匹配的文本、位置和规则信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleMatch {
    /// 匹配到的规则 ID
    pub rule_id: String,
    /// 匹配到的规则名称
    pub rule_name: String,
    /// 匹配到的文本内容
    pub matched_text: String,
    /// 起始位置（字节偏移）
    pub start: usize,
    /// 结束位置（字节偏移）
    pub end: usize,
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

    /// 对文本进行规则匹配
    ///
    /// 返回所有匹配结果，包含匹配位置和规则信息。
    ///
    /// # 参数
    /// - `text`: 要匹配的文本内容
    ///
    /// # 返回
    /// 匹配结果列表
    pub fn match_text(&self, text: &str) -> Vec<RuleMatch> {
        let mut matches = Vec::new();

        for rule in self.enabled_rules() {
            match &rule.rule_type {
                RuleType::Regex(pattern) => {
                    if let Ok(re) = Regex::new(pattern) {
                        for m in re.find_iter(text) {
                            matches.push(RuleMatch {
                                rule_id: rule.id.clone(),
                                rule_name: rule.name.clone(),
                                matched_text: m.as_str().to_string(),
                                start: m.start(),
                                end: m.end(),
                            });
                        }
                    }
                }
                RuleType::Dictionary(words) => {
                    for word in words {
                        let mut start = 0;
                        while let Some(pos) = text[start..].find(word) {
                            let abs_start = start + pos;
                            let abs_end = abs_start + word.len();
                            matches.push(RuleMatch {
                                rule_id: rule.id.clone(),
                                rule_name: rule.name.clone(),
                                matched_text: word.clone(),
                                start: abs_start,
                                end: abs_end,
                            });
                            start = abs_end;
                        }
                    }
                }
                RuleType::Heuristic(heuristic_type) => {
                    let heuristic_matches = match heuristic_type {
                        // 语言相关
                        HeuristicType::Address => HeuristicMatcher::match_address(text),
                        HeuristicType::PersonName => HeuristicMatcher::match_person_name(text),
                        HeuristicType::Organization => HeuristicMatcher::match_organization(text),
                        // 通用
                        HeuristicType::Date => HeuristicMatcher::match_date(text),
                        HeuristicType::Amount => HeuristicMatcher::match_amount(text),
                        HeuristicType::Phone => HeuristicMatcher::match_phone(text),
                        HeuristicType::Email => HeuristicMatcher::match_email(text),
                        HeuristicType::IdNumber => HeuristicMatcher::match_id_number(text),
                        HeuristicType::CreditCard => HeuristicMatcher::match_credit_card(text),
                    };

                    for m in heuristic_matches {
                        matches.push(RuleMatch {
                            rule_id: rule.id.clone(),
                            rule_name: rule.name.clone(),
                            matched_text: m.text,
                            start: m.start,
                            end: m.end,
                        });
                    }
                }
            }
        }

        // 按起始位置排序
        matches.sort_by_key(|m| m.start);
        matches
    }

    /// 对文本进行脱敏
    ///
    /// 将所有匹配的文本替换为脱敏标记。
    ///
    /// # 参数
    /// - `text`: 要脱敏的文本内容
    /// - `replacement`: 替换字符，默认为 "█"
    ///
    /// # 返回
    /// 脱敏后的文本
    pub fn redact_text(&self, text: &str, replacement: Option<&str>) -> String {
        let matches = self.match_text(text);
        if matches.is_empty() {
            return text.to_string();
        }

        let rep = replacement.unwrap_or("█");
        let mut result = String::with_capacity(text.len());
        let mut last_end = 0;

        for m in matches {
            // 添加匹配之前的文本
            if m.start > last_end {
                result.push_str(&text[last_end..m.start]);
            }
            // 添加替换文本（根据原文长度生成）
            let rep_len = m.matched_text.chars().count();
            for _ in 0..rep_len {
                result.push_str(rep);
            }
            last_end = m.end;
        }

        // 添加最后一段未匹配的文本
        if last_end < text.len() {
            result.push_str(&text[last_end..]);
        }

        result
    }
}
