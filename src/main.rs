#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod decoder;
mod encoder;
mod gui;
mod utils;
mod header;
mod security;
mod converter;
mod stream_encoder;
mod stream_decoder;

fn main() -> Result<(), slint::PlatformError> {
    gui::run()
}
