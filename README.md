### TODO list
[ ] server login page

### 命令行与 GUI

| 命令 | 模式 | GUI |
|------|------|-----|
| `itunnel`（无 `-s`/`-c`） | Client | 是（默认托盘） |
| `itunnel --gui` | Client | 是 |
| `itunnel --client` 或 `-c` | Client | 否（仅 Actix，无 Tauri） |
| `itunnel --client --gui` | Client | 是 |
| `itunnel --server` 或 `-s` | Server | 否 |
| `itunnel --server --gui` | Server | 是 |

**headless**：无 Tauri、无系统托盘/原生窗口，只跑 Actix（Web/API）+ Ctrl+C 清理；适合服务器或无图形环境。代码里用字段名 `headless` 表示这种状态。

无 GUI（headless）时只跑 Actix，不启动 Tauri（Linux 上不初始化 GTK）。

```bash
cargo run -- --server
cargo run -- --client --gui
./target/debug/itunnel --client
```

### HTTP 监听（Web / API）

`src/main.rs` 从项目根目录 `.env` 读取 `ListenAddress`、`ListenPort`（不区分大小写）；未设置时默认 **`127.0.0.1:8181`**。本机浏览器访问时，若 `ListenAddress=0.0.0.0`，请使用 `http://127.0.0.1:<ListenPort>`。

### 实现要点

1. **`parse_startup_options()`**  
   解析 `app_mode`（`-s` / `--server` / `-c` / `--client`，后出现者覆盖先出现者；无模式旗标则默认 client）与 `--gui`。出现 `-s`/`-c` 且未传 `--gui` 时 `headless == true`；无参数或带 `--gui` 时启用 Tauri。

2. **`headless == true` 时**  
   在构建 `tauri::Builder` 之前返回：只做日志、状态、Ctrl+C 线程，然后 `spawn_actix_background`，主线程用 `mpsc::recv()` 阻塞直到收到信号。

3. **`resolve_static_dir_headless()`**  
   Release 且无 Tauri 时不用 `app.path().resource_dir()`，依次尝试当前目录下 `frontend/dist`、可执行文件旁 `frontend/dist`（日志与路径一致性；页面仍主要由 `EmbeddedAssets` 提供）。

4. **`spawn_actix_background`**  
   与 Tauri `setup` 里启动 Actix 的逻辑共用，避免重复。
