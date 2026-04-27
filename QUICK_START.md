# iTunnel Client Mode - 快速开始指南

## 📚 概述

该实施为iTunnel添加了**Client/Server模式**支持，并为Client模式设计了全新的UI。用户可以通过命令行参数选择运行模式。

**HTTP / API 基础地址**：由项目根 `.env` 的 `ListenAddress`、`ListenPort` 决定；未设置时默认 `http://127.0.0.1:8181`。开发时若修改了 `ListenPort`，需同步 `frontend/vite.config.ts` 里 dev server 的 `/api` 代理目标，以及 `frontend/src/services/api.ts` 中的 `baseUrl`（或使用与页面同源的相对路径）。

---

## 🚀 快速启动

### 1. 后端编译和运行

```bash
cd /Users/haogle/github/itunnel

# 编译调试版本
cargo build

# 无 GUI，仅 Actix（Client / Server）
./target/debug/itunnel --client
./target/debug/itunnel --server

# 带 Tauri 托盘 + Actix
./target/debug/itunnel --client --gui
./target/debug/itunnel --server --gui

# 无参数：默认 Client + Tauri
./target/debug/itunnel
```

### 2. 前端开发环境

```bash
cd /Users/haogle/github/itunnel/frontend

# 首次运行，安装依赖
npm install

# 启动开发服务器（会自动打开浏览器）
npm run dev

# 访问: http://localhost:5173/
# API 代理到后端（默认 http://127.0.0.1:8181，与 .env ListenPort 一致）
```

### 3. 生产构建

```bash
# 前端构建
cd frontend
npm run build

# 后端生产版本
cd ..
cargo build --release
```

---

## 🎯 主要功能

### Client模式首页 (`/`)
- **连接状态指示** - 显示当前连接状态（已连接/正在连接/未连接）
- **一键连接按钮** - 快速连接或断开
- **当前Endpoint显示** - 显示已选择的VPN服务器
- **增强模式摘要** - 显示TCP/混淆/代理模式状态
- **快速导航** - 快捷链接到设置和端点选择页面

### Endpoint选择页面 (`/endpoints`)
- **Endpoint列表** - 显示所有可用的VPN服务器
- **搜索和过滤** - 按名称、地址或地理位置搜索
- **实时选择** - 一键切换服务器
- **延迟显示** - 显示每个服务器的延迟
- **来源标签** - 区分来自订阅的和手动添加的端点

### 增强模式设置页面 (`/settings/enhance-mode`)
- **Protocol选择** - UDP（快速）或 TCP（兼容）
- **随机混淆** - 启用/禁用，支持自定义密钥
- **代理模式** - Split（分流）或 Global（全局）
- **配置保存** - 实时保存到后端
- **配置摘要** - 显示当前配置状态

---

## 🔌 API参考

以下 URL 使用默认端口 **8181**；若 `.env` 中配置了 `ListenPort`，请替换 URL 中的端口。

### 模式检测
```bash
# 获取当前运行模式
curl http://127.0.0.1:8181/api/mode

# 返回例子:
# {
#   "mode": "client",
#   "success": true
# }
```

### Endpoint管理
```bash
# 获取所有endpoints
curl http://127.0.0.1:8181/api/endpoints

# 选择endpoint
curl -X POST http://127.0.0.1:8181/api/endpoints/select \
  -H "Content-Type: application/json" \
  -d '{"endpoint_id": "endpoint-123"}'
```

### 增强模式设置
```bash
# 获取当前设置
curl http://127.0.0.1:8181/api/settings/enhance-mode

# 保存设置
curl -X POST http://127.0.0.1:8181/api/settings/enhance-mode \
  -H "Content-Type: application/json" \
  -d '{
    "protocol": "tcp",
    "obfuscate": true,
    "obfuscateKey": "your-custom-key",
    "proxyMode": "global"
  }'
```

---

## 📁 重要文件说明

### 后端新增/修改
```
src/wg/config.rs
  - AppMode enum         ← Client/Server区分
  - Protocol enum        ← TCP/UDP选择
  - ProxyMode enum       ← Split/Global模式
  - EnhanceMode struct   ← 增强模式配置
  - EndpointInfo struct  ← 端点信息

src/api/local_api.rs
  - get_mode_handler()           ← GET /api/mode
  - get_endpoints_handler()      ← GET /api/endpoints
  - select_endpoint_handler()    ← POST /api/endpoints/select
  - get_enhance_mode_handler()   ← GET /api/settings/enhance-mode
  - save_enhance_mode_handler()  ← POST /api/settings/enhance-mode

src/main.rs
  - parse_cli_args()  ← 命令行参数解析
  - parse_cli_args()对应的初始化逻辑
```

### 前端新增/修改
```
frontend/src/stores/
  - wireguard.ts        ← WireGuard连接状态
  - endpoints.ts        ← Endpoint管理
  - settings.ts         ← 增强模式设置

frontend/src/services/
  - api.ts              ← 基础HTTP客户端
  - wireguard-api.ts    ← WireGuard API封装

frontend/src/composables/
  - useWireGuardMode.ts ← 模式检测和初始化

frontend/src/pages/
  - ClientOverview.vue            ← 新: 首页仪表板
  - Endpoints.vue                 ← 新: Endpoint选择
  - Settings/EnhanceMode.vue      ← 新: 增强模式设置
```

---

## 🔧 开发技巧

### 调试后端日志
```bash
# 设置日志级别
RUST_LOG=debug ./target/debug/itunnel -c

# 查看特定模块日志
RUST_LOG=itunnel::api=debug ./target/debug/itunnel -c
```

### 浏览器开发者工具
```
F12 或 Cmd+Option+I (macOS)

Network标签：
  - 查看API调用
  - 检查响应数据
  - 监控网络延迟

Console标签：
  - 查看JavaScript错误
  - 测试 fetch 调用
  - 查看store状态
```

### Pinia DevTools
```
# 在浏览器中安装 Pinia DevTools 扩展
# 可以实时查看和修改store状态
```

---

## 🧪 测试清单

### 基本测试
- [ ] 启动 `itunnel -c` 后端
- [ ] 启动 `npm run dev` 前端
- [ ] 访问 http://localhost:5173
- [ ] 页面加载正常，无错误

### 模式测试
- [ ] 首页显示"未连接"状态
- [ ] 点击"Endpoints"能加载端点列表
- [ ] 选择一个Endpoint后状态更新
- [ ] 点击"Settings"能访问增强模式页面

### 增强模式测试
- [ ] 切换Protocol (UDP ↔ TCP)
- [ ] 启用/禁用混淆
- [ ] 设置自定义混淆密钥
- [ ] 选择代理模式 (Split ↔ Global)
- [ ] 验证"Save Changes"按钮变为激活
- [ ] 保存后设置被持久化

### API测试
- [ ] GET /api/mode 返回正确的模式
- [ ] GET /api/endpoints 返回端点列表
- [ ] POST /api/endpoints/select 能选择端点
- [ ] GET/POST /api/settings/enhance-mode 工作正常

---

## ⚠️ 注意事项

1. **权限** - 在macOS上运行WireGuard可能需要sudo权限
   ```bash
   sudo ./target/debug/itunnel --client
   ```

2. **端口占用** - 确保 `.env` 中 `ListenPort`（默认 8181）未被占用
   ```bash
   lsof -i :8181   # 将 8181 换成你的 ListenPort
   ```

3. **CORS问题** - 当前前后端分离，确保API请求没有跨域问题
   - 开发环境：前端 `localhost:5173` → 后端 `127.0.0.1:<ListenPort>`（默认 8181）
   - 如有问题，可在main.rs中启用CORS

4. **数据持久化** - Store中的数据在浏览器刷新后会重新加载

---

## 📚 更多信息

详细的实施说明请参考: [CLIENT_MODE_IMPLEMENTATION.md](./CLIENT_MODE_IMPLEMENTATION.md)

---

**版本**: 1.0  
**最后更新**: 2026-04-17  
**状态**: 可用于开发和测试 ✅
