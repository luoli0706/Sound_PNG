# Sequence Frame Plugin / 序列帧插件

## Introduction / 简介
The **Sequence Frame Plugin** extends Sound_PNG to support **directory-based steganography**. Instead of hiding data in a single image or audio file, it distributes the payload across a sequence of PNG images found in a folder. This is ideal for hiding large files in video image sequences or animation frames without noticeably altering any single frame.

**序列帧插件**扩展了 Sound_PNG 以支持**基于目录的隐写术**。它不是将数据隐藏在单个图像或音频文件中，而是将负载分散在文件夹中的一系列 PNG 图像中。这非常适合将大文件隐藏在视频图像序列或动画帧中，而不会显着改变任何单个帧。

## Installation / 安装
1. Ensure `sequence_frame_plugin.sn` (or `.dll`/`.so` renamed to `.sn`) is in the `Plugins` folder next to the `Sound_PNG` executable.
2. Open Sound_PNG and go to the **Settings** tab.
3. Check the box for **"Sequence Frame Plugin"** to enable it.

1. 确保 `sequence_frame_plugin.sn`（或重命名为 `.sn` 的 `.dll`/`.so`）位于 `Sound_PNG` 可执行文件旁边的 `Plugins` 文件夹中。
2. 打开 Sound_PNG 并转到 **设置 (Settings)** 选项卡。
3. 勾选 **"Sequence Frame Plugin"** 以启用它。

## Usage / 使用方法

### Encoding (Hiding) / 编码（隐藏）
1. Go to **Universal Mode** -> **Encode**.
2. Check the **"Sequence Mode (Folder)"** box (only visible if plugin is enabled).
3. **Payload**: Select the file you want to hide.
4. **Container**: Select a **Folder** containing your PNG sequence.
   - **Requirement**: The folder must contain PNG files named in order (e.g., `frame_001.png`, `frame_002.png`, ...). The plugin processes them alphabetically.
5. **Output**: Select an empty **Folder** where the output images will be saved.
6. Click **Encode**. The plugin will recreate the PNG sequence in the output folder with the payload hidden inside.

1. 转到 **通用模式 (Universal Mode)** -> **编码 (Encode)**。
2. 勾选 **"序列模式 (文件夹) / Sequence Mode (Folder)"** 复选框（仅在插件启用时可见）。
3. **负载 (Payload)**: 选择您要隐藏的文件。
4. **容器 (Container)**: 选择包含 PNG 序列的 **文件夹**。
   - **要求**: 文件夹必须包含按顺序命名的 PNG 文件（例如 `frame_001.png`, `frame_002.png`...）。插件将按字母顺序处理它们。
5. **输出 (Output)**: 选择一个空的 **文件夹**，用于保存输出图像。
6. 点击 **执行编码 (Encode)**。插件将在输出文件夹中重建 PNG 序列，并将负载隐藏在其中。

### Decoding (Extracting) / 解码（提取）
1. Go to **Universal Mode** -> **Decode**.
2. Check the **"Sequence Mode (Folder)"** box.
3. **Input**: Select the **Folder** containing the encoded PNG sequence.
4. **Output**: Select the path to save the extracted file.
5. Click **Decode**.

1. 转到 **通用模式 (Universal Mode)** -> **解码 (Decode)**。
2. 勾选 **"序列模式 (文件夹) / Sequence Mode (Folder)"** 复选框。
3. **输入 (Input)**: 选择包含已编码 PNG 序列的 **文件夹**。
4. **输出 (Output)**: 选择保存提取文件的路径。
5. 点击 **执行解码 (Decode)**。

## Technical Details / 技术细节
- **Distribution**: The total size of the payload (plus header) is calculated and divided evenly among all valid PNG files found in the directory.
- **Capacity**: Total Capacity = (Width * Height * 3 bytes) * Number of Frames.
- **Format**: Output images are forced to 16-bit RGBA PNGs to maximize capacity and quality.

- **分布**: 负载（加上头信息）的总大小计算后，平均分配给目录中找到的所有有效 PNG 文件。
- **容量**: 总容量 = (宽 * 高 * 3 字节) * 帧数。
- **格式**: 输出图像强制为 16 位 RGBA PNG，以最大化容量和质量。
