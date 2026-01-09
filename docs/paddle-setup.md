# PaddleOCR 引擎与模型向导方案

## 目标
- 默认安装包不内置 OCR 引擎与模型，降低体积。
- 首次使用 OCR 时提供一键向导：在线下载或离线导入。
- 全流程可审计：记录引擎/模型版本与哈希。

## 分发策略
- 平台包：`paddleocr-{os}-{arch}.zip`
- 模型包：`models-{version}.zip`
- Manifest 清单：`manifest.json`

Manifest 示例：
```
{
  "version": "1.0",
  "engine": {
    "win-x64": {"url": "https://.../paddleocr-win-x64.zip", "sha256": "..."},
    "mac-arm64": {"url": "https://.../paddleocr-mac-arm64.zip", "sha256": "..."},
    "linux-x64": {"url": "https://.../paddleocr-linux-x64.zip", "sha256": "..."}
  },
  "models": {
    "default": {"url": "https://.../models-ppocr.zip", "sha256": "..."}
  }
}
```

## 向导流程
1) 启动检查：
   - 判断 `ocr_engine_path` 与 `ocr_model_path` 是否存在且可执行/可读。
2) 未安装：显示向导入口与状态提示。
3) 选择方式：
   - 在线下载：根据平台识别选择引擎包 + 模型包。
   - 离线导入：选择本地 zip 或目录。
4) 下载/解压：
   - 校验 SHA256。
   - 解压至应用数据目录：`<app_data>/linch-redact/ocr/`。
5) 完成：更新配置并进入可用状态。

## 跨平台注意事项
- Windows: 引擎可执行 `.exe`，解压后需记录绝对路径。
- macOS: 若需签名/公证，后续可追加；当前仅本地执行。
- Linux: 解压后确保可执行位；必要时提示用户修复权限。

## 配置与审计
- 配置写入：
  - `ocr_engine_path`, `ocr_model_path`, `engine_version`, `model_version`
- 审计记录：
  - 引擎版本/模型版本 + hash + 获取方式（online/offline）
