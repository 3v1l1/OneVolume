use crate::config::SUPPORTED_APPS;

pub struct Detector;

impl Detector {
    pub fn is_supported(app_name: &str) -> bool {
        SUPPORTED_APPS
            .iter()
            .any(|app| app.eq_ignore_ascii_case(app_name))
    }

    /// Is this node an app's audio *playback* stream — the kind of node
    /// that carries a movie or show's actual sound — as opposed to a
    /// sink, a mic input, a camera, MIDI, or a driver node? Those other
    /// kinds are the majority of what PipeWire reports and OneVolume
    /// has no business touching them.
    pub fn is_audio_playback_stream(media_class: &str) -> bool {
        media_class == "Stream/Output/Audio"
    }
}
