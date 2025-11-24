mod decoder;
mod encoder;
mod utils;
mod gui;

fn main() -> anyhow::Result<()> {
    gui::run()?;
    Ok(())
}