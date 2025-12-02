use sound_png_api::{ContainerEncoder, ContainerDecoder, ByteStream, PluginMetadata};
use anyhow::{Result, Context, anyhow};
use std::fs::{self, File};
use std::path::Path;
use std::io::{Write, Read, BufReader};

struct SequenceFramePlugin;

impl SequenceFramePlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "Sequence Frame Plugin".to_string(),
            description: "Distributes payload across a sequence of PNG images (folder).".to_string(),
            version: "0.1.0".to_string(),
            author: "Sound_PNG Team".to_string(),
        }
    }
}

impl ContainerEncoder for SequenceFramePlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata()
    }

    fn supported_extensions(&self) -> Vec<String> {
        // Supports directories implicitly, but UI might check for specific files inside
        vec!["seq_dir".to_string()] // Custom marker for directory mode
    }

    fn encode(
        &self,
        container_path: &Path, // This should be a DIRECTORY containing sorted PNGs
        output_path: &Path,    // This should be a DIRECTORY to write output PNGs
        byte_stream: &mut ByteStream<File>,
        on_progress: Box<dyn Fn(f32) + Send + Sync>
    ) -> Result<()> {
        // 1. Scan Container Directory
        if !container_path.is_dir() {
            return Err(anyhow!("Container path must be a directory for Sequence Plugin."));
        }
        
        let mut png_files: Vec<_> = fs::read_dir(container_path)?
            .filter_map(|entry| entry.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map_or(false, |ext| ext == "png"))
            .collect();
            
        png_files.sort(); // Sort by name (001.png, 002.png...)
        
        if png_files.is_empty() {
            return Err(anyhow!("No PNG files found in container directory."));
        }

        if !output_path.exists() {
            fs::create_dir_all(output_path)?;
        }

        // 2. Calculate Distribution
        // ByteStream knows total length (header + payload).
        let total_len = byte_stream.total_len();
        let num_frames = png_files.len() as u64;
        
        // Bytes per frame (ceiling division to ensure we cover everything)
        // Actually, we should fill frames sequentially or evenly?
        // "Balanced Distribution" implies evenly.
        let bytes_per_frame = (total_len + num_frames - 1) / num_frames;
        
        let mut processed_bytes = 0u64;
        
        for (i, input_png) in png_files.iter().enumerate() {
            let file_name = input_png.file_name().unwrap();
            let output_png = output_path.join(file_name);
            
            // Open Input PNG
            let file_in = File::open(input_png)?;
            let decoder = png::Decoder::new(file_in);
            let mut reader = decoder.read_info()?;
            let info = reader.info().clone();
            
            // Prepare Output PNG
            let file_out = File::create(&output_png)?;
            let mut encoder = png::Encoder::new(file_out, info.width, info.height);
            encoder.set_color(png::ColorType::Rgba); // Force Output to RGBA 16-bit for capacity
            encoder.set_depth(png::BitDepth::Sixteen);
            let mut writer = encoder.write_header()?;
            
            // Read Input Image Data (assume fits in memory for now, row-by-row better)
            // Since `png` crate `next_frame` reads whole frame, we use that.
            // Optimally we should stream rows.
            
            let bpp_in = info.bytes_per_pixel();
            let mut raw_pixels = vec![0u8; reader.output_buffer_size()];
            reader.next_frame(&mut raw_pixels)?;
            
            // Output Buffer (16-bit RGBA = 8 bytes per pixel)
            let mut out_data = Vec::with_capacity((info.width * info.height * 8) as usize);
            
            let mut pixel_idx = 0;
            while pixel_idx < raw_pixels.len() {
                let r = raw_pixels[pixel_idx];
                let g = raw_pixels[pixel_idx+1];
                let b = raw_pixels[pixel_idx+2];
                // Handle Alpha if exists
                let a = if info.color_type == png::ColorType::Rgba { raw_pixels[pixel_idx+3] } else { 255 };
                
                pixel_idx += bpp_in;
                
                // Logic: Read next byte from stream IF we haven't exceeded this frame's quota
                // AND if the stream isn't exhausted.
                
                // Frame Quota Check
                // bytes_per_frame is how much we WANT to put in.
                // But `byte_stream` is a continuous stream.
                // We just pull from it. 
                // Wait, if we pull evenly, we might run out of pixels in one frame?
                // If frame is too small for `bytes_per_frame`, we fail?
                // For MVP, assume frames are large enough.
                
                // Actually, simpler logic:
                // Just write until `bytes_per_frame` is reached for this frame OR stream ends.
                // But wait, `byte_stream` is continuous. If we stop pulling, the next frame picks up.
                // BUT we need to pad?
                // If we stop embedding in the middle of an image, the rest of the image is just copy?
                // Yes.
                
                // Embed 3 bytes per pixel? Or 1?
                // Sound_PNG standard is 3 bytes (R,G,B LSBs) per pixel.
                
                let mut r16 = (r as u16) << 8;
                let mut g16 = (g as u16) << 8;
                let mut b16 = (b as u16) << 8;
                let a16 = (a as u16) << 8 | 0xFF; // Alpha opaque-ish or keep
                
                // Embed R
                if processed_bytes < (i as u64 + 1) * bytes_per_frame && processed_bytes < total_len {
                     let byte = byte_stream.next_byte();
                     r16 |= byte as u16;
                     processed_bytes += 1;
                } else {
                     // Pad or Just Copy?
                     // Sound_PNG usually pads with 0 if stream ends.
                     // Here we just leave LSB as 0 (effectively padding) or Copy original LSB?
                     // Original LSB of 8-bit expanded to 16-bit is 0. So it's 0.
                }
                
                // Embed G
                if processed_bytes < (i as u64 + 1) * bytes_per_frame && processed_bytes < total_len {
                     let byte = byte_stream.next_byte();
                     g16 |= byte as u16;
                     processed_bytes += 1;
                }
                
                // Embed B
                if processed_bytes < (i as u64 + 1) * bytes_per_frame && processed_bytes < total_len {
                     let byte = byte_stream.next_byte();
                     b16 |= byte as u16;
                     processed_bytes += 1;
                }
                
                out_data.write_all(&r16.to_be_bytes())?;
                out_data.write_all(&g16.to_be_bytes())?;
                out_data.write_all(&b16.to_be_bytes())?;
                out_data.write_all(&a16.to_be_bytes())?;
            }
            
            writer.write_image_data(&out_data)?;
            
            on_progress((i + 1) as f32 / num_frames as f32);
        }
        
        Ok(())
    }
}

// Decoder Implementation
impl ContainerDecoder for SequenceFramePlugin {
    fn metadata(&self) -> PluginMetadata {
        self.metadata()
    }

    fn supported_extensions(&self) -> Vec<String> {
        vec!["seq_dir".to_string()]
    }

    fn decode(
        &self,
        input_path: &Path,
        on_progress: Box<dyn Fn(f32) + Send + Sync>
    ) -> Result<Box<dyn Read + Send>> {
        // Input is a directory.
        // We need to chain readers for all PNGs in order.
        // `SequenceReader` struct that implements Read.
        
        if !input_path.is_dir() {
            return Err(anyhow!("Input must be a directory."));
        }
        
        let mut png_files: Vec<_> = fs::read_dir(input_path)?
            .filter_map(|entry| entry.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map_or(false, |ext| ext == "png"))
            .collect();
            
        png_files.sort();
        
        Ok(Box::new(SequenceReader::new(png_files, on_progress)))
    }
}

struct SequenceReader {
    files: Vec<std::path::PathBuf>,
    current_file_idx: usize,
    current_buffer: std::collections::VecDeque<u8>,
    // We need to read row-by-row to extract LSBs.
    // Since we need `Read`, we'll buffer one image at a time (or row).
    // For simplicity: Load whole image LSBs into buffer when needed.
    on_progress: Box<dyn Fn(f32) + Send + Sync>,
}

impl SequenceReader {
    fn new(files: Vec<std::path::PathBuf>, on_progress: Box<dyn Fn(f32) + Send + Sync>) -> Self {
        Self {
            files,
            current_file_idx: 0,
            current_buffer: std::collections::VecDeque::new(),
            on_progress,
        }
    }
    
    fn load_next_file(&mut self) -> Result<bool> {
        if self.current_file_idx >= self.files.len() {
            return Ok(false);
        }
        
        let path = &self.files[self.current_file_idx];
        let file = File::open(path)?;
        let decoder = png::Decoder::new(file);
        let mut reader = decoder.read_info()?;
        let mut buf = vec![0u8; reader.output_buffer_size()];
        reader.next_frame(&mut buf)?;
        
        // Extract LSBs
        // Buffer is 16-bit RGBA likely.
        // Format: R16 G16 B16 A16 (Big Endian)
        // We want LSB of R, G, B.
        
        let mut idx = 0;
        while idx < buf.len() {
            // R
            if idx + 1 < buf.len() {
                self.current_buffer.push_back(buf[idx+1]); // LSB is second byte in BE
            }
            // G
            if idx + 3 < buf.len() {
                self.current_buffer.push_back(buf[idx+3]);
            }
            // B
            if idx + 5 < buf.len() {
                self.current_buffer.push_back(buf[idx+5]);
            }
            // Skip A
            idx += 8; 
        }
        
        (self.on_progress)((self.current_file_idx + 1) as f32 / self.files.len() as f32);
        self.current_file_idx += 1;
        Ok(true)
    }
}

impl Read for SequenceReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut total_read = 0;
        while total_read < buf.len() {
            if let Some(b) = self.current_buffer.pop_front() {
                buf[total_read] = b;
                total_read += 1;
            } else {
                // Load next file
                match self.load_next_file() {
                    Ok(true) => continue,
                    Ok(false) => break, // EOF
                    Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
                }
            }
        }
        Ok(total_read)
    }
}

#[no_mangle]
pub extern "Rust" fn _create_encoder() -> Box<dyn ContainerEncoder> {
    Box::new(SequenceFramePlugin)
}

#[no_mangle]
pub extern "Rust" fn _create_decoder() -> Box<dyn ContainerDecoder> {
    Box::new(SequenceFramePlugin)
}