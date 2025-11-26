mod decoder;
mod encoder;
mod gui;
mod utils;
mod converter;
mod header;
mod security;
mod repro_test;

fn main() -> anyhow::Result<()> {
    gui::run()?;
    Ok(())
}
