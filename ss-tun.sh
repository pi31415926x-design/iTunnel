#!/bin/bash

# ====================================================
# 配置区域
# ====================================================
CONFIG_FILE="ss-config.json"       # 你的 sslocal 配置文件
TUN_DEV="utun9982"                 # 建议固定一个 utun 编号
LOCAL_IP="10.0.0.1"             # TUN 虚拟网卡本地 IP
REMOTE_IP="10.0.0.2"            # TUN 虚拟网卡对端 IP (网关)

# 一旦脚本中任何命令返回非零状态，立即退出
set -e

# ====================================================
# 权限与环境检查
# ====================================================
if [ "$EUID" -ne 0 ]; then 
  echo "错误: 必须使用 sudo 运行此脚本"
  exit 1
fi

if [ ! -f "$CONFIG_FILE" ]; then
  echo "错误: 找不到配置文件 $CONFIG_FILE"
  exit 1
fi

# 提取服务器 IP (支持从 JSON 提取)
SERVER_IP=$(grep '"server"' "$CONFIG_FILE" | sed -E 's/.*"server": "([^"]+)".*/\1/')
if [[ -z "$SERVER_IP" ]]; then
    echo "错误: 无法从配置文件中提取 server IP"
    exit 1
fi

# 获取当前默认物理网关和网卡
OLD_GW=$(route -n get default | grep gateway | awk '{print $2}')
OLD_IFACE=$(route -n get default | grep interface | awk '{print $2}')

if [[ -z "$OLD_GW" ]]; then
    echo "错误: 无法获取当前物理网关，请检查网络连接"
    exit 1
fi

# ====================================================
# 资源清理函数 (退出时触发)
# ====================================================
cleanup() {
    echo ""
    echo "正在恢复网络配置..."
    # 恢复默认网关
    route delete default >/dev/null 2>&1 || true
    route add default "$OLD_GW" >/dev/null 2>&1 || true
    # 删除服务器特定路由
    route delete "$SERVER_IP" >/dev/null 2>&1 || true
    # 杀掉后台进程
    if [ ! -z "$SS_PID" ]; then
        kill "$SS_PID" >/dev/null 2>&1 || true
    fi
    echo "清理完毕，已退出。"
}

# 绑定信号，无论脚本正常结束还是被 Ctrl+C 终止都会执行 cleanup
trap cleanup EXIT INT TERM

# ====================================================
# 执行逻辑
# ====================================================

echo "1. 启动 sslocal 后台进程..."
~/github/shadowsocks-rust-new/target/release/sslocal -c "$CONFIG_FILE" &
SS_PID=$!

# 检查进程是否启动成功
sleep 2
if ! kill -0 $SS_PID > /dev/null 2>&1; then
    echo "错误: sslocal 启动失败，请检查配置文件和端口是否冲突"
    exit 1
fi

echo "2. 配置网卡 $TUN_DEV..."
# 分配 IP 并激活
ifconfig "$TUN_DEV" "$LOCAL_IP" "$REMOTE_IP" up || { echo "网卡配置失败"; exit 1; }

echo "3. 设置直连路由 (防止回环)..."
# 确保发往代理服务器的数据包通过物理网卡发出
route add "$SERVER_IP" "$OLD_GW"

echo "4. 接管全局流量..."
# 修改默认路由，将流量导向 TUN 对端 IP
route delete default
route add default "$REMOTE_IP"

echo "------------------------------------------------"
echo "成功！当前所有流量已通过 $TUN_DEV 转发。"
echo "代理服务器: $SERVER_IP (直连通过 $OLD_GW)"
echo "按 Ctrl+C 停止代理并恢复原始网络"
echo "------------------------------------------------"

# 保持脚本运行，等待信号
while true; do sleep 1; done