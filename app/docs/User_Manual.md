# Sound_PNG User Manual / Sound_PNG 用户手册

## Version: Beta 3.0 (Streaming Engine)

### New in Beta 3.0: True Streaming / 真·流式处理
- **Zero Memory Overhead**: Process files of ANY size (1GB, 10GB, 100GB...) with minimal RAM usage (approx 32MB).
- **零内存开销**: 处理任何大小的文件（1GB, 10GB, 100GB...）仅需极少内存（约 32MB）。
- **Speed**: Faster processing due to zero-copy pipeline.
- **速度**: 零拷贝管道带来更快的处理速度。

### Batch Mode / 批量模式 (New!)
Process multiple files at once.
一次处理多个文件。

- **Batch Encode**: Select multiple Payloads and one Container. The app will create multiple copies of the container, each hiding one payload.
- **批量编码**: 选择多个负载文件和一个容器文件。应用将创建容器的多个副本，每个副本隐藏一个负载。
- **Batch Decode**: Select multiple Encoded files. The app will extract payloads from all of them to a folder.
- **批量解码**: 选择多个已编码文件。应用将把所有负载提取到一个文件夹中。

## Modes / 模式

### 1. Standard Mode (Voice/Picture) / 标准模式（语音/图片）
Designed for the classic use case: hiding a picture in a song, or a song in a picture.
专为经典用例设计：在歌曲中隐藏图片，或在图片中隐藏歌曲。

### 2. Universal Mode (Any File) / 通用模式（任意文件）
Hide ANY file inside a PNG or WAV.
将任何文件隐藏在 PNG 或 WAV 中。

## Features / 特性

### Stream Buffer / 流缓冲区
In **Settings**, you can adjust the buffer size (default 64KB). Larger buffers might slightly improve speed on SSDs but use more RAM.
在**设置**中，您可以调整缓冲区大小（默认 64KB）。较大的缓冲区可能会略微提高 SSD 上的速度，但会占用更多内存。

### Image Resizing / 图片缩放 (Decode Only)
When "PNG" is selected in "Extract Mode", you can resize the extracted image (Original, 75%, 50%, 25%) to save disk space. This option is hidden for other file types.
当“提取模式”选择“PNG”时，您可以缩放提取的图片（原始大小，75%，50%，25%）以节省磁盘空间。此选项对其他文件类型隐藏。

### Extract Presets / 提取预设
Force the output extension if Auto-Detect fails or if you want to mask the type. Now supports **MP4**.
如果自动检测失败或您想伪装类型，可强制指定输出扩展名。现已支持 **MP4**。