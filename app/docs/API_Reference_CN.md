# Sound_PNG API 参考文档

## 核心概念

插件 API 围绕 `ContainerEncoder` 和 `ContainerDecoder` 特质构建。插件是动态加载的库（`.sn` 扩展名，即标准共享库）。

## 特质 (Traits)

### `ContainerEncoder` (容器编码器)

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

### `ContainerDecoder` (容器解码器)

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

## 插件入口点

插件必须导出以下符号：

```rust
#[no_mangle]
pub extern "Rust" fn _create_encoder() -> Box<dyn ContainerEncoder>;

#[no_mangle]
pub extern "Rust" fn _create_decoder() -> Box<dyn ContainerDecoder>;
```

## ByteStream (字节流)

`ByteStream` 结构体提供了一个高级接口，用于读取待嵌入的字节。它处理：
- 头信息管理
- 负载读取
- 加密 (ChaCha8)
- 缓冲区管理

插件应调用 `byte_stream.next_byte()` 来获取下一个要嵌入的字节。

## 序列帧插件 (内置示例)

此插件演示了如何将目录作为容器处理。
- **容器**: 包含有序 PNG 文件（`001.png`, `002.png`...）的目录。
- **逻辑**: 将负载均匀分布在文件夹中的所有图像上。
- **UI**: 启用时，通用模式下会出现“序列模式”复选框。
