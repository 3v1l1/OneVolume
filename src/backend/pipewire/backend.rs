use std::sync::mpsc::{Receiver, Sender};

use pipewire as pw;

use crate::backend::events::{PipeWireEvent, UiCommand};

use super::runtime::{Runtime, SharedRuntime};

pub struct PipeWireClient {
    runtime: SharedRuntime,
    sender: Sender<PipeWireEvent>,
}

impl PipeWireClient {
    pub fn new(
        sender: Sender<PipeWireEvent>,
        command_receiver: Receiver<UiCommand>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        pw::init();

        println!("✓ PipeWire initialized");

        Ok(Self {
            runtime: Runtime::new(command_receiver),
            sender,
        })
    }

    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Step 1: create the PipeWire main loop.
        let mainloop = match pw::main_loop::MainLoopRc::new(None) {
            Ok(value) => {
                println!("✓ PipeWire MainLoop created");
                value
            }
            Err(err) => {
                eprintln!("❌ PipeWire MainLoop creation failed: {err:?}");
                return Err(err.into());
            }
        };

        // Step 2: create the PipeWire context.
        let context = match pw::context::ContextRc::new(&mainloop, None) {
            Ok(value) => {
                println!("✓ PipeWire Context created");
                value
            }
            Err(err) => {
                eprintln!("❌ PipeWire Context creation failed: {err:?}");
                return Err(err.into());
            }
        };

        // Step 3: connect the context to the PipeWire daemon.
        let core = match context.connect_rc(None) {
            Ok(value) => {
                println!("✓ Connected to PipeWire");
                value
            }
            Err(err) => {
                eprintln!("❌ PipeWire core connection failed: {err:?}");
                return Err(err.into());
            }
        };

        // Step 4: obtain the registry.
        let registry = match core.get_registry_rc() {
            Ok(value) => value,
            Err(err) => {
                eprintln!("❌ PipeWire registry creation failed: {err:?}");
                return Err(err.into());
            }
        };

        // Step 5: attach the OneVolume registry listeners.
        super::registry::attach(self.runtime.clone(), &registry, &core, self.sender.clone())?;

        // Step 6: wait for the initial registry synchronization.
        let pending = core.sync(0)?;

        let sender_for_sync = self.sender.clone();

        let _listener = core
            .add_listener_local()
            .done(move |id, seq| {
                if id == pw::core::PW_ID_CORE && seq == pending {
                    println!(
                        "✓ Initial sync complete — OneVolume is now watching for supported apps"
                    );

                    // Notify GTK that PipeWire is genuinely connected
                    // and the initial registry sync has completed.
                    let _ = sender_for_sync.send(PipeWireEvent::Connected);
                }
            })
            .register();

        // Keep the PipeWire event loop alive for the lifetime of OneVolume.
        mainloop.run();

        Ok(())
    }
}
