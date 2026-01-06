#!/bin/bash

# --- 自动探测物理网卡 (LAN) ---
# 逻辑：查找默认路由 (0.0.0.0/default) 指向的物理接口
AUTO_LAN_IF=$(route -n get default 2>/dev/null | grep 'interface:' | awk '{print $2}')

# --- 配置区 ---
PF_CONF="/tmp/pf_vpn_gateway.conf"

if [ "$EUID" -ne 0 ]; then
  echo "❌ 错误: 请使用 sudo 运行此脚本"
  exit 1
fi

if [ "$#" -lt 2 ]; then
    echo "💡 用法: sudo $0 [VPN接口名] {on|off}"
    echo "   例子: sudo $0 utun3 on"
    exit 1
fi

VPN_IF=$1
ACTION=$2
LAN_IF=${AUTO_LAN_IF:-en0} # 如果自动探测失败，保底使用 en0

case "$ACTION" in
  on)
    # 检查接口是否存在
    if ! ifconfig "$VPN_IF" > /dev/null 2>&1; then
        echo "❌ 错误: VPN 接口 $VPN_IF 未找到，请先连接 VPN。"
        exit 1
    fi

    echo "🚀 启动网关模式..."
    echo "📍 探测到物理网卡 (LAN): $LAN_IF"
    echo "🌐 转发至 VPN 接口: $VPN_IF"

    # 1. 开启内核转发
    sysctl -w net.inet.ip.forwarding=1 > /dev/null
    sysctl -w net.inet6.ip6.forwarding=1 > /dev/null

    # 2. 生成动态 PF 规则
    # 使用 :network 动态获取该网卡所在的子网
    cat <<EOF > $PF_CONF
# IPv4 NAT
nat on $VPN_IF inet from $LAN_IF:network to any -> ($VPN_IF)
# IPv6 NAT
nat on $VPN_IF inet6 from $LAN_IF:network to any -> ($VPN_IF)
EOF

    # 3. 刷新并加载 PF
    pfctl -F all > /dev/null 2>&1
    pfctl -e > /dev/null 2>&1
    pfctl -f $PF_CONF > /dev/null 2>&1
    
    echo "✅ 状态: 运行中。请将其他设备的网关设为 $(ipconfig getifaddr $LAN_IF)"
    ;;

  off)
    echo "🛑 正在关闭转发..."
    sysctl -w net.inet.ip.forwarding=0 > /dev/null
    sysctl -w net.inet6.ip6.forwarding=0 > /dev/null
    pfctl -f /etc/pf.conf > /dev/null 2>&1
    pfctl -d > /dev/null 2>&1
    [ -f $PF_CONF ] && rm $PF_CONF
    echo "✅ 状态: 已恢复默认。"
    ;;

  *)
    echo "💡 用法: sudo $0 $VPN_IF {on|off}"
    exit 1
esac