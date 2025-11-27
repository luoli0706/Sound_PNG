use rfd::FileDialog;
use slint::{PlatformError, SharedString, Weak, ComponentHandle, CloseRequestResponse};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::{path::PathBuf, thread, fs};

slint::include_modules!();

enum WorkerMessage {
    // Beta 1.0 Legacy
    Encode {
        voice_in: PathBuf,
        picture_in: PathBuf,
        key_in: Option<PathBuf>,
        output: PathBuf,
        format: String,
        use_encryption: bool,
    },
    Decode {
        input: PathBuf,
        key_in: Option<PathBuf>,
        voice_out: PathBuf,
        picture_out: PathBuf,
    },
    // Beta 2.0 Universal
    EncodeGeneric {
        payload: PathBuf,
        container: PathBuf,
        key: Option<PathBuf>,
        output: PathBuf,
        encrypt: bool,
    },
    DecodeGeneric {
        input: PathBuf,
        payload_out: PathBuf,
        container_out: Option<PathBuf>,
        key: Option<PathBuf>,
        force_ext: Option<String>, // If preset is used, override extension logic
    },
    Analyze {
        input: PathBuf,
        mode: String, // "Standard" or "Universal"
    },
}

enum UIMessage {
    Status(SharedString),
    AnalysisResult { encrypted: bool, mode: String },
    Progress(f32),
    Busy(bool),
}

pub fn run() -> Result<(), PlatformError> {
    let ui = AppWindow::new()?;
    let ui_handle = ui.as_weak();

    let (ui_tx, ui_rx) = channel::<UIMessage>();
    let (worker_tx, worker_rx) = channel::<WorkerMessage>();

    // Worker thread
    thread::spawn(move || {
        worker_thread_main(worker_rx, ui_tx);
    });
    
    // Splash Timer
    let ui_handle_splash = ui_handle.clone();
    let splash_timer = slint::Timer::default();
    splash_timer.start(slint::TimerMode::SingleShot, std::time::Duration::from_secs(2), move || {
        if let Some(ui) = ui_handle_splash.upgrade() {
            ui.set_show_splash(false);
        }
    });
    
    // Window Close Request Handler
    let ui_handle_close = ui_handle.clone();
    ui.window().on_close_requested(move || {
        if let Some(ui) = ui_handle_close.upgrade() {
            ui.set_show_exit_dialog(true);
        }
        CloseRequestResponse::KeepWindowShown
    });
    
    // Minimize / Close
    let ui_handle_min = ui_handle.clone();
    ui.on_minimize_window(move || {
        if let Some(ui) = ui_handle_min.upgrade() {
            ui.window().set_minimized(true);
        }
    });
    
    let ui_handle_exit = ui_handle.clone();
    ui.on_close_window(move || {
        if let Some(ui) = ui_handle_exit.upgrade() {
            ui.window().hide(); // Break run loop
        }
    });
    
    // Manual
    let ui_handle_manual = ui_handle.clone();
    ui.on_open_manual(move || {
        if let Some(ui) = ui_handle_manual.upgrade() {
            // Load and Parse MD
            let text = include_str!("../docs/User_Manual.md");
            let mut blocks = Vec::new();
            let mut in_code = false;
            
            for line in text.lines() {
                let line_trim = line.trim();
                
                if line_trim.starts_with("```") {
                    in_code = !in_code;
                    continue;
                }
                
                if in_code {
                     blocks.push(MdBlock { block_type: "code".into(), text: line.into() });
                     continue;
                }
                
                if line_trim.starts_with("# ") {
                    blocks.push(MdBlock { block_type: "h1".into(), text: line_trim[2..].into() });
                } else if line_trim.starts_with("## ") {
                    blocks.push(MdBlock { block_type: "h2".into(), text: line_trim[3..].into() });
                } else if line_trim.starts_with("### ") {
                    blocks.push(MdBlock { block_type: "h3".into(), text: line_trim[4..].into() });
                } else if line_trim.starts_with("- ") {
                    blocks.push(MdBlock { block_type: "li".into(), text: line_trim[2..].into() });
                } else if !line_trim.is_empty() {
                    blocks.push(MdBlock { block_type: "p".into(), text: line_trim.into() });
                }
            }
            
            let model = std::rc::Rc::new(slint::VecModel::from(blocks));
            ui.set_manual_content(model.into());
            ui.set_show_manual(true);
        }
    });

    // === STANDARD MODE CALLBACKS ===
    
    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_input_voice(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().add_filter("Audio", &["wav", "mp3"]).pick_file() {
            ui.set_input_voice_path(path.to_string_lossy().to_string().into());
            check_std_encode(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_input_picture(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().add_filter("Image", &["png", "jpg"]).pick_file() {
            ui.set_input_picture_path(path.to_string_lossy().to_string().into());
            check_std_encode(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_key_file(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().pick_file() {
            ui.set_key_file_path(path.to_string_lossy().to_string().into());
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_output(move || {
        let ui = ui_handle_clone.unwrap();
        let ext = if ui.get_output_format() == "WAV" { "wav" } else { "png" };
        if let Some(path) = FileDialog::new().add_filter(ext, &[ext]).save_file() {
            ui.set_output_path(path.to_string_lossy().to_string().into());
            check_std_encode(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    let worker_tx_std = worker_tx.clone();
    ui.on_request_encode(move || {
        let ui = ui_handle_clone.unwrap();
        let voice_in: PathBuf = ui.get_input_voice_path().to_string().into();
        let picture_in: PathBuf = ui.get_input_picture_path().to_string().into();
        let key_str = ui.get_key_file_path().to_string();
        let key_in = if key_str.is_empty() { None } else { Some(PathBuf::from(key_str)) };
        let output: PathBuf = ui.get_output_path().to_string().into();
        let format = ui.get_output_format().to_string();
        let use_encryption = ui.get_use_encryption();

        worker_tx_std.send(WorkerMessage::Encode {
            voice_in, picture_in, key_in, output, format, use_encryption
        }).unwrap();
    });

    let ui_handle_clone = ui_handle.clone();
    let worker_tx_analyze = worker_tx.clone();
    ui.on_browse_decode_input(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().add_filter("Encoded", &["wav", "png"]).pick_file() {
            ui.set_decode_input_path(path.to_string_lossy().to_string().into());
            ui.set_input_analyzed(false);
            check_std_decode(&ui);
            worker_tx_analyze.send(WorkerMessage::Analyze { input: path, mode: "Standard".into() }).unwrap();
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_decode_key_file(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().pick_file() {
            ui.set_decode_key_file_path(path.to_string_lossy().to_string().into());
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_decode_output_voice(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().add_filter("Audio", &["wav"]).save_file() {
            ui.set_decode_output_voice_path(path.to_string_lossy().to_string().into());
            check_std_decode(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_decode_output_picture(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().add_filter("Image", &["png"]).save_file() {
            ui.set_decode_output_picture_path(path.to_string_lossy().to_string().into());
            check_std_decode(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    let worker_tx_dec = worker_tx.clone();
    ui.on_request_decode(move || {
        let ui = ui_handle_clone.unwrap();
        let input: PathBuf = ui.get_decode_input_path().to_string().into();
        let key_str = ui.get_decode_key_file_path().to_string();
        let key_in = if key_str.is_empty() { None } else { Some(PathBuf::from(key_str)) };
        let voice_out: PathBuf = ui.get_decode_output_voice_path().to_string().into();
        let picture_out: PathBuf = ui.get_decode_output_picture_path().to_string().into();

        worker_tx_dec.send(WorkerMessage::Decode {
            input, key_in, voice_out, picture_out
        }).unwrap();
    });

    // === UNIVERSAL MODE CALLBACKS ===

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_uni_payload(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().set_title("Select Payload (Any File)").pick_file() {
            ui.set_uni_payload_path(path.to_string_lossy().to_string().into());
            check_uni_encode(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_uni_container(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().add_filter("Container", &["png", "wav", "jpg", "jpeg", "mp3"]).pick_file() {
            ui.set_uni_container_path(path.to_string_lossy().to_string().into());
            check_uni_encode(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_uni_key(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().pick_file() {
            ui.set_uni_key_path(path.to_string_lossy().to_string().into());
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_uni_output(move || {
        let ui = ui_handle_clone.unwrap();
        // Infer extension from Container
        let container = ui.get_uni_container_path().to_string();
        let ext = if container.to_lowercase().ends_with("wav") || container.to_lowercase().ends_with("mp3") { "wav" } else { "png" };
        
        if let Some(path) = FileDialog::new().add_filter(ext, &[ext]).save_file() {
            ui.set_uni_output_path(path.to_string_lossy().to_string().into());
            check_uni_encode(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    let worker_tx_uni_enc = worker_tx.clone();
    ui.on_request_uni_encode(move || {
        let ui = ui_handle_clone.unwrap();
        let payload: PathBuf = ui.get_uni_payload_path().to_string().into();
        let container: PathBuf = ui.get_uni_container_path().to_string().into();
        let key_str = ui.get_uni_key_path().to_string();
        let key = if key_str.is_empty() { None } else { Some(PathBuf::from(key_str)) };
        let output: PathBuf = ui.get_uni_output_path().to_string().into();
        let encrypt = ui.get_uni_use_encryption();

        worker_tx_uni_enc.send(WorkerMessage::EncodeGeneric {
            payload, container, key, output, encrypt
        }).unwrap();
    });

    // Uni Decode
    let ui_handle_clone = ui_handle.clone();
    let worker_tx_analyze_uni = worker_tx.clone();
    ui.on_browse_uni_decode_input(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().add_filter("Encoded", &["wav", "png"]).pick_file() {
            ui.set_uni_decode_input_path(path.to_string_lossy().to_string().into());
            ui.set_uni_decode_analyzed(false);
            check_uni_decode(&ui);
            worker_tx_analyze_uni.send(WorkerMessage::Analyze { input: path, mode: "Universal".into() }).unwrap();
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_uni_decode_key(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().pick_file() {
            ui.set_uni_decode_key_path(path.to_string_lossy().to_string().into());
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_uni_decode_payload_out(move || {
        let ui = ui_handle_clone.unwrap();
        
        // Get Preset Extension logic
        // 0=Auto, 1=PNG, 2=ZIP, 3=APK, 4=EXE
        let preset_idx = ui.get_uni_decode_preset_index();
        let filter_ext = match preset_idx {
            1 => Some("png"),
            2 => Some("zip"),
            3 => Some("apk"),
            4 => Some("exe"),
            _ => None,
        };
        
        let mut dialog = FileDialog::new().set_title("Save Payload As...");
        if let Some(ext) = filter_ext {
            dialog = dialog.add_filter(ext, &[ext]);
        }
        
        if let Some(path) = dialog.save_file() {
            ui.set_uni_decode_payload_out(path.to_string_lossy().to_string().into());
            check_uni_decode(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_uni_decode_container_out(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().set_title("Save Container As...").save_file() {
            ui.set_uni_decode_container_out(path.to_string_lossy().to_string().into());
        }
    });

    let ui_handle_clone = ui_handle.clone();
    let worker_tx_uni_dec = worker_tx.clone();
    ui.on_request_uni_decode(move || {
        let ui = ui_handle_clone.unwrap();
        let input: PathBuf = ui.get_uni_decode_input_path().to_string().into();
        let key_str = ui.get_uni_decode_key_path().to_string();
        let key = if key_str.is_empty() { None } else { Some(PathBuf::from(key_str)) };
        let payload_out: PathBuf = ui.get_uni_decode_payload_out().to_string().into();
        let cont_str = ui.get_uni_decode_container_out().to_string();
        let container_out = if cont_str.is_empty() { None } else { Some(PathBuf::from(cont_str)) };
        
        // Get Preset for forcing
        let preset_idx = ui.get_uni_decode_preset_index();
        let force_ext = match preset_idx {
            1 => Some("png".to_string()),
            2 => Some("zip".to_string()),
            3 => Some("apk".to_string()),
            4 => Some("exe".to_string()),
            _ => None,
        };

        worker_tx_uni_dec.send(WorkerMessage::DecodeGeneric {
            input, payload_out, container_out, key, force_ext
        }).unwrap();
    });

    // --- UI Message Handler ---
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(50), // Faster updates for progress
        move || {
            while let Ok(message) = ui_rx.try_recv() {
                handle_ui_message(ui_handle.clone(), message);
            }
        },
    );

    ui.run()
}

fn worker_thread_main(worker_rx: Receiver<WorkerMessage>, ui_tx: Sender<UIMessage>) {
    while let Ok(message) = worker_rx.recv() {
        ui_tx.send(UIMessage::Busy(true)).unwrap();
        
        let ui_tx_clone = ui_tx.clone();
        let on_progress = move |p: f32| {
            let _ = ui_tx_clone.send(UIMessage::Progress(p));
        };

        match message {
            // Standard Mode
            WorkerMessage::Encode { voice_in, picture_in, key_in, output, format, use_encryption } => {
                ui_tx.send(UIMessage::Status("Encoding (Std)...".into())).unwrap();
                match crate::encoder::encode(&voice_in, &picture_in, key_in.as_ref(), &output, use_encryption, &format, on_progress) {
                    Ok(_) => ui_tx.send(UIMessage::Status("Standard Encoding Complete!".into())).unwrap(),
                    Err(e) => ui_tx.send(UIMessage::Status(format!("Error: {}", e).into())).unwrap(),
                }
            }
            WorkerMessage::Decode { input, key_in, voice_out, picture_out } => {
                ui_tx.send(UIMessage::Status("Decoding (Std)...".into())).unwrap();
                match crate::decoder::decode(&input, &voice_out, &picture_out, key_in.as_ref(), on_progress) {
                    Ok(_) => ui_tx.send(UIMessage::Status("Standard Decoding Complete!".into())).unwrap(),
                    Err(e) => ui_tx.send(UIMessage::Status(format!("Error: {}", e).into())).unwrap(),
                }
            }
            // Universal Mode
            WorkerMessage::EncodeGeneric { payload, container, key, output, encrypt } => {
                ui_tx.send(UIMessage::Status("Encoding (Uni)...".into())).unwrap();
                
                // Determine Payload Ext
                let payload_ext = payload.extension().and_then(|s| s.to_str()).map(|s| s.to_string());
                
                match crate::converter::load_file_as_bytes(&payload) {
                    Ok(bytes) => {
                        let key_bytes = key.as_deref().map(|p| crate::converter::load_file_as_bytes(p).unwrap());
                        match crate::encoder::encode_data(
                            &bytes, &container, key_bytes.as_deref(), &output, encrypt, 
                            payload_ext.as_deref(), on_progress
                        ) {
                            Ok(_) => ui_tx.send(UIMessage::Status("Universal Encoding Complete!".into())).unwrap(),
                            Err(e) => ui_tx.send(UIMessage::Status(format!("Error: {}", e).into())).unwrap(),
                        }
                    },
                    Err(e) => ui_tx.send(UIMessage::Status(format!("Error Loading Payload: {}", e).into())).unwrap(),
                }
            }
            WorkerMessage::DecodeGeneric { input, payload_out, container_out, key, force_ext } => {
                ui_tx.send(UIMessage::Status("Decoding (Uni)...".into())).unwrap();
                match crate::decoder::decode_data(&input, &payload_out, container_out.as_ref(), key.as_ref(), on_progress) {
                    Ok(ext) => {
                        // Rename logic
                        if force_ext.is_some() {
                             // Preset used. Trust the payload_out extension (which was forced by Save Dialog or user).
                             ui_tx.send(UIMessage::Status("Universal Decoding Complete! (Preset Applied)".into())).unwrap();
                        } else if !ext.is_empty() {
                             // Auto-Detect Logic
                             let current_ext = payload_out.extension().and_then(|s| s.to_str()).unwrap_or("");
                             if current_ext != ext {
                                  let mut new_path = payload_out.clone();
                                  new_path.set_extension(&ext);
                                  if let Err(e) = fs::rename(&payload_out, &new_path) {
                                      ui_tx.send(UIMessage::Status(format!("Decoded (Rename Failed: {}).", e).into())).unwrap();
                                  } else {
                                      ui_tx.send(UIMessage::Status(format!("Universal Decoding Complete! (Saved as .{})", ext).into())).unwrap();
                                  }
                             } else {
                                  ui_tx.send(UIMessage::Status("Universal Decoding Complete!".into())).unwrap();
                             }
                        } else {
                             ui_tx.send(UIMessage::Status("Universal Decoding Complete!".into())).unwrap();
                        }
                    },
                    Err(e) => ui_tx.send(UIMessage::Status(format!("Error: {}", e).into())).unwrap(),
                }
            }
            // Analysis
            WorkerMessage::Analyze { input, mode } => {
                ui_tx.send(UIMessage::Status("Analyzing...".into())).unwrap();
                match crate::decoder::analyze_header(&input) {
                    Ok(encrypted) => {
                        ui_tx.send(UIMessage::AnalysisResult { encrypted, mode }).unwrap();
                        let msg = if encrypted { "File Encrypted. Key Required." } else { "File Clean. No Key Needed." };
                        ui_tx.send(UIMessage::Status(msg.into())).unwrap();
                    },
                    Err(e) => ui_tx.send(UIMessage::Status(format!("Check Error: {}", e).into())).unwrap(),
                }
            }
        }
        ui_tx.send(UIMessage::Busy(false)).unwrap();
    }
}

fn handle_ui_message(ui_handle: Weak<AppWindow>, message: UIMessage) {
    if let Some(ui) = ui_handle.upgrade() {
        match message {
            UIMessage::Status(status) => {
                let theme = ui.global::<Theme>();
                let s = status.as_str();
                let color = if s.starts_with("Error") || s.starts_with("Check Error") { theme.get_error() } 
                           else if s.contains("Complete") { theme.get_success() } 
                           else { theme.get_text_normal() };
                ui.set_status_text(status);
                ui.set_status_color(color);
            }
            UIMessage::AnalysisResult { encrypted, mode } => {
                if mode == "Standard" {
                    ui.set_is_encrypted_source(encrypted);
                    ui.set_input_analyzed(true);
                    check_std_decode(&ui);
                } else {
                    ui.set_uni_decode_encrypted(encrypted);
                    ui.set_uni_decode_analyzed(true);
                    check_uni_decode(&ui);
                }
            }
            UIMessage::Progress(p) => {
                ui.set_progress_value(p);
            }
            UIMessage::Busy(b) => {
                ui.set_is_busy(b);
                if b { ui.set_progress_value(0.0); }
            }
        }
    }
}

fn check_std_encode(ui: &AppWindow) {
    let enabled = !ui.get_input_voice_path().is_empty() && !ui.get_input_picture_path().is_empty() && !ui.get_output_path().is_empty();
    ui.set_encode_button_enabled(enabled);
}

fn check_std_decode(ui: &AppWindow) {
    let enabled = !ui.get_decode_input_path().is_empty() && !ui.get_decode_output_voice_path().is_empty() && !ui.get_decode_output_picture_path().is_empty() && ui.get_input_analyzed();
    ui.set_decode_button_enabled(enabled);
}

fn check_uni_encode(ui: &AppWindow) {
    let enabled = !ui.get_uni_payload_path().is_empty() && !ui.get_uni_container_path().is_empty() && !ui.get_uni_output_path().is_empty();
    ui.set_uni_encode_enabled(enabled);
}

fn check_uni_decode(ui: &AppWindow) {
    let enabled = !ui.get_uni_decode_input_path().is_empty() && !ui.get_uni_decode_payload_out().is_empty() && ui.get_uni_decode_analyzed();
    ui.set_uni_decode_enabled(enabled);
}
