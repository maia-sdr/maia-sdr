# Changelog

All notable changes to maia-httpd will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2023-04-07

### Added

- Optionally prepend timestamp to recording filename.
- Optional recording maximum duration.

### Changed

- Build against buildroot toolchain instead of cross.
- Spectrometer: skip f32 conversion when no receivers are connected.
- Update to tower-http 0.4.

## [0.1.1] - 2023-03-18

### Fixed

- Do not use chrono default features: this fixes a potential security bug by
  avoiding a dependency on an old version of the time crate.

## [0.1.0] - 2023-02-10

### Added

- Initial release of maia-httpd: supports a waterfall data server in a
  WebSocket, a REST API to control the FPGA core and the AD9361 IIO device, and
  download of IQ recordings in SigMF format.
