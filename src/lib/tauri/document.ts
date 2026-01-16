import { invoke } from "@tauri-apps/api/core"
import type { DocumentPage, FileType, Rule } from "@/types"

/// 后端返回的文档信息
export interface DocumentInfo {
  path: string
  name: string
  file_type: string
  pages: DocumentPage[]
  total_pages: number
  supported_features: string[]
}

/// 规则匹配结果
export interface RuleMatch {
  rule_id: string
  rule_name: string
  matched_text: string
  start: number
  end: number
}

/// 匹配预览结果
export interface MatchPreviewResult {
  matches: RuleMatch[]
  total_count: number
}

/// 脱敏结果
export interface RedactionResult {
  success: boolean
  output_path: string | null
  matches_count: number
  message: string
}

/// 前端规则格式（传给后端）
interface FrontendRule {
  id: string
  name: string
  enabled: boolean
  is_system: boolean
  ruleType: string
  pattern: string
}

/// 将前端规则转换为后端格式
function convertRules(rules: Rule[]): FrontendRule[] {
  return rules.map((rule) => ({
    id: rule.id,
    name: rule.name,
    enabled: rule.enabled,
    is_system:
      rule.id.startsWith("id_") ||
      rule.id.startsWith("phone_") ||
      rule.id.startsWith("email") ||
      rule.id.startsWith("bank_"),
    ruleType: rule.ruleType,
    pattern: rule.pattern,
  }))
}

/// 加载文档
export async function loadDocument(filePath: string): Promise<DocumentInfo> {
  return invoke<DocumentInfo>("load_document", { filePath })
}

/// 预览匹配结果
export async function previewMatches(filePath: string, rules: Rule[]): Promise<MatchPreviewResult> {
  return invoke<MatchPreviewResult>("preview_matches", {
    filePath,
    rules: convertRules(rules),
  })
}

/// 执行脱敏
export async function applyRedaction(
  filePath: string,
  rules: Rule[],
  outputPath: string
): Promise<RedactionResult> {
  return invoke<RedactionResult>("apply_redaction", {
    filePath,
    rules: convertRules(rules),
    outputPath,
  })
}

/// 判断文件类型
export function getFileType(filePath: string): FileType | null {
  const ext = filePath.split(".").pop()?.toLowerCase()
  switch (ext) {
    case "pdf":
      return "pdf"
    case "txt":
      return "txt"
    case "md":
      return "md"
    case "docx":
      return "docx"
    default:
      return null
  }
}

/// 判断是否支持的文件类型
export function isSupportedFileType(filePath: string): boolean {
  return getFileType(filePath) !== null
}
