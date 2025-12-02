use anyhow::{Context, Result};
use std::path::Path;
use std::fs::File;
use std::io::Read;
use minimp3::{Decoder, Frame, Error};

use image::{DynamicImage, io::Reader as ImageReader};

/// 加载图像为 DynamicImage 对象，用于像素级操作
pub fn load_image_object(path: &Path) -> Result<DynamicImage> {
    // println!("Loading Image: {:?}", path);
    let mut reader = ImageReader::open(path).context(format!("Failed to open image file: {:?}", path))?.with_guessed_format()?;
    
    if reader.format().is_none() {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if ext.eq_ignore_ascii_case("png") {
                reader.set_format(image::ImageFormat::Png);
            } else if ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg") {
                reader.set_format(image::ImageFormat::Jpeg);
            }
        }
    }

    let image = reader.decode().context("Failed to decode image")?;
    Ok(image)
}

/// 加载音频数据为 16-bit PCM 样本
/// 支持 .wav (通过 hound) 和 .mp3 (通过 minimp3)
pub fn load_audio_as_pcm(path: &Path) -> Result<Vec<i16>> {
    let extension = path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "wav" => load_wav(path),
        "mp3" => load_mp3(path),
        _ => Err(anyhow::anyhow!("Unsupported audio format: {}", extension)),
    }
}

/// 加载图像或其他文件为原始字节流
/// 目前所有非音频文件都作为原始字节流处理（即 Payload）
pub fn load_file_as_bytes(path: &Path) -> Result<Vec<u8>> {
    let mut file = File::open(path).context("Failed to open file")?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).context("Failed to read file")?;
    Ok(buffer)
}

fn load_wav(path: &Path) -> Result<Vec<i16>> {
    // Use the robust normalization logic from utils
    // Note: We discard the spec here as the caller currently infers it or uses default.
    // Ideally we'd return (Spec, Data) from load_audio_as_pcm too.
    let path_buf = path.to_path_buf();
    let (_, samples) = crate::utils::read_and_normalize_wav(&path_buf)?;
    Ok(samples)
}

fn load_mp3(path: &Path) -> Result<Vec<i16>> {
    let file = File::open(path).context("Failed to open MP3 file")?;
    let mut decoder = Decoder::new(file);
    let mut pcm_data = Vec::new();

    loop {
        match decoder.next_frame() {
            Ok(Frame { data, .. }) => {
                pcm_data.extend(data);
            },
            Err(Error::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("Error decoding MP3: {:?}", e)),
        }
    }
    Ok(pcm_data)
}
