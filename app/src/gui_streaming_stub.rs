use rfd::FileDialog;
use slint::{PlatformError, SharedString, Weak, ComponentHandle, CloseRequestResponse};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::{path::PathBuf, thread, fs, process::Command};

slint::include_modules!();

mod stream_encoder;
mod stream_decoder;

enum WorkerMessage {
    // Legacy: Encode, Decode (Standard)
    Encode { ... }, // (kept for compatibility if needed, but we can map to new stream)
    Decode { ... },

    // Beta 3.0 Streaming
    EncodeStream {
        payload: PathBuf,
        container: PathBuf,
        key: Option<PathBuf>,
        output: PathBuf,
        encrypt: bool,
    },
    DecodeStream {
        input: PathBuf,
        output_path: PathBuf, // Payload output
        key: Option<PathBuf>,
    },
    
    Analyze { ... },
}

// ... rest of gui.rs adapted to use stream_encoder::encode_stream and stream_decoder::decode_stream
// Need to update `run` and `worker_thread_main`.
// Note: Since I cannot overwrite partial file easily with huge logic, I will rewrite the file logic now.
// Assuming I need to fully replace the `worker_thread_main` match arms.
