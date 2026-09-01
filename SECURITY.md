# Security Policy

## Reporting a Vulnerability

Please do not publicly disclose a suspected security vulnerability before it has been investigated.

For security issues, please use GitHub's Private Vulnerability Reporting feature:

https://github.com/3v1l1/OneVolume/security/advisories/new

Please do not open a public GitHub issue for an undisclosed security vulnerability.

When reporting an issue, please include:

- A clear description of the vulnerability
- Steps to reproduce it
- The affected OneVolume version or commit
- Any relevant logs, screenshots, or proof-of-concept details

Please avoid including secrets, private keys, passwords, personal data, or other sensitive information in reports.

## Supported Versions

Security fixes will be prioritized for actively maintained releases and the current development version.

Beta releases may contain known bugs and incomplete security hardening.

## Scope

OneVolume is a Linux desktop application that interacts with the local PipeWire audio system to observe supported media streams and adjust playback volume.

Because OneVolume is distributed as an AppImage, it does not provide the same sandbox isolation as a Flatpak application. Users should only run OneVolume builds obtained from a trusted source and should verify release checksums where provided.

## Release Integrity

Official releases may include:

- SHA-256 checksums for release artifacts
- GPG-signed Git release tags

Users should verify these integrity and authenticity mechanisms where possible before running a release.
