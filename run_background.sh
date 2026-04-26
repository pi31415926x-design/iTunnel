#!/bin/bash

# iTunnel 快速启动脚本 - 脱离 Terminal 运行
# 使用方法: 在仓库根目录执行 ./run_background.sh

cd "$(dirname "$0")" || exit 1

# 与 .env 中 ListenPort 一致（不区分大小写，默认 8181）
HTTP_LISTEN_PORT=8181
if [ -f .env ]; then
  _v=$(awk -F= '/^[Ll]isten[Pp]ort=/{gsub(/^[ \t]+|[ \t]+$/,"",$2); print $2; exit}' .env 2>/dev/null)
  [ -n "$_v" ] && HTTP_LISTEN_PORT="$_v"
fi

echo "🚀 正在启动 iTunnel..."

# 构建 release 版本
echo "📦 构建应用..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ 构建失败"
    exit 1
fi

# 检查是否已经在运行
if lsof -Pi :"${HTTP_LISTEN_PORT}" -sTCP:LISTEN -t >/dev/null ; then
    echo "⚠️  iTunnel 已经在运行 (端口 ${HTTP_LISTEN_PORT} 被占用)"
    echo "如需重启，请先运行: ./stop.sh"
    exit 1
fi

# 使用 nohup 在后台运行，并将输出重定向到日志文件
echo "🔧 启动后台服务..."
sudo nohup ./target/release/itunnel > /tmp/itunnel.log 2>&1 &

# 保存 PID
echo $! > /tmp/itunnel.pid

# 等待服务启动
sleep 2

# 检查是否成功启动
if lsof -Pi :"${HTTP_LISTEN_PORT}" -sTCP:LISTEN -t >/dev/null ; then
    echo "✅ iTunnel 已成功启动！"
    echo ""
    echo "📊 状态信息:"
    echo "   - PID: $(cat /tmp/itunnel.pid)"
    echo "   - 日志: /tmp/itunnel.log"
    echo "   - Web UI: http://127.0.0.1:${HTTP_LISTEN_PORT}"
    echo ""
    echo "💡 提示:"
    echo "   - 查看日志: tail -f /tmp/itunnel.log"
    echo "   - 停止服务: ./stop.sh"
    echo "   - 打开配置: open http://127.0.0.1:${HTTP_LISTEN_PORT}"
    echo ""
    echo "🎉 现在可以安全关闭此 Terminal 窗口"
else
    echo "❌ 启动失败，请查看日志: cat /tmp/itunnel.log"
    exit 1
fi
