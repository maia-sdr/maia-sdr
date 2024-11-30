# Changelog

All notable changes to maia-httpd will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.5.3 - 2024-11-30

### Added

- Geolocation to REST API
- HTTPS server

### Changed

- Strip all symbols in release binary
- Disable some unused tokio features

## 0.5.2 - 2024-10-12

### Changed

- Moved ichige function to pm-remez

## 0.5.1 - 2024-09-07

### Changed

- Updated dependencies

## 0.5.0 - 2024-05-05

### Added

- DDC
- Recorder stopping state
- Version information page

### Changed

- Refactor application state

## 0.4.0 - 2024-02-23

### Added

- IQEngine backend API methods and IQEngine client serving.

### Changed

- Handle IQ recording mmap() as a global object instead of per request.

## 0.3.1 - 2023-11-19

### Fixed

- SigMF version format.

### Changed

- Updated dependencies.
- Optimized code generation.

## 0.3.0 - 2023-09-29

### Added

- Spectrometer peak detect mode.

## 0.2.3 - 2023-09-03

### Fixed

- Formatting of SigMF core:datetime field.

### Changed

- Updated dependencies.

## 0.2.2 - 2023-06-10

### Changed

- Panic in uClibc with rustc >= 1.69.0.

## 0.2.1 - 2023-06-10

### Changed

- Updated dependencies.

## 0.2.0 - 2023-04-07

### Added

- Optionally prepend timestamp to recording filename.
- Optional recording maximum duration.

### Changed

- Build against buildroot toolchain instead of cross.
- Spectrometer: skip f32 conversion when no receivers are connected.
- Update to tower-http 0.4.

## 0.1.1 - 2023-03-18

### Fixed

- Do not use chrono default features: this fixes a potential security bug by
  avoiding a dependency on an old version of the time crate.

## 0.1.0 - 2023-02-10

### Added

- Initial release of maia-httpd: supports a waterfall data server in a
  WebSocket, a REST API to control the FPGA core and the AD9361 IIO device, and
  download of IQ recordings in SigMF format.
