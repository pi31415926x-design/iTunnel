## 测试函数
sudo cargo test test_create_tun -- --ignored --nocapture


sudo cargo test test_wg_turn_on -- --nocapture
sudo ifconfig utun9981 10.99.0.96 10.99.0.96 netmask 255.255.0.0
sudo ip route add 10.99.0.0/16 dev utun9981
sudo ip route add 0.0.0.0/1 dev utun9981
sudo ip route add 128.0.0.0/1 dev utun9981
route -n add -inet -host 54.249.221.90 192.168.1.1


## itunnel-bin 启动脚本
```
#!/bin/bash
DIR=$(cd "$(dirname "$0")"; pwd)
/usr/bin/osascript -e "do shell script \"'$DIR/iTunnel-bin' > /tmp/itunnel.log 2>&1 &\" with administrator privileges"
```


sudo cargo test test_verify_and_cfg -- --ignored --nocapture


## peers speed test

kr
52.78.213.238
2406:da12:c88:9100:2193:38b4:6050:bcec

jp
54.249.221.90
2406:da14:2ea:4600:ade3:d9e5:e626:a1af

jp
13.231.209.151
2406:da14:2ea:4600:88b9:3372:8c5b:68ce

us-west
35.91.75.187
2600:1f14:1605:a100:e999:15fb:7f66:1772