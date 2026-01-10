#!/bin/bash
# 下载 pdfium 库用于本地开发和 CI/CD 构建
#
# 用法: ./scripts/setup-pdfium.sh
#
# 支持的平台:
#   - Linux x64
#   - Linux arm64
#   - macOS x64
#   - macOS arm64
#   - Windows x64

set -e

# 检查目标目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
LIBS_DIR="$PROJECT_ROOT/src-tauri/libs"

# 检测平台和架构
detect_platform() {
    local os=$(uname -s | tr '[:upper:]' '[:lower:]')
    local arch=$(uname -m)

    case "$os" in
        linux)
            case "$arch" in
                x86_64) echo "linux-x64" ;;
                aarch64) echo "linux-arm64" ;;
                *) echo "unsupported" ;;
            esac
            ;;
        darwin)
            case "$arch" in
                x86_64) echo "mac-x64" ;;
                arm64) echo "mac-arm64" ;;
                *) echo "unsupported" ;;
            esac
            ;;
        mingw*|msys*|cygwin*)
            echo "win-x64"
            ;;
        *)
            echo "unsupported"
            ;;
    esac
}

PLATFORM=$(detect_platform)

# 根据平台确定库文件名
get_library_name() {
    case "$PLATFORM" in
        linux-*) echo "libpdfium.so" ;;
        mac-*) echo "libpdfium.dylib" ;;
        win-*) echo "pdfium.dll" ;;
        *) echo "" ;;
    esac
}

LIBRARY_NAME=$(get_library_name)

# 只检查当前平台对应的库文件是否存在
if [ -n "$LIBRARY_NAME" ] && [ -f "$LIBS_DIR/$LIBRARY_NAME" ]; then
    echo "Pdfium ($LIBRARY_NAME) already installed at $LIBS_DIR"
    exit 0
fi

# 使用 bblanchon/pdfium-binaries 的最新版本
PDFIUM_BASE_URL="https://github.com/bblanchon/pdfium-binaries/releases/latest/download"

if [ "$PLATFORM" = "unsupported" ]; then
    echo "Error: Unsupported platform"
    exit 1
fi

echo "Detected platform: $PLATFORM"

# 设置下载 URL
DOWNLOAD_URL="${PDFIUM_BASE_URL}/pdfium-${PLATFORM}.tgz"
echo "Download URL: $DOWNLOAD_URL"

# 创建目标目录
mkdir -p "$LIBS_DIR"

# 下载并解压
TEMP_FILE=$(mktemp)
echo "Downloading pdfium..."
curl -L -o "$TEMP_FILE" "$DOWNLOAD_URL"

echo "Extracting to $LIBS_DIR..."
tar xzf "$TEMP_FILE" -C "$LIBS_DIR" --strip-components=1

# 清理
rm -f "$TEMP_FILE"

# 显示结果
echo ""
echo "Pdfium installed successfully!"
echo "Library location: $LIBS_DIR"
ls -la "$LIBS_DIR"
