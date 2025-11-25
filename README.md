# Sound PNG — Alpha 1.1 / 声音隐写工具 Alpha 1.1

Sound PNG is the alpha 1.1 build of a Rust + Slint desktop app for hiding 16-bit PNG byte streams inside 32-bit WAV containers with zero perceptible loss. / Sound PNG 是一个基于 Rust 与 Slint 的桌面应用，Alpha 1.1 版本实现了在 32-bit WAV 容器中无损封装 16-bit PNG 数据流。

> Built using BMAD Method workflows plus Google's Gemini CLI for planning, specification, and automated checks. / 本项目采用 **BMAD Method** 工作流与 **Google Gemini CLI** 协作完成规划、需求与自动化检视。

---

## Quick Start / 快速开始

1. **Install prerequisites / 安装依赖**
   - Rust 1.75+ toolchain with `cargo`
   - Windows 10/11、macOS 或主流 Linux（具备 Slint 所需桌面图形栈）
2. **Clone repository / 克隆仓库**
   ```powershell
   git clone https://github.com/luoli0706/Sound_PNG.git
   cd Sound_PNG/sound_png
   ```
3. **Run GUI encoder / 运行图形界面编码器**
   ```powershell
   cargo run --release
   ```
   - Select a 16-bit PCM WAV (carrier) and PNG (payload). / 通过界面选择 16-bit PCM WAV 载体与 PNG 负载。
4. **CLI decode / 命令行解码**
   ```powershell
   cargo run --release -- --mode decode --input <32bit.wav> --wav-out <wav_path> --png-out <png_path>
   ```
5. **Run tests / 执行测试**
   ```powershell
   cargo test
   ```

---

## Project Overview / 项目说明

- **Tech Stack / 技术栈**: Rust 2021, Slint UI, Clap, Hound, rfd, flate2.
- **Core Flow / 核心流程**: Split MSB/LSB from 16-bit streams, merge into 32-bit PCM samples, then reverse the bit operations for extraction.
- **Tooling / 开发工具**: Requirements captured via BMAD Method agent workflows; Gemini CLI drives scripted experiments and doc syncing.
- **Current Focus / 当前重点**: Stabilize encoder throughput, refine decoder validation, maintain responsive Slint UI for large WAV/PNG pairs.

---

## Alpha 1.1 Notes & Limitations / Alpha 1.1 说明与限制

- Experimental build; not production-ready. / 试验版本，暂不建议用于生产环境。
- Only supports 16-bit PCM WAV input and outputs signed 32-bit PCM WAV. / 仅支持 16-bit PCM 输入与 32-bit PCM (signed) 输出。
- PNG payload size must fit within WAV duration; no streaming mode yet. / PNG 负载需可完全容纳于 WAV 时长，尚未支持流式处理。
- No cryptographic protection; data is only hidden via bit interleaving. / 未提供加密，数据仅通过位拆分隐藏。
- Tested on Windows 11 + Rust 1.75; other platforms require additional validation. / 仅在 Windows 11 + Rust 1.75 上验证，其它平台尚需测试。

---

## Contributing / 贡献指南

1. Branch from `main` and keep commits focused. / 基于 `main` 建立分支并保持精简提交。
2. Run `cargo fmt && cargo clippy && cargo test` before pushing. / 提交前需通过格式化、静态检查与测试。
3. Document architecture changes (English or Chinese). / 更新架构或流程时请附中英文简要说明。

---

## License / 许可

MIT License — see `LICENSE`. / 采用 MIT 许可，详见 `LICENSE` 文件。