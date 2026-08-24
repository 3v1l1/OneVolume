use crate::diagnostics::Diagnostics;

/// Startup diagnostics only. Live app state (current app, loudness,
/// gain, capture status) flows through `PipeWireEvent`/`LiveState`
/// straight to the UI now instead of living here — this used to also
/// hold `pipewire_connected`/`current_app`/`enabled` fields, but those
/// were never actually updated after the PipeWireClient duplication
/// was removed from app.rs, and are fully superseded by that channel.
#[derive(Debug, Clone)]
pub struct AppState {
    pub diagnostics: Diagnostics,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            diagnostics: Diagnostics::new(),
        }
    }
}
