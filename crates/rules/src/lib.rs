//! Rules schema and matching.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePack {
    pub name: String,
    pub version: String,
    pub hash: String,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub rule_type: RuleType,
    pub scope: PageScope,
    pub action: RuleAction,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleType {
    Keyword { value: String },
    Regex { pattern: String },
    Dictionary { name: String, entries: Vec<String> },
    Region { name: String, page: u32, bbox: BBox },
    PageRule { pages: PageScope },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    RedactText,
    RedactRegion,
    RemovePage,
    KeepOnlyPages,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PageScope {
    All,
    Page(u32),
    Range { start: u32, end: u32 },
    List(Vec<u32>),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchHit {
    pub page: u32,
    pub bbox: BBox,
    pub rule_id: String,
    pub snippet: String,
    pub confidence: Option<f32>,
}

pub fn match_text(_text: &str, _rules: &[Rule]) -> Vec<MatchHit> {
    Vec::new()
}
