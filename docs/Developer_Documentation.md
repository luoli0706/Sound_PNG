# Sound_PNG Developer Documentation / Sound_PNG 开发者文档

## Architecture / 架构

### Core Modules / 核心模块
- **Encoder (`encoder.rs`)**: Handles compression, encryption, and embedding.
- **编码器 (`encoder.rs`)**: 处理压缩、加密和嵌入。
- **Decoder (`decoder.rs`)**: Handles extraction, decryption, and decompression.
- **解码器 (`decoder.rs`)**: 处理提取、解密和解压。
- **Security (`security.rs`)**: Implements the "Four Judges" system.
- **安全 (`security.rs`)**: 实现 "四法官" 系统。
- **Converter (`converter.rs`)**: Handles file I/O and normalization.
- **转换器 (`converter.rs`)**: 处理文件 I/O 和归一化。
- **GUI (`gui.rs`, `ui.slint`)**: Slint-based frontend.
- **GUI (`gui.rs`, `ui.slint`)**: 基于 Slint 的前端。

### Data Flow / 数据流

#### Encoding / 编码
1.  **Input**: Audio (PCM) + Image (Bytes).
    **输入**: 音频 (PCM) + 图片 (字节)。
2.  **Compression**: DEFLATE compression on the Payload.
    **压缩**: 对负载进行 DEFLATE 压缩。
3.  **Security**:
    **安全**:
    - **Hash**: SHA-256 of compressed payload.
    - **哈希**: 压缩负载的 SHA-256 哈希。
    - **Encryption**: ChaCha8 Stream Cipher (seeded by Timestamp + Key File).
    - **加密**: ChaCha8 流密码（由时间戳 + 密钥文件播种）。
4.  **Header Construction**: Magic "SPNG" + Metadata + Hash.
    **头构建**: 魔数 "SPNG" + 元数据 + 哈希。
5.  **Embedding**:
    **嵌入**:
    - **WAV Mode**: Data embedded in LSB of 32-bit integer samples.
    - **WAV 模式**: 数据嵌入在 32 位整数样本的 LSB 中。
    - **PNG Mode**: Data embedded in Low Byte of 16-bit RGBA channels (RGB only).
    - **PNG 模式**: 数据嵌入在 16 位 RGBA 通道的低字节中（仅 RGB）。
6.  **Output**: 32-bit WAV or 16-bit PNG.
    **输出**: 32 位 WAV 或 16 位 PNG。

#### Decoding / 解码
1.  **Analysis**: Reads header to detect Encryption flag.
    **分析**: 读取头以检测加密标志。
2.  **Extraction**: Reads raw stream from container.
    **提取**: 从容器中读取原始流。
3.  **Validation**: Checks Header Magic.
    **验证**: 检查头魔数。
4.  **Decryption**: Uses Key File (if required) to decrypt.
    **解密**: 使用密钥文件（如果需要）进行解密。
5.  **Integrity Check**: Compares calculated Hash vs Header Hash.
    **完整性检查**: 比较计算的哈希与头哈希。
6.  **Decompression**: Inflates payload.
    **解压**: 解压负载。
7.  **Output**: Restores files.
    **输出**: 恢复文件。

### Key Technical Decisions / 关键技术决策

#### PNG Embedding (High Density) / PNG 嵌入（高密度）
- **Container**: 16-bit RGBA.
- **容器**: 16 位 RGBA。
- **Strategy**: Store 3 bytes of data per pixel (Low Bytes of R, G, B).
- **策略**: 每个像素存储 3 字节数据（R, G, B 的低字节）。
- **Alpha Channel**: Ignored (kept opaque) to prevent data loss from premultiplication or transparency optimizations in image libraries.
- **Alpha 通道**: 忽略（保持不透明），以防止图像库中的预乘或透明度优化导致数据丢失。
- **Iteration**: Strict Row-Major iteration using `into_raw()` (Decoder) and explicit loops (Encoder) to ensure byte alignment.
- **迭代**: 使用 `into_raw()` (解码器) 和显式循环 (编码器) 进行严格的行主序迭代，以确保字节对齐。
- **Auto-Expand**: If payload > capacity, the image is upscaled using `Lanczos3` filter to provide sufficient pixels.
- **自动扩容**: 如果负载 > 容量，则使用 `Lanczos3` 滤波器放大图片以提供足够的像素。

#### WAV Embedding / WAV 嵌入
- **Container**: 32-bit Integer WAV.
- **容器**: 32 位整数 WAV。
- **Strategy**: Store 2 bytes of data per sample (Lower 16 bits). High 16 bits contain the original audio (slightly noisy but playable).
- **策略**: 每个样本存储 2 字节数据（低 16 位）。高 16 位包含原始音频（有轻微噪音但可播放）。

### Security / 安全
- **ChaCha8**: Fast, cryptographically secure stream cipher.
- **ChaCha8**: 快速、加密安全的流密码。
- **Key Mixing**: Physical Key File bytes are XORed into the ChaCha8 stream, ensuring that even if the Timestamp is known, the stream cannot be reproduced without the file.
- **密钥混合**: 物理密钥文件字节被异或到 ChaCha8 流中，确保即使时间戳已知，如果没有文件也无法重现流。