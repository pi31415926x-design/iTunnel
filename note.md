## 测试函数
sudo cargo test test_create_tun -- --ignored --nocapture


sudo cargo test test_wg_turn_on_valid_fd -- --ignored --nocapture
ifconfig utun9981 10.99.0.7 10.99.0.7 netmask 255.255.0.0
ip route add 10.99.0.0/16 dev utun9981
ip route add 0.0.0.0/1 dev utun9981
ip route add 128.0.0.0/1 dev utun9981
route -n add -inet -host 54.249.221.90 192.168.1.1


## itunnel-bin 启动脚本
```
#!/bin/bash
DIR=$(cd "$(dirname "$0")"; pwd)
/usr/bin/osascript -e "do shell script \"'$DIR/iTunnel-bin' > /tmp/itunnel.log 2>&1 &\" with administrator privileges"
```