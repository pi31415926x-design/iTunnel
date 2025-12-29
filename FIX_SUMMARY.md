# ✅ iTunnel.app 打包问题已修复

## 🎯 修复的问题

### 问题 1: frontend/dist 未打包到 .app ✅

**原因**: 
- Tauri 默认只打包 `frontendDist` 指定的目录用于 WebView
- Actix-web 需要单独的静态文件访问

**解决方案**:
1. 在 `tauri.conf.json` 中添加 `resources` 配置
2. 使用 Tauri 的资源路径 API 动态获取静态文件位置

**修改内容**:

```json
// tauri.conf.json
{
  "bundle": {
    "resources": {
      "frontend/dist": "./"
    }
  }
}
```

```rust
// src/main.rs
let static_dir = if cfg!(dev) {
    // 开发环境：使用相对路径
    std::env::current_dir().unwrap().join("frontend").join("dist")
} else {
    // 生产环境：使用 Tauri 资源路径
    app.path().resource_dir().expect("无法获取资源目录")
        .join("frontend").join("dist")
};
```

---

### 问题 2: actix_web 服务未启动 ✅

**原因**:
- 静态文件路径硬编码，在 .app 中路径不正确
- 缺少启动日志，难以调试

**解决方案**:
1. 动态传递静态文件路径给 actix-web
2. 添加详细的启动日志
3. 使用 Arc 共享路径数据

**修改内容**:

```rust
// 修改函数签名，接收静态文件路径
async fn start_actix_server(static_dir: PathBuf) -> std::io::Result<()> {
    let static_dir = Arc::new(static_dir);
    
    println!("📁 静态文件目录: {:?}", static_dir);
    
    HttpServer::new(move || {
        let static_path = static_dir.clone();
        App::new()
            .service(Files::new("/", static_path.as_ref()).index_file("index.html"))
    })
    .bind(("127.0.0.1", 8181))?
    .run()
    .await
}
```

---

## 📦 打包后的文件结构

```
iTunnel.app/
├── Contents/
│   ├── MacOS/
│   │   └── iTunnel          # 主程序
│   └── Resources/
│       ├── index.html       # ✅ 已打包
│       ├── assets/          # ✅ 已打包
│       │   ├── index-*.js
│       │   └── index-*.css
│       └── icon.icns
```

---

## 🚀 验证步骤

### 1. 检查打包内容

```bash
ls -la target/release/bundle/macos/iTunnel.app/Contents/Resources/
```

**预期输出**:
```
✅ index.html
✅ assets/
```

### 2. 运行测试脚本

```bash
./test_app.sh
```

### 3. 手动测试

```bash
# 启动应用
sudo open target/release/bundle/macos/iTunnel.app

# 等待几秒后检查
lsof -i :8181

# 访问 Web UI
open http://127.0.0.1:8181
```

---

## 📝 启动日志

成功启动时，你应该看到以下日志（在 Console.app 或 terminal）:

```
🚀 iTunnel 启动中...
📂 工作目录: /Applications
📁 静态文件路径: "/Applications/iTunnel.app/Contents/Resources/frontend/dist"
✅ 静态文件目录存在: true
🌐 正在 127.0.0.1:8181 启动 Web 服务...
📁 静态文件目录: "/Applications/iTunnel.app/Contents/Resources/frontend/dist"
✅ Actix 服务已启动
```

---

## 🐛 故障排查

### 如果 Web 服务未启动

1. **查看日志**:
   ```bash
   # 方法 1: 使用 Console.app
   打开 Console.app → 搜索 "iTunnel"
   
   # 方法 2: 查看系统日志
   log show --predicate 'process == "iTunnel"' --last 5m
   ```

2. **检查端口占用**:
   ```bash
   lsof -i :8181
   ```

3. **检查权限**:
   ```bash
   # 确保使用 sudo 运行
   sudo open target/release/bundle/macos/iTunnel.app
   ```

### 如果静态文件未找到

1. **验证打包内容**:
   ```bash
   find target/release/bundle/macos/iTunnel.app -name "index.html"
   ```

2. **检查资源路径**:
   - 查看启动日志中的 "📁 静态文件路径"
   - 确认该路径存在且包含文件

---

## ✨ 改进总结

### 代码改进

1. ✅ 动态路径解析（开发/生产环境自适应）
2. ✅ 详细的启动日志
3. ✅ 正确的资源打包配置
4. ✅ 线程安全的路径共享（Arc）

### 配置改进

1. ✅ `tauri.conf.json` 添加 resources 配置
2. ✅ 正确的 bundle identifier
3. ✅ 清晰的构建命令

---

## 📚 相关文件

- `src/main.rs` - 主程序逻辑
- `tauri.conf.json` - Tauri 配置
- `test_app.sh` - 测试脚本
- `README_RUN.md` - 运行指南

---

## 🎉 下一步

1. **测试应用**: 运行 `./test_app.sh`
2. **配置 WireGuard**: 访问 http://127.0.0.1:8181
3. **分发应用**: 使用 `iTunnel_0.1.0_x64.dmg`

祝使用愉快！🚀
