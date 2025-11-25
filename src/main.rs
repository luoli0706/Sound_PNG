mod decoder;
mod encoder;
mod gui;
mod utils;

fn main() -> anyhow::Result<()> {
    gui::run()?;
    Ok(())
}
