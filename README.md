# Sound PNG - 通用隐写工具 (Universal Steganography Tool)

Sound PNG 是一款先进的跨平台隐写术工具，旨在安全、高效地将任意文件隐藏到 WAV 音频、PNG 图片或自定义容器格式中。

**最新版本 (Latest Version): v1.3.1**

---

## 核心特性 (Key Features)

### 1. 通用隐写模式 (Universal Steganography Mode)
- **多格式支持**: 支持将**任意格式**的文件（负载）隐藏到 **WAV 音频** 或 **PNG 图片**（容器）中。
- **透明加密与压缩**: 
  - 所有负载数据在嵌入前自动经过 **Deflate** 算法进行无损压缩，最大化利用容器空间。
  - 支持 **AES 加密**（可选），需提供密钥文件，确保数据即使被提取也无法解读。
- **流式处理架构 (True Streaming Pipeline)**: 
  - 采用内存高效的流式读写技术，支持处理 **GB 级**甚至更大的文件，内存占用极低，仅受磁盘空间限制。
- **智能容量预检**: 
  - 内置智能算法自动计算容器的最大可用容量（WAV 采样点 / PNG 像素），在编码前校验负载大小，防止数据溢出损坏容器。

### 2. 强大的插件系统 (Plugin System)
Sound PNG v1.3.1 引入了高度可扩展的插件架构，支持动态加载外部功能：
- **序列帧模式 (Sequence Frame Plugin)**: 
  - 专为视频隐写设计。支持将超大文件分割并分散隐藏到一个 **PNG 序列帧文件夹**中（例如从视频提取的每一帧）。
  - 支持从文件夹自动读取并按顺序重组恢复原始文件。
- **Python 桥接 (Python Bridge Plugin)**:
  - **无限扩展**: 允许调用本地 Python 环境进行自定义编码/解码。
  - **本地通信**: 通过本地 REST API (默认端口 6657) 与 Python 进程通信，支持集成复杂的 AI 模型（如深度学习隐写）或自定义加密算法。
  - **灵活配置**: 支持在设置中自定义 Python 解释器路径和通信端口，并提供独立的控制台窗口查看 Python 运行日志。

### 3. 高效批量处理 (Batch Processing)
- **批量编码**: 支持一次性导入多个负载文件，并将它们分别隐藏到同一个容器模板中，自动批量生成对应的隐写文件。
- **批量解码**: 支持批量导入多个隐写文件，一键提取所有隐藏内容到指定目录。

### 4. 专业开发者工具 (Developer Tools)
- **实时控制台**: 在设置中开启“开发者模式”后，主界面底部将显示实时日志控制台。
- **彩色日志**: 日志系统支持 INFO, WARN, ERROR 等级别，并以不同颜色高亮显示，方便调试和追踪错误。

---

## 文档资源 (Documentation)

详细的文档已移动至应用程序源码目录下：

- **用户手册 (User Manual)**: [app/docs/User_Manual.md](app/docs/User_Manual.md)
  - 包含详细的操作指南、界面说明和常见问题解答。
- **开发者文档 (Developer Documentation)**: [app/docs/Developer_Documentation.md](app/docs/Developer_Documentation.md)
  - 包含架构设计、插件开发指南和 API 参考。

---

## 快速开始 (Quick Start)

### 编码 (Encoding)
1. 启动程序，进入 **Encode (编码)** 标签页。
2. **Payload**: 选择要隐藏的文件。
3. **Container**: 选择载体文件 (PNG/WAV)。
4. (可选) **Encryption**: 勾选并选择密钥文件。
5. 点击 **Encode**。

### 解码 (Decoding)
1. 进入 **Decode (解码)** 标签页。
2. **Input**: 选择隐写后的文件。
3. **Key**: (若加密) 提供正确的密钥文件。
4. **Output**: 设置输出路径。
5. 点击 **Decode**。

### Python 插件配置
1. 确保已安装 Python 3.x。
2. 在 **Settings (设置)** -> **Plugins** 中启用 "Python Bridge Plugin"。
3. 在下方配置区域点击 "Browse" 选择 `python.exe` 路径。
4. 编码/解码操作将自动调用 Python 后端。

---

## 常见问题 (FAQ)

**Q: 为什么提示 "Payload too large"?**
A: 容器容量必须大于负载（压缩后）。建议使用更高分辨率的图片或更长的音频。

**Q: 插件无法加载？**
A: 请确保 `Plugins` 文件夹完整，且 Python 环境路径配置正确。

---

## 许可证 (License)

MIT License