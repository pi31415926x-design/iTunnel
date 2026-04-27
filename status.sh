#!/bin/bash

# iTunnel 状态检查脚本（在仓库根目录执行）

cd "$(dirname "$0")" || exit 1

HTTP_LISTEN_PORT=8181
if [ -f .env ]; then
  _v=$(awk -F= '/^[Ll]isten[Pp]ort=/{gsub(/^[ \t]+|[ \t]+$/,"",$2); print $2; exit}' .env 2>/dev/null)
  [ -n "$_v" ] && HTTP_LISTEN_PORT="$_v"
fi

echo "📊 iTunnel 状态检查"
echo "===================="
echo ""

# 检查端口
if lsof -Pi :"${HTTP_LISTEN_PORT}" -sTCP:LISTEN -t >/dev/null ; then
    PID=$(lsof -ti:"${HTTP_LISTEN_PORT}")
    echo "✅ 服务状态: 运行中"
    echo "📍 进程 PID: $PID"
    echo "🌐 Web UI: http://127.0.0.1:${HTTP_LISTEN_PORT}"
    echo ""
    
    # 显示进程信息
    echo "💻 进程信息:"
    ps -p $PID -o pid,ppid,%cpu,%mem,etime,command
    echo ""
    
    # 检查日志文件
    if [ -f /tmp/itunnel.log ]; then
        echo "📝 最近日志 (最后 10 行):"
        tail -10 /tmp/itunnel.log
    fi
else
    echo "❌ 服务状态: 未运行"
    echo ""
    echo "💡 启动服务: ./run_background.sh"
fi
