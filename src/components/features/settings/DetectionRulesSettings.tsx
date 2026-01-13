import { useMemo, useState } from "react"
import { Trash2, RotateCcw, Plus } from "lucide-react"
import { Input } from "@/components/ui/input"
import { Switch } from "@/components/ui/switch"
import { Button } from "@/components/ui/button"
import { useDetectionRulesStore } from "@/stores"
import { builtinRuleIds } from "@/stores/useDetectionRulesStore"

type RuleDraft = {
  name: string
  ruleType: "regex" | "keyword"
  pattern: string
}

export function DetectionRulesSettings() {
  const rules = useDetectionRulesStore((s) => s.rules)
  const addRule = useDetectionRulesStore((s) => s.addRule)
  const updateRule = useDetectionRulesStore((s) => s.updateRule)
  const removeRule = useDetectionRulesStore((s) => s.removeRule)
  const resetRules = useDetectionRulesStore((s) => s.resetRules)
  const [draft, setDraft] = useState<RuleDraft>({
    name: "",
    ruleType: "regex",
    pattern: "",
  })

  const builtinIds = useMemo(() => builtinRuleIds, [])

  const handleAdd = () => {
    if (!draft.name.trim() || !draft.pattern.trim()) return
    addRule({
      name: draft.name.trim(),
      ruleType: draft.ruleType,
      pattern: draft.pattern.trim(),
      enabled: true,
    })
    setDraft({ name: "", ruleType: "regex", pattern: "" })
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium">检测规则</h3>
        <Button variant="ghost" size="sm" onClick={resetRules}>
          <RotateCcw className="h-4 w-4 mr-1" />
          重置
        </Button>
      </div>

      <div className="rounded-lg border p-3 space-y-2">
        <div className="text-xs text-muted-foreground">新增自定义规则</div>
        <div className="grid grid-cols-2 gap-2">
          <Input
            value={draft.name}
            onChange={(e) => setDraft((prev) => ({ ...prev, name: e.target.value }))}
            placeholder="规则名称"
          />
          <select
            value={draft.ruleType}
            onChange={(e) =>
              setDraft((prev) => ({
                ...prev,
                ruleType: e.target.value as RuleDraft["ruleType"],
              }))
            }
            className="h-9 w-full rounded-md border border-input bg-transparent px-3 text-sm"
          >
            <option value="regex">正则</option>
            <option value="keyword">关键词</option>
          </select>
        </div>
        <div className="flex items-center gap-2">
          <Input
            value={draft.pattern}
            onChange={(e) => setDraft((prev) => ({ ...prev, pattern: e.target.value }))}
            placeholder="正则或关键词"
          />
          <Button size="sm" onClick={handleAdd}>
            <Plus className="h-4 w-4 mr-1" />
            添加
          </Button>
        </div>
      </div>

      <div className="space-y-3">
        {rules.map((rule) => (
          <div key={rule.id} className="rounded-lg border p-3 space-y-2">
            <div className="flex items-center justify-between gap-3">
              <Input
                value={rule.name}
                onChange={(e) => updateRule(rule.id, { name: e.target.value })}
                className="h-8"
              />
              <Switch
                checked={rule.enabled}
                onCheckedChange={() => updateRule(rule.id, { enabled: !rule.enabled })}
              />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <select
                value={rule.ruleType}
                onChange={(e) =>
                  updateRule(rule.id, { ruleType: e.target.value as RuleDraft["ruleType"] })
                }
                className="h-9 w-full rounded-md border border-input bg-transparent px-3 text-sm"
              >
                <option value="regex">正则</option>
                <option value="keyword">关键词</option>
              </select>
              <Input
                value={rule.pattern}
                onChange={(e) => updateRule(rule.id, { pattern: e.target.value })}
              />
            </div>
            <div className="flex justify-end">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => removeRule(rule.id)}
                disabled={builtinIds.has(rule.id)}
              >
                <Trash2 className="h-4 w-4 mr-1" />
                删除
              </Button>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
