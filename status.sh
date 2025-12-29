#!/bin/bash

# iTunnel 状态检查脚本

echo "📊 iTunnel 状态检查"
echo "===================="
echo ""

# 检查端口
if lsof -Pi :8181 -sTCP:LISTEN -t >/dev/null ; then
    PID=$(lsof -ti:8181)
    echo "✅ 服务状态: 运行中"
    echo "📍 进程 PID: $PID"
    echo "🌐 Web UI: http://127.0.0.1:8181"
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
