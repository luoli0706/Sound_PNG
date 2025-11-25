use rfd::FileDialog;
use slint::{PlatformError, SharedString, Weak};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::{path::PathBuf, thread};

slint::include_modules!();

enum WorkerMessage {
    Encode {
        wav_in: PathBuf,
        png_in: PathBuf,
        wav_out: PathBuf,
    },
    Decode {
        wav_in: PathBuf,
        wav_out: PathBuf,
        png_out: PathBuf,
    },
}

enum UIMessage {
    Status(SharedString),
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
    ui.on_browse_input_wav(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new()
            .add_filter("WAV Files", &["wav"])
            .set_title("Select Input WAV")
            .pick_file()
        {
            ui.set_input_wav_path(path.to_string_lossy().to_string().into());
            check_all_encode_paths(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_input_png(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new()
            .add_filter("PNG Files", &["png"])
            .set_title("Select Input PNG")
            .pick_file()
        {
            ui.set_input_png_path(path.to_string_lossy().to_string().into());
            check_all_encode_paths(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_output_wav(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new()
            .add_filter("WAV Files", &["wav"])
            .set_title("Save Output WAV As...")
            .save_file()
        {
            ui.set_output_wav_path(path.to_string_lossy().to_string().into());
            check_all_encode_paths(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    let worker_tx_clone = worker_tx.clone();
    ui.on_request_encode(move || {
        let ui = ui_handle_clone.unwrap();
        let wav_in: PathBuf = ui.get_input_wav_path().to_string().into();
        let png_in: PathBuf = ui.get_input_png_path().to_string().into();
        let wav_out: PathBuf = ui.get_output_wav_path().to_string().into();

        worker_tx_clone
            .send(WorkerMessage::Encode {
                wav_in,
                png_in,
                wav_out,
            })
            .unwrap();
    });

    // --- Decode Callbacks ---
    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_decode_input_wav(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new()
            .add_filter("WAV Files", &["wav"])
            .set_title("Select Input WAV")
            .pick_file()
        {
            ui.set_decode_input_wav_path(path.to_string_lossy().to_string().into());
            check_all_decode_paths(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_decode_output_wav(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new()
            .add_filter("WAV Files", &["wav"])
            .set_title("Save Restored WAV As...")
            .save_file()
        {
            ui.set_decode_output_wav_path(path.to_string_lossy().to_string().into());
            check_all_decode_paths(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    ui.on_browse_decode_output_png(move || {
        let ui = ui_handle_clone.unwrap();
        if let Some(path) = FileDialog::new()
            .add_filter("PNG Files", &["png"])
            .set_title("Save Restored PNG As...")
            .save_file()
        {
            ui.set_decode_output_png_path(path.to_string_lossy().to_string().into());
            check_all_decode_paths(&ui);
        }
    });

    let ui_handle_clone = ui_handle.clone();
    let worker_tx_clone_decode = worker_tx.clone();
    ui.on_request_decode(move || {
        let ui = ui_handle_clone.unwrap();
        let wav_in: PathBuf = ui.get_decode_input_wav_path().to_string().into();
        let wav_out: PathBuf = ui.get_decode_output_wav_path().to_string().into();
        let png_out: PathBuf = ui.get_decode_output_png_path().to_string().into();

        worker_tx_clone_decode
            .send(WorkerMessage::Decode {
                wav_in,
                wav_out,
                png_out,
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
                wav_in,
                png_in,
                wav_out,
            } => {
                ui_tx.send(UIMessage::Status("Encoding...".into())).unwrap();
                match crate::encoder::encode(&wav_in, &png_in, &wav_out) {
                    Ok(_) => ui_tx
                        .send(UIMessage::Status("Encoding complete!".into()))
                        .unwrap(),
                    Err(e) => ui_tx
                        .send(UIMessage::Status(format!("Error: {}", e).into()))
                        .unwrap(),
                }
            }
            WorkerMessage::Decode {
                wav_in,
                wav_out,
                png_out,
            } => {
                ui_tx.send(UIMessage::Status("Decoding...".into())).unwrap();
                match crate::decoder::decode(&wav_in, &wav_out, &png_out) {
                    Ok(_) => ui_tx
                        .send(UIMessage::Status("Decoding complete!".into()))
                        .unwrap(),
                    Err(e) => ui_tx
                        .send(UIMessage::Status(format!("Error: {}", e).into()))
                        .unwrap(),
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

                let color = if status_str.starts_with("Error") {
                    theme.get_error()
                } else if status_str.ends_with("complete!") {
                    theme.get_success()
                } else {
                    theme.get_text_normal()
                };

                ui.set_status_text(status);
                ui.set_status_color(color);
            }
        }
    }
}

fn check_all_encode_paths(ui: &AppWindow) {
    let all_set = !ui.get_input_wav_path().is_empty()
        && !ui.get_input_png_path().is_empty()
        && !ui.get_output_wav_path().is_empty();
    ui.set_encode_button_enabled(all_set);
}

fn check_all_decode_paths(ui: &AppWindow) {
    let all_set = !ui.get_decode_input_wav_path().is_empty()
        && !ui.get_decode_output_wav_path().is_empty()
        && !ui.get_decode_output_png_path().is_empty();
    ui.set_decode_button_enabled(all_set);
}
