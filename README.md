# Linch · 文档脱敏器

本地桌面工具，用于生成不可恢复的脱敏 PDF 副本，并保留处理记录。

## 启动

安装依赖：

```
npm install
```

下载 pdfium 库（用于 PDF 渲染和文本提取）：

```bash
# Linux / macOS / Windows (Git Bash)
./scripts/setup-pdfium.sh
```

> **Windows 用户注意**：需要在 Git Bash 中运行此脚本，不要用 PowerShell 或 CMD。如果脚本执行后仍然报错 `LoadLibraryExW error 126`，请检查 `src-tauri/libs/pdfium.dll` 是否存在，不存在则重新运行脚本。

启动桌面版（Tauri）：

```
npm run tauri:dev
```

只看界面（不启 Tauri）：

```
npm run dev
```

## 目录结构

- `src/` 前端界面
- `src-tauri/` 桌面后端
- `crates/` Rust 核心模块
- `docs/` 方案与说明
