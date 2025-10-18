use eframe::egui::Context;
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub struct TransmitterReloader<T> {
    transmitter: mpsc::Sender<T>,
    ctx: Context,
}

impl<T> TransmitterReloader<T> {
    pub fn new(transmitter: mpsc::Sender<T>, ctx: Context) -> Self {
        Self { transmitter, ctx }
    }

    pub fn send(&self, message: T) -> Result<(), mpsc::SendError<T>> {
        self.transmitter
            .send(message)
            .inspect(|_| self.ctx.request_repaint())
    }
}
