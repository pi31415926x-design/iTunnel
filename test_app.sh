#!/bin/bash

# iTunnel.app 测试脚本

echo "🧪 iTunnel.app 测试"
echo "=================="
echo ""

APP_PATH="./target/release/bundle/macos/iTunnel.app"

if [ ! -d "$APP_PATH" ]; then
    echo "❌ 未找到 iTunnel.app"
    echo "请先运行: cargo tauri build"
    exit 1
fi

echo "✅ 找到应用: $APP_PATH"
echo ""

# 检查资源文件
echo "📁 检查打包的资源文件:"
echo ""
ls -lh "$APP_PATH/Contents/Resources/"
echo ""

# 检查 frontend/dist 内容
if [ -f "$APP_PATH/Contents/Resources/frontend/dist/index.html" ]; then
    echo "✅ frontend/dist/index.html 已打包"
else
    echo "❌ frontend/dist/index.html 未找到"
fi

if [ -d "$APP_PATH/Contents/Resources/frontend/dist/assets" ]; then
    echo "✅ frontend/dist/assets 目录已打包"
    echo "   文件数量: $(ls -1 $APP_PATH/Contents/Resources/frontend/dist/assets | wc -l)"
else
    echo "❌ frontend/dist/assets 目录未找到"
fi

echo ""
echo "🚀 启动应用测试..."
echo ""
echo "请注意:"
echo "1. 应用会在系统托盘显示图标"
echo "2. 查看 Console.app 中的日志输出"
echo "3. 检查是否有以下日志:"
echo "   - 🚀 iTunnel 启动中..."
echo "   - 📁 静态文件路径: ..."
echo "   - 🌐 正在 127.0.0.1:8181 启动 Web 服务..."
echo "   - ✅ Actix 服务已启动"
echo ""
echo "按 Ctrl+C 停止测试"
echo ""

# 启动应用
sudo open "$APP_PATH"

# 等待几秒
sleep 3

# 检查端口
echo "🔍 检查端口 8181..."
if lsof -Pi :8181 -sTCP:LISTEN -t >/dev/null ; then
    echo "✅ 端口 8181 正在监听"
    echo ""
    echo "🌐 测试 Web 服务..."
    curl -s http://127.0.0.1:8181 | head -5
    echo ""
    echo ""
    echo "✅ 测试完成！"
    echo ""
    echo "💡 提示:"
    echo "   - 访问: http://127.0.0.1:8181"
    echo "   - 点击托盘图标 → 配置"
    echo "   - 查看日志: tail -f /tmp/itunnel.log"
else
    echo "❌ 端口 8181 未监听"
    echo ""
    echo "💡 故障排查:"
    echo "   1. 查看 Console.app 中的日志"
    echo "   2. 检查是否有权限问题"
    echo "   3. 确认端口未被占用"
fi
