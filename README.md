# Rocoflex

<p align="center">
  <img src="assets/app-icon.png" alt="Rocoflex 图标" width="96" height="96">
</p>

使用 Rust + egui 开发的变声引擎与编辑器。

## 构建与运行

需要 Rust 1.99.1，版本由 `rust-toolchain.toml` 指定。

### Windows

需要 Rust、MSVC、Windows SDK 和支持 DX12 的显卡。

```powershell
cargo run --release
```

## 模型与配置

首次使用请在设置中手动选择分析及变声模型和预设目录，程序不会自动探测资源路径，也不附带预设和模型。
