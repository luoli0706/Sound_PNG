# Sound_PNG User Manual / Sound_PNG 用户手册

## Modes / 模式

### 1. Standard Mode (Voice/Picture) / 标准模式（语音/图片）
Designed for the classic use case: hiding a picture in a song, or a song in a picture.
专为经典用例设计：在歌曲中隐藏图片，或在图片中隐藏歌曲。

- **Encode**: Select "Voice" and "Picture". The app decides which is the container based on output format.
- **编码**: 选择 "Voice" (语音) 和 "Picture" (图片)。应用根据输出格式决定哪个是容器。
- **Decode**: Restore both files.
- **解码**: 恢复两个文件。

### 2. Universal Mode (Any File) / 通用模式（任意文件）
Designed for advanced users. Hide ANY file inside a PNG or WAV.
专为高级用户设计。将任何文件隐藏在 PNG 或 WAV 中。

- **Payload**: The file you want to hide (e.g., `secret.zip`).
- **负载**: 您想要隐藏的文件（例如 `secret.zip`）。
- **Container**: The file that will carry the secret (must be `.png` or `.wav`).
- **容器**: 承载秘密的文件（必须是 `.png` 或 `.wav`）。
- **Output**: The resulting file (looks like the Container).
- **输出**: 结果文件（看起来像容器）。
- **Extract Mode**: Choose how to save the decoded payload.
    - **Auto**: Automatically detects the original extension.
    - **Force PNG/ZIP/etc**: Forces the output to have a specific extension.
- **提取模式**: 选择如何保存解码后的负载。
    - **Auto**: 自动检测原始扩展名。
    - **Force PNG/ZIP/etc**: 强制输出具有特定扩展名。

**Homomorphic Steganography**: You can hide a PNG inside another PNG using this mode!
**同态隐写**: 您可以使用此模式将一个 PNG 隐藏在另一个 PNG 中！

## Analysis / 分析
When decoding, click "Browse" to select the file. The app will analyze the header and tell you if it's encrypted.
解码时，点击 "Browse" 选择文件。应用将分析头并告诉您是否已加密。
- **"File Encrypted"**: You MUST provide the Key File.
- **"File Encrypted"**: 您必须提供密钥文件。
- **"File Clean"**: No key needed.
- **"File Clean"**: 不需要密钥。

## Settings / 设置
- **Language**: Switch between English and Chinese.
- **语言**: 在英文和中文之间切换。
- **Theme**: Switch between Light and Dark mode.
- **主题**: 在浅色和深色模式之间切换。
- **Manual**: View this manual directly in the app.
- **说明书**: 直接在应用中查看本说明书。