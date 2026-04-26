# iTunnel 部署指南 - 脱离 Terminal 运行

## 📋 当前架构说明

iTunnel 是一个基于 Tauri 2.x 的桌面应用程序，已经具备完整的后台运行能力：

- **GUI 框架**: Tauri (Rust + Web)
- **前端**: Vue.js + Vite
- **后端**: Actix-web (HTTP 服务器)
- **系统集成**: 系统托盘 + 菜单

## 🎯 三种运行方式

### 方式 1: Tauri 开发模式（当前使用）

```bash
# 需要 terminal 保持打开
cargo run
```

**特点**:
- ✅ 快速开发调试
- ❌ 需要 terminal 窗口
- ❌ 依赖开发环境

---

### 方式 2: Tauri 打包应用（推荐 - 完全脱离 Terminal）

#### 步骤 1: 升级 Node.js（如需要）

```bash
# 检查当前版本
node --version

# 需要 Node.js 20.19+ 或 22.12+
# 使用 nvm 升级
nvm install 22
nvm use 22
```

#### 步骤 2: 构建前端

```bash
cd frontend
npm install
npm run build
cd ..
```

#### 步骤 3: 修改 tauri.conf.json

确保 `frontendDist` 指向正确的构建目录：

```json
{
  "build": {
    "frontendDist": "../frontend/dist"
  }
}
```

#### 步骤 4: 打包 Tauri 应用

```bash
# macOS 打包
npm install -g @tauri-apps/cli
cargo tauri build

# 或者使用 cargo-bundle
cargo install cargo-bundle
cargo bundle --release
```

#### 步骤 5: 运行打包后的应用

**macOS**:
```bash
# 应用位置
./target/release/bundle/macos/itunnel.app

# 双击运行，或命令行启动
open ./target/release/bundle/macos/itunnel.app
```

**特点**:
- ✅ 完全脱离 terminal
- ✅ 双击启动
- ✅ 系统托盘常驻
- ✅ 可分发给其他用户

---

### 方式 3: 使用 launchd 服务（macOS 后台服务）

如果需要开机自启动和完全后台运行：

#### 创建 plist 文件

```bash
sudo nano ~/Library/LaunchAgents/com.itunnel.app.plist
```

内容：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.itunnel.app</string>
    
    <key>ProgramArguments</key>
    <array>
        <string>/Users/haogle/github/itunnel/target/release/itunnel</string>
    </array>
    
    <key>RunAtLoad</key>
    <true/>
    
    <key>KeepAlive</key>
    <true/>
    
    <key>StandardOutPath</key>
    <string>/tmp/itunnel.log</string>
    
    <key>StandardErrorPath</key>
    <string>/tmp/itunnel.error.log</string>
</dict>
</plist>
```

#### 加载服务

```bash
# 加载服务
launchctl load ~/Library/LaunchAgents/com.itunnel.app.plist

# 启动服务
launchctl start com.itunnel.app

# 停止服务
launchctl stop com.itunnel.app

# 卸载服务
launchctl unload ~/Library/LaunchAgents/com.itunnel.app.plist
```

---

## 🔧 当前代码已支持的功能

### 1. 系统托盘 ✅

```rust
// src/main.rs 已实现
let _tray = TrayIconBuilder::new()
    .icon(app.default_window_icon().unwrap().clone())
    .menu(&menu)
    .build(app)?;
```

### 2. 后台 HTTP 服务 ✅

在独立线程运行 Actix，不阻塞 GUI；监听地址与端口由项目根 `.env` 的 `ListenAddress`、`ListenPort` 决定（默认 `127.0.0.1:8181`）。实现见 `src/main.rs` 中 `spawn_actix_background` / `start_actix_server`。

### 3. 托盘菜单功能 ✅

- **配置**: 打开浏览器访问 Web UI（默认 `http://127.0.0.1:8181`，以 `.env` 为准）
- **退出**: 完全退出应用

---

## 🚀 快速部署（推荐流程）

### 开发环境

```bash
# Terminal 1: 启动后端
sudo cargo watch -x run

# Terminal 2: 启动前端开发服务器
cd frontend && npm run dev
```

### 生产环境

```bash
# 1. 构建前端
cd frontend && npm run build && cd ..

# 2. 构建 Release 版本
cargo build --release

# 3. 运行（仍在 terminal；二进制名与 Cargo 包名一致，一般为 itunnel）
./target/release/itunnel

# 4. 或打包成应用（完全脱离 terminal）
cargo tauri build
```

---

## 📝 注意事项

### 权限问题

由于 WireGuard 需要 root 权限，有两种方案：

#### 方案 A: 使用 sudo 运行

```bash
sudo ./target/release/itunnel
```

#### 方案 B: 设置 setuid（不推荐，安全风险）

```bash
sudo chown root:wheel ./target/release/itunnel
sudo chmod u+s ./target/release/itunnel
```

#### 方案 C: 使用 Helper Tool（推荐）

创建一个特权 helper 工具，主应用通过 IPC 调用：

```rust
// 需要实现 SMJobBless 或类似机制
// 参考 Tauri 的 tauri-plugin-privileged-helper
```

---

## 🎨 用户体验优化

### 1. 隐藏 Dock 图标（仅托盘运行）

修改 `tauri.conf.json`:

```json
{
  "app": {
    "macOSPrivateApi": true,
    "windows": [{
      "visible": false,
      "skipTaskbar": true
    }]
  }
}
```

### 2. 开机自启动

打包后的应用可以在"系统偏好设置 > 用户与群组 > 登录项"中添加。

### 3. 通知支持

```rust
// 添加依赖
tauri-plugin-notification = "2"

// 使用通知
app.notification()
    .builder()
    .title("iTunnel")
    .body("WireGuard 已连接")
    .show()?;
```

---

## 🐛 故障排查

### 应用无法启动

```bash
# 查看日志
tail -f /tmp/itunnel.log
tail -f /tmp/itunnel.error.log
```

### 端口占用

```bash
# 检查端口（将 8181 换成 .env 中的 ListenPort，未设置时默认为 8181）
lsof -i :8181

# 杀死进程
kill -9 <PID>
```

### 权限问题

```bash
# 检查文件权限
ls -la ./target/release/itunnel

# 重新编译
cargo clean
cargo build --release
```

---

## 📦 分发应用

### macOS

```bash
# 打包后的应用位置
./target/release/bundle/macos/itunnel.app

# 创建 DMG（需要额外工具）
# 或直接压缩
zip -r itunnel.app.zip ./target/release/bundle/macos/itunnel.app
```

### 代码签名（可选）

```bash
# 需要 Apple Developer 账号
codesign --force --deep --sign "Developer ID Application: Your Name" itunnel.app
```

---

## 🎯 总结

**当前状态**: 程序已经具备脱离 terminal 运行的所有基础设施

**推荐方案**: 使用 `cargo tauri build` 打包成独立应用

**关键优势**:
- ✅ 系统托盘常驻
- ✅ 后台服务自动启动
- ✅ Web UI 通过浏览器访问
- ✅ 完全脱离 terminal

**下一步**: 
1. 升级 Node.js 到 20.19+
2. 执行 `cargo tauri build`
3. 双击运行 `itunnel.app`
