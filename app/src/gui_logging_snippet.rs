use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;

pub struct ChannelWriter {
    sender: Sender<String>,
}

impl ChannelWriter {
    pub fn new(sender: Sender<String>) -> Self {
        Self { sender }
    }
}

impl std::io::Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let msg = String::from_utf8_lossy(buf).to_string();
        let _ = self.sender.send(msg);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}