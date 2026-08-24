/// A snapshot of what OneVolume is doing right now, sent from the
/// PipeWire thread to the GTK thread. Plain data only (no PipeWire
/// types) since it has to cross a thread boundary — `Runtime` itself
/// (Rc-based) can't safely do that directly.
#[derive(Debug, Clone)]
pub struct LiveState {
    pub capture_running: bool,
    /// Name of a currently-playing supported app, if any. `None` when
    /// nothing is actively "Running" (paused, or nothing open) even
    /// if a supported app is technically still registered.
    pub current_app: Option<String>,
    pub active_stream_count: usize,
    pub loudness_db: f32,
    pub gain_db: f32,
    /// Mirrors the last `UiCommand::SetEnabled` the backend actually
    /// processed — lets the UI confirm its own toggle took effect
    /// rather than only trusting its own local click state.
    pub enabled: bool,
}

impl Default for LiveState {
    fn default() -> Self {
        Self {
            capture_running: false,
            current_app: None,
            active_stream_count: 0,
            loudness_db: 0.0,
            gain_db: 0.0,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PipeWireEvent {
    /// Sent once, right after the registry is attached and the initial
    /// sync completes — i.e. actually connected and watching, not just
    /// "the process started".
    Connected,

    /// Sent roughly once a second while the capture stream is active,
    /// carrying a fresh snapshot for the UI to display.
    StateUpdate(LiveState),
}

/// The other direction: GTK thread → PipeWire thread. GTK can't touch
/// `Runtime` directly (it's `Rc`-based, not `Send`), so button clicks
/// etc. go through this channel instead, same pattern as
/// `PipeWireEvent` but reversed.
#[derive(Debug, Clone)]
pub enum UiCommand {
    /// User toggled the Enable/Disable button. When disabled, the
    /// capture stream keeps measuring (so the log/UI numbers stay
    /// live) but stops writing gain to real volume — and immediately
    /// writes back neutral (1.0x) so disabling actually hands control
    /// back to the user's normal volume right away, not just freezes
    /// wherever gain last was.
    SetEnabled(bool),
}
