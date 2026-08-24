use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use gtk::{Align, Application, ApplicationWindow, Box, Button, Frame, Label, Orientation};

use crate::backend::events::PipeWireEvent;
use crate::backend::service::PipeWireService;

pub fn build_ui(app: &Application, service: Rc<PipeWireService>) {
    // =========================
    // Title
    // =========================

    let title = Label::builder()
        .label("🎬 OneVolume")
        .halign(Align::Start)
        .build();

    let subtitle = Label::builder()
        .label("Set the volume once.")
        .halign(Align::Start)
        .build();

    // =========================
    // Watching
    // =========================

    let watching_title = Label::builder()
        .label("<b>Watching</b>")
        .use_markup(true)
        .halign(Align::Start)
        .build();

    let watching = Label::builder()
        .label("No active media")
        .halign(Align::Start)
        .build();

    // =========================
    // Status
    // =========================

    let status_title = Label::builder()
        .label("<b>Status</b>")
        .use_markup(true)
        .halign(Align::Start)
        .build();

    let status = Label::builder()
        .label("🟡 Watching — nothing playing")
        .halign(Align::Start)
        .build();

    // =========================
    // Loudness
    // =========================

    let loudness_title = Label::builder()
        .label("<b>Loudness</b>")
        .use_markup(true)
        .halign(Align::Start)
        .build();

    let loudness = Label::builder().label("— dB").halign(Align::Start).build();

    // =========================
    // Gain
    // =========================

    let gain_title = Label::builder()
        .label("<b>Gain</b>")
        .use_markup(true)
        .halign(Align::Start)
        .build();

    let gain = Label::builder().label("— dB").halign(Align::Start).build();

    // =========================
    // Enable / Disable
    // =========================

    // Backend starts enabled, so the UI starts in the matching state.
    let enabled = Rc::new(Cell::new(true));

    let enable_button = Button::builder().label("Disable").build();

    let service_for_button = service.clone();
    let enabled_for_button = enabled.clone();
    let status_for_button = status.clone();

    enable_button.connect_clicked(move |button| {
        let new_enabled = !enabled_for_button.get();

        enabled_for_button.set(new_enabled);
        service_for_button.set_enabled(new_enabled);

        if new_enabled {
            button.set_label("Disable");
            status_for_button.set_label("🟢 Active — waiting for media");
        } else {
            button.set_label("Enable");
            status_for_button.set_label("⏸️ Disabled — normal volume restored");
        }
    });

    // =========================
    // Diagnostics Button
    // =========================

    let diagnostics_button = Button::builder().label("▶ Diagnostics").build();

    // =========================
    // Diagnostics Panel
    // =========================

    let diagnostics_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();

    diagnostics_box.append(
        &Label::builder()
            .label("✓ UI Loaded")
            .halign(Align::Start)
            .build(),
    );

    let diag_pipewire = Label::builder()
        .label("○ Waiting for PipeWire...")
        .halign(Align::Start)
        .build();

    diagnostics_box.append(&diag_pipewire);

    let diag_media = Label::builder()
        .label("○ Waiting for media...")
        .halign(Align::Start)
        .build();

    diagnostics_box.append(&diag_media);

    let diagnostics_frame = Frame::builder().child(&diagnostics_box).build();

    // Hidden by default.
    diagnostics_frame.set_visible(false);

    // =========================
    // Diagnostics Toggle
    // =========================

    let frame = diagnostics_frame.clone();
    let button = diagnostics_button.clone();

    diagnostics_button.connect_clicked(move |_| {
        let visible = frame.is_visible();

        if visible {
            frame.set_visible(false);
            button.set_label("▶ Diagnostics");
        } else {
            frame.set_visible(true);
            button.set_label("▼ Diagnostics");
        }
    });

    // =========================
    // Main Layout
    // =========================

    let layout = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(15)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .build();

    layout.append(&title);
    layout.append(&subtitle);

    layout.append(&watching_title);
    layout.append(&watching);

    layout.append(&status_title);
    layout.append(&status);

    layout.append(&loudness_title);
    layout.append(&loudness);

    layout.append(&gain_title);
    layout.append(&gain);

    layout.append(&enable_button);

    layout.append(&diagnostics_button);
    layout.append(&diagnostics_frame);

    // =========================
    // Window
    // =========================

    let window = ApplicationWindow::builder()
        .application(app)
        .title("OneVolume")
        .default_width(600)
        .default_height(450)
        .child(&layout)
        .build();

    // =========================
    // Live State Polling
    // =========================

    let watching_for_timer = watching.clone();
    let status_for_timer = status.clone();
    let loudness_for_timer = loudness.clone();
    let gain_for_timer = gain.clone();
    let enable_button_for_timer = enable_button.clone();
    let enabled_for_timer = enabled.clone();

    glib::timeout_add_local(Duration::from_millis(500), move || {
        for event in service.poll() {
            match event {
                PipeWireEvent::Connected => {
                    diag_pipewire.set_label("✓ PipeWire Connected");
                }

                PipeWireEvent::StateUpdate(state) => {
                    match &state.current_app {
                        Some(app_name) => {
                            watching_for_timer.set_label(app_name);
                            diag_media.set_label(&format!("✓ Media detected: {app_name}"));
                        }

                        None => {
                            watching_for_timer.set_label("No active media");
                            diag_media.set_label("○ Waiting for media...");
                        }
                    }

                    // Keep the button synchronized with the backend.
                    if state.enabled != enabled_for_timer.get() {
                        enabled_for_timer.set(state.enabled);

                        enable_button_for_timer.set_label(if state.enabled {
                            "Disable"
                        } else {
                            "Enable"
                        });
                    }

                    if !state.enabled {
                        status_for_timer.set_label("⏸️ Disabled — normal volume restored");
                    } else if state.capture_running {
                        status_for_timer.set_label(&format!(
                            "🟢 Active — {} stream(s)",
                            state.active_stream_count
                        ));
                    } else {
                        status_for_timer.set_label("🟡 Watching — nothing playing");
                    }

                    loudness_for_timer.set_label(&format!("{:.1} dB", state.loudness_db));

                    gain_for_timer.set_label(&format!("{:+.1} dB", state.gain_db));
                }
            }
        }

        glib::ControlFlow::Continue
    });

    window.present();
}
