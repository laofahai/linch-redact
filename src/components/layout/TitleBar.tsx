import { useTranslation } from "react-i18next"
import {
  WindowControls,
  LanguageSwitcher,
  ThemeSwitcher,
  startDragging,
  toggleMaximize,
} from "@linch-tech/desktop-core"
import { Logo } from "@/components/shared/Logo"

export function TitleBar() {
  const { t } = useTranslation()
  const handleMouseDown = async (e: React.MouseEvent) => {
    // 只响应左键，且不是双击
    if (e.button === 0 && e.detail === 1) {
      try {
        await startDragging()
      } catch (e) {
        console.error("Failed to start dragging:", e)
      }
    }
  }

  const handleDoubleClick = async () => {
    try {
      await toggleMaximize()
    } catch (e) {
      console.error("Failed to toggle maximize:", e)
    }
  }

  return (
    <header
      className="flex h-10 shrink-0 items-center justify-between border-b bg-card px-3 select-none"
      onMouseDown={handleMouseDown}
      onDoubleClick={handleDoubleClick}
    >
      {/* 左侧 Logo 和标题 */}
      <div className="flex items-center gap-2 pointer-events-none">
        <Logo className="h-5 w-5" />
        <span className="text-sm font-medium">{t("app.name")}</span>
      </div>

      {/* 右侧：语言切换 + 主题切换 + 窗口控制按钮 */}
      <div className="flex items-center gap-2" onMouseDown={(e) => e.stopPropagation()}>
        <LanguageSwitcher />
        <ThemeSwitcher />
        <WindowControls />
      </div>
    </header>
  )
}
