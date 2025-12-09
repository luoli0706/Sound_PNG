# Sound PNG 开发者文档 (Developer Documentation)

**版本**: v1.3.1

## 目录
1. [架构概览](#1-架构概览)
2. [核心模块](#2-核心模块)
3. [流式处理管道](#3-流式处理管道)
4. [插件系统开发](#4-插件系统开发)
5. [Python 桥接协议](#5-python-桥接协议)
6. [构建指南](#6-构建指南)

---

## 1. 架构概览 (Architecture Overview)
Sound PNG v1.3.1 采用模块化设计，核心逻辑与 UI 分离，通过 Rust 的 Channel 进行通信。

- **App (GUI)**: 基于 Slint UI 框架，负责用户交互、状态管理和配置。
- **Core (Rust)**: 负责文件 I/O、加密 (AES)、压缩 (Deflate)、LSB 嵌入/提取。
- **API (sound_png_api)**: 定义了插件接口 (`ContainerEncoder`, `ContainerDecoder`) 和共享数据结构 (`ByteStream`)。
- **Plugins**: 动态链接库 (DLL/SO)，扩展核心功能。

---

## 2. 核心模块 (Core Modules)
- **`stream_encoder.rs`**: 实现了通用的编码流程。
  1. 读取负载流 -> Deflate 压缩 -> 计算 Hash -> AES 加密。
  2. 生成头部信息 (`Header`)。
  3. 构建 `ByteStream`（按位/字节流）。
  4. 将 `ByteStream` 嵌入到容器中（WAV/PNG）。
- **`stream_decoder.rs`**: 实现了通用的解码流程。
  1. 从容器提取 LSB 数据 -> 组装 `ByteStream`。
  2. 解析头部信息 -> 校验 Hash。
  3. AES 解密 -> Deflate 解压 -> 写入输出文件。
- **`plugin_loader.rs`**: 基于 `libloading` 实现的插件管理器，负责扫描 `Plugins` 目录并加载符合 ABI 的动态库。

---

## 3. 流式处理管道 (Streaming Pipeline)
为了支持超大文件（>RAM），v1.3.1 彻底重构了读写逻辑。

- **ByteStream**: 一个核心迭代器，它封装了压缩、加密和头部逻辑。上层消费者只需调用 `next_byte()` 即可获取下一个待嵌入的字节，无需关心底层复杂的转换。
- **内存占用**: 理论上仅需维持缓冲区大小（默认 64KB），可处理 TB 级文件。

---

## 4. 插件系统开发 (Plugin Development)
开发者可以通过实现 `sound_png_api` 定义的 Trait 来创建插件。

### 接口定义
```rust
pub trait ContainerEncoder {
    fn metadata(&self) -> PluginMetadata;
    fn supported_extensions(&self) -> Vec<String>;
    fn encode(&self, container: &Path, output: &Path, stream: &mut ByteStream, cb: ProgressCallback) -> Result<()>;
}
```

### 导出符号
插件必须导出以下 C ABI 符号：
- `_create_encoder`
- `_create_decoder`

### 示例
参考 `plugins/sequence_frame` 的实现。

---

## 5. Python 桥接协议 (Python Bridge Protocol)
Python 桥接插件通过本地 REST API 与 Python 进程通信。

- **端口**: 默认 6657 (可配置)。
- **启动**: Rust 主程序通过 `Command::spawn` 启动 `server.py`。
- **环境变量**: 启动时注入 `SPNG_PYTHON_PATH` 和 `SPNG_PYTHON_PORT`。

### API 端点
- **POST /encode**:
  - Body: `{ "container": "path", "output": "path", "payload_tmp": "path_to_temp_payload" }`
  - 逻辑: Python 脚本读取 `container` 和 `payload_tmp`，处理后写入 `output`。
- **POST /decode**:
  - Body: `{ "input": "path" }`
  - Response: 二进制流（解密后的负载数据）。

---

## 6. 构建指南 (Build Guide)

### 环境要求
- Rust (Stable)
- Slint 依赖 (Qt5/6 或系统原生库)
- Python 3.x (用于桥接插件)

### 编译命令
```bash
# 编译主程序
cargo build --release --bin sound_png

# 编译插件
cargo build --release --lib -p sn_py_bridge
cargo build --release --lib -p sequence_frame_plugin
```

### 打包结构
```
/
  Sound_PNG.exe
  Plugins/
    sn_py_bridge.dll
    sequence_frame.dll
    server.py
  docs/
    User_Manual.md
```