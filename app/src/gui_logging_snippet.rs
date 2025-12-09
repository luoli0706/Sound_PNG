use tracing::{info, instrument};

// In gui.rs run function, inject logging into callbacks.
// For example:
ui.on_browse_input_voice(move || {
    info!("User action: Browse Input Voice");
    let ui = ui_handle_clone.unwrap();
    if let Some(path) = FileDialog::new().add_filter("Audio", &["wav", "mp3"]).pick_file() {
        info!("Selected Voice: {:?}", path);
        ui.set_input_voice_path(path.to_string_lossy().to_string().into());
        check_std_encode(&ui);
    } else {
        info!("Browse cancelled");
    }
});

// Similarly for all other callbacks...
// I will rewrite gui.rs completely to include all logging calls.
