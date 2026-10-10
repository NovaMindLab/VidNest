#!/usr/bin/env bash
# ==============================================================================
# Local Video Intelligence (VidNest) - macOS 一键解锁与启动助手
# ==============================================================================
# 此脚本用于解除 macOS Sequoia / Sonoma 对未签名或 Ad-hoc 签名软件的 Gatekeeper 拦截
# 解决 "已损坏，无法打开 / 无法验证开发者" 的误报问题。
# ==============================================================================

clear
cat <<'BANNER'
================================================================================
  🚀 Local Video Intelligence (VidNest) - macOS 安全隔离解除工具
================================================================================
BANNER

echo "正在定位应用程序..."

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_CANDIDATES=(
  "/Applications/Local Video Intelligence.app"
  "/Applications/VidNest.app"
  "${HOME}/Applications/Local Video Intelligence.app"
  "${HOME}/Applications/VidNest.app"
  "${DIR}/Local Video Intelligence.app"
  "${DIR}/VidNest.app"
  "${DIR}/../Local Video Intelligence.app"
  "${DIR}/../VidNest.app"
)

TARGET_APP=""
for path in "${APP_CANDIDATES[@]}"; do
  if [ -d "$path" ]; then
    TARGET_APP="$path"
    break
  fi
done

if [ -z "$TARGET_APP" ]; then
  # 尝试 Spotlight 搜索
  echo "正在全局检索已安装的应用..."
  SEARCH_RESULT=$(mdfind "kMDItemCFBundleIdentifier == 'com.vidnest.localvideointelligence'" 2>/dev/null | head -n 1 || true)
  if [ -n "$SEARCH_RESULT" ] && [ -d "$SEARCH_RESULT" ]; then
    TARGET_APP="$SEARCH_RESULT"
  else
    TARGET_APP="/Applications/Local Video Intelligence.app"
  fi
fi

echo "📍 目标应用路径: $TARGET_APP"

if [ ! -d "$TARGET_APP" ]; then
  echo ""
  echo "⚠️  未在系统 /Applications 目录下找到该应用！"
  echo "👉 请先将 DMG 镜像窗口中的应用图标拖动复制到「应用程序 (Applications)」文件夹中，"
  echo "   然后再次双击运行本脚本。"
  echo ""
  read -n 1 -s -r -p "按任意键退出..."
  echo ""
  exit 1
fi

echo ""
echo "🛡️  正在解除 macOS Gatekeeper 安全隔离标记 (com.apple.quarantine)..."

# 1. 尝试当前权限移除隔离标记
QUARANTINE_ERROR=false
if ! xattr -rd com.apple.quarantine "$TARGET_APP" 2>/dev/null; then
  QUARANTINE_ERROR=true
fi

# 2. 如果权限不足，请求管理员 sudo 权限
if [ "$QUARANTINE_ERROR" = true ]; then
  echo "🔒 检测到需要管理员权限以修改 /Applications 目录下的扩展属性，请输入电脑开机密码："
  if sudo xattr -rd com.apple.quarantine "$TARGET_APP"; then
    QUARANTINE_ERROR=false
  fi
fi

# 3. 补齐本地 Ad-hoc 签名重签（防止二进制被篡改校验阻断）
if command -v codesign >/dev/null 2>&1; then
  codesign --force --deep --sign - "$TARGET_APP" 2>/dev/null || true
fi

echo ""
if [ "$QUARANTINE_ERROR" = false ]; then
  echo "================================================================================"
  echo "✅ 解锁成功！安全隔离属性已完全清除。"
  echo "================================================================================"
  echo ""
  read -p "是否立即启动应用？[Y/n]: " -n 1 -r REPLY || true
  echo ""
  if [[ -z "$REPLY" || "$REPLY" =~ ^[Yy]$ ]]; then
    echo "🚀 正在启动 $TARGET_APP ..."
    open "$TARGET_APP"
  else
    echo "💡 您现在可以直接从「启动台 (Launchpad)」或「访达 ➔ 应用程序」双击运行应用！"
  fi
else
  echo "❌ 解除隔离失败，请在终端手动执行以下命令："
  echo "   sudo xattr -rd com.apple.quarantine \"$TARGET_APP\""
fi

echo ""
read -n 1 -s -r -p "按任意键关闭此窗口..."
echo ""
exit 0
