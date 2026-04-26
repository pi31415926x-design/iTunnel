#!/bin/bash

# iTunnel 停止脚本（在仓库根目录执行）

cd "$(dirname "$0")" || exit 1

HTTP_LISTEN_PORT=8181
if [ -f .env ]; then
  _v=$(awk -F= '/^[Ll]isten[Pp]ort=/{gsub(/^[ \t]+|[ \t]+$/,"",$2); print $2; exit}' .env 2>/dev/null)
  [ -n "$_v" ] && HTTP_LISTEN_PORT="$_v"
fi

echo "🛑 正在停止 iTunnel..."

# 检查 PID 文件是否存在
if [ -f /tmp/itunnel.pid ]; then
    PID=$(cat /tmp/itunnel.pid)
    
    # 检查进程是否存在
    if ps -p $PID > /dev/null; then
        echo "📍 找到进程 PID: $PID"
        sudo kill $PID
        
        # 等待进程结束
        sleep 1
        
        if ps -p $PID > /dev/null; then
            echo "⚠️  进程未响应，强制终止..."
            sudo kill -9 $PID
        fi
        
        echo "✅ iTunnel 已停止"
        rm /tmp/itunnel.pid
    else
        echo "⚠️  进程不存在 (PID: $PID)"
        rm /tmp/itunnel.pid
    fi
else
    echo "⚠️  未找到 PID 文件"
    
    # 尝试通过端口查找进程
    PORT_PID=$(lsof -ti:"${HTTP_LISTEN_PORT}")
    if [ ! -z "$PORT_PID" ]; then
        echo "📍 通过端口找到进程: $PORT_PID"
        sudo kill $PORT_PID
        echo "✅ iTunnel 已停止"
    else
        echo "ℹ️  iTunnel 未在运行"
    fi
fi
