import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Shield, ChevronDown, ChevronUp, Loader2, Plus, Pencil, Trash2 } from "lucide-react"
import { Switch } from "@/components/ui/switch"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import { useDetectionRulesStore, builtinRuleIds } from "@/stores/useDetectionRulesStore"
import { useFileStore } from "@/stores/useFileStore"
import { previewMatches, type RuleMatch } from "@/lib/tauri/document"
import { RuleEditDialog } from "./RuleEditDialog"
import type { Rule } from "@/types"

export function RulesPanel() {
  const { t } = useTranslation()
  const rules = useDetectionRulesStore((s) => s.rules)
  const toggleRule = useDetectionRulesStore((s) => s.toggleRule)
  const addRule = useDetectionRulesStore((s) => s.addRule)
  const updateRule = useDetectionRulesStore((s) => s.updateRule)
  const removeRule = useDetectionRulesStore((s) => s.removeRule)
  const selectedDocument = useFileStore((s) => s.getSelectedDocument())

  const [expanded, setExpanded] = useState(true)
  const [scanning, setScanning] = useState(false)
  const [matches, setMatches] = useState<RuleMatch[]>([])
  const [scanComplete, setScanComplete] = useState(false)

  // 规则编辑对话框状态
  const [editDialogOpen, setEditDialogOpen] = useState(false)
  const [editingRule, setEditingRule] = useState<Rule | null>(null)

  const enabledRulesCount = rules.filter((r) => r.enabled).length

  // 分类规则：启发式规则和自定义规则
  const heuristicRules = rules.filter((r) => r.ruleType === "heuristic")
  const customRules = rules.filter((r) => r.ruleType !== "heuristic")

  const handleScan = async () => {
    if (!selectedDocument || selectedDocument.status !== "ready") return

    setScanning(true)
    setScanComplete(false)
    setMatches([])

    try {
      const enabledRules = rules.filter((r) => r.enabled)
      const result = await previewMatches(selectedDocument.path, enabledRules)
      setMatches(result.matches)
      setScanComplete(true)
    } catch (e) {
      console.error("扫描失败:", e)
    } finally {
      setScanning(false)
    }
  }

  const handleAddRule = () => {
    setEditingRule(null)
    setEditDialogOpen(true)
  }

  const handleEditRule = (rule: Rule) => {
    setEditingRule(rule)
    setEditDialogOpen(true)
  }

  const handleSaveRule = (ruleData: Omit<Rule, "id"> | Rule) => {
    if ("id" in ruleData) {
      // 编辑现有规则
      updateRule(ruleData.id, ruleData)
    } else {
      // 添加新规则
      addRule(ruleData)
    }
  }

  const handleDeleteRule = (id: string) => {
    removeRule(id)
  }

  return (
    <div className="flex flex-col h-full">
      {/* 标题栏 */}
      <div
        className="flex items-center justify-between cursor-pointer py-2"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex items-center gap-2">
          <Shield className="h-4 w-4 text-primary" />
          <span className="text-sm font-medium">{t("detectionRules.title")}</span>
          <span className="text-xs text-muted-foreground">
            ({enabledRulesCount}/{rules.length})
          </span>
        </div>
        {expanded ? (
          <ChevronUp className="h-4 w-4 text-muted-foreground" />
        ) : (
          <ChevronDown className="h-4 w-4 text-muted-foreground" />
        )}
      </div>

      {expanded && (
        <>
          {/* 添加规则按钮 */}
          <Button
            variant="outline"
            size="sm"
            className="mb-2 w-full justify-start gap-2"
            onClick={(e) => {
              e.stopPropagation()
              handleAddRule()
            }}
          >
            <Plus className="h-3.5 w-3.5" />
            {t("detectionRules.addCustomRule")}
          </Button>

          {/* 规则列表 */}
          <ScrollArea className="flex-1 -mx-1 px-1">
            <div className="space-y-3">
              {/* 启发式规则（智能识别） */}
              {heuristicRules.length > 0 && (
                <div className="space-y-1">
                  <div className="text-xs text-muted-foreground px-2 py-1">
                    {t("detectionRules.heuristicRules")}
                  </div>
                  {heuristicRules.map((rule) => (
                    <div
                      key={rule.id}
                      className="flex items-center justify-between rounded-md px-2 py-1.5 hover:bg-muted/50"
                    >
                      <div className="flex items-center gap-2 min-w-0 flex-1">
                        <Switch
                          checked={rule.enabled}
                          onCheckedChange={() => toggleRule(rule.id)}
                          className="shrink-0"
                        />
                        <span className="text-sm truncate">{rule.name}</span>
                        <span className="text-[10px] px-1 py-0.5 rounded bg-blue-500/10 text-blue-500 shrink-0">
                          AI
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              )}

              {/* 自定义规则 */}
              {customRules.length > 0 && (
                <div className="space-y-1">
                  <div className="text-xs text-muted-foreground px-2 py-1">
                    {t("detectionRules.customRules")}
                  </div>
                  {customRules.map((rule) => {
                    const isBuiltin = builtinRuleIds.has(rule.id)
                    return (
                      <div
                        key={rule.id}
                        className="group flex items-center justify-between rounded-md px-2 py-1.5 hover:bg-muted/50"
                      >
                        <div className="flex items-center gap-2 min-w-0 flex-1">
                          <Switch
                            checked={rule.enabled}
                            onCheckedChange={() => toggleRule(rule.id)}
                            className="shrink-0"
                          />
                          <span className="text-sm truncate">{rule.name}</span>
                          {isBuiltin && (
                            <span className="text-[10px] px-1 py-0.5 rounded bg-primary/10 text-primary shrink-0">
                              {t("detectionRules.builtinRules")}
                            </span>
                          )}
                        </div>
                        {/* 操作按钮 */}
                        <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                          {!isBuiltin && (
                            <>
                              <button
                                className="p-1 hover:bg-muted rounded"
                                onClick={(e) => {
                                  e.stopPropagation()
                                  handleEditRule(rule)
                                }}
                              >
                                <Pencil className="h-3 w-3 text-muted-foreground" />
                              </button>
                              <button
                                className="p-1 hover:bg-destructive/10 rounded"
                                onClick={(e) => {
                                  e.stopPropagation()
                                  handleDeleteRule(rule.id)
                                }}
                              >
                                <Trash2 className="h-3 w-3 text-destructive" />
                              </button>
                            </>
                          )}
                        </div>
                      </div>
                    )
                  })}
                </div>
              )}
            </div>
          </ScrollArea>

          {/* 扫描按钮 */}
          <div className="pt-3 border-t mt-3">
            <Button
              size="sm"
              className="w-full"
              disabled={
                !selectedDocument ||
                selectedDocument.status !== "ready" ||
                scanning ||
                enabledRulesCount === 0
              }
              onClick={handleScan}
            >
              {scanning ? (
                <>
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  {t("common.scanning")}
                </>
              ) : (
                t("detection.scan")
              )}
            </Button>

            {/* 扫描结果 */}
            {scanComplete && (
              <div className="mt-2 text-sm text-center">
                {matches.length > 0 ? (
                  <span className="text-destructive">
                    {t("detection.foundResults", { count: matches.length })}
                  </span>
                ) : (
                  <span className="text-muted-foreground">{t("detection.noResults")}</span>
                )}
              </div>
            )}
          </div>
        </>
      )}

      {/* 规则编辑对话框 */}
      <RuleEditDialog
        open={editDialogOpen}
        onOpenChange={setEditDialogOpen}
        rule={editingRule}
        onSave={handleSaveRule}
      />
    </div>
  )
}
