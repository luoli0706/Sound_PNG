# Sound_PNG Developer Documentation / Sound_PNG 开发者文档

## Beta 2.0 Architecture / Beta 2.0 架构

### Universal Encoding / 通用编码
The core logic (`encoder.rs`) has been refactored to `encode_data` which accepts raw bytes `&[u8]` as payload.
核心逻辑 (`encoder.rs`) 已重构为 `encode_data`，接受原始字节 `&[u8]` 作为负载。

- **Payload Agnostic**: The system treats payload as a binary blob. It compresses it using Deflate before embedding.
- **负载无关**: 系统将负载视为二进制块。在嵌入之前使用 Deflate 对其进行压缩。
- **Container Logic**:
    - If Container is PNG -> Uses `encode_as_png` (LSB Low Byte of 16-bit RGBA).
    - If Container is WAV -> Uses `encode_as_wav` (LSB 16-bit of 32-bit PCM).

### PNG Robustness / PNG 鲁棒性
- **Alpha Channel**: We force Alpha to `0xFFFF` (Opaque) in the output PNG. This prevents image libraries from optimizing away RGB values of transparent pixels, ensuring 100% data retention.
- **Alpha 通道**: 我们在输出 PNG 中强制 Alpha 为 `0xFFFF` (不透明)。这可以防止图像库优化掉透明像素的 RGB 值，确保 100% 数据保留。
- **Decoder**: Uses `load_from_memory_with_format` to ensure correct format detection even for generated buffers.
- **解码器**: 使用 `load_from_memory_with_format` 确保即使对于生成的缓冲区也能正确检测格式。

### Universal Decoding / 通用解码
- **Payload Extraction**: Decoupled from file extension. Extracts binary blob.
- **负载提取**: 与文件扩展名解耦。提取二进制块。
- **Container Restoration**: Optional. User can choose to restore the "clean" container or just the payload.
- **容器恢复**: 可选。用户可以选择恢复“干净”的容器或仅恢复负载。
- **File Type Preservation**: The extension of the original payload is stored in the header (8 bytes).
- **文件类型保留**: 原始负载的扩展名存储在头文件中 (8 字节)。

### UI & UX / 用户界面与体验
- **Slint UI**: Fully reactive UI with nested TabWidgets.
- **In-App Manual**: Documentation is embedded into the binary (`include_str!`) and rendered using a custom Markdown parser.
- **内置说明书**: 文档嵌入到二进制文件中 (`include_str!`) 并使用自定义 Markdown 解析器渲染。
- **Progress Indication**: Real-time progress bars for encoding/decoding operations via thread messaging.
- **进度指示**: 通过线程消息传递实现编码/解码操作的实时进度条。