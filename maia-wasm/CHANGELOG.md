# Changelog

All notable changes to maia-wasm will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2023-03-18

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

## [0.1.0] - 2023-02-10

### Added

- Initial release of maia-wasm: WebGL2 waterfall obtaining data from a
  WebSocket and HTML form-based UI to interact with the REST API of
  maia-httpd.
