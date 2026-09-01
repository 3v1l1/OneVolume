# OneVolume

**OneVolume is a Linux desktop audio leveler designed to reduce sudden volume changes while watching movies, videos, and other media.**

Built with:

- Rust
- GTK4
- PipeWire

OneVolume continuously analyzes active media audio locally and dynamically adjusts gain to make loud and quiet sections more comfortable to listen to.

> ⚠️ **Beta software:** OneVolume is currently in beta development. Bugs, unexpected behavior, and incomplete functionality may still occur.

---

## Features

- 🎚️ Dynamic media volume leveling
- 🔊 Fast response to sudden loud peaks
- 📉 Gradual recovery after loud sections
- 🎧 PipeWire audio capture
- 🧠 RMS and peak-based audio analysis
- 🎛️ Dynamic per-application PipeWire volume control
- 🌐 Firefox media detection
- 🌐 Brave media detection
- 🎬 VLC media player support
- 🖥️ GTK4 desktop interface
- 📦 AppImage distribution
- 🔐 GPG-signed Git release tags
- #️⃣ SHA-256 release checksums

---

## How It Works

OneVolume monitors the audio level of supported media applications and calculates an appropriate gain adjustment.

The leveler uses two complementary measurements:

- **RMS level** — represents the general loudness of the audio.
- **Peak level** — detects sudden loud sounds and transients.

Peak detection uses a peak follower:

- Peaks rise quickly.
- Peaks decay gradually.
- A single quiet buffer does not immediately release the detector.
- Sudden loud sounds can trigger fast protection.

The goal is to reduce large volume jumps without aggressively changing normal dialogue or quiet sections.

All audio analysis and gain calculation are performed locally on the user's machine.

---

## Distribution

### Recommended: AppImage

AppImage is currently the recommended distribution format for OneVolume.

OneVolume needs to interact with the PipeWire audio streams of other desktop applications in order to dynamically control their volume. The native Linux build and AppImage have been tested successfully with:

- VLC
- Brave
- Firefox

### Download

Download the latest AppImage from the GitHub Releases page:

https://github.com/3v1l1/OneVolume/releases

Make it executable:

```bash
chmod +x OneVolume-x86_64.AppImage

Run it:

./OneVolume-x86_64.AppImage

No Rust toolchain or development packages are required to run the released AppImage.

Verify Your Download

Each release provides a SHA256SUMS file.

After downloading the AppImage and SHA256SUMS, verify the file:

sha256sum -c SHA256SUMS

A successful verification looks like:

OneVolume-x86_64.AppImage: OK

The SHA-256 checksum verifies the integrity of the downloaded file against the checksum published with the release.

GPG-signed release tags

Git release tags are GPG-signed.

To verify a tag after cloning the repository:

git verify-tag v0.5.0-beta.1

You should only trust a signed tag after verifying that its signing key belongs to the expected project maintainer.

Never publish or share your private GPG key.

Flatpak Status

Flatpak packaging is currently experimental and not the recommended distribution format.

OneVolume can detect and capture supported applications through PipeWire when sandboxed, but the Flatpak sandbox currently prevents reliable cross-application volume control required by OneVolume.

For this reason, the supported release format for v0.5.0-beta.1 is AppImage.

The Flatpak manifest remains in the repository for development and future investigation.

Supported Applications

Current testing includes:

Firefox
Brave
VLC

Application detection, audio processing, and volume behavior are still under active development.

Current Status
v0.5.0-beta.1

Current functionality includes:

GTK4 interface
PipeWire audio capture
RMS level detection
Peak detection
Smoothed peak following
Dynamic gain adjustment
Fast response to loud peaks
Gradual gain recovery
Silence handling
Per-application PipeWire volume control
Firefox testing
Brave testing
VLC testing
AppImage distribution
GPG-signed release tags
SHA-256 release verification

The application is functional, but the audio-leveling algorithm continues to be tested and refined.

Privacy

OneVolume processes the audio it analyzes locally on the user's Linux system.

The application does not require a cloud service for its audio-leveling function.

OneVolume needs access to the local audio system in order to observe supported media streams and adjust their playback volume. It does not require unrestricted filesystem access for normal operation.

Build From Source
Requirements

You need:

Linux
Rust / Cargo
GTK4 development files
PipeWire development files
pkg-config
Rust

The recommended way to install Rust is through rustup:

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

Then load the Cargo environment:

source "$HOME/.cargo/env"

Verify:

rustc --version
cargo --version
Build Dependencies
openSUSE
sudo zypper install \
  gtk4-devel \
  pipewire-devel \
  pkgconf-pkg-config
Fedora
sudo dnf install \
  gtk4-devel \
  pipewire-devel \
  pkgconf-pkg-config
Debian / Ubuntu / Linux Mint
sudo apt install \
  libgtk-4-dev \
  libpipewire-0.3-dev \
  pkg-config
Arch / Manjaro
sudo pacman -S \
  gtk4 \
  pipewire \
  pkgconf
Clone the Repository
git clone https://github.com/3v1l1/OneVolume.git
cd OneVolume
Build
cargo build --release

The resulting binary is:

target/release/onevolume
Run From Source
cargo run --release
Testing

Run the automated tests:

cargo test --all-targets --all-features

Check formatting:

cargo fmt --check

Run Clippy with warnings treated as errors:

cargo clippy --all-targets --all-features -- -D warnings

For changes affecting audio processing or volume control, testing with real media playback is strongly recommended.

Contributing

Contributions are welcome.

Before submitting changes, please run:

cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features

For audio-related changes, test with real media playback using supported applications.

Security

Please do not disclose security vulnerabilities publicly before giving the project maintainer an opportunity to investigate and fix them.

See SECURITY.md for vulnerability-reporting guidance.

License

OneVolume is licensed under the Apache License 2.0.

See LICENSE for the complete license text.

Project

GitHub:

https://github.com/3v1l1/OneVolume
