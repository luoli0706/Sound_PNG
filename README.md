# Sound_PNG (Beta 1.0)

A high-performance, secure, bi-directional steganography tool written in Rust. Hide your secrets in plain sight—or plain sound.
一个高性能、安全的双向隐写工具，使用 Rust 编写。将您的秘密隐藏在显眼处——或平凡的声音中。

## 🌟 Key Features / 主要特性

### 1. Bi-Directional Steganography / 双向隐写
- **Voice Carrier**: Hide any file (Image, Text, etc.) inside a 32-bit WAV audio file.
- **语音载体**: 将任何文件（图片、文本等）隐藏在 32 位 WAV 音频文件中。
- **Picture Carrier**: Hide audio (or any file) inside a PNG image.
- **图片载体**: 将音频（或任何文件）隐藏在 PNG 图片中。
- **Auto-Expand**: Automatically resizes the container image to fit large payloads.
- **自动扩容**: 自动调整容器图片大小以适应较大的负载。

### 2. Multi-Format Support / 多格式支持
- **Audio Inputs**: WAV, MP3 (Normalized to 16-bit PCM).
- **音频输入**: WAV, MP3（归一化为 16 位 PCM）。
- **Payload Inputs**: PNG, JPG, JPEG, or any binary file.
- **负载输入**: PNG, JPG, JPEG 或任何二进制文件。
- **Output**: 
    - 32-bit PCM WAV (Lossless Container).
    - 16-bit RGBA PNG (Lossless Container).
- **输出**:
    - 32 位 PCM WAV（无损容器）。
    - 16 位 RGBA PNG（无损容器）。

### 3. The "Four Judges" Security System / "四法官" 安全系统
An optional, military-grade security layer.
可选的军用级安全层。
- **1st Judge (Encryption)**: Stream cipher (XOR) using ChaCha8.
- **第一法官（加密）**: 使用 ChaCha8 的流密码（XOR）。
- **2nd Judge (Unpredictability)**: Timestamp-based dynamic seeding.
- **第二法官（不可预测性）**: 基于时间戳的动态种子。
- **3rd Judge (Integrity)**: SHA-256 Hash verification to detect tampering.
- **第三法官（完整性）**: SHA-256 哈希校验以检测篡改。
- **4th Judge (Physical Key)**: Optional external Key File mixed into the encryption stream.
- **第四法官（物理密钥）**: 可选的外部密钥文件，混合入加密流中。

## 📦 Installation / 安装

Download the latest release `Sound_PNG_Beta_1_0.exe`.
下载最新发布的 `Sound_PNG_Beta_1_0.exe`。

## 📖 Documentation / 文档

- [User Manual / 用户手册](docs/User_Manual.md) - How to use the tool. / 如何使用工具。
- [Developer Documentation / 开发者文档](docs/Developer_Documentation.md) - Architecture and Logic. / 架构与逻辑。

## 🛠 Build from Source / 源码构建

```bash
cd sound_png
cargo build --release
```

## 📝 License / 许可证
MIT License
