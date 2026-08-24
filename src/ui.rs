use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use gtk::{
    Align, Application, ApplicationWindow, Box, Button, Frame, Label, Orientation, ProgressBar,
};

use crate::backend::events::PipeWireEvent;
use crate::backend::service::PipeWireService;

pub fn build_ui(app: &Application, service: Rc<PipeWireService>) {
    // ============================================================
    // State
    // ============================================================

    let enabled = Rc::new(Cell::new(true));

    // ============================================================
    // Header
    // ============================================================

    let title = Label::builder()
        .label("🎬 OneVolume")
        .halign(Align::Start)
        .build();

    title.add_css_class("title-1");

    let subtitle = Label::builder()
        .label("Keep your volume steady.")
        .halign(Align::Start)
        .build();

    subtitle.add_css_class("dim-label");

    let header = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(4)
        .build();

    header.append(&title);
    header.append(&subtitle);

    // ============================================================
    // Main status
    // ============================================================

    let status = Label::builder()
        .label("🟡 Watching — nothing playing")
        .halign(Align::Start)
        .build();

    status.add_css_class("heading");

    // ============================================================
    // Currently Playing
    // ============================================================

    let watching_label = Label::builder()
        .label("CURRENTLY PLAYING")
        .halign(Align::Start)
        .build();

    watching_label.add_css_class("caption-heading");

    let watching = Label::builder()
        .label("No active media")
        .halign(Align::Start)
        .build();

    watching.add_css_class("title-2");

    let media_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(6)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(18)
        .margin_end(18)
        .build();

    media_box.append(&watching_label);
    media_box.append(&watching);

    let media_frame = Frame::builder().child(&media_box).build();

    // ============================================================
    // Loudness
    // ============================================================

    let loudness_heading = Label::builder()
        .label("LOUDNESS")
        .halign(Align::Start)
        .build();

    loudness_heading.add_css_class("caption-heading");

    let loudness = Label::builder().label("— dB").halign(Align::Start).build();

    loudness.add_css_class("title-2");

    let loudness_bar = ProgressBar::builder()
        .fraction(0.0)
        .show_text(false)
        .hexpand(true)
        .height_request(8)
        .build();

    let loudness_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .build();

    loudness_box.append(&loudness_heading);
    loudness_box.append(&loudness);
    loudness_box.append(&loudness_bar);

    // ============================================================
    // Gain
    // ============================================================

    let gain_heading = Label::builder()
        .label("ONEVOLUME ADJUSTMENT")
        .halign(Align::Start)
        .build();

    gain_heading.add_css_class("caption-heading");

    let gain = Label::builder()
        .label("0.0 dB")
        .halign(Align::Start)
        .build();

    gain.add_css_class("title-2");

    let gain_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(10)
        .build();

    gain_box.append(&gain_heading);
    gain_box.append(&gain);

    // ============================================================
    // Enable / Disable
    // ============================================================

    let enable_button = Button::builder()
        .label("Disable OneVolume")
        .hexpand(true)
        .height_request(44)
        .build();

    enable_button.add_css_class("suggested-action");

    let service_for_button = service.clone();
    let enabled_for_button = enabled.clone();
    let status_for_button = status.clone();

    enable_button.connect_clicked(move |button| {
        let new_enabled = !enabled_for_button.get();

        enabled_for_button.set(new_enabled);
        service_for_button.set_enabled(new_enabled);

        if new_enabled {
            button.set_label("Disable OneVolume");
            button.add_css_class("suggested-action");
            status_for_button.set_label("🟢 Active — waiting for media");
        } else {
            button.set_label("Enable OneVolume");
            button.remove_css_class("suggested-action");
            status_for_button.set_label("⏸️ Disabled — normal volume restored");
        }
    });

    // ============================================================
    // Diagnostics
    // ============================================================

    let diagnostics_button = Button::builder()
        .label("▶ Diagnostics")
        .halign(Align::Start)
        .build();

    diagnostics_button.add_css_class("flat");

    let diagnostics_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(14)
        .margin_end(14)
        .build();

    let diag_pipewire = Label::builder()
        .label("○ Waiting for PipeWire...")
        .halign(Align::Start)
        .build();

    let diag_media = Label::builder()
        .label("○ Waiting for media...")
        .halign(Align::Start)
        .build();

    diagnostics_box.append(&diag_pipewire);
    diagnostics_box.append(&diag_media);

    let diagnostics_frame = Frame::builder().child(&diagnostics_box).build();

    diagnostics_frame.set_visible(false);

    let frame = diagnostics_frame.clone();
    let diagnostics_toggle = diagnostics_button.clone();

    diagnostics_button.connect_clicked(move |_| {
        let visible = frame.is_visible();

        if visible {
            frame.set_visible(false);
            diagnostics_toggle.set_label("▶ Diagnostics");
        } else {
            frame.set_visible(true);
            diagnostics_toggle.set_label("▼ Diagnostics");
        }
    });

    // ============================================================
    // Main Layout
    // ============================================================

    let content = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(18)
        .margin_top(26)
        .margin_bottom(26)
        .margin_start(26)
        .margin_end(26)
        .build();

    content.append(&header);
    content.append(&status);
    content.append(&media_frame);
    content.append(&loudness_box);
    content.append(&gain_box);
    content.append(&enable_button);
    content.append(&diagnostics_button);
    content.append(&diagnostics_frame);

    // ============================================================
    // Window
    // ============================================================

    let window = ApplicationWindow::builder()
        .application(app)
        .title("OneVolume")
        .default_width(560)
        .default_height(520)
        .resizable(false)
        .child(&content)
        .build();

    // ============================================================
    // Live State Polling
    // ============================================================

    let watching_for_timer = watching.clone();
    let status_for_timer = status.clone();
    let loudness_for_timer = loudness.clone();
    let gain_for_timer = gain.clone();
    let enable_button_for_timer = enable_button.clone();
    let enabled_for_timer = enabled.clone();
    let loudness_bar_for_timer = loudness_bar.clone();

    glib::timeout_add_local(Duration::from_millis(500), move || {
        for event in service.poll() {
            match event {
                PipeWireEvent::Connected => {
                    diag_pipewire.set_label("✓ PipeWire Connected");
                }

                PipeWireEvent::StateUpdate(state) => {
                    // ------------------------------------------------
                    // Current application
                    // ------------------------------------------------

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

                    // ------------------------------------------------
                    // Enable state
                    // ------------------------------------------------

                    if state.enabled != enabled_for_timer.get() {
                        enabled_for_timer.set(state.enabled);

                        if state.enabled {
                            enable_button_for_timer.set_label("Disable OneVolume");
                            enable_button_for_timer.add_css_class("suggested-action");
                        } else {
                            enable_button_for_timer.set_label("Enable OneVolume");
                            enable_button_for_timer.remove_css_class("suggested-action");
                        }
                    }

                    // ------------------------------------------------
                    // Main status
                    // ------------------------------------------------

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

                    // ------------------------------------------------
                    // Loudness
                    // ------------------------------------------------

                    loudness_for_timer.set_label(&format!("{:.1} dB", state.loudness_db));

                    // Map approximately:
                    // -60 dB = 0%
                    //   0 dB = 100%
                    let loudness_fraction = ((state.loudness_db + 60.0) / 60.0).clamp(0.0, 1.0);

                    loudness_bar_for_timer.set_fraction(loudness_fraction as f64);

                    // ------------------------------------------------
                    // Gain
                    // ------------------------------------------------

                    gain_for_timer.set_label(&format!("{:+.1} dB", state.gain_db));
                }
            }
        }

        glib::ControlFlow::Continue
    });

    window.present();
}
