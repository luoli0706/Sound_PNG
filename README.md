# Sound PNG — Alpha 1.1

Sound PNG is the alpha 1.1 release of a Rust + Slint desktop tool for hiding 16-bit PNG byte streams inside 32-bit WAV containers without sacrificing fidelity. The encoder splits incoming audio/video data into MSB/LSB halves, merges them into a single stream, and lets you recover both sources losslessly.

## Quick Start

1. **Install prerequisites**
   - Rust toolchain 1.75+ with `cargo`
   - Windows 10/11, macOS, or Linux with desktop GPU drivers for Slint
2. **Clone & enter the project**
   ```powershell
   git clone https://github.com/luoli0706/Sound_PNG.git
   cd Sound_PNG/sound_png
   ```
3. **Run the app**
   ```powershell
   cargo run --release
   ```
   - Use the UI file pickers to supply a 16-bit PCM WAV (carrier) and PNG (payload).
4. **Decode**
   ```powershell
   cargo run --release -- --mode decode --input <32bit.wav> --wav-out <wav_path> --png-out <png_path>
   ```
5. **Execute tests (optional)**
   ```powershell
   cargo test
   ```

## Project Overview

- **Tech Stack**: Rust 2021, Slint UI, Clap CLI, and Hound for WAV handling.
- **Core Flow**: Combine WAV MSBs and PNG LSBs into a 32-bit PCM stream, then reverse the bit operations during extraction.
- **Current Goals**: Stabilize encoder/decoder performance, validate 32-bit PCM output, and keep the UI responsive for large assets.
- **Release Notes**: Alpha 1.1 focuses on build reproducibility, improved CLI flags, and updated dependency versions for Slint and Clap.

## Contributing

1. Create feature branches from `main`.
2. Run `cargo fmt && cargo clippy && cargo test`.
3. Submit a PR with a brief architecture note.

## License

This sub-project ships under the MIT License (see `LICENSE`).