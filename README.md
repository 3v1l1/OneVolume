# 🎬 OneVolume

**Set the volume once.**

OneVolume is a Linux audio leveler that dynamically adjusts the volume of supported applications to reduce sudden loudness changes while watching movies, videos, or other media.

## 🚀 Recommended: AppImage

The AppImage is the recommended way to run OneVolume.

OneVolume needs direct access to PipeWire audio streams so it can adjust the volume of individual applications.

The AppImage has been tested with:

- VLC
- Firefox
- Brave

### Download

Download the latest AppImage from the GitHub Releases page.

Make it executable:

```bash
chmod +x OneVolume-x86_64.AppImage

Run it:

./OneVolume-x86_64.AppImage
Verify the download

SHA-256 checksums are provided with each release.

sha256sum -c SHA256SUMS

For release authenticity, verify the signed Git tag when applicable.

To verify: `gpg --import docs/gpg-public-key.asc` then `git tag -v v0.5.0-beta.1`.

Key fingerprint: `02CE 0A43 1158 28F6 88E1 4DCB 6AE7 B5AC F7AE 61B1`

🔊 How it works

OneVolume monitors the system's audio output through PipeWire and calculates a target gain based on the detected loudness.

The application then applies the calculated volume directly to the relevant PipeWire audio stream.

This allows the leveler to respond continuously while media is playing.

🎯 Supported applications

OneVolume currently supports detection and volume control for:

- Firefox
- Brave
- Chromium
- Google Chrome
- Microsoft Edge
- VLC
- Discord

Additional applications may work when they expose their audio streams through PipeWire in a compatible way.

🧪 Flatpak

A Flatpak build is retained as experimental.

Flatpak's sandboxing can restrict access to other applications' audio streams, which prevents reliable cross-application volume control in some environments.

For the full OneVolume functionality, use the AppImage or native build.

🔐 Privacy

OneVolume processes audio levels locally on your computer.

It does not upload your audio, microphone data, playback data, or application audio to a remote server.

No account is required.

🛡️ Security

Please see SECURITY.md for reporting security vulnerabilities.

Do not publicly disclose an undisclosed vulnerability before coordinating a fix.

🏗️ Building from source

Requirements:

Rust
Cargo
GTK4 development libraries
PipeWire development libraries

Clone the repository:

git clone git@github.com:3v1l1/OneVolume.git
cd OneVolume

Build:

cargo build --release

Run:

./target/release/onevolume
🧪 Development / testing

Dry-run mode:

ONEVOLUME_DRY_RUN=1 ./target/release/onevolume

This calculates the leveler gain without modifying real application volume.

📦 Project status

OneVolume is currently in beta.

The AppImage is the recommended distribution format for testing the current cross-application PipeWire volume-control functionality.

Feedback, bug reports, and security reports are welcome.

📄 License

See the repository for license information.
