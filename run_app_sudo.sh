#!/bin/bash

# iTunnel Root 运行脚本
# 用于解决 TUN 设备创建权限问题

APP_PATH="./target/release/bundle/macos/iTunnel.app/Contents/MacOS/iTunnel"

if [ ! -f "$APP_PATH" ]; then
    echo "❌ 未找到可执行文件: $APP_PATH"
    echo "请先运行: cargo tauri build"
    exit 1
fi

echo "🚀 以 Root 权限启动 iTunnel..."
echo "请在弹出的提示中输入密码"
echo ""

sudo "$APP_PATH"
