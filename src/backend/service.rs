use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::backend::events::{PipeWireEvent, UiCommand};

pub struct PipeWireService {
    handle: JoinHandle<()>,
    receiver: Receiver<PipeWireEvent>,
    command_sender: Sender<UiCommand>,
}

impl PipeWireService {
    pub fn start() -> Self {
        let (event_tx, event_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            run_pipewire(event_tx, command_rx);
        });

        Self {
            handle,
            receiver: event_rx,
            command_sender: command_tx,
        }
    }

    pub fn poll(&self) -> Vec<PipeWireEvent> {
        let mut events = Vec::new();

        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);
        }

        events
    }

    /// Toggle the leveler on/off from the UI. Fire-and-forget — if the
    /// PipeWire thread isn't up yet (channel disconnected), this just
    /// silently no-ops rather than panicking, since the button being
    /// clicked before the backend is ready isn't a real error case.
    pub fn set_enabled(&self, enabled: bool) {
        let _ = self.command_sender.send(UiCommand::SetEnabled(enabled));
    }

    #[allow(dead_code)]
    pub fn join(self) {
        let _ = self.handle.join();
    }
}

fn run_pipewire(sender: Sender<PipeWireEvent>, command_receiver: Receiver<UiCommand>) {
    match crate::backend::pipewire::PipeWireClient::new(sender, command_receiver) {
        Ok(client) => {
            if let Err(err) = client.run() {
                eprintln!("PipeWire error: {err}");
            }
        }
        Err(err) => {
            eprintln!("Failed to initialize PipeWire: {err}");
        }
    }
}
