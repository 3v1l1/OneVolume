# OneVolume

**OneVolume is a Linux desktop audio leveler designed to reduce sudden volume changes while watching movies, videos, and other media.**

Built with:

- Rust
- GTK4
- PipeWire
- Flatpak

OneVolume continuously monitors the audio level of active media and dynamically adjusts gain to make loud and quiet sections more comfortable to listen to.

> ⚠️ **Alpha software:** OneVolume is currently an early development release. Bugs, unexpected behavior, and incomplete functionality are expected.

---

## Features

- 🎚️ Dynamic media volume leveling
- 🔊 Fast response to sudden loud peaks
- 📉 Gradual recovery after loud sections
- 🎧 PipeWire audio capture
- 🧠 Smoothed peak detection using a peak follower
- 🌐 Firefox and Brave media detection
- 🎬 Designed for movies, trailers, videos, and other media
- 🖥️ GTK4 desktop interface
- 📦 Flatpak packaging
- 🔐 GPG-signed Flatpak release
- 🔑 Public GPG signing key
- #️⃣ SHA-256 release checksum

---

## How It Works

OneVolume monitors the audio level of active media and calculates an appropriate gain adjustment.

The leveler uses two complementary measurements:

- **RMS level** — represents the general loudness of the audio.
- **Peak level** — detects sudden loud sounds and transients.

Peak detection uses a **peak follower**:

- Peaks rise immediately.
- Peaks decay gradually.
- A single quiet audio buffer cannot immediately make the detector release.
- Sudden loud sounds can still trigger fast protection.

The goal is to reduce large volume jumps without aggressively changing normal dialogue or quiet sections.

---

## Installation

## Recommended: Flatpak

The easiest way to use OneVolume is to install the pre-built Flatpak from the GitHub Releases page:

https://github.com/3v1l1/OneVolume/releases

### Requirements

You need:

- Linux
- Flatpak
- PipeWire audio

You **do not need** Rust, Cargo, GTK development packages, PipeWire development packages, `pkg-config`, or Flatpak Builder to run the released Flatpak.

### Install Flatpak

#### openSUSE

```bash
sudo zypper install flatpak
```

#### Fedora

```bash
sudo dnf install flatpak
```

#### Debian / Ubuntu / Linux Mint

```bash
sudo apt install flatpak
```

#### Arch / Manjaro

```bash
sudo pacman -S flatpak
```

## Install OneVolume

Download the latest `.flatpak` file from the GitHub Releases page.

Then install it:

```bash
flatpak install --user ./OneVolume-*.flatpak
```

Run:

```bash
flatpak run com.onevolume.OneVolume
```

---

## Verify the Download

OneVolume releases provide both a SHA-256 checksum and a public GPG signing key.

### SHA-256

Download the `.flatpak` file and its corresponding `.sha256` file.

Then run:

```bash
sha256sum -c OneVolume-<version>.flatpak.sha256
```

A successful verification looks like:

```text
OneVolume-<version>.flatpak: OK
```

The checksum confirms that the downloaded Flatpak matches the exact file published by the release.

### GPG Verification

The release also includes the public OneVolume signing key.

Import the public key:

```bash
gpg --import onevolume-signing-key.asc
```

The release Flatpak is built from a GPG-signed Flatpak repository and distributed with the public signing key.

The public signing key can be obtained from the corresponding GitHub release.

Never publish or share your private GPG key. Only the public key should be distributed.

## Build From Source

Building OneVolume from source requires additional development packages.

### Required Build Dependencies

You need:

- Rust / Cargo
- GTK4 development files
- PipeWire development files
- `pkg-config`
- Flatpak
- Flatpak Builder

The GTK4 and PipeWire development packages are required for building, not for running the pre-built Flatpak.

### Rust

The recommended way to install Rust is through `rustup`:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation, restart your terminal or load the Cargo environment:

```bash
source "$HOME/.cargo/env"
```

Verify:

```bash
rustc --version
cargo --version
```

### Build Dependencies by Distribution

#### openSUSE

```bash
sudo zypper install \
  flatpak \
  flatpak-builder \
  gtk4-devel \
  pipewire-devel \
  pkgconf-pkg-config
```

Install Rust separately using `rustup` as described above.

#### Fedora

```bash
sudo dnf install \
  flatpak \
  flatpak-builder \
  gtk4-devel \
  pipewire-devel \
  pkgconf-pkg-config
```

Install Rust separately using `rustup` as described above.

#### Debian / Ubuntu / Linux Mint

```bash
sudo apt install \
  flatpak \
  flatpak-builder \
  libgtk-4-dev \
  libpipewire-0.3-dev \
  pkg-config
```

Install Rust separately using `rustup` as described above.

#### Arch / Manjaro

```bash
sudo pacman -S \
  flatpak \
  flatpak-builder \
  gtk4 \
  pipewire \
  pkgconf
```

Install Rust separately using `rustup` as described above.

### Clone the Repository

```bash
git clone https://github.com/3v1l1/OneVolume.git
cd OneVolume
```

### Build the Application

Build the Rust application:

```bash
cargo build --release
```

The resulting binary will be:

```text
target/release/onevolume
```

### Run From Source

You can run the application directly after building:

```bash
cargo run --release
```

### Build the Flatpak

Build the Flatpak using the included manifest:

```bash
flatpak-builder \
  --user \
  --install \
  --force-clean \
  build-dir \
  packaging/com.onevolume.OneVolume.yml
```

Then run:

```bash
flatpak run com.onevolume.OneVolume
```

### Flatpak Permissions

OneVolume runs inside a Flatpak sandbox.

The current application manifest grants access to:

- Wayland
- X11 fallback
- PulseAudio compatibility
- Native PipeWire socket
- Freedesktop desktop portals

These permissions are required for the application's graphical interface and audio functionality.

OneVolume does not require unrestricted filesystem access.

---

## Supported Media Detection

Current testing includes:

- Firefox
- Brave

Media detection and audio behavior are still under active development.

---

## Current Status

### Alpha

OneVolume is currently in the **0.1.0-alpha** development stage.

Current functionality includes:

- GTK4 interface
- PipeWire audio capture
- RMS level detection
- Peak detection
- Smoothed peak following
- Dynamic gain adjustment
- Fast response to loud peaks
- Gradual gain recovery
- Silence handling
- Firefox media testing
- Brave media testing
- Flatpak packaging
- GPG-signed releases
- SHA-256 release verification

The application is functional, but the audio-leveling algorithm is still being tuned and tested.

---

## Testing

The project includes automated Rust tests for the leveler and peak detection logic.

Run:

```bash
cargo test
```

Format the source:

```bash
cargo fmt
```

Check formatting:

```bash
cargo fmt --check
```

Check compilation:

```bash
cargo check
```

For changes affecting audio processing, testing with real media playback is strongly recommended.

---

## Release Verification

Released Flatpak builds are accompanied by:

- SHA-256 checksum
- Public GPG signing key
- GPG-signed Git tags
- GPG-signed development commits

GitHub Releases:

https://github.com/3v1l1/OneVolume/releases

---

## Contributing

OneVolume is open source and contributions are welcome.

Before submitting changes, please run:

```bash
cargo fmt
cargo check
cargo test
```

For changes affecting audio processing, testing with real media playback is strongly recommended.

---

## License

OneVolume is licensed under the **Apache License 2.0**.

See [`LICENSE`](LICENSE) for the complete license text.

---

## Project

GitHub:

https://github.com/3v1l1/OneVolume

OneVolume is a Linux-focused project built around Rust, GTK4, PipeWire, and Flatpak.
