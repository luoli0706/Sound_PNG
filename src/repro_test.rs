#[cfg(test)]
mod reproduction_test {
    use image::{ImageBuffer, Rgba, io::Reader as ImageReader};
    use std::path::PathBuf;
    use std::fs;
    use crate::encoder;
    use crate::decoder;

    #[test]
    fn test_bit_preservation() -> anyhow::Result<()> {
        let root = PathBuf::from("target/test_debug");
        if !root.exists() { fs::create_dir_all(&root)?; }
        let png_path = root.join("bit_test.png");
        
        let width = 100;
        let height = 100;
        let mut img = ImageBuffer::<Rgba<u16>, Vec<u16>>::new(width, height);
        
        // Fill with known pattern
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            // High byte: Gradient. Low byte: Magic value.
            let r = ((x as u16) << 8) | 0x12;
            let g = ((y as u16) << 8) | 0x34;
            let b = 0x8000 | 0x56;
            let a = 0xFFFF;
            *pixel = Rgba([r, g, b, a]);
        }
        
        img.save(&png_path)?;
        
        // Load back
        let loaded = ImageReader::open(&png_path)?.decode()?.into_rgba16();
        
        for (x, y, pixel) in loaded.enumerate_pixels() {
            let r = pixel[0];
            let g = pixel[1];
            let b = pixel[2];
            
            if (r & 0xFF) != 0x12 {
                anyhow::bail!("R channel corruption at {},{}: Expected 0x12, Got 0x{:X} (Full: 0x{:X})", x, y, r & 0xFF, r);
            }
            if (g & 0xFF) != 0x34 {
                anyhow::bail!("G channel corruption at {},{}: Expected 0x34, Got 0x{:X} (Full: 0x{:X})", x, y, g & 0xFF, g);
            }
            if (b & 0xFF) != 0x56 {
                anyhow::bail!("B channel corruption at {},{}: Expected 0x56, Got 0x{:X} (Full: 0x{:X})", x, y, b & 0xFF, b);
            }
        }
        
        println!("Bit Preservation Test Passed!");
        Ok(())
    }

    #[test]
    fn test_reproduce_desktop_failure() -> anyhow::Result<()> {
        let root = PathBuf::from("target/test_debug");
        if !root.exists() {
            println!("Skipping reproduction test: target/test_debug not found");
            return Ok(());
        }
        
        let wav_in = root.join("demo_5.wav");
        let png_in = root.join("test.jpg"); 
        let png_out = root.join("repro_out.png");
        
        let decoded_wav = root.join("repro_restored.wav");
        let decoded_png = root.join("repro_restored.png");

        if !wav_in.exists() || !png_in.exists() {
             println!("Skipping reproduction test: Input files missing");
             return Ok(());
        }

        println!("Starting Reproduction Test...");
        
        match encoder::encode(&wav_in, &png_in, None, &png_out, false, "PNG") {
            Ok(_) => println!("Encoding OK"),
            Err(e) => println!("Encoding Error: {}", e),
        }
        
        if !png_out.exists() {
            anyhow::bail!("Encoding failed to create output file");
        }
        println!("Encoding Complete. Output size: {}", fs::metadata(&png_out)?.len());

        match decoder::decode(&png_out, &decoded_wav, &decoded_png, None) {
            Ok(_) => println!("Decoding OK"),
            Err(e) => println!("Decoding Error: {}", e),
        }

        Ok(())
    }
}