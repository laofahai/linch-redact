//! 启发式匹配算法
//!
//! 多语言敏感信息识别引擎，支持：
//! - 语言相关匹配：地址、人名、组织（按语言区分）
//! - 通用匹配：日期、金额、电话、邮箱、身份证号、信用卡（跨语言通用）
//!
//! 架构设计：
//! - 配置驱动：所有匹配规则从配置文件加载
//! - 语言感知：使用 whatlang 自动检测语言
//! - 可扩展：支持添加新的匹配类型和语言
//! - 数据分离：每个语言的数据文件独立存储

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use whatlang::{detect, Lang};

// ============================================================================
// 配置数据结构
// ============================================================================

#[derive(Debug, Deserialize)]
struct HeuristicsConfig {
    #[allow(dead_code)]
    version: String,
    supported_languages: Vec<String>,
    #[allow(dead_code)]
    default_language: String,
    languages: HashMap<String, LanguageConfig>,
    common: CommonConfig,
    separators: String,
}

#[derive(Debug, Deserialize)]
struct LanguageConfig {
    #[allow(dead_code)]
    name: String,
    address: Option<AddressConfig>,
    person_name: Option<PersonNameConfig>,
    organization: Option<OrganizationConfig>,
}

#[derive(Debug, Deserialize)]
struct AddressConfig {
    keywords: Option<Vec<String>>,
    keywords_file: Option<String>,
    patterns: Option<Vec<String>>,
    min_keywords: Option<usize>,
    min_length: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct PersonNameConfig {
    surnames_file: Option<String>,
    excluded_words_file: Option<String>,
    double_surnames: Option<Vec<String>>,
    name_length: Option<NameLengthConfig>,
    patterns: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct NameLengthConfig {
    min: usize,
    max: usize,
}

#[derive(Debug, Deserialize)]
struct OrganizationConfig {
    suffixes_file: Option<String>,
    patterns: Option<Vec<String>>,
    min_prefix_chars: usize,
    max_prefix_chars: usize,
}

#[derive(Debug, Deserialize)]
struct CommonConfig {
    date: PatternConfig,
    amount: PatternConfig,
    phone: PatternConfig,
    email: PatternConfig,
    id_number: PatternConfig,
    credit_card: PatternConfig,
}

#[derive(Debug, Deserialize)]
struct PatternConfig {
    patterns: Vec<String>,
}

// ============================================================================
// 静态配置加载
// ============================================================================

static CONFIG: Lazy<HeuristicsConfig> = Lazy::new(|| {
    let json_str = include_str!("../../data/heuristics.json");
    serde_json::from_str(json_str).expect("Failed to parse heuristics.json")
});

// 语言数据缓存
static LANGUAGE_DATA: Lazy<HashMap<String, LanguageData>> = Lazy::new(|| {
    let mut data = HashMap::new();

    for lang_code in &CONFIG.supported_languages {
        if let Some(lang_config) = CONFIG.languages.get(lang_code) {
            let lang_data = load_language_data(lang_code, lang_config);
            data.insert(lang_code.clone(), lang_data);
        }
    }

    data
});

// 预编译正则表达式
static COMMON_PATTERNS: Lazy<CommonPatterns> = Lazy::new(|| CommonPatterns {
    date: compile_patterns(&CONFIG.common.date.patterns),
    amount: compile_patterns(&CONFIG.common.amount.patterns),
    phone: compile_patterns(&CONFIG.common.phone.patterns),
    email: compile_patterns(&CONFIG.common.email.patterns),
    id_number: compile_patterns(&CONFIG.common.id_number.patterns),
    credit_card: compile_patterns(&CONFIG.common.credit_card.patterns),
});

// ============================================================================
// 语言数据结构
// ============================================================================

struct LanguageData {
    // 地址
    address_keywords: HashSet<String>,
    address_patterns: Vec<Regex>,
    address_min_keywords: usize,
    address_min_length: usize,

    // 人名
    single_surnames: HashSet<String>,
    double_surnames: HashSet<String>,
    excluded_words: HashSet<String>,
    name_patterns: Vec<Regex>,
    name_min_len: usize,
    name_max_len: usize,

    // 组织
    org_suffixes: Vec<String>,
    org_patterns: Vec<Regex>,
    org_min_prefix: usize,
    org_max_prefix: usize,
}

struct CommonPatterns {
    date: Vec<Regex>,
    amount: Vec<Regex>,
    phone: Vec<Regex>,
    email: Vec<Regex>,
    id_number: Vec<Regex>,
    credit_card: Vec<Regex>,
}

// ============================================================================
// 数据加载函数
// ============================================================================

fn load_language_data(lang_code: &str, config: &LanguageConfig) -> LanguageData {
    // 加载地址数据
    let (address_keywords, address_patterns, address_min_keywords, address_min_length) =
        if let Some(addr) = &config.address {
            let keywords = load_keywords(addr.keywords_file.as_deref(), addr.keywords.as_ref());
            let patterns = addr
                .patterns
                .as_ref()
                .map(|p| compile_patterns(p))
                .unwrap_or_default();
            (
                keywords,
                patterns,
                addr.min_keywords.unwrap_or(2),
                addr.min_length.unwrap_or(4),
            )
        } else {
            (HashSet::new(), Vec::new(), 2, 4)
        };

    // 加载人名数据
    let (
        single_surnames,
        double_surnames,
        excluded_words,
        name_patterns,
        name_min_len,
        name_max_len,
    ) = if let Some(person) = &config.person_name {
        let (single, double) = load_surnames(
            person.surnames_file.as_deref(),
            person.double_surnames.as_ref(),
            lang_code,
        );
        let excluded = load_excluded_words(person.excluded_words_file.as_deref());
        let patterns = person
            .patterns
            .as_ref()
            .map(|p| compile_patterns(p))
            .unwrap_or_default();
        let (min_len, max_len) = person
            .name_length
            .as_ref()
            .map(|nl| (nl.min, nl.max))
            .unwrap_or((1, 2));
        (single, double, excluded, patterns, min_len, max_len)
    } else {
        (
            HashSet::new(),
            HashSet::new(),
            HashSet::new(),
            Vec::new(),
            1,
            2,
        )
    };

    // 加载组织数据
    let (org_suffixes, org_patterns, org_min_prefix, org_max_prefix) =
        if let Some(org) = &config.organization {
            let suffixes = load_org_suffixes(org.suffixes_file.as_deref());
            let patterns = org
                .patterns
                .as_ref()
                .map(|p| compile_patterns(p))
                .unwrap_or_default();
            (
                suffixes,
                patterns,
                org.min_prefix_chars,
                org.max_prefix_chars,
            )
        } else {
            (Vec::new(), Vec::new(), 2, 20)
        };

    LanguageData {
        address_keywords,
        address_patterns,
        address_min_keywords,
        address_min_length,
        single_surnames,
        double_surnames,
        excluded_words,
        name_patterns,
        name_min_len,
        name_max_len,
        org_suffixes,
        org_patterns,
        org_min_prefix,
        org_max_prefix,
    }
}

fn load_keywords(
    file_path: Option<&str>,
    inline_keywords: Option<&Vec<String>>,
) -> HashSet<String> {
    let mut keywords = HashSet::new();

    // 从文件加载
    if let Some(path) = file_path {
        let content = load_data_file(path);
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                keywords.insert(line.to_string());
            }
        }
    }

    // 从内联配置加载
    if let Some(inline) = inline_keywords {
        for kw in inline {
            keywords.insert(kw.clone());
        }
    }

    keywords
}

fn load_surnames(
    file_path: Option<&str>,
    double_surnames_config: Option<&Vec<String>>,
    lang_code: &str,
) -> (HashSet<String>, HashSet<String>) {
    let mut single = HashSet::new();
    let mut double = HashSet::new();

    // 加载复姓列表
    if let Some(ds) = double_surnames_config {
        for s in ds {
            double.insert(s.clone());
        }
    }

    // 从文件加载姓氏
    if let Some(path) = file_path {
        let content = load_data_file(path);

        match lang_code {
            "zh" => {
                // 中文：连续字符，需要按字符分割
                for ch in content.chars() {
                    if !ch.is_whitespace() {
                        single.insert(ch.to_string());
                    }
                }
            }
            "ja" | "ko" => {
                // 日语/韩语：每行一个姓氏
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        single.insert(line.to_string());
                    }
                }
            }
            _ => {
                // 其他语言：每行一个姓氏
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        single.insert(line.to_string());
                    }
                }
            }
        }
    }

    (single, double)
}

fn load_excluded_words(file_path: Option<&str>) -> HashSet<String> {
    let mut words = HashSet::new();

    if let Some(path) = file_path {
        let content = load_data_file(path);
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                words.insert(line.to_string());
            }
        }
    }

    words
}

fn load_org_suffixes(file_path: Option<&str>) -> Vec<String> {
    let mut suffixes = Vec::new();

    if let Some(path) = file_path {
        let content = load_data_file(path);
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                suffixes.push(line.to_string());
            }
        }
    }

    suffixes
}

fn load_data_file(relative_path: &str) -> String {
    // 使用 include_str! 静态嵌入数据文件
    match relative_path {
        "languages/zh/surnames.txt" => {
            include_str!("../../data/languages/zh/surnames.txt").to_string()
        }
        "languages/zh/address_keywords.txt" => {
            include_str!("../../data/languages/zh/address_keywords.txt").to_string()
        }
        "languages/zh/excluded_words.txt" => {
            include_str!("../../data/languages/zh/excluded_words.txt").to_string()
        }
        "languages/zh/org_suffixes.txt" => {
            include_str!("../../data/languages/zh/org_suffixes.txt").to_string()
        }
        "languages/en/org_suffixes.txt" => {
            include_str!("../../data/languages/en/org_suffixes.txt").to_string()
        }
        "languages/ja/surnames.txt" => {
            include_str!("../../data/languages/ja/surnames.txt").to_string()
        }
        "languages/ja/org_suffixes.txt" => {
            include_str!("../../data/languages/ja/org_suffixes.txt").to_string()
        }
        "languages/ko/surnames.txt" => {
            include_str!("../../data/languages/ko/surnames.txt").to_string()
        }
        "languages/ko/org_suffixes.txt" => {
            include_str!("../../data/languages/ko/org_suffixes.txt").to_string()
        }
        _ => String::new(),
    }
}

fn compile_patterns(patterns: &[String]) -> Vec<Regex> {
    patterns.iter().filter_map(|p| Regex::new(p).ok()).collect()
}

// ============================================================================
// 语言检测
// ============================================================================

/// 支持的语言代码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageCode {
    Chinese,  // zh
    English,  // en
    Japanese, // ja
    Korean,   // ko
    Unknown,
}

impl LanguageCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            LanguageCode::Chinese => "zh",
            LanguageCode::English => "en",
            LanguageCode::Japanese => "ja",
            LanguageCode::Korean => "ko",
            LanguageCode::Unknown => "zh", // 默认中文
        }
    }
}

/// 使用 whatlang 检测文本语言
pub fn detect_language(text: &str) -> LanguageCode {
    // 先用 whatlang 检测
    if let Some(info) = detect(text) {
        match info.lang() {
            Lang::Cmn => LanguageCode::Chinese, // 普通话/简体中文
            Lang::Eng => LanguageCode::English,
            Lang::Jpn => LanguageCode::Japanese,
            Lang::Kor => LanguageCode::Korean,
            _ => {
                // whatlang 可能检测为其他语言，但我们只支持这四种
                // 回退到字符检测
                detect_by_chars(text)
            }
        }
    } else {
        // whatlang 无法检测，使用字符检测
        detect_by_chars(text)
    }
}

/// 基于字符的语言检测（作为回退方案）
fn detect_by_chars(text: &str) -> LanguageCode {
    let mut han_count = 0; // CJK 汉字
    let mut japanese_count = 0; // 平假名/片假名
    let mut korean_count = 0; // 韩文
    let mut total_cjk = 0;

    for c in text.chars() {
        match c {
            '\u{4E00}'..='\u{9FFF}' => {
                han_count += 1;
                total_cjk += 1;
            }
            '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}' => {
                japanese_count += 1;
                total_cjk += 1;
            }
            '\u{AC00}'..='\u{D7AF}' | '\u{1100}'..='\u{11FF}' => {
                korean_count += 1;
                total_cjk += 1;
            }
            _ => {}
        }
    }

    let total_chars = text.chars().filter(|c| !c.is_whitespace()).count();

    if total_chars == 0 {
        return LanguageCode::Unknown;
    }

    // 如果有显著的日语假名，判定为日语
    if japanese_count > 0 && (total_cjk == 0 || japanese_count as f64 / total_cjk as f64 > 0.1) {
        return LanguageCode::Japanese;
    }

    // 如果有韩文，判定为韩语
    if korean_count > 0 {
        return LanguageCode::Korean;
    }

    // 如果 CJK 汉字占比超过 30%，判定为中文
    if han_count as f64 / total_chars as f64 > 0.3 {
        return LanguageCode::Chinese;
    }

    LanguageCode::English
}

// ============================================================================
// 匹配结果
// ============================================================================

/// 匹配结果
pub struct HeuristicMatch {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

// ============================================================================
// 辅助函数
// ============================================================================

fn is_separator(c: char) -> bool {
    c.is_whitespace() || CONFIG.separators.contains(c)
}

fn is_cjk_char(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3040}'..='\u{30FF}' | '\u{AC00}'..='\u{D7AF}')
}

// ============================================================================
// 启发式匹配器
// ============================================================================

/// 启发式匹配器
///
/// 提供多语言敏感信息识别能力
pub struct HeuristicMatcher;

impl HeuristicMatcher {
    // ========================================================================
    // 语言相关的匹配（地址、人名、组织）
    // ========================================================================

    /// 识别地址
    pub fn match_address(text: &str) -> Vec<HeuristicMatch> {
        let lang = detect_language(text);
        Self::match_address_for_lang(text, lang)
    }

    /// 为指定语言识别地址
    fn match_address_for_lang(text: &str, lang: LanguageCode) -> Vec<HeuristicMatch> {
        let lang_code = lang.as_str();
        let lang_data = match LANGUAGE_DATA.get(lang_code) {
            Some(data) => data,
            None => return Vec::new(),
        };

        let mut matches = Vec::new();

        // 1. 先尝试正则匹配
        for pattern in &lang_data.address_patterns {
            for m in pattern.find_iter(text) {
                matches.push(HeuristicMatch {
                    text: m.as_str().to_string(),
                    start: m.start(),
                    end: m.end(),
                });
            }
        }

        // 2. 关键词匹配（主要用于 CJK 语言）
        if !lang_data.address_keywords.is_empty() {
            let chars: Vec<char> = text.chars().collect();
            let mut i = 0;

            while i < chars.len() {
                let segment_start = i;
                let mut segment = String::new();
                let mut keyword_count = 0;

                while i < chars.len() && !is_separator(chars[i]) {
                    segment.push(chars[i]);
                    i += 1;
                }

                if segment.chars().count() >= lang_data.address_min_length {
                    for keyword in &lang_data.address_keywords {
                        if segment.contains(keyword.as_str()) {
                            keyword_count += 1;
                        }
                    }
                }

                if keyword_count >= lang_data.address_min_keywords {
                    let byte_start = text
                        .char_indices()
                        .nth(segment_start)
                        .map(|(idx, _)| idx)
                        .unwrap_or(0);
                    let byte_end = byte_start + segment.len();

                    matches.push(HeuristicMatch {
                        text: segment,
                        start: byte_start,
                        end: byte_end,
                    });
                }

                while i < chars.len() && is_separator(chars[i]) {
                    i += 1;
                }
            }
        }

        matches.sort_by_key(|m| m.start);
        matches.dedup_by(|a, b| a.start == b.start);
        matches
    }

    /// 识别人名
    pub fn match_person_name(text: &str) -> Vec<HeuristicMatch> {
        let lang = detect_language(text);
        Self::match_person_name_for_lang(text, lang)
    }

    /// 为指定语言识别人名
    fn match_person_name_for_lang(text: &str, lang: LanguageCode) -> Vec<HeuristicMatch> {
        let lang_code = lang.as_str();
        let lang_data = match LANGUAGE_DATA.get(lang_code) {
            Some(data) => data,
            None => return Vec::new(),
        };

        let mut matches = Vec::new();

        // 1. 先尝试正则匹配（主要用于英文）
        for pattern in &lang_data.name_patterns {
            for m in pattern.find_iter(text) {
                matches.push(HeuristicMatch {
                    text: m.as_str().to_string(),
                    start: m.start(),
                    end: m.end(),
                });
            }
        }

        // 2. 基于姓氏的匹配（主要用于 CJK 语言）
        if !lang_data.single_surnames.is_empty() {
            let chars: Vec<char> = text.chars().collect();
            let mut i = 0;

            while i < chars.len() {
                let mut found = false;

                // 检查复姓
                if !lang_data.double_surnames.is_empty() && i + 3 <= chars.len() {
                    let surname: String = chars[i..i + 2].iter().collect();
                    if lang_data.double_surnames.contains(&surname) {
                        for name_len in lang_data.name_min_len..=lang_data.name_max_len {
                            if i + 2 + name_len <= chars.len() {
                                let name_chars = &chars[i + 2..i + 2 + name_len];
                                if name_chars.iter().all(|c| is_cjk_char(*c)) {
                                    let full_name: String =
                                        chars[i..i + 2 + name_len].iter().collect();
                                    let byte_start =
                                        text.char_indices().nth(i).map(|(idx, _)| idx).unwrap_or(0);
                                    let byte_end = byte_start + full_name.len();

                                    matches.push(HeuristicMatch {
                                        text: full_name,
                                        start: byte_start,
                                        end: byte_end,
                                    });
                                    i += 2 + name_len;
                                    found = true;
                                    break;
                                }
                            }
                        }
                    }
                }

                // 检查单姓
                if !found && i + 1 < chars.len() {
                    let surname = chars[i].to_string();
                    if lang_data.single_surnames.contains(&surname) {
                        for name_len in lang_data.name_min_len..=lang_data.name_max_len {
                            if i + 1 + name_len <= chars.len() {
                                let name_chars = &chars[i + 1..i + 1 + name_len];
                                if name_chars.iter().all(|c| is_cjk_char(*c)) {
                                    let full_name: String =
                                        chars[i..i + 1 + name_len].iter().collect();

                                    if !lang_data.excluded_words.contains(&full_name) {
                                        let byte_start = text
                                            .char_indices()
                                            .nth(i)
                                            .map(|(idx, _)| idx)
                                            .unwrap_or(0);
                                        let byte_end = byte_start + full_name.len();

                                        matches.push(HeuristicMatch {
                                            text: full_name,
                                            start: byte_start,
                                            end: byte_end,
                                        });
                                        i += 1 + name_len;
                                        found = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                if !found {
                    i += 1;
                }
            }
        }

        matches.sort_by_key(|m| m.start);
        matches.dedup_by(|a, b| a.start == b.start);
        matches
    }

    /// 识别组织机构名称
    pub fn match_organization(text: &str) -> Vec<HeuristicMatch> {
        let lang = detect_language(text);
        Self::match_organization_for_lang(text, lang)
    }

    /// 为指定语言识别组织名称
    fn match_organization_for_lang(text: &str, lang: LanguageCode) -> Vec<HeuristicMatch> {
        let lang_code = lang.as_str();
        let lang_data = match LANGUAGE_DATA.get(lang_code) {
            Some(data) => data,
            None => return Vec::new(),
        };

        let mut matches = Vec::new();

        // 1. 先尝试正则匹配
        for pattern in &lang_data.org_patterns {
            for cap in pattern.captures_iter(text) {
                if let Some(m) = cap.get(0) {
                    matches.push(HeuristicMatch {
                        text: m.as_str().to_string(),
                        start: m.start(),
                        end: m.end(),
                    });
                }
            }
        }

        // 2. 基于后缀的匹配
        for suffix in &lang_data.org_suffixes {
            let mut start = 0;
            while let Some(pos) = text[start..].find(suffix.as_str()) {
                let suffix_start = start + pos;
                let prefix_text = &text[..suffix_start];
                let prefix_chars: Vec<char> = prefix_text.chars().collect();

                let mut org_start_char_idx = prefix_chars.len();
                let mut char_count = 0;

                for (idx, c) in prefix_chars.iter().enumerate().rev() {
                    if is_separator(*c) {
                        break;
                    }
                    char_count += 1;
                    org_start_char_idx = idx;

                    if char_count >= lang_data.org_max_prefix {
                        break;
                    }
                }

                let org_start = prefix_text
                    .char_indices()
                    .nth(org_start_char_idx)
                    .map(|(idx, _)| idx)
                    .unwrap_or(suffix_start);
                let org_end = suffix_start + suffix.len();

                let org_name = &text[org_start..org_end];
                let prefix_char_count = org_name.chars().count() - suffix.chars().count();

                if prefix_char_count >= lang_data.org_min_prefix {
                    matches.push(HeuristicMatch {
                        text: org_name.to_string(),
                        start: org_start,
                        end: org_end,
                    });
                }

                start = org_end;
            }
        }

        matches.sort_by_key(|m| m.start);
        matches.dedup_by_key(|m| m.start);
        matches
    }

    // ========================================================================
    // 通用匹配（日期、金额、电话、邮箱、身份证、信用卡）
    // ========================================================================

    /// 识别日期
    pub fn match_date(text: &str) -> Vec<HeuristicMatch> {
        match_with_patterns(text, &COMMON_PATTERNS.date)
    }

    /// 识别金额
    pub fn match_amount(text: &str) -> Vec<HeuristicMatch> {
        match_with_patterns(text, &COMMON_PATTERNS.amount)
    }

    /// 识别电话号码
    pub fn match_phone(text: &str) -> Vec<HeuristicMatch> {
        match_with_patterns(text, &COMMON_PATTERNS.phone)
    }

    /// 识别邮箱地址
    pub fn match_email(text: &str) -> Vec<HeuristicMatch> {
        match_with_patterns(text, &COMMON_PATTERNS.email)
    }

    /// 识别身份证号/社会安全号
    pub fn match_id_number(text: &str) -> Vec<HeuristicMatch> {
        match_with_patterns(text, &COMMON_PATTERNS.id_number)
    }

    /// 识别信用卡号
    pub fn match_credit_card(text: &str) -> Vec<HeuristicMatch> {
        match_with_patterns(text, &COMMON_PATTERNS.credit_card)
    }
}

/// 使用正则表达式列表进行匹配
fn match_with_patterns(text: &str, patterns: &[Regex]) -> Vec<HeuristicMatch> {
    let mut matches = Vec::new();

    for pattern in patterns {
        for m in pattern.find_iter(text) {
            matches.push(HeuristicMatch {
                text: m.as_str().to_string(),
                start: m.start(),
                end: m.end(),
            });
        }
    }

    matches.sort_by_key(|m| m.start);
    matches.dedup_by(|a, b| a.start == b.start && a.end == b.end);
    matches
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language_chinese() {
        assert_eq!(detect_language("这是中文文本"), LanguageCode::Chinese);
        assert_eq!(detect_language("北京市朝阳区"), LanguageCode::Chinese);
    }

    #[test]
    fn test_detect_language_english() {
        assert_eq!(
            detect_language("This is English text"),
            LanguageCode::English
        );
        assert_eq!(detect_language("Hello World"), LanguageCode::English);
    }

    #[test]
    fn test_detect_language_japanese() {
        assert_eq!(detect_language("これは日本語です"), LanguageCode::Japanese);
        // 注意：纯汉字的日语文本可能被检测为中文，这是已知限制
        // 包含假名的文本可以准确检测
        assert_eq!(
            detect_language("東京都渋谷区にあります"),
            LanguageCode::Japanese
        );
    }

    #[test]
    fn test_detect_language_korean() {
        assert_eq!(detect_language("이것은 한국어입니다"), LanguageCode::Korean);
        assert_eq!(detect_language("서울특별시 강남구"), LanguageCode::Korean);
    }

    #[test]
    fn test_match_address_zh() {
        let text = "我家住在北京市朝阳区建国路88号国贸大厦";
        let matches = HeuristicMatcher::match_address(text);
        assert!(!matches.is_empty());
    }

    #[test]
    fn test_match_person_name_zh() {
        let text = "张三和李四是好朋友，欧阳明也认识他们";
        let matches = HeuristicMatcher::match_person_name(text);
        assert!(matches.len() >= 2);
    }

    #[test]
    fn test_match_person_name_ja() {
        let text = "佐藤さんと鈴木さんは友達です";
        let matches = HeuristicMatcher::match_person_name(text);
        println!(
            "JA Name matches: {:?}",
            matches.iter().map(|m| &m.text).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_match_organization_zh() {
        let text = "北京科技有限公司与清华大学合作";
        let matches = HeuristicMatcher::match_organization(text);
        assert!(!matches.is_empty());
    }

    #[test]
    fn test_match_organization_ja() {
        let text = "トヨタ自動車株式会社は東京大学と提携";
        let matches = HeuristicMatcher::match_organization(text);
        println!(
            "JA Org matches: {:?}",
            matches.iter().map(|m| &m.text).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_match_date() {
        let text = "会议日期：2024年1月15日，截止日期 2024-12-31";
        let matches = HeuristicMatcher::match_date(text);
        assert!(matches.len() >= 2);
    }

    #[test]
    fn test_match_amount() {
        let text = "总金额为¥12,345.67元，折合USD 1,700.00";
        let matches = HeuristicMatcher::match_amount(text);
        assert!(matches.len() >= 1);
    }

    #[test]
    fn test_match_phone() {
        let text = "联系电话：13812345678，或拨打 (555) 123-4567";
        let matches = HeuristicMatcher::match_phone(text);
        assert!(matches.len() >= 1);
    }

    #[test]
    fn test_match_email() {
        let text = "请发送邮件至 test@example.com 或 support@company.org";
        let matches = HeuristicMatcher::match_email(text);
        assert_eq!(matches.len(), 2);
    }
}
