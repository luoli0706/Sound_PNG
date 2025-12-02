# Sound_PNG API Reference

## Core Concepts

The Plugin API revolves around the `ContainerEncoder` and `ContainerDecoder` traits. Plugins are dynamically loaded libraries (`.sn` extension, which are standard shared libraries).

## Traits

### `ContainerEncoder`

```rust
pub trait ContainerEncoder: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    fn supported_extensions(&self) -> Vec<String>;
    fn encode(
        &self, 
        container_path: &Path, 
        output_path: &Path, 
        byte_stream: &mut ByteStream<File>,
        on_progress: Box<dyn Fn(f32) + Send>
    ) -> Result<()>;
}
```

### `ContainerDecoder`

```rust
pub trait ContainerDecoder: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    fn supported_extensions(&self) -> Vec<String>;
    fn decode(
        &self,
        input_path: &Path,
        on_progress: Box<dyn Fn(f32) + Send>
    ) -> Result<Box<dyn Read + Send>>;
}
```

## Plugin Entry Points

Plugins must export the following symbols:

```rust
#[no_mangle]
pub extern "Rust" fn _create_encoder() -> Box<dyn ContainerEncoder>;

#[no_mangle]
pub extern "Rust" fn _create_decoder() -> Box<dyn ContainerDecoder>;
```

## ByteStream

The `ByteStream` struct provides a high-level interface for reading bytes to be embedded. It handles:
- Header management
- Payload reading
- Encryption (ChaCha8)
- Buffer management

Plugins should call `byte_stream.next_byte()` to get the next byte to embed.

## Sequence Frame Plugin (Built-in Example)

This plugin demonstrates how to handle a directory as a container.
- **Container:** A directory containing ordered PNG files (`001.png`, `002.png`...).
- **Logic:** Distributes the payload evenly across all images in the folder.
- **UI:** When enabled, a "Sequence Mode" checkbox appears in Universal Mode.
