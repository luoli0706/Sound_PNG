# Sound_PNG Developer Documentation / Sound_PNG 开发者文档

## Versioning Standard / 版本规范
- **Format**: `vX.Y.Z-beta` or `vX.Y.Z`.
- **Current Version**: `v1.3.0-beta`.

## v1.3.0-beta Architecture

### Plugin System / 插件系统
Sound_PNG now supports dynamic extensions via shared libraries (`.dll` on Windows, `.so` on Linux) renamed to `.sn`.

- **Loader**: The main application scans the `Plugins` directory at startup.
- **Interface**: Plugins must implement the `ContainerEncoder` trait defined in `sound_png_api`.
- **ABI**: Currently uses Rust ABI (requires exact compiler version match). Future versions may stabilize this with C-ABI.

### Creating a Plugin / 创建插件
1. Create a new Rust library crate.
2. Add `sound_png_api` as a dependency.
3. Set `crate-type = ["cdylib"]`.
4. Implement `ContainerEncoder` and export a `_create_encoder` symbol.

```rust
#[no_mangle]
pub extern "Rust" fn _create_encoder() -> Box<dyn ContainerEncoder> {
    Box::new(MyPluginEncoder)
}
```

### Core Modules (Shared)
- **sound_png_api**: Defines `ByteStream`, `ContainerEncoder` trait.
- **stream_encoder.rs**: Orchestrates the pipeline, delegates to plugins or built-in handlers.
- **stream_decoder.rs**: (Currently built-in only) Streaming decoding logic.

### Backend Implementation (Axum)
- **Handlers**: `encode_handler` and `decode_handler` use `multipart` to stream uploads to temporary files.
- **Resize Logic**: `decode_handler` accepts a `resize` parameter.

### Desktop UI (Slint)
- **Plugin Manager**: Loads `.sn` files and registers them.
- **Update Checker**: Checks GitHub Releases.
