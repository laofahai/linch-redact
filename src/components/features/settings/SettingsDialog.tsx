import { useState } from "react"
import { useTranslation } from "react-i18next"
import {
  Settings,
  Info,
  Languages,
  Palette,
  RefreshCw,
  Download,
  CheckCircle2,
  AlertCircle,
  ScanText,
  ShieldAlert,
} from "lucide-react"
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Logo } from "@/components/shared/Logo"
import { ThemeSwitcher, LanguageSwitcher, useUpdater, useConfig } from "@linch-tech/desktop-core"
import { useOcrStore } from "@/stores/useOcrStore"
import { cn } from "@/lib/utils"
import { DetectionRulesSettings } from "@/components/features/settings/DetectionRulesSettings"

type SettingsTab = "general" | "ocr" | "detection" | "about"

interface SettingsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function SettingsDialog({ open, onOpenChange }: SettingsDialogProps) {
  const { t } = useTranslation()
  const config = useConfig()
  const [activeTab, setActiveTab] = useState<SettingsTab>("general")
  const { status, updateInfo, progress, error, check, download, install } = useUpdater({
    enabled: config.features?.updater !== false,
  })
  const engineStatus = useOcrStore((s) => s.engineStatus)
  const currentEngine = useOcrStore((s) => s.currentEngine)
  const setCurrentEngine = useOcrStore((s) => s.setCurrentEngine)
  const openOcrDialog = useOcrStore((s) => s.openDialog)
  const loadStatus = useOcrStore((s) => s.loadStatus)
  const isLoading = useOcrStore((s) => s.isLoading)

  const handleCheckUpdate = async () => {
    try {
      await check()
    } catch (err) {
      console.error("Update check failed", err)
    }
  }

  const handleDownload = async () => {
    try {
      await download()
    } catch (err) {
      console.error("Download failed", err)
    }
  }

  const renderUpdateButton = () => {
    switch (status) {
      case "checking":
        return (
          <Button disabled className="w-56">
            <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
            {t("settings.about.checking")}
          </Button>
        )

      case "available":
        return (
          <div className="space-y-2 text-center">
            <div className="text-sm text-primary font-medium">
              {t("settings.about.new_version")}: {updateInfo?.version}
            </div>
            <Button onClick={handleDownload} className="w-56">
              <Download className="mr-2 h-4 w-4" />
              {t("settings.about.download_update")}
            </Button>
          </div>
        )

      case "downloading":
        return (
          <div className="space-y-2 w-56">
            <Button disabled className="w-full">
              <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
              {progress?.percent || 0}%
            </Button>
            <div className="h-1.5 bg-muted rounded-full overflow-hidden">
              <div
                className="h-full bg-primary transition-all duration-300"
                style={{ width: `${progress?.percent || 0}%` }}
              />
            </div>
          </div>
        )

      case "ready":
        return (
          <div className="space-y-2 text-center">
            <div className="flex items-center justify-center gap-2 text-sm text-green-600">
              <CheckCircle2 className="h-4 w-4" />
              {t("settings.about.ready_to_install")}
            </div>
            <Button onClick={install} className="w-56">
              {t("settings.about.restart_now")}
            </Button>
          </div>
        )

      case "up-to-date":
        return (
          <div className="space-y-2 text-center">
            <div className="flex items-center justify-center gap-2 text-sm text-muted-foreground">
              <CheckCircle2 className="h-4 w-4 text-green-600" />
              {t("settings.about.up_to_date")}
            </div>
            <Button variant="outline" onClick={handleCheckUpdate} className="w-56">
              {t("settings.about.check_updates")}
            </Button>
          </div>
        )

      case "check-error":
      case "download-error":
        return (
          <div className="space-y-2 text-center">
            <div className="flex items-center justify-center gap-2 text-sm text-destructive">
              <AlertCircle className="h-4 w-4" />
              {error?.message || t("settings.about.check_error")}
            </div>
            <Button
              variant="outline"
              onClick={status === "check-error" ? handleCheckUpdate : handleDownload}
              className="w-56"
            >
              {t("settings.about.retry")}
            </Button>
          </div>
        )

      default:
        return (
          <Button onClick={handleCheckUpdate} className="w-56">
            {t("settings.about.check_updates")}
          </Button>
        )
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl p-0 gap-0 overflow-hidden">
        <DialogHeader className="sr-only">
          <DialogTitle>{t("settings.title")}</DialogTitle>
        </DialogHeader>

        <div className="flex h-[480px]">
          {/* Tab 侧边栏 */}
          <aside className="border-r bg-muted/20 p-2 space-y-1 w-40 shrink-0">
            <Button
              variant="ghost"
              className={cn(
                "w-full justify-start gap-2",
                activeTab === "general"
                  ? "bg-primary/5 text-primary font-semibold"
                  : "text-muted-foreground"
              )}
              onClick={() => setActiveTab("general")}
            >
              <Settings className="h-4 w-4" />
              {t("settings.tabs.general")}
            </Button>
            <Button
              variant="ghost"
              className={cn(
                "w-full justify-start gap-2",
                activeTab === "ocr"
                  ? "bg-primary/5 text-primary font-semibold"
                  : "text-muted-foreground"
              )}
              onClick={() => setActiveTab("ocr")}
            >
              <ScanText className="h-4 w-4" />
              OCR 引擎
            </Button>
            <Button
              variant="ghost"
              className={cn(
                "w-full justify-start gap-2",
                activeTab === "detection"
                  ? "bg-primary/5 text-primary font-semibold"
                  : "text-muted-foreground"
              )}
              onClick={() => setActiveTab("detection")}
            >
              <ShieldAlert className="h-4 w-4" />
              敏感检测
            </Button>
            <Button
              variant="ghost"
              className={cn(
                "w-full justify-start gap-2",
                activeTab === "about"
                  ? "bg-primary/5 text-primary font-semibold"
                  : "text-muted-foreground"
              )}
              onClick={() => setActiveTab("about")}
            >
              <Info className="h-4 w-4" />
              {t("settings.tabs.about")}
            </Button>
          </aside>

          {/* 内容区 */}
          <div className="flex-1 p-6 overflow-y-auto">
            {activeTab === "general" && (
              <div className="space-y-6">
                <div className="space-y-3">
                  <h3 className="text-sm font-medium flex items-center gap-2">
                    <Languages className="h-4 w-4" />
                    {t("settings.language_select")}
                  </h3>
                  <div className="rounded-lg border p-4 bg-card">
                    <LanguageSwitcher variant="full" size="sm" />
                  </div>
                </div>

                <div className="space-y-3">
                  <h3 className="text-sm font-medium flex items-center gap-2">
                    <Palette className="h-4 w-4" />
                    {t("settings.theme_select")}
                  </h3>
                  <div className="rounded-lg border p-4 bg-card">
                    <ThemeSwitcher variant="full" size="sm" />
                  </div>
                </div>
              </div>
            )}

            {activeTab === "ocr" && (
              <div className="space-y-6">
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <h3 className="text-sm font-medium">选择 OCR 引擎</h3>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => loadStatus()}
                      disabled={isLoading}
                    >
                      <RefreshCw className={cn("h-4 w-4", isLoading && "animate-spin")} />
                    </Button>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    OCR 引擎用于识别图片格式 PDF 中的文字
                  </p>
                </div>

                <div className="space-y-3">
                  {/* Tesseract */}
                  <button
                    onClick={() => setCurrentEngine("tesseract")}
                    className={cn(
                      "w-full rounded-lg border p-4 text-left transition-all",
                      currentEngine === "tesseract"
                        ? "border-primary bg-primary/5"
                        : "hover:bg-muted/50"
                    )}
                  >
                    <div className="flex items-center justify-between">
                      <div className="space-y-1">
                        <div className="font-medium">Tesseract</div>
                        <div className="text-xs text-muted-foreground">
                          开源 OCR 引擎，支持多语言
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        {engineStatus?.tesseract.installed ? (
                          <span className="text-xs text-green-600 flex items-center gap-1">
                            <CheckCircle2 className="h-3 w-3" />
                            已安装
                          </span>
                        ) : (
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={(e) => {
                              e.stopPropagation()
                              openOcrDialog()
                            }}
                          >
                            安装
                          </Button>
                        )}
                      </div>
                    </div>
                  </button>

                  {/* Paddle OCR */}
                  <button
                    onClick={() => setCurrentEngine("paddle")}
                    className={cn(
                      "w-full rounded-lg border p-4 text-left transition-all",
                      currentEngine === "paddle"
                        ? "border-primary bg-primary/5"
                        : "hover:bg-muted/50"
                    )}
                  >
                    <div className="flex items-center justify-between">
                      <div className="space-y-1">
                        <div className="font-medium">Paddle OCR</div>
                        <div className="text-xs text-muted-foreground">
                          百度飞桨 OCR，中文识别更准确
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        {engineStatus?.paddle.installed ? (
                          <span className="text-xs text-green-600 flex items-center gap-1">
                            <CheckCircle2 className="h-3 w-3" />
                            已安装
                          </span>
                        ) : (
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={(e) => {
                              e.stopPropagation()
                              openOcrDialog()
                            }}
                          >
                            安装
                          </Button>
                        )}
                      </div>
                    </div>
                  </button>
                </div>
              </div>
            )}

            {activeTab === "detection" && <DetectionRulesSettings />}

            {activeTab === "about" && (
              <div className="flex flex-col items-center justify-center space-y-6 pt-6">
                <Logo className="h-20 w-20 text-primary" />
                <div className="text-center space-y-1">
                  <h2 className="text-2xl font-bold">{t(config.brand.name)}</h2>
                  <p className="text-muted-foreground">
                    {t("settings.about.current_version")}:{" "}
                    <span className="font-mono text-foreground">{config.brand.version}</span>
                  </p>
                </div>

                {renderUpdateButton()}

                <div className="text-xs text-muted-foreground pt-4 text-center max-w-sm">
                  <p>{t("settings.about.footer_line1")}</p>
                  <p>{t("settings.about.footer_line2")}</p>
                </div>
              </div>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
