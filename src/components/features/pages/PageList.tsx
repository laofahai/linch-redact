import { useTranslation } from "react-i18next"
import { Trash2 } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip"
import { useFileStore, useEditorStore } from "@/stores"
import { cn } from "@/lib/utils"

interface PageListProps {
  hideTitle?: boolean
}

export function PageList({ hideTitle = false }: PageListProps) {
  const { t } = useTranslation()
  const files = useFileStore((s) => s.files)
  const selectedFileId = useFileStore((s) => s.selectedFileId)
  const setPageAction = useFileStore((s) => s.setPageAction)
  const currentPage = useEditorStore((s) => s.currentPage)
  const setCurrentPage = useEditorStore((s) => s.setCurrentPage)
  const getMasksByPage = useEditorStore((s) => s.getMasksByPage)

  const selectedFile = files.find((f) => f.id === selectedFileId) ?? null
  const masksByPage = selectedFileId ? getMasksByPage(selectedFileId) : {}

  if (!selectedFile) {
    return null
  }

  const pages = selectedFile.pages

  const handleDelete = (pageIndex: number) => {
    const page = pages[pageIndex]
    if (!page) return

    // 计算非删除页面数量
    const nonDeletedCount = pages.filter((p) => p.action !== "delete").length

    // 如果当前页面已删除，则恢复；否则删除
    if (page.action === "delete") {
      setPageAction(selectedFile.id, pageIndex, "keep")
      toast.success(t("sidebar.pageRestored"))
    } else {
      if (nonDeletedCount <= 1) {
        toast.warning(t("sidebar.cannotDeleteLastPage"), {
          description: t("sidebar.cannotDeleteLastPageDesc"),
        })
        return
      }
      setPageAction(selectedFile.id, pageIndex, "delete")
      toast.success(t("sidebar.pageMarkedDelete"))
    }
  }

  return (
    <TooltipProvider delayDuration={300}>
      <div className="space-y-2">
        {!hideTitle && (
          <div className="flex items-center justify-between">
            <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {t("sidebar.pageList")}
            </h3>
            <span className="text-xs text-muted-foreground">
              {t("sidebar.pageCount", { count: pages.length })}
            </span>
          </div>
        )}

        {pages.length === 0 ? (
          <p className="text-xs text-muted-foreground">{t("sidebar.loadingPages")}</p>
        ) : (
          <div className="space-y-0.5">
            {pages.map((page) => {
              const maskCount = masksByPage[page.index]?.length ?? 0
              const isDeleted = page.action === "delete"
              const isSelected = currentPage === page.index

              return (
                <div
                  key={page.index}
                  className={cn(
                    "flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors",
                    isSelected
                      ? "bg-primary/15 text-foreground ring-1 ring-primary/30 ring-inset"
                      : "hover:bg-muted",
                    isDeleted && "opacity-50"
                  )}
                  onClick={() => setCurrentPage(page.index)}
                >
                  <span className={cn("flex-1", isDeleted && "line-through text-muted-foreground")}>
                    {t("sidebar.pageN", { n: page.index + 1 })}
                  </span>

                  {/* 脱敏区域数量 */}
                  {maskCount > 0 && !isDeleted && (
                    <span className="text-xs text-primary bg-primary/10 px-1.5 py-0.5 rounded">
                      {t("sidebar.maskCount", { count: maskCount })}
                    </span>
                  )}

                  {/* 删除按钮 */}
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon"
                        className={cn(
                          "h-6 w-6 shrink-0",
                          isDeleted && "bg-destructive/20 text-destructive"
                        )}
                        onClick={(e) => {
                          e.stopPropagation()
                          handleDelete(page.index)
                        }}
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </Button>
                    </TooltipTrigger>
                    <TooltipContent side="top">
                      <p>{isDeleted ? t("sidebar.restorePage") : t("sidebar.deletePage")}</p>
                    </TooltipContent>
                  </Tooltip>
                </div>
              )
            })}
          </div>
        )}
      </div>
    </TooltipProvider>
  )
}
