# Linch · 文档脱敏器

本地桌面工具，用于生成不可恢复的脱敏 PDF 副本，并保留处理记录。

## 启动

安装依赖：

```
npm install
```

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
