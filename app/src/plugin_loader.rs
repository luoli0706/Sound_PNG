use libloading::{Library, Symbol};
use sound_png_api::{ContainerEncoder, ContainerDecoder, PluginMetadata};
use std::path::Path;
use std::fs;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

pub struct LoadedPlugin {
    pub encoder: Option<Box<dyn ContainerEncoder>>,
    pub decoder: Option<Box<dyn ContainerDecoder>>,
    pub metadata: PluginMetadata,
    pub enabled: bool,
    #[allow(dead_code)]
    lib: Arc<Library>,
}

pub struct PluginManager {
    plugins: HashMap<String, LoadedPlugin>, // Keyed by name
}

impl PluginManager {
    pub fn new() -> Self {
        Self { plugins: HashMap::new() }
    }

    pub fn load_plugins(&mut self, dir: &Path) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("sn") {
                    unsafe {
                        if let Ok(lib) = Library::new(&path) {
                            let lib = Arc::new(lib);
                            
                            // Try load encoder
                            let enc_func: Result<Symbol<fn() -> Box<dyn ContainerEncoder>>, _> = lib.get(b"_create_encoder");
                            let encoder = if let Ok(f) = enc_func { Some(f()) } else { None };
                            
                            // Try load decoder
                            let dec_func: Result<Symbol<fn() -> Box<dyn ContainerDecoder>>, _> = lib.get(b"_create_decoder");
                            let decoder = if let Ok(f) = dec_func { Some(f()) } else { None };

                            if let Some(enc) = &encoder {
                                let meta = enc.metadata();
                                self.plugins.insert(meta.name.clone(), LoadedPlugin {
                                    encoder,
                                    decoder, // Might be None if only Encoder
                                    metadata: meta,
                                    enabled: false, // Default disabled
                                    lib: lib.clone(),
                                });
                                println!("Loaded Plugin: {:?}", path);
                            } else if let Some(dec) = &decoder {
                                let meta = dec.metadata();
                                self.plugins.insert(meta.name.clone(), LoadedPlugin {
                                    encoder: None,
                                    decoder,
                                    metadata: meta,
                                    enabled: false,
                                    lib: lib.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    pub fn set_plugin_enabled(&mut self, name: &str, enabled: bool) {
        if let Some(plugin) = self.plugins.get_mut(name) {
            plugin.enabled = enabled;
        }
    }
    
    pub fn get_enabled_plugins_meta(&self) -> Vec<PluginMetadata> {
        self.plugins.values()
            .filter(|p| p.enabled)
            .map(|p| p.metadata.clone())
            .collect()
    }

    pub fn get_all_plugins_meta(&self) -> Vec<(PluginMetadata, bool)> {
        self.plugins.values()
            .map(|p| (p.metadata.clone(), p.enabled))
            .collect()
    }
    
    pub fn get_encoder(&self, name: &str) -> Option<&dyn ContainerEncoder> {
        if let Some(plugin) = self.plugins.get(name) {
            if plugin.enabled {
                return plugin.encoder.as_deref();
            }
        }
        None
    }

    // Helper to find by functionality if we don't know the name, 
    // but for this UI, we will explicitly select the plugin mode.
    pub fn get_decoder_by_ext(&self, ext: &str) -> Option<&dyn ContainerDecoder> {
        for plugin in self.plugins.values() {
            if plugin.enabled {
                if let Some(dec) = &plugin.decoder {
                    if dec.supported_extensions().contains(&ext.to_string()) {
                        return Some(dec.as_ref());
                    }
                }
            }
        }
        None
    }
}

// Make it thread-safe
unsafe impl Send for PluginManager {}
unsafe impl Sync for PluginManager {}