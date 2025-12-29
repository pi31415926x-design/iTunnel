#!/bin/bash

# iTunnel Tauri 构建脚本
# 由于 Node.js 版本问题，我们手动处理构建流程

set -e

echo "🔨 iTunnel Tauri 构建脚本"
echo "========================="
echo ""

# 检查前端构建目录
if [ ! -d "frontend/dist" ]; then
    echo "⚠️  前端未构建，创建占位目录..."
    mkdir -p frontend/dist
    echo '<!DOCTYPE html><html><head><meta charset="utf-8"><title>iTunnel</title></head><body><h1>iTunnel</h1><p>请通过系统托盘菜单访问配置页面</p></body></html>' > frontend/dist/index.html
fi

echo "📦 开始构建 Tauri 应用..."
echo ""

# 设置环境变量，跳过前端构建命令
export TAURI_SKIP_DEVSERVER_CHECK=true

# 构建
cargo tauri build --no-bundle

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ 构建成功！"
    echo ""
    echo "📍 可执行文件位置:"
    echo "   ./target/release/app"
    echo ""
    echo "🚀 运行应用:"
    echo "   sudo ./target/release/app"
    echo ""
    echo "💡 或使用后台运行脚本:"
    echo "   ./run_background.sh"
else
    echo ""
    echo "❌ 构建失败"
    exit 1
fi
