# Sound_PNG User Manual / Sound_PNG 用户手册

## Overview / 概述
Sound_PNG allows you to hide files inside other files securely.
Sound_PNG 允许您安全地将文件隐藏在其他文件中。
- **Hide a Picture inside a Voice file (WAV).**
- **将图片隐藏在语音文件 (WAV) 中。**
- **Hide a Voice file inside a Picture (PNG).**
- **将语音文件隐藏在图片 (PNG) 中。**

## Encoding (Hiding Data) / 编码（隐藏数据）

1.  **Launch the App**: Run `Sound_PNG_Beta_1_0.exe`.
    **启动应用**: 运行 `Sound_PNG_Beta_1_0.exe`。
2.  **Select Tab**: Go to the **Encode** tab.
    **选择标签页**: 进入 **Encode** (编码) 标签页。
3.  **Select Inputs**:
    **选择输入**:
    - **Input Voice**: Click "Browse" to select your Audio container (or payload).
    - **输入语音**: 点击 "Browse" 选择您的音频容器（或负载）。
    - **Input Picture**: Click "Browse" to select your Image container (or payload).
    - **输入图片**: 点击 "Browse" 选择您的图片容器（或负载）。
4.  **Security (Optional)**:
    **安全（可选）**:
    - Check "Enable High Security" to encrypt the data.
    - 勾选 "Enable High Security" (启用高安全性) 以加密数据。
    - If checked, you can optionally select a **Key File**. This acts as a physical key. You MUST have this file to decode later.
    - 如果勾选，您可以选择一个 **Key File** (密钥文件)。这充当物理密钥。您必须拥有此文件才能在稍后解码。
5.  **Select Format**:
    **选择格式**:
    - **WAV**: Hides the Picture inside the Voice.
    - **WAV**: 将图片隐藏在语音中。
    - **PNG**: Hides the Voice inside the Picture. (The picture will be resized if the voice file is large).
    - **PNG**: 将语音隐藏在图片中。（如果语音文件较大，图片将自动调整大小）。
6.  **Save Output**: Click "Save..." to choose where to save the encoded file.
    **保存输出**: 点击 "Save..." 选择保存编码文件的位置。
7.  **Encode**: Click the **Encode** button. Wait for "Encoding complete!".
    **编码**: 点击 **Encode** 按钮。等待 "Encoding complete!" (编码完成)。

## Decoding (Restoring Data) / 解码（恢复数据）

1.  **Select Tab**: Go to the **Decode** tab.
    **选择标签页**: 进入 **Decode** (解码) 标签页。
2.  **Select Input**: Click "Browse" to select the encoded file (WAV or PNG).
    **选择输入**: 点击 "Browse" 选择已编码的文件 (WAV 或 PNG)。
3.  **Analysis**:
    **分析**:
    - The app will automatically analyze the file.
    - 应用将自动分析文件。
    - If it says **"File Encrypted. Key Required."**, you MUST select the same **Key File** used during encoding.
    - 如果提示 **"File Encrypted. Key Required."** (文件已加密。需要密钥)，您必须选择编码时使用的同一个 **Key File**。
    - If it says "File Clean", no key is needed.
    - 如果提示 "File Clean" (文件未加密)，则不需要密钥。
4.  **Select Outputs**:
    **选择输出**:
    - **Save Voice**: Choose where to save the restored audio.
    - **保存语音**: 选择保存恢复后音频的位置。
    - **Save Picture**: Choose where to save the restored image.
    - **保存图片**: 选择保存恢复后图片的位置。
5.  **Decode**: Click **Decode**.
    **解码**: 点击 **Decode** 按钮。
6.  **Result**: If successful, you will see "Decoding complete!". If the data was tampered with or the key is wrong, you will see a "Security Alert".
    **结果**: 如果成功，您将看到 "Decoding complete!" (解码完成)。如果数据被篡改或密钥错误，您将看到 "Security Alert" (安全警报)。

## FAQ / 常见问题

**Q: Why is my PNG output larger than the original image?**
**问: 为什么我的 PNG 输出比原始图片大？**
A: If the audio file you are hiding is large, Sound_PNG automatically enlarges the image canvas to make space for the data.
答: 如果您隐藏的语音文件较大，Sound_PNG 会自动放大图片画布以为数据腾出空间。

**Q: Can I use a JPG as a container?**
**问: 我可以使用 JPG 作为容器吗？**
A: Yes, you can select a JPG as Input Picture. However, the Output will always be a **PNG** (Lossless) to ensure data integrity.
答: 是的，您可以选择 JPG 作为输入图片。但是，输出始终为 **PNG**（无损），以确保数据完整性。