use gtk::Application;
use gtk::prelude::*;

use crate::backend::service::PipeWireService;
use crate::state::AppState;
use crate::ui;

pub fn run() {
    // =========================
    // Application State
    // =========================

    let mut state = AppState::new();

    // =========================
    // Start PipeWire Worker Thread
    // =========================

    // This spawns the actual PipeWire connection + registry watching
    // on its own thread (mainloop.run() blocks, so it can't share a
    // thread with GTK's own main loop). There used to be a second,
    // separate PipeWireClient created here just to check "did
    // pw::init() succeed" — but that one was never actually run,
    // creating a duplicate, unused PipeWire context. Removed; the
    // service below is the only PipeWire connection now, and its
    // `Connected` event (sent from inside the real running client) is
    // what the UI listens for instead.
    let service = std::rc::Rc::new(PipeWireService::start());
    state.diagnostics.add("✓ PipeWire Worker Started");

    // =========================
    // Diagnostics
    // =========================

    state.diagnostics.add("✓ GTK Initialized");
    state.diagnostics.add("✓ Backend Ready");

    // =========================
    // Terminal Output
    // =========================

    println!("==============================");
    println!("      OneVolume Starting");
    println!("==============================");

    for message in state.diagnostics.all() {
        println!("{message}");
    }

    println!("==============================");

    // =========================
    // GTK Application
    // =========================

    let app = Application::builder()
        .application_id("com.onevolume.OneVolume")
        .build();

    app.connect_activate(move |app| {
        ui::build_ui(app, service.clone());
    });

    app.run();
}
