use crate::config::SUPPORTED_APPS;

pub struct Detector;

impl Detector {
    pub fn is_supported(app_name: &str) -> bool {
        if app_name.eq_ignore_ascii_case("VLC media player")
            || app_name
                .to_ascii_lowercase()
                .starts_with("vlc media player (")
        {
            return true;
        }

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

#[cfg(test)]
mod tests {
    use super::Detector;

    #[test]
    fn detects_vlc_runtime_name() {
        assert!(Detector::is_supported("VLC media player"));
        assert!(Detector::is_supported("VLC media player (LibVLC 3.0.23)"));
        assert!(Detector::is_supported("vlc media player (LibVLC 4.0.0)"));
    }

    #[test]
    fn rejects_unrelated_vlc_name() {
        assert!(!Detector::is_supported("Not VLC media player"));
        assert!(!Detector::is_supported("My VLC media player tool"));
    }

    #[test]
    fn keeps_existing_supported_apps_working() {
        assert!(Detector::is_supported("Firefox"));
        assert!(Detector::is_supported("Brave"));
        assert!(Detector::is_supported("Chromium"));
    }
}
