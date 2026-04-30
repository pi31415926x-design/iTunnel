本目录需放置 Linux 版 WireGuard/FFI 动态库，文件名须为:

  libwg-go.so

与 Windows 下 libs/windows/libwg-go.dll 同源（Go c-shared 构建的 Linux
产物），须与当前程序 FFI 一致；仓库默认不为空或不含该二进制，需自行从内部
构建流程或已打包产物中取得。

放好后在仓库根目录执行:
  bash scripts/pack-release-linux.sh

或先把已有 .so 同时拷到 libs/linux 与 release/linux:
  bash scripts/place-libwg-linux.sh /path/to/libwg-go.so
