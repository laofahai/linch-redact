import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import { Download, Loader2, Check, X, Settings, Terminal, Copy } from "lucide-react"
import { listen } from "@tauri-apps/api/event"
import { toast } from "sonner"
import {
  getCurrentPlatform,
  installTesseractOcr,
  type TesseractInstallProgress,
} from "@/lib/tauri/ocr"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Button } from "@/components/ui/button"
import { Switch } from "@/components/ui/switch"
import { Label } from "@/components/ui/label"
import { Progress } from "@/components/ui/progress"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Input } from "@/components/ui/input"
import { useOcrStore } from "@/stores"
import { ocrManifest } from "@/data/ocrManifest"
import type { OcrEngineType, TesseractConfig } from "@/types"

interface DownloadProgress {
  fileName: string
  fileIndex: number
  totalFiles: number
  downloaded: number
  total: number | null
  percent: number
}

export function OcrSetupDialog() {
  const { t } = useTranslation()
  const dialogOpen = useOcrStore((s) => s.dialogOpen)
  const closeDialog = useOcrStore((s) => s.closeDialog)
  const isInstalling = useOcrStore((s) => s.isInstalling)
  const statusMessage = useOcrStore((s) => s.statusMessage)
  const useMirror = useOcrStore((s) => s.useMirror)
  const setUseMirror = useOcrStore((s) => s.setUseMirror)
  const installPaddleModels = useOcrStore((s) => s.installPaddleModels)
  const engineStatus = useOcrStore((s) => s.engineStatus)
  const currentEngine = useOcrStore((s) => s.currentEngine)
  const setCurrentEngine = useOcrStore((s) => s.setCurrentEngine)
  const loadStatus = useOcrStore((s) => s.loadStatus)
  const saveTesseractConfig = useOcrStore((s) => s.saveTesseractConfig)

  const [progress, setProgress] = useState<DownloadProgress | null>(null)
  const [tesseractConfig, setTesseractConfig] = useState<TesseractConfig>({
    lang: "chi_sim+eng",
    psm: 6,
    oem: 1,
  })
  const [platform, setPlatform] = useState<string>("")
  const [tesseractInstalling, setTesseractInstalling] = useState(false)
  const [tesseractProgress, setTesseractProgress] = useState<TesseractInstallProgress | null>(null)
  const [showTesseractAdvanced, setShowTesseractAdvanced] = useState(false)
  const [wslCommand, setWslCommand] = useState<string | null>(null)

  const model = ocrManifest.models.ppocr_v5

  // 加载状态和平台信息
  useEffect(() => {
    if (dialogOpen) {
      loadStatus()
      getCurrentPlatform().then(setPlatform).catch(console.error)
    }
  }, [dialogOpen, loadStatus])

  // 监听 Tesseract 安装进度
  useEffect(() => {
    const unlisten = listen<TesseractInstallProgress>("tesseract-install-progress", (event) => {
      setTesseractProgress(event.payload)
      if (event.payload.done) {
        setTesseractInstalling(false)
        if (event.payload.success) {
          toast.success(event.payload.message)
          loadStatus() // 刷新状态
        } else if (event.payload.error) {
          toast.error(event.payload.error)
        }
      }
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [loadStatus])

  // 监听下载进度事件
  useEffect(() => {
    const unlisten = listen<DownloadProgress>("ocr-download-progress", (event) => {
      setProgress(event.payload)
    })

    return () => {
      unlisten.then((fn) => fn())
    }
  }, [])

  const handleInstallPaddle = async () => {
    setProgress(null)
    try {
      await installPaddleModels()
    } catch (error) {
      console.error("Install failed:", error)
    }
  }

  const handleEngineChange = async (engine: OcrEngineType) => {
    try {
      await setCurrentEngine(engine)
    } catch (error) {
      console.error("Failed to change engine:", error)
    }
  }

  const handleSaveTesseractConfig = async () => {
    try {
      await saveTesseractConfig(tesseractConfig)
    } catch (error) {
      console.error("Failed to save config:", error)
    }
  }

  const handleInstallTesseract = async () => {
    setTesseractInstalling(true)
    setTesseractProgress(null)
    setWslCommand(null)
    try {
      await installTesseractOcr()
    } catch (error: unknown) {
      console.error("Tesseract install failed:", error)
      setTesseractInstalling(false)
      const errorStr = String(error)
      // 检测 WSL 手动安装情况
      if (errorStr.includes("WSL_MANUAL:")) {
        const cmd = errorStr.split("WSL_MANUAL:")[1]
        setWslCommand(cmd)
      } else {
        toast.error(`${t("ocr.installFailed")}: ${error}`)
      }
    }
  }

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text)
    toast.success(t("common.copiedToClipboard"))
  }

  // 计算模型大小
  const totalSize = model
    ? ((model.files.det.size + model.files.rec.size) / 1024 / 1024).toFixed(0)
    : "?"

  // 格式化字节
  const formatBytes = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  }

  const paddleInstalled = engineStatus?.paddle.installed ?? false
  const tesseractInstalled = engineStatus?.tesseract.installed ?? false

  return (
    <Dialog open={dialogOpen} onOpenChange={closeDialog}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>{t("ocr.title")}</DialogTitle>
          <DialogDescription>{t("ocr.subtitle")}</DialogDescription>
        </DialogHeader>

        <Tabs value={currentEngine} onValueChange={(v) => handleEngineChange(v as OcrEngineType)}>
          <TabsList className="grid w-full grid-cols-2">
            <TabsTrigger value="paddle" className="flex items-center gap-2">
              Paddle OCR
              {paddleInstalled ? (
                <Check className="h-3 w-3 text-green-500" />
              ) : (
                <X className="h-3 w-3 text-red-500" />
              )}
            </TabsTrigger>
            <TabsTrigger value="tesseract" className="flex items-center gap-2">
              Tesseract
              {tesseractInstalled ? (
                <Check className="h-3 w-3 text-green-500" />
              ) : (
                <X className="h-3 w-3 text-red-500" />
              )}
            </TabsTrigger>
          </TabsList>

          <TabsContent value="paddle" className="space-y-4 mt-4">
            {paddleInstalled ? (
              <div className="rounded-md bg-green-50 dark:bg-green-900/20 p-3 text-sm">
                <p className="font-medium text-green-700 dark:text-green-400">
                  {t("ocr.paddle.installed")}
                </p>
                <p className="text-muted-foreground mt-1">
                  {t("ocr.version")}: {engineStatus?.paddle.modelVersion || "PP-OCRv5"}
                </p>
              </div>
            ) : (
              <>
                {model && (
                  <div className="rounded-md bg-muted p-3 text-sm space-y-1">
                    <p>
                      <strong>{model.name}</strong>
                    </p>
                    <p className="text-muted-foreground">{model.description}</p>
                    <p className="text-muted-foreground">
                      {t("ocr.paddle.versionSize", { version: model.version, size: totalSize })}
                    </p>
                  </div>
                )}

                <div className="flex items-center justify-between">
                  <div className="space-y-0.5">
                    <Label htmlFor="mirror">{t("ocr.useMirror")}</Label>
                    <p className="text-xs text-muted-foreground">{t("ocr.useMirrorHint")}</p>
                  </div>
                  <Switch
                    id="mirror"
                    checked={useMirror}
                    onCheckedChange={setUseMirror}
                    disabled={isInstalling}
                  />
                </div>

                {/* 下载进度 */}
                {isInstalling && progress && (
                  <div className="space-y-2">
                    <div className="flex justify-between text-sm">
                      <span className="text-muted-foreground">
                        [{progress.fileIndex}/{progress.totalFiles}] {progress.fileName}
                      </span>
                      <span className="text-muted-foreground">
                        {formatBytes(progress.downloaded)}
                        {progress.total && ` / ${formatBytes(progress.total)}`}
                      </span>
                    </div>
                    <Progress value={progress.percent} className="h-2" />
                  </div>
                )}

                <Button className="w-full" onClick={handleInstallPaddle} disabled={isInstalling}>
                  {isInstalling ? (
                    <>
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      {progress
                        ? `${t("ocr.downloadingModel")} (${progress.fileIndex}/${progress.totalFiles}) ${progress.percent.toFixed(0)}%`
                        : `${t("common.loading")}`}
                    </>
                  ) : (
                    <>
                      <Download className="mr-2 h-4 w-4" />
                      {t("common.install")} (~{totalSize} MB)
                    </>
                  )}
                </Button>
              </>
            )}
          </TabsContent>

          <TabsContent value="tesseract" className="space-y-4 mt-4">
            {tesseractInstalled ? (
              <div className="rounded-md bg-green-50 dark:bg-green-900/20 p-3 text-sm">
                <p className="font-medium text-green-700 dark:text-green-400">
                  {t("ocr.tesseract.detected")}
                </p>
                <p className="text-muted-foreground mt-1">
                  {t("ocr.version")}: {engineStatus?.tesseract.version || "unknown"}
                </p>
                {engineStatus?.tesseract.binaryPath && (
                  <p className="text-muted-foreground text-xs mt-1 font-mono">
                    {engineStatus.tesseract.binaryPath}
                  </p>
                )}
                {engineStatus?.tesseract.availableLangs &&
                  engineStatus.tesseract.availableLangs.length > 0 && (
                    <p className="text-muted-foreground text-xs mt-1">
                      {engineStatus.tesseract.availableLangs.slice(0, 5).join(", ")}
                      {engineStatus.tesseract.availableLangs.length > 5 &&
                        ` +${engineStatus.tesseract.availableLangs.length - 5}`}
                    </p>
                  )}
              </div>
            ) : (
              <div className="space-y-3">
                <div className="rounded-md bg-yellow-50 dark:bg-yellow-900/20 p-3 text-sm">
                  <p className="text-yellow-700 dark:text-yellow-400">
                    {t("ocr.tesseract.notInstalled")}
                  </p>
                </div>

                {/* 安装进度 */}
                {tesseractInstalling && tesseractProgress && (
                  <div className="rounded-md bg-blue-50 dark:bg-blue-900/20 p-3 text-sm">
                    <div className="flex items-center gap-2">
                      <Loader2 className="h-4 w-4 animate-spin text-blue-500" />
                      <span className="text-blue-700 dark:text-blue-400">
                        {tesseractProgress.message}
                      </span>
                    </div>
                  </div>
                )}

                {/* WSL 手动安装命令 */}
                {wslCommand ? (
                  <div className="space-y-2">
                    <p className="text-xs text-muted-foreground">{t("ocr.wslManualInstall")}</p>
                    <div className="flex items-center gap-2">
                      <code className="flex-1 px-2 py-1.5 text-xs bg-muted rounded font-mono overflow-x-auto">
                        {wslCommand}
                      </code>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => copyToClipboard(wslCommand)}
                      >
                        <Copy className="h-3.5 w-3.5" />
                      </Button>
                    </div>
                    <p className="text-xs text-muted-foreground">{t("ocr.wslRefreshHint")}</p>
                    <Button
                      variant="outline"
                      className="w-full"
                      onClick={() => {
                        setWslCommand(null)
                        loadStatus()
                      }}
                    >
                      {t("common.refresh")}
                    </Button>
                  </div>
                ) : (
                  <Button
                    className="w-full"
                    onClick={handleInstallTesseract}
                    disabled={tesseractInstalling || !platform}
                  >
                    {tesseractInstalling ? (
                      <>
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                        {t("common.processing")}
                      </>
                    ) : (
                      <>
                        <Terminal className="mr-2 h-4 w-4" />
                        {t("ocr.tesseract.oneClickInstall")}
                      </>
                    )}
                  </Button>
                )}
              </div>
            )}

            {/* 手动配置按钮 */}
            {tesseractInstalled && !showTesseractAdvanced && (
              <Button
                variant="ghost"
                size="sm"
                className="w-full text-muted-foreground"
                onClick={() => setShowTesseractAdvanced(true)}
              >
                <Settings className="mr-2 h-3.5 w-3.5" />
                {t("settings.title", { defaultValue: "Settings" })}
              </Button>
            )}

            {/* 配置区域 - 点击手动配置后显示 */}
            {tesseractInstalled && showTesseractAdvanced && (
              <div className="space-y-3 border-t pt-3">
                <div className="space-y-1.5">
                  <Label htmlFor="tess-binary">{t("ocr.tesseract.binaryPath")}</Label>
                  <Input
                    id="tess-binary"
                    placeholder={t("ocr.tesseract.binaryPathPlaceholder")}
                    value={tesseractConfig.binaryPath || ""}
                    onChange={(e) =>
                      setTesseractConfig({
                        ...tesseractConfig,
                        binaryPath: e.target.value || undefined,
                      })
                    }
                  />
                </div>

                <div className="space-y-1.5">
                  <Label htmlFor="tess-data">{t("ocr.tesseract.tessdataPath")}</Label>
                  <Input
                    id="tess-data"
                    placeholder={t("ocr.tesseract.tessdataPathPlaceholder")}
                    value={tesseractConfig.tessdataPath || ""}
                    onChange={(e) =>
                      setTesseractConfig({
                        ...tesseractConfig,
                        tessdataPath: e.target.value || undefined,
                      })
                    }
                  />
                </div>

                <div className="space-y-1.5">
                  <Label>{t("ocr.tesseract.language")}</Label>
                  <div className="flex flex-wrap gap-1.5">
                    {(engineStatus?.tesseract.availableLangs || []).slice(0, 20).map((lang) => {
                      const selectedLangs = (tesseractConfig.lang || "").split("+").filter(Boolean)
                      const isSelected = selectedLangs.includes(lang)
                      return (
                        <button
                          key={lang}
                          type="button"
                          onClick={() => {
                            const newLangs = isSelected
                              ? selectedLangs.filter((l) => l !== lang)
                              : [...selectedLangs, lang]
                            setTesseractConfig({
                              ...tesseractConfig,
                              lang: newLangs.join("+") || undefined,
                            })
                          }}
                          className={`px-2 py-0.5 text-xs rounded-full border transition-colors ${
                            isSelected
                              ? "bg-primary text-primary-foreground border-primary"
                              : "bg-muted/50 text-muted-foreground border-transparent hover:border-muted-foreground/30"
                          }`}
                        >
                          {lang}
                        </button>
                      )
                    })}
                  </div>
                  {(tesseractConfig.lang || "").split("+").filter(Boolean).length > 0 && (
                    <p className="text-xs text-muted-foreground mt-1">
                      {t("ocr.tesseract.selectedLangs", { langs: tesseractConfig.lang })}
                    </p>
                  )}
                </div>

                <div className="flex gap-2">
                  <Button className="flex-1" variant="outline" onClick={handleSaveTesseractConfig}>
                    {t("common.save")}
                  </Button>
                  <Button variant="ghost" onClick={() => setShowTesseractAdvanced(false)}>
                    {t("common.close")}
                  </Button>
                </div>
              </div>
            )}
          </TabsContent>
        </Tabs>

        {statusMessage && !isInstalling && (
          <div className="rounded-md bg-destructive/10 p-2 text-sm text-destructive">
            {t(statusMessage)}
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}
