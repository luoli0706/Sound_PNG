use rfd::FileDialog;
use slint::{PlatformError, SharedString, Weak};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::{path::PathBuf, thread};

slint::include_modules!();

enum WorkerMessage {
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
    Analyze {
        input: PathBuf,
    },
}

enum UIMessage {
    Status(SharedString),
    AnalysisResult { encrypted: bool },
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

    // --- Encode Callbacks ---
    
    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_input_voice(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new()
            .add_filter("Audio Files", &["wav", "mp3"])
            .set_title("Select Input Voice")
            .pick_file()
        {
            ui.set_input_voice_path(path.to_string_lossy().to_string().into());
            check_all_encode_paths(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_input_picture(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new()
            .add_filter("Image Files", &["png", "jpg", "jpeg"])
            .set_title("Select Input Picture")
            .pick_file()
        {
            ui.set_input_picture_path(path.to_string_lossy().to_string().into());
            check_all_encode_paths(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_key_file(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new()
            .set_title("Select Key File")
            .pick_file()
        {
            ui.set_key_file_path(path.to_string_lossy().to_string().into());
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_output(move || {
        let ui = ui_handle_clone.unwrap();
        let format = ui.get_output_format();
        let ext = if format == "WAV" { "wav" } else { "png" };
        let title = format!("Save Output {}", format);
        
        if let Some(path) = FileDialog::new()
            .add_filter(format.as_str(), &[ext])
            .set_title(&title)
            .save_file()
        {
            ui.set_output_path(path.to_string_lossy().to_string().into());
            check_all_encode_paths(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    let worker_tx_clone = worker_tx.clone();
    ui.on_request_encode(move || {
        let ui = ui_handle_clone.unwrap();
        let voice_in: PathBuf = ui.get_input_voice_path().to_string().into();
        let picture_in: PathBuf = ui.get_input_picture_path().to_string().into();
        let key_path_str = ui.get_key_file_path().to_string();
        let key_in = if key_path_str.is_empty() { None } else { Some(PathBuf::from(key_path_str)) };
        let output: PathBuf = ui.get_output_path().to_string().into();
        let format = ui.get_output_format().to_string();
        let use_encryption = ui.get_use_encryption();

        worker_tx_clone
            .send(WorkerMessage::Encode {
                voice_in,
                picture_in,
                key_in,
                output,
                format,
                use_encryption,
            })
            .unwrap();
    });

    // --- Decode Callbacks ---
    
    let ui_handle_clone = ui_handle.clone();
    let worker_tx_clone_analyze = worker_tx.clone();
    ui.on_browse_decode_input(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new()
            .add_filter("Encoded Files", &["wav", "png"])
            .set_title("Select Encoded File")
            .pick_file()
        {
            let path_str = path.to_string_lossy().to_string();
            ui.set_decode_input_path(path_str.into());
            
            // Reset analysis state
            ui.set_input_analyzed(false);
            check_all_decode_paths(&ui); // This will disable button because input_analyzed is false
            
            // Trigger Analysis
            worker_tx_clone_analyze.send(WorkerMessage::Analyze { input: path }).unwrap();
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_decode_key_file(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new()
            .set_title("Select Key File")
            .pick_file()
        {
            ui.set_decode_key_file_path(path.to_string_lossy().to_string().into());
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_decode_output_voice(move || {
        let ui = ui_handle_clone.unwrap();
        // Enforce WAV extension
        if let Some(path) = FileDialog::new()
            .add_filter("Audio Files", &["wav"]) 
            .set_title("Save Restored Voice (WAV)")
            .save_file()
        {
            ui.set_decode_output_voice_path(path.to_string_lossy().to_string().into());
            check_all_decode_paths(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_decode_output_picture(move || {
        let ui = ui_handle_clone.unwrap();
        // Enforce PNG extension
        if let Some(path) = FileDialog::new()
            .add_filter("Image Files", &["png"])
            .set_title("Save Restored Picture (PNG)")
            .save_file()
        {
            ui.set_decode_output_picture_path(path.to_string_lossy().to_string().into());
            check_all_decode_paths(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    let worker_tx_clone_decode = worker_tx.clone();
    ui.on_request_decode(move || {
        let ui = ui_handle_clone.unwrap();
        let input: PathBuf = ui.get_decode_input_path().to_string().into();
        let key_path_str = ui.get_decode_key_file_path().to_string();
        let key_in = if key_path_str.is_empty() { None } else { Some(PathBuf::from(key_path_str)) };
        let voice_out: PathBuf = ui.get_decode_output_voice_path().to_string().into();
        let picture_out: PathBuf = ui.get_decode_output_picture_path().to_string().into();

        worker_tx_clone_decode
            .send(WorkerMessage::Decode {
                input,
                key_in,
                voice_out,
                picture_out,
            })
            .unwrap();
    });

    // --- UI Message Handler ---
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(100),
        move || {
            if let Ok(message) = ui_rx.try_recv() {
                handle_ui_message(ui_handle.clone(), message);
            }
        },
    );

    ui.run()
}

fn worker_thread_main(worker_rx: Receiver<WorkerMessage>, ui_tx: Sender<UIMessage>) {
    while let Ok(message) = worker_rx.recv() {
        match message {
            WorkerMessage::Encode {
                voice_in,
                picture_in,
                key_in,
                output,
                format,
                use_encryption,
            } => {
                ui_tx.send(UIMessage::Status("Encoding...".into())).unwrap();
                match crate::encoder::encode(&voice_in, &picture_in, key_in.as_ref(), &output, use_encryption, &format) {
                    Ok(_) => ui_tx.send(UIMessage::Status("Encoding complete!".into())).unwrap(),
                    Err(e) => ui_tx.send(UIMessage::Status(format!("Error: {}", e).into())).unwrap(),
                }
            }
            WorkerMessage::Decode {
                input,
                key_in,
                voice_out,
                picture_out,
            } => {
                ui_tx.send(UIMessage::Status("Decoding...".into())).unwrap();
                match crate::decoder::decode(&input, &voice_out, &picture_out, key_in.as_ref()) {
                    Ok(_) => ui_tx.send(UIMessage::Status("Decoding complete!".into())).unwrap(),
                    Err(e) => ui_tx.send(UIMessage::Status(format!("Error: {}", e).into())).unwrap(),
                }
            }
            WorkerMessage::Analyze { input } => {
                ui_tx.send(UIMessage::Status("Analyzing file...".into())).unwrap();
                match crate::decoder::analyze_header(&input) {
                    Ok(encrypted) => {
                        ui_tx.send(UIMessage::AnalysisResult { encrypted }).unwrap();
                        let msg = if encrypted { "File Encrypted. Key Required." } else { "File Clean. No Key Needed." };
                         ui_tx.send(UIMessage::Status(msg.into())).unwrap();
                    },
                    Err(e) => {
                         ui_tx.send(UIMessage::Status(format!("Check Error: {}", e).into())).unwrap();
                    }
                }
            }
        }
    }
}

fn handle_ui_message(ui_handle: Weak<AppWindow>, message: UIMessage) {
    if let Some(ui) = ui_handle.upgrade() {
        match message {
            UIMessage::Status(status) => {
                let theme = ui.global::<Theme>();
                let status_str = status.as_str();

                let color = if status_str.starts_with("Error") || status_str.contains("Failed") {
                    theme.get_error()
                } else if status_str.ends_with("complete!") || status_str.contains("Success") {
                    theme.get_success()
                } else if status_str.contains("Encrypted") {
                    theme.get_primary() // Blue info
                } else {
                    theme.get_text_normal()
                };

                ui.set_status_text(status);
                ui.set_status_color(color);
            }
            UIMessage::AnalysisResult { encrypted } => {
                ui.set_is_encrypted_source(encrypted);
                ui.set_input_analyzed(true);
                check_all_decode_paths(&ui);
            }
        }
    }
}

fn check_all_encode_paths(ui: &AppWindow) {
    let all_set = !ui.get_input_voice_path().is_empty()
        && !ui.get_input_picture_path().is_empty()
        && !ui.get_output_path().is_empty();
    ui.set_encode_button_enabled(all_set);
}

fn check_all_decode_paths(ui: &AppWindow) {
    let all_set = !ui.get_decode_input_path().is_empty()
        && !ui.get_decode_output_voice_path().is_empty()
        && !ui.get_decode_output_picture_path().is_empty()
        && ui.get_input_analyzed(); // Check validation state
    ui.set_decode_button_enabled(all_set);
}