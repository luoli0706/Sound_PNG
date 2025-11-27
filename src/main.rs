#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod decoder;
mod encoder;
mod gui;
mod utils;
mod converter;
mod header;
mod security;

fn main() -> anyhow::Result<()> {
    gui::run()?;
    Ok(())
}
