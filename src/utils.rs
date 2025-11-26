use anyhow::{anyhow, Context, Result};
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use std::fs;
use std::path::PathBuf;

/// Reads a WAV file and normalizes its samples to 16-bit integers.
/// Supports 16-bit Int, 24-bit Int, 32-bit Int, and 32-bit Float formats.
pub fn read_and_normalize_wav(path: &PathBuf) -> Result<(WavSpec, Vec<i16>)> {
    let mut reader = WavReader::open(path).context("Failed to open WAV file.")?;
    let spec = reader.spec();

    let samples = match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .map(|s| s.context("Failed to read i16 sample"))
            .collect::<Result<Vec<_>>>()?,
        (SampleFormat::Int, 24) => reader
            .samples::<i32>()
            .map(|s| {
                s.map(|sample| (sample >> 8) as i16)
                    .context("Failed to read i24 sample")
            })
            .collect::<Result<Vec<_>>>()?,
        (SampleFormat::Int, 32) => reader
            .samples::<i32>()
            .map(|s| {
                s.map(|sample| (sample >> 16) as i16)
                    .context("Failed to read i32 sample")
            })
            .collect::<Result<Vec<_>>>()?,
        (SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .map(|s| {
                s.map(|sample| (sample * i16::MAX as f32) as i16)
                    .context("Failed to read f32 sample")
            })
            .collect::<Result<Vec<_>>>()?,
        _ => {
            return Err(anyhow!(
                "Unsupported sample format: {:?} with {}-bit samples",
                spec.sample_format,
                spec.bits_per_sample
            ));
        }
    };

    Ok((spec, samples))
}

// The old function is kept for compatibility, but now uses the new normalization.
#[allow(dead_code)]
pub fn read_wav_16bit(path: &PathBuf) -> anyhow::Result<(WavSpec, Vec<i16>)> {
    read_and_normalize_wav(path)
}

#[allow(dead_code)]
pub fn read_png(path: &PathBuf) -> anyhow::Result<Vec<u8>> {
    let data = fs::read(path)?;
    Ok(data)
}

pub fn write_wav_32bit(path: &PathBuf, mut spec: WavSpec, data: &[i32]) -> anyhow::Result<()> {
    spec.sample_format = hound::SampleFormat::Int;
    spec.bits_per_sample = 32;
    let mut writer = WavWriter::create(path, spec)?;
    for &sample in data {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(())
}

pub fn write_wav_16bit(path: &PathBuf, mut spec: WavSpec, data: &[i16]) -> anyhow::Result<()> {
    spec.sample_format = hound::SampleFormat::Int;
    spec.bits_per_sample = 16;
    let mut writer = WavWriter::create(path, spec)?;
    for &sample in data {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(())
}

pub fn write_png(path: &PathBuf, data: &[u8]) -> anyhow::Result<()> {
    fs::write(path, data)?;
    Ok(())
}
