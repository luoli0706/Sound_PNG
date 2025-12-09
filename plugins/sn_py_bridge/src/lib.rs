use sound_png_api::{ContainerEncoder, ContainerDecoder, ByteStream, PluginMetadata};
use anyhow::{Result, anyhow};
use std::path::Path;
use std::process::{Command, Stdio};
use std::io::{Read, Write};
use std::sync::{Once, Mutex};
use reqwest::blocking::Client;
use std::time::Duration;

static INIT: Once = Once::new();
static SERVER_STARTED: Mutex<bool> = Mutex::new(false);

fn ensure_server_running() -> Result<()> {
    let port = std::env::var("SPNG_PYTHON_PORT").unwrap_or_else(|_| "6657".to_string());
    let python_path = std::env::var("SPNG_PYTHON_PATH").unwrap_or_else(|_| "python".to_string());
    let health_url = format!("http://localhost:{}/health", port);

    // 1. Check health
    let client = Client::new();
    if client.get(&health_url).timeout(Duration::from_millis(500)).send().is_ok() {
        return Ok(());
    }

    // 2. Start Server
    let mut binding = SERVER_STARTED.lock().unwrap();
    if *binding { return Ok(()); } 

    let server_script = std::env::current_exe()?.parent().unwrap().join("Plugins").join("server.py");
    
    if !server_script.exists() {
        return Err(anyhow!("server.py not found at {:?}", server_script));
    }

    // Launch in new console window
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(&["/C", "start", "Python Bridge", &python_path, server_script.to_str().unwrap(), &port])
            .spawn()?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Linux/Mac: just spawn background for now, or use x-terminal-emulator
        Command::new(&python_path)
            .arg(&server_script)
            .arg(&port)
            .spawn()?;
    }

    // Wait for startup
    std::thread::sleep(Duration::from_secs(2));
    *binding = true;
    Ok(())
}

struct PyBridgePlugin;

impl ContainerEncoder for PyBridgePlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "Python Bridge".to_string(),
            description: "Delegates to local Python process via REST".to_string(),
            version: "0.2.0".to_string(),
            author: "Sound_PNG".to_string(),
        }
    }

    fn supported_extensions(&self) -> Vec<String> {
        vec!["py".to_string()] 
    }

    fn encode(
        &self,
        container_path: &Path,
        output_path: &Path,
        byte_stream: &mut ByteStream<std::fs::File>,
        _on_progress: Box<dyn Fn(f32) + Send + Sync>
    ) -> Result<()> {
        ensure_server_running()?;
        let port = std::env::var("SPNG_PYTHON_PORT").unwrap_or_else(|_| "6657".to_string());
        
        // Dump Stream to Temp File
        let temp_dir = std::env::temp_dir();
        let temp_payload = temp_dir.join("spng_py_payload.tmp");
        let mut f = std::fs::File::create(&temp_payload)?;
        let mut buffer = Vec::new();
        
        let len = byte_stream.total_len();
        for _ in 0..len {
            buffer.push(byte_stream.next_byte());
        }
        f.write_all(&buffer)?;
        
        let client = Client::new();
        let body = serde_json::json!({
            "container": container_path,
            "output": output_path,
            "payload_tmp": temp_payload
        });
        
        let res = client.post(format!("http://localhost:{}/encode", port))
            .json(&body)
            .send()?;
            
        if !res.status().is_success() {
            return Err(anyhow!("Python Server Error: {}", res.status()));
        }
        
        let _ = std::fs::remove_file(temp_payload);
        Ok(())
    }
}

impl ContainerDecoder for PyBridgePlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "Python Bridge".to_string(),
            description: "Delegates to local Python process via REST".to_string(),
            version: "0.2.0".to_string(),
            author: "Sound_PNG".to_string(),
        }
    }

    fn supported_extensions(&self) -> Vec<String> {
        vec!["py".to_string()]
    }

    fn decode(
        &self,
        input_path: &Path,
        _on_progress: Box<dyn Fn(f32) + Send + Sync>
    ) -> Result<Box<dyn Read + Send>> {
        ensure_server_running()?;
        let port = std::env::var("SPNG_PYTHON_PORT").unwrap_or_else(|_| "6657".to_string());
        
        let client = Client::new();
        let body = serde_json::json!({
            "input": input_path
        });
        
        let res = client.post(format!("http://localhost:{}/decode", port))
            .json(&body)
            .send()?;
            
        if !res.status().is_success() {
            return Err(anyhow!("Python Server Error: {}", res.status()));
        }
        
        let bytes = res.bytes()?.to_vec();
        Ok(Box::new(std::io::Cursor::new(bytes)))
    }
}

#[no_mangle]
pub extern "Rust" fn _create_encoder() -> Box<dyn ContainerEncoder> {
    Box::new(PyBridgePlugin)
}

#[no_mangle]
pub extern "Rust" fn _create_decoder() -> Box<dyn ContainerDecoder> {
    Box::new(PyBridgePlugin)
}
