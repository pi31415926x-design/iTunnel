# 🎉 iTunnel 构建成功！

## ✅ 构建完成

你的 iTunnel 应用已经成功构建！

**可执行文件位置**（`cargo build --release` 后）: `target/release/itunnel`（包名与 `Cargo.toml` 中 `name` 一致）

**HTTP 端口**：默认 `8181`，由项目根目录 `.env` 中 `ListenPort` 覆盖；下文 `http://127.0.0.1:8181` 在无自定义端口时适用。

---

## 🚀 三种运行方式

### 方式 1: 直接运行（推荐）

```bash
sudo ./target/release/itunnel
```

**特点**:
- ✅ 系统托盘图标
- ✅ 后台 HTTP 服务（默认 http://127.0.0.1:8181，以 `.env` 为准）
- ✅ 可以关闭 terminal，应用继续运行

### 方式 2: 使用后台脚本

```bash
./run_background.sh
```

**特点**:
- ✅ 自动后台运行
- ✅ 日志输出到 `/tmp/itunnel.log`
- ✅ 保存 PID 便于管理
- ✅ 关闭 terminal 后继续运行

### 方式 3: 打包成 .app（可选）

```bash
cargo tauri build
```

这会创建一个完整的 macOS 应用包，可以双击启动。

---

## 📋 管理命令

### 查看状态
```bash
./status.sh
```

### 停止服务
```bash
./stop.sh
```

### 查看日志
```bash
tail -f /tmp/itunnel.log
```

---

## 🎯 使用托盘菜单

运行后，你会在 macOS 菜单栏看到 iTunnel 图标：

1. **配置** - 打开浏览器访问 Web UI（默认 `http://127.0.0.1:8181`，见 `.env` 的 `ListenAddress` / `ListenPort`）
2. **退出** - 完全退出应用

---

## 💡 快速开始

```bash
# 1. 启动应用
sudo ./target/release/itunnel

# 2. 关闭 terminal 窗口（应用继续运行）

# 3. 点击托盘图标 → 配置
#    浏览器会打开 Web UI（端口与 .env 中 ListenPort 一致，默认 8181）

# 4. 在 Settings 页面配置 WireGuard
```

---

## 🔧 配置说明

### Settings 页面功能

**Interface 标签**:
- Address: WireGuard 接口 IP 地址
- Listen Port: UDP 监听端口（默认 51820）
- Private Key: 点击 "Generate Key" 自动生成
- 选项: TCP Mode, Server Mode, Global Mode

**Peers 标签**:
- Public Key: 对端公钥
- Preshared Key: 预共享密钥（可选）
- Allowed IPs: 允许的 IP 范围
- Endpoint: 对端地址和端口
- Change Route: 是否修改路由表

**特性**:
- ✅ 自动保存到浏览器 localStorage
- ✅ 刷新页面自动恢复
- ✅ 实时表单验证
- ✅ 密钥可见性切换
- ✅ 成功/错误通知

---

## 📁 文件说明

- `target/release/itunnel` - 主程序（Rust 二进制）
- `run_background.sh` - 后台启动脚本
- `stop.sh` - 停止脚本
- `status.sh` - 状态检查脚本
- `/tmp/itunnel.log` - 运行日志
- `/tmp/itunnel.pid` - 进程 ID

---

## ⚠️ 注意事项

1. **需要 root 权限**: WireGuard 操作需要 sudo
2. **端口占用**: 确保 `.env` 中 `ListenPort`（默认 `8181`）未被占用
3. **防火墙**: 可能需要允许应用通过防火墙

---

## 🐛 故障排查

### 应用无法启动

```bash
# 查看日志
cat /tmp/itunnel.log

# 检查端口（将 8181 换成你的 ListenPort）
lsof -i :8181
```

### 托盘图标不显示

- 确保应用有权限显示通知和托盘图标
- 检查"系统偏好设置 > 安全性与隐私"

### WireGuard 配置失败

- 确保使用 sudo 运行
- 检查配置格式是否正确
- 查看 `/tmp/itunnel.log` 中的错误信息

---

## 🎨 下一步

1. **开机自启动**: 添加到"系统偏好设置 > 用户与群组 > 登录项"
2. **代码签名**: 如需分发，使用 Apple Developer 账号签名
3. **完整打包**: 运行 `cargo tauri build` 创建 .app 和 .dmg

---

## 📞 支持

如有问题，请查看:
- 日志文件: `/tmp/itunnel.log`
- 部署文档: `DEPLOYMENT.md`

祝使用愉快！🎉
