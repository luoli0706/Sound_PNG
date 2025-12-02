# Sound_PNG (v1.3.0-beta)

A high-performance, secure, bi-directional steganography tool written in Rust. Hide your secrets in plain sight—or plain sound.
一个高性能、安全的双向隐写工具，使用 Rust 编写。将您的秘密隐藏在显眼处——或平凡的声音中。

## 🌟 Key Features / 主要特性

### 1. True Streaming Engine / 真·流式引擎 (New in v1.3.0-beta)
- **Zero Memory Overhead**: Process files of ANY size (1GB, 10GB, 100GB...) with minimal RAM usage (approx 32MB).
- **零内存开销**: 处理任何大小的文件（1GB, 10GB, 100GB...）仅需极少内存（约 32MB）。
- **Speed**: Faster processing due to zero-copy pipeline.
- **速度**: 零拷贝管道带来更快的处理速度。

### 2. Plugin System / 插件系统 (New!)
- **Extendable**: Drop `.sn` plugin files into the `Plugins` folder to add new container formats or encoding strategies.
- **可扩展**: 将 `.sn` 插件文件放入 `Plugins` 文件夹即可添加新的容器格式或编码策略。
- **Sequence Frame Support**: Includes a demo plugin to split payload across a sequence of PNGs.
- **序列帧支持**: 包含一个演示插件，可将负载分割到一系列 PNG 图片中。

### 3. Universal Steganography / 通用隐写
- **Arbitrary Binary Embedding**: Hide ANY file (ZIP, EXE, MP4, etc.) inside a PNG or WAV container.
- **任意二进制嵌入**: 将任何文件（ZIP, EXE, MP4 等）隐藏在 PNG 或 WAV 容器中。

### 4. Security & Features / 安全与特性
- **ChaCha8 Encryption**: Military-grade stream cipher with Physical Key support.
- **ChaCha8 加密**: 具有物理密钥支持的军用级流密码。
- **Update Checker**: Automatically checks for new versions on GitHub.
- **更新检查**: 自动检查 GitHub 上的新版本。

## 📦 Installation / 安装

Download `Sound_PNG_v1.3.0-beta.exe` from [Releases](https://github.com/luoli0706/Sound_PNG/releases).
Ensure the `Plugins` folder is in the same directory.
从 [Releases](https://github.com/luoli0706/Sound_PNG/releases) 下载 `Sound_PNG_v1.3.0-beta.exe`。
确保 `Plugins` 文件夹在同一目录下。

## 📖 Documentation / 文档

- [User Manual / 用户手册](docs/User_Manual.md)
- [Developer Documentation / 开发者文档](docs/Developer_Documentation.md)

## 🛠 Build from Source / 源码构建

```bash
cd sound_png
cargo build --release
```

## 📝 License / 许可证
MIT License