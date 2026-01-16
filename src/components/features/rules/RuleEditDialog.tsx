import { useState, useEffect } from "react"
import { useTranslation } from "react-i18next"
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group"
import type { Rule } from "@/types"

interface RuleEditDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  rule?: Rule | null
  onSave: (rule: Omit<Rule, "id"> | Rule) => void
}

export function RuleEditDialog({ open, onOpenChange, rule, onSave }: RuleEditDialogProps) {
  const { t } = useTranslation()
  const isEditing = !!rule

  const [name, setName] = useState("")
  const [ruleType, setRuleType] = useState<"regex" | "keyword">("keyword")
  const [pattern, setPattern] = useState("")
  const [error, setError] = useState("")

  useEffect(() => {
    if (open) {
      if (rule) {
        setName(rule.name)
        // 启发式规则不能编辑，只支持 keyword 和 regex
        const type =
          rule.ruleType === "keyword" || rule.ruleType === "regex" ? rule.ruleType : "keyword"
        setRuleType(type)
        setPattern(rule.pattern)
      } else {
        setName("")
        setRuleType("keyword")
        setPattern("")
      }
      setError("")
    }
  }, [open, rule])

  const handleSave = () => {
    // 验证
    if (!name.trim()) {
      setError(t("detectionRules.ruleName") + " is required")
      return
    }
    if (!pattern.trim()) {
      setError(t("detectionRules.pattern") + " is required")
      return
    }

    // 如果是正则表达式，验证语法
    if (ruleType === "regex") {
      try {
        new RegExp(pattern)
      } catch {
        setError("Invalid regex pattern")
        return
      }
    }

    const ruleData = {
      name: name.trim(),
      ruleType,
      pattern: pattern.trim(),
      enabled: true,
    }

    if (isEditing && rule) {
      onSave({ ...ruleData, id: rule.id })
    } else {
      onSave(ruleData)
    }

    onOpenChange(false)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>
            {isEditing ? t("common.save") : t("detectionRules.addCustomRule")}
          </DialogTitle>
        </DialogHeader>

        <div className="grid gap-4 py-4">
          {/* 规则名称 */}
          <div className="grid gap-2">
            <Label htmlFor="name">{t("detectionRules.ruleName")}</Label>
            <Input
              id="name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={t("detectionRules.ruleName")}
            />
          </div>

          {/* 规则类型 */}
          <div className="grid gap-2">
            <Label>Type</Label>
            <RadioGroup
              value={ruleType}
              onValueChange={(v) => setRuleType(v as "regex" | "keyword")}
              className="flex gap-4"
            >
              <div className="flex items-center space-x-2">
                <RadioGroupItem value="keyword" id="keyword" />
                <Label htmlFor="keyword" className="font-normal cursor-pointer">
                  {t("detectionRules.ruleType.keyword")}
                </Label>
              </div>
              <div className="flex items-center space-x-2">
                <RadioGroupItem value="regex" id="regex" />
                <Label htmlFor="regex" className="font-normal cursor-pointer">
                  {t("detectionRules.ruleType.regex")}
                </Label>
              </div>
            </RadioGroup>
          </div>

          {/* 匹配内容 */}
          <div className="grid gap-2">
            <Label htmlFor="pattern">{t("detectionRules.pattern")}</Label>
            <Input
              id="pattern"
              value={pattern}
              onChange={(e) => setPattern(e.target.value)}
              placeholder={
                ruleType === "keyword" ? "keyword1, keyword2, keyword3" : "\\d{4}-\\d{2}-\\d{2}"
              }
            />
            <p className="text-xs text-muted-foreground">
              {ruleType === "keyword"
                ? "Enter keywords separated by commas"
                : "Enter a regular expression pattern"}
            </p>
          </div>

          {/* 错误提示 */}
          {error && <p className="text-sm text-destructive">{error}</p>}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("common.cancel")}
          </Button>
          <Button onClick={handleSave}>{t("common.save")}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
