# Changelog

All notable changes to maia-wasm will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.6.1 - 2024-11-30

### Added

- HTTPS support
- Geolocation
- Other tab in settings dialog

### Changed

- Responsive layout tweaks for narrow screens

### Fixed

- Typo in manifest.json icon size

## 0.6.0 - 2024-10-12

### Added

- Added spectrum display to waterfall

### Changed

- Added `enabled` property to `RenderObject`
- Exposed many private functions and macros of UI as public

## 0.5.1 - 2024-09-07

### Changed

- Updated dependencies

## 0.5.0 - 2024-05-05

### Added

- DDC
- Settings dialog
- Recorder stopping state
- Version information

## 0.4.2 - 2024-02-23

### Added

- Link to view IQ recording in IQEngine.

## 0.4.1 - 2023-11-19

### Fixed

- Waterfall crash in high screen resolutions.

### Changed

- Updated dependencies.
- Optimized code generation.

## 0.4.0 - 2023-09-29

### Added

- Spectrometer mode control.

## 0.3.2 - 2023-09-03

### Changed

- Updated dependencies.

## 0.3.1 - 2023-06-10

### Changed

- Updated dependencies.

## 0.3.0 - 2023-04-07

### Added

- Inferno waterfall colormap.
- Optionally prepend timestamp to recording filename.
- Optional recording maximum duration.

### Changed

- Disable RX gain input field in AGC modes.

## 0.2.0 - 2023-03-18

### Added

- Standalone waterfall example.

### Fixed

- JS promise failure in the UI code when a PATCH request fails.
- Avoid PATCHing RX gain in non-manual AGC mode.
- Calculation of center coordinates in pointer interaction with waterfall.

### Changed

- Refactor to improve waterfall reusability.
- Waterfall size: the waterfall will grow vertically to occupy the empty
  viewport space.

## 0.1.0 - 2023-02-10

### Added

- Initial release of maia-wasm: WebGL2 waterfall obtaining data from a
  WebSocket and HTML form-based UI to interact with the REST API of
  maia-httpd.
