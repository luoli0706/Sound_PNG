use rfd::FileDialog;
use slint::{PlatformError, SharedString, Weak, ComponentHandle, CloseRequestResponse};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::{path::PathBuf, thread, fs, process::Command};
use slint::Model;
use serde::Deserialize;

slint::include_modules!();

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    #[allow(dead_code)]
    html_url: String,
}

enum WorkerMessage {
    // Beta 3.0 Unified Streaming Logic
    EncodeStream {
        payload_path: PathBuf,
        container_path: PathBuf,
        key_path: Option<PathBuf>,
        output_path: PathBuf,
        encrypt: bool,
        buffer_size_kb: usize,
        is_std_mode: bool, 
    },
    DecodeStream {
        input_path: PathBuf,
        output_path: PathBuf,
        key_path: Option<PathBuf>,
        buffer_size_kb: usize,
        preset_ext: Option<String>,
        resize_factor: Option<f32>, // 1.0, 0.75, 0.5, 0.25
    },
    Analyze {
        input: PathBuf,
        mode: String,
    },
    BatchEncode {
        payloads: Vec<PathBuf>,
        container: PathBuf,
        key: Option<PathBuf>,
        out_dir: PathBuf,
        encrypt: bool,
        buffer_size_kb: usize,
    },
    BatchDecode {
        inputs: Vec<PathBuf>,
        key: Option<PathBuf>,
        out_dir: PathBuf,
        buffer_size_kb: usize,
    },
}

enum UIMessage {
    Status(SharedString),
    AnalysisResult { encrypted: bool, mode: String },
    Progress(f32),
    Busy(bool),
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

fn check_for_updates(ui_handle: Weak<AppWindow>) {
    thread::spawn(move || {
        // Use blocking client in this thread
        let client = reqwest::blocking::Client::new();
        // User-Agent is required by GitHub API
        let res = client.get("https://api.github.com/repos/luoli0706/Sound_PNG/releases/latest")
            .header("User-Agent", "Sound_PNG_App")
            .send();

        if let Ok(resp) = res {
            if let Ok(release) = resp.json::<Release>() {
                // Current logic: If tag differs from "Beta 3.0" and "v3.0", assume update.
                // In a real app, use SemVer.
                let current_tag = "Beta 3.0";
                // Normalized check
                if release.tag_name != current_tag && release.tag_name != "v3.0" && release.tag_name != "V3.0" {
                     let _ = slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_handle.upgrade() {
                            let settings = ui.global::<Settings>();
                            settings.set_update_available(true);
                            settings.set_update_version(release.tag_name.into());
                        }
                    });
                }
            }
        }
    });
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
    
    // Check for updates
    check_for_updates(ui_handle.clone());
    
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
    
    // Update Link
    ui.on_open_update_url(move || {
        let _ = open::that("https://github.com/luoli0706/Sound_PNG/releases");
    });
    
    // Manual
    let ui_handle_manual = ui_handle.clone();
    ui.on_open_manual(move || {
        if let Some(ui) = ui_handle_manual.upgrade() {
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
        let settings = ui.global::<Settings>();
        let buffer_size = settings.get_stream_buffer_size() as usize;

        // Standard Mode Mapping
        let (payload, container) = if format == "WAV" {
            // Hide Picture in Voice
            (picture_in, voice_in)
        } else {
            // Hide Voice in Picture
            (voice_in, picture_in)
        };

        worker_tx_std.send(WorkerMessage::EncodeStream {
            payload_path: payload,
            container_path: container,
            key_path: key_in,
            output_path: output,
            encrypt: use_encryption,
            buffer_size_kb: buffer_size,
            is_std_mode: true,
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
        
        let ext = input.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        let output_path = if ext == "png" {
            PathBuf::from(ui.get_decode_output_voice_path().to_string())
        } else {
            PathBuf::from(ui.get_decode_output_picture_path().to_string())
        };
        
        let settings = ui.global::<Settings>();
        let buffer_size = settings.get_stream_buffer_size() as usize;

        worker_tx_dec.send(WorkerMessage::DecodeStream {
            input_path: input,
            output_path,
            key_path: key_in,
            buffer_size_kb: buffer_size,
            preset_ext: None,
            resize_factor: None,
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
        
        let settings = ui.global::<Settings>();
        let buffer_size = settings.get_stream_buffer_size() as usize;

        worker_tx_uni_enc.send(WorkerMessage::EncodeStream {
            payload_path: payload,
            container_path: container,
            key_path: key,
            output_path: output,
            encrypt,
            buffer_size_kb: buffer_size,
            is_std_mode: false,
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
        
        let preset_idx = ui.get_uni_decode_preset_index();
        let filter_ext = match preset_idx {
            1 => Some("png"),
            2 => Some("zip"),
            3 => Some("apk"),
            4 => Some("exe"),
            5 => Some("mp4"), // New Preset
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
        
        let preset_idx = ui.get_uni_decode_preset_index();
        let force_ext = match preset_idx {
            1 => Some("png".to_string()),
            2 => Some("zip".to_string()),
            3 => Some("apk".to_string()),
            4 => Some("exe".to_string()),
            5 => Some("mp4".to_string()),
            _ => None,
        };
        
        let resize_idx = ui.get_uni_decode_resize_index();
        let resize_factor = match resize_idx {
            1 => Some(0.75),
            2 => Some(0.50),
            3 => Some(0.25),
            _ => None, // Original
        };
        
        let settings = ui.global::<Settings>();
        let buffer_size = settings.get_stream_buffer_size() as usize;

        worker_tx_uni_dec.send(WorkerMessage::DecodeStream {
            input_path: input,
            output_path: payload_out,
            key_path: key,
            buffer_size_kb: buffer_size,
            preset_ext: force_ext,
            resize_factor,
        }).unwrap();
    });
    
    // Batch Callbacks
    let ui_handle_clone = ui_handle.clone();
    ui.on_batch_enc_add_payloads(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(files) = FileDialog::new().set_title("Select Payloads").pick_files() {
            let current = ui.get_batch_enc_payloads();
            let mut new_list = Vec::new();
            for i in 0..current.row_count() {
                if let Some(s) = current.row_data(i) {
                    new_list.push(s);
                }
            }
            for f in files {
                new_list.push(f.to_string_lossy().to_string().into());
            }
            let model = std::rc::Rc::new(slint::VecModel::from(new_list));
            ui.set_batch_enc_payloads(model.into());
            check_batch_enc(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_batch_enc_clear_payloads(move || {
        let ui = ui_handle_clone.unwrap();
        let model = std::rc::Rc::new(slint::VecModel::from(Vec::<SharedString>::new()));
        ui.set_batch_enc_payloads(model.into());
        check_batch_enc(&ui);
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_batch_enc_browse_container(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().add_filter("Container", &["png", "wav"]).pick_file() {
            ui.set_batch_enc_container(path.to_string_lossy().to_string().into());
            check_batch_enc(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_batch_enc_browse_out_dir(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().pick_folder() {
            ui.set_batch_enc_out_dir(path.to_string_lossy().to_string().into());
            check_batch_enc(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_batch_enc_browse_key(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().pick_file() {
            ui.set_batch_enc_key(path.to_string_lossy().to_string().into());
        }
    });

    let ui_handle_clone = ui_handle.clone();
    let worker_tx_batch_enc = worker_tx.clone();
    ui.on_request_batch_encode(move || {
        let ui = ui_handle_clone.unwrap();
        let payloads_slint = ui.get_batch_enc_payloads();
        let mut payloads = Vec::new();
        for i in 0..payloads_slint.row_count() {
            if let Some(s) = payloads_slint.row_data(i) {
                payloads.push(PathBuf::from(s.as_str()));
            }
        }
        
        let container: PathBuf = ui.get_batch_enc_container().to_string().into();
        let out_dir: PathBuf = ui.get_batch_enc_out_dir().to_string().into();
        let key_str = ui.get_batch_enc_key().to_string();
        let key = if key_str.is_empty() { None } else { Some(PathBuf::from(key_str)) };
        let encrypt = ui.get_batch_enc_encrypt();
        let settings = ui.global::<Settings>();
        let buffer_size = settings.get_stream_buffer_size() as usize;

        worker_tx_batch_enc.send(WorkerMessage::BatchEncode {
            payloads,
            container,
            key,
            out_dir,
            encrypt,
            buffer_size_kb: buffer_size,
        }).unwrap();
    });

    // Batch Decode
    let ui_handle_clone = ui_handle.clone();
    ui.on_batch_dec_add_inputs(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(files) = FileDialog::new().add_filter("Encoded", &["png", "wav"]).pick_files() {
            let current = ui.get_batch_dec_inputs();
            let mut new_list = Vec::new();
            for i in 0..current.row_count() {
                if let Some(s) = current.row_data(i) {
                    new_list.push(s);
                }
            }
            for f in files {
                new_list.push(f.to_string_lossy().to_string().into());
            }
            let model = std::rc::Rc::new(slint::VecModel::from(new_list));
            ui.set_batch_dec_inputs(model.into());
            check_batch_dec(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_batch_dec_clear_inputs(move || {
        let ui = ui_handle_clone.unwrap();
        let model = std::rc::Rc::new(slint::VecModel::from(Vec::<SharedString>::new()));
        ui.set_batch_dec_inputs(model.into());
        check_batch_dec(&ui);
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_batch_dec_browse_out_dir(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().pick_folder() {
            ui.set_batch_dec_out_dir(path.to_string_lossy().to_string().into());
            check_batch_dec(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_batch_dec_browse_key(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new().pick_file() {
            ui.set_batch_dec_key(path.to_string_lossy().to_string().into());
        }
    });

    let ui_handle_clone = ui_handle.clone();
    let worker_tx_batch_dec = worker_tx.clone();
    ui.on_request_batch_decode(move || {
        let ui = ui_handle_clone.unwrap();
        let inputs_slint = ui.get_batch_dec_inputs();
        let mut inputs = Vec::new();
        for i in 0..inputs_slint.row_count() {
            if let Some(s) = inputs_slint.row_data(i) {
                inputs.push(PathBuf::from(s.as_str()));
            }
        }
        
        let out_dir: PathBuf = ui.get_batch_dec_out_dir().to_string().into();
        let key_str = ui.get_batch_dec_key().to_string();
        let key = if key_str.is_empty() { None } else { Some(PathBuf::from(key_str)) };
        
        let settings = ui.global::<Settings>();
        let buffer_size = settings.get_stream_buffer_size() as usize;

        worker_tx_batch_dec.send(WorkerMessage::BatchDecode {
            inputs,
            key,
            out_dir,
            buffer_size_kb: buffer_size,
        }).unwrap();
    });

    // --- UI Message Handler ---
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(50), 
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
            WorkerMessage::EncodeStream { payload_path, container_path, key_path, output_path, encrypt, buffer_size_kb, is_std_mode } => {
                let mode_str = if is_std_mode { "Std" } else { "Uni" };
                ui_tx.send(UIMessage::Status(format!("Encoding ({} Stream)...", mode_str).into())).unwrap();
                
                let payload_ext = payload_path.extension().and_then(|s| s.to_str()).map(|s| s.to_string());
                
                match fs::File::open(&payload_path) {
                    Ok(mut payload_file) => {
                        match crate::stream_encoder::encode_stream(
                            &mut payload_file, 
                            &container_path, 
                            key_path.as_ref(), 
                            &output_path, 
                            encrypt, 
                            payload_ext.as_deref(),
                            buffer_size_kb,
                            on_progress
                        ) {
                            Ok(_) => ui_tx.send(UIMessage::Status(format!("{} Encoding Complete!", mode_str).into())).unwrap(),
                            Err(e) => ui_tx.send(UIMessage::Status(format!("Error: {}", e).into())).unwrap(),
                        }
                    },
                    Err(e) => ui_tx.send(UIMessage::Status(format!("Error opening payload: {}", e).into())).unwrap(),
                }
            },
            WorkerMessage::DecodeStream { input_path, output_path, key_path, buffer_size_kb, preset_ext, resize_factor } => {
                ui_tx.send(UIMessage::Status("Decoding (Stream)...".into())).unwrap();
                match crate::stream_decoder::decode_stream(
                    &input_path, 
                    &output_path, 
                    key_path.as_ref(), 
                    buffer_size_kb,
                    on_progress
                ) {
                    Ok(ext) => {
                        // Rename Logic
                        let final_ext = preset_ext.unwrap_or(if !ext.is_empty() { ext.clone() } else { "bin".to_string() });
                        let mut final_path = output_path.clone();
                        final_path.set_extension(&final_ext);
                        
                        if output_path != final_path {
                            let _ = fs::rename(&output_path, &final_path);
                        }
                        
                        // Image Resizing
                        if let Some(factor) = resize_factor {
                            // Only if extension implies image
                            let final_ext_str = final_ext.to_lowercase();
                            if final_ext_str == "png" || final_ext_str == "jpg" || final_ext_str == "jpeg" {
                                ui_tx.send(UIMessage::Status("Resizing Image...".into())).unwrap();
                                if let Ok(img) = image::open(&final_path) {
                                    let (w, h) = (img.width(), img.height());
                                    let new_w = (w as f32 * factor) as u32;
                                    let new_h = (h as f32 * factor) as u32;
                                    let resized = img.resize(new_w, new_h, image::imageops::FilterType::Lanczos3);
                                    if let Err(e) = resized.save(&final_path) {
                                        ui_tx.send(UIMessage::Status(format!("Resize Failed: {}", e).into())).unwrap();
                                    } else {
                                        ui_tx.send(UIMessage::Status("Resize Complete!".into())).unwrap();
                                    }
                                }
                            }
                        }
                        
                        ui_tx.send(UIMessage::Status("Decoding Complete!".into())).unwrap();
                    },
                    Err(e) => ui_tx.send(UIMessage::Status(format!("Error: {}", e).into())).unwrap(),
                }
            },
            WorkerMessage::BatchEncode { payloads, container, key, out_dir, encrypt, buffer_size_kb } => {
                ui_tx.send(UIMessage::Status("Starting Batch Encode...".into())).unwrap();
                let total = payloads.len();
                let container_ext = container.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                
                for (i, payload_path) in payloads.iter().enumerate() {
                    let payload_name = payload_path.file_stem().and_then(|s| s.to_str()).unwrap_or("payload");
                    let output_name = format!("{}_embedded.{}", payload_name, container_ext);
                    let output_path = out_dir.join(output_name);
                    
                    ui_tx.send(UIMessage::Status(format!("Encoding {}/{} : {}", i+1, total, payload_name).into())).unwrap();
                    
                    let payload_ext = payload_path.extension().and_then(|s| s.to_str()).map(|s| s.to_string());
                    
                    match fs::File::open(payload_path) {
                        Ok(mut payload_file) => {
                            if let Err(e) = crate::stream_encoder::encode_stream(
                                &mut payload_file, 
                                &container, 
                                key.as_ref(), 
                                &output_path, 
                                encrypt, 
                                payload_ext.as_deref(),
                                buffer_size_kb,
                                |p| { let _ = ui_tx.send(UIMessage::Progress((i as f32 + p) / total as f32)); }
                            ) {
                                ui_tx.send(UIMessage::Status(format!("Error on {}: {}", payload_name, e).into())).unwrap();
                            }
                        },
                        Err(e) => ui_tx.send(UIMessage::Status(format!("Error opening {}: {}", payload_name, e).into())).unwrap(),
                    }
                }
                ui_tx.send(UIMessage::Status("Batch Encoding Complete!".into())).unwrap();
            },
            WorkerMessage::BatchDecode { inputs, key, out_dir, buffer_size_kb } => {
                ui_tx.send(UIMessage::Status("Starting Batch Decode...".into())).unwrap();
                let total = inputs.len();
                
                for (i, input_path) in inputs.iter().enumerate() {
                    let input_name = input_path.file_stem().and_then(|s| s.to_str()).unwrap_or("input");
                    let output_base = out_dir.join(input_name); // Temp base
                    
                    ui_tx.send(UIMessage::Status(format!("Decoding {}/{} : {}", i+1, total, input_name).into())).unwrap();
                    
                    match crate::stream_decoder::decode_stream(
                        input_path, 
                        &output_base, 
                        key.as_ref(), 
                        buffer_size_kb,
                        |p| { let _ = ui_tx.send(UIMessage::Progress((i as f32 + p) / total as f32)); }
                    ) {
                        Ok(ext) => {
                            if !ext.is_empty() {
                                 let mut final_path = output_base.clone();
                                 final_path.set_extension(&ext);
                                 let _ = fs::rename(&output_base, &final_path);
                            }
                        },
                        Err(e) => ui_tx.send(UIMessage::Status(format!("Error on {}: {}", input_name, e).into())).unwrap(),
                    }
                }
                ui_tx.send(UIMessage::Status("Batch Decoding Complete!".into())).unwrap();
            },
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

fn check_batch_enc(ui: &AppWindow) {
    let enabled = ui.get_batch_enc_payloads().row_count() > 0 
        && !ui.get_batch_enc_container().is_empty() 
        && !ui.get_batch_enc_out_dir().is_empty();
    ui.set_batch_enc_enabled(enabled);
}

fn check_batch_dec(ui: &AppWindow) {
    let enabled = ui.get_batch_dec_inputs().row_count() > 0 && !ui.get_batch_dec_out_dir().is_empty();
    ui.set_batch_dec_enabled(enabled);
}