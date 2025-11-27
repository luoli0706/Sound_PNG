# Sound_PNG (Beta 2.0)

A high-performance, secure, bi-directional steganography tool written in Rust. Hide your secrets in plain sight—or plain sound.
一个高性能、安全的双向隐写工具，使用 Rust 编写。将您的秘密隐藏在显眼处——或平凡的声音中。

## 🌟 Key Features / 主要特性

### 1. Universal Steganography / 通用隐写 (New in Beta 2.0)
- **Arbitrary Binary Embedding**: Hide ANY file (ZIP, EXE, etc.) inside a PNG or WAV container.
- **任意二进制嵌入**: 将任何文件（ZIP, EXE 等）隐藏在 PNG 或 WAV 容器中。
- **Homomorphic Hiding**: Hide a PNG inside another PNG, or a WAV inside another WAV.
- **同态隐藏**: 将 PNG 隐藏在另一个 PNG 中，或将 WAV 隐藏在另一个 WAV 中。

### 2. Bi-Directional Steganography / 双向隐写 (Standard Mode)
- **Voice Carrier**: Hide any file (Image, Text, etc.) inside a 32-bit WAV audio file.
- **语音载体**: 将任何文件（图片、文本等）隐藏在 32 位 WAV 音频文件中。
- **Picture Carrier**: Hide audio (or any file) inside a PNG image.
- **图片载体**: 将音频（或任何文件）隐藏在 PNG 图片中。
- **Auto-Expand**: Automatically resizes the container image to fit large payloads.
- **自动扩容**: 自动调整容器图片大小以适应较大的负载。

### 3. Security / 安全
- **ChaCha8 Encryption**: Military-grade stream cipher with Physical Key support.
- **ChaCha8 加密**: 具有物理密钥支持的军用级流密码。
- **Data Integrity**: SHA-256 Hash verification.
- **数据完整性**: SHA-256 哈希校验。

## 📦 Installation / 安装

Download `Sound_PNG_Beta_2_0.exe`.
下载 `Sound_PNG_Beta_2_0.exe`。

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