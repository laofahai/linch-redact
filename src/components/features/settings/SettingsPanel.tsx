import { Checkbox } from "@/components/ui/checkbox"
import { Label } from "@/components/ui/label"
import { Separator } from "@/components/ui/separator"
import { useSettingsStore } from "@/stores"
import { ModeSelector } from "./ModeSelector"

export function SettingsPanel() {
  const settings = useSettingsStore((s) => s.settings)
  const toggleCleaning = useSettingsStore((s) => s.toggleCleaning)
  const toggleVerification = useSettingsStore((s) => s.toggleVerification)

  const cleaningOptions = [
    { key: "documentInfo" as const, label: "文档信息" },
    { key: "xmpMetadata" as const, label: "XMP 元数据" },
    { key: "hiddenData" as const, label: "隐藏数据" },
    { key: "annotations" as const, label: "批注内容" },
    { key: "forms" as const, label: "表单字段" },
    { key: "attachments" as const, label: "附件文件" },
    { key: "javascript" as const, label: "脚本代码" },
  ]

  const verificationOptions = [
    { key: "textRecheck" as const, label: "文本复查" },
    { key: "imageSampling" as const, label: "图像抽检" },
    { key: "outputReport" as const, label: "输出报告" },
  ]

  return (
    <div className="space-y-4">
      {/* 脱敏模式选择 */}
      <ModeSelector />

      <Separator />
      {/* 清理选项 */}
      <div className="space-y-2">
        <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
          清理内容
        </h3>
        <div className="space-y-2">
          {cleaningOptions.map((option) => (
            <div key={option.key} className="flex items-center gap-2">
              <Checkbox
                id={option.key}
                checked={settings.cleaning[option.key]}
                onCheckedChange={() => toggleCleaning(option.key)}
              />
              <Label
                htmlFor={option.key}
                className="text-sm font-normal cursor-pointer"
              >
                {option.label}
              </Label>
            </div>
          ))}
        </div>
      </div>

      {/* 验证选项 */}
      <div className="space-y-2">
        <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
          检查与输出
        </h3>
        <div className="space-y-2">
          {verificationOptions.map((option) => (
            <div key={option.key} className="flex items-center gap-2">
              <Checkbox
                id={option.key}
                checked={settings.verification[option.key]}
                onCheckedChange={() => toggleVerification(option.key)}
              />
              <Label
                htmlFor={option.key}
                className="text-sm font-normal cursor-pointer"
              >
                {option.label}
              </Label>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
