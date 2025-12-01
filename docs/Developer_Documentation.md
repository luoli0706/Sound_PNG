# Sound_PNG Developer Documentation / Sound_PNG 开发者文档

## Beta 3.0 Architecture: Streaming Pipeline

### Core Concept
The entire encoding/decoding process has been rewritten to use `std::io::Read` and `std::io::Write` traits, eliminating intermediate `Vec<u8>` buffers.

### Modules
- **stream_encoder.rs**:
    - `encode_stream`: Orchestrates the pipeline.
    - `ByteStream`: A state machine that acts as a `Read` iterator. It streams bytes sequentially from:
        1. **Header** (Memory)
        2. **Payload** (File -> Deflate -> Encrypt -> ByteStream)
    - **Embedding**: Uses `png::Writer` (row-by-row) or `hound::WavWriter` (sample-by-sample) to embed data from `ByteStream` into the container stream.

- **stream_decoder.rs**:
    - `decode_stream`: Orchestrates the pipeline.
    - `ContainerReader`: A `Read` implementation that extracts LSBs from PNG rows or WAV samples on-the-fly.
    - `DecryptReader`: Wraps `ContainerReader` and applies ChaCha8 XOR decryption.
    - **Decompression**: `flate2::read::DeflateDecoder` wraps `DecryptReader`.
    - **Output**: Streams directly to `File::create`.

### Batch Processing
Batch operations reuse the streaming logic by iterating over input lists and managing output paths.
- **Naming Convention**: `{InputName}_{Suffix}.{Ext}` logic is handled in `gui.rs`.

### Memory Footprint
- **Beta 2.0**: `O(FileSize)` (e.g., 1GB file -> 2GB+ RAM).
- **Beta 3.0**: `O(BufferSize)` (Constant ~32MB, configurable).

### UI Logic
- **Resize Visibility**: The image resizing dropdown is conditionally rendered only when `uni-decode-preset-index == 1` (PNG).