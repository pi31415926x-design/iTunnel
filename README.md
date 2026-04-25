已在工程里加上 无 Tauri 的 headless / CLI 启动路径，用法如下。

## 用法
在命令行加上任一标志即可 不启动 Tauri（无窗口、无托盘），只跑 Actix（127.0.0.1:8181） 和原来的 Ctrl+C 清理：

--cli
--headless
-n
可与原有模式组合，例如：
```
cargo run -- --cli              # 默认 client 模式 + 无 GUI
cargo run -- --cli --server    # server 模式 + 无 GUI
./itunnel --headless -s
```
### 实现要点
1. parse_startup_options()
解析 app_mode（-s / -c）和 headless（--cli / --headless / -n）。

2. headless == true 时
在构建 tauri::Builder 之前返回：只做日志、状态、Ctrl+C 线程，然后 spawn_actix_background，主线程用 mpsc::recv() 永久阻塞（进程靠 Ctrl+C → handle_exit → exit 结束）。

3. resolve_static_dir_headless()
Release 且无 Tauri 时不能用 app.path().resource_dir()，因此按顺序尝试：当前目录下的 frontend/dist、可执行文件旁边的 frontend/dist（仅用于日志/一致性；页面仍主要由 EmbeddedAssets 提供）。

4. spawn_actix_background
与原先 Tauri setup 里启动 Actix 的逻辑共用，避免重复。

若你希望 headless 下 连 Actix 都不要（纯后台进程），需要再单独加标志和分支，可以说一下目标行为我帮你改。