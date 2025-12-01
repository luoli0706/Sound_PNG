use anyhow::{anyhow, Context, Result};
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;

/// Reads a WAV file and returns an iterator over its normalized 16-bit samples.
/// Supports 16-bit Int, 24-bit Int, 32-bit Int, and 32-bit Float formats.
/// This avoids loading all samples into memory.
pub struct WavIterator {
    reader: WavReader<BufReader<File>>,
    spec: WavSpec,
}

impl WavIterator {
    pub fn new(path: &PathBuf) -> Result<Self> {
        let reader = WavReader::open(path).context("Failed to open WAV file.")?;
        let spec = reader.spec();
        Ok(Self { reader, spec })
    }

    pub fn spec(&self) -> WavSpec {
        self.spec
    }
    
    pub fn len(&self) -> u32 {
        self.reader.len()
    }

    pub fn into_iter(self) -> Box<dyn Iterator<Item = Result<i16>> + Send> {
        let spec = self.spec;
        let reader = self.reader;

        match (spec.sample_format, spec.bits_per_sample) {
            (SampleFormat::Int, 16) => Box::new(reader.into_samples::<i16>().map(|s| s.context("Read error"))),
            (SampleFormat::Int, 24) => Box::new(reader.into_samples::<i32>().map(|s| {
                s.context("Read error").map(|sample| (sample >> 8) as i16)
            })),
            (SampleFormat::Int, 32) => Box::new(reader.into_samples::<i32>().map(|s| {
                s.context("Read error").map(|sample| (sample >> 16) as i16)
            })),
            (SampleFormat::Float, 32) => Box::new(reader.into_samples::<f32>().map(|s| {
                s.context("Read error").map(|sample| (sample * i16::MAX as f32) as i16)
            })),
            _ => Box::new(std::iter::once(Err(anyhow!(
                "Unsupported format: {:?} {} bits",
                spec.sample_format,
                spec.bits_per_sample
            )))),
        }
    }
}

// Kept for legacy/non-streaming small files if needed, but implemented via iterator now to reduce duplication logic.
pub fn read_and_normalize_wav(path: &PathBuf) -> Result<(WavSpec, Vec<i16>)> {
    let iter = WavIterator::new(path)?;
    let spec = iter.spec();
    let samples = iter.into_iter().collect::<Result<Vec<_>>>()?;
    Ok((spec, samples))
}

// The old function is kept for compatibility.
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
