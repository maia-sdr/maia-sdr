# Changelog

All notable changes to maia-sdr will be documented in this file. Each of the
components of maia-sdr (maia-hdl, maia-httpd, maia-wasm) has its own more
detailed changelog inside the component directory.

maia-sdr is versioned in synchronization with all its components. A new version
tag for maia-sdr bumps the version of each of the components that had
changes. The versioning of each component evolves independently, according to
whether there are no changes, only non-API-breaking changes, or API-breaking
changes since the last version tag for maia-sdr. The semantic version of
maia-sdr is bumped according to whether any components had API-breaking changes
(this causes an API-breaking bump in the maia-sdr version number).

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.10.0] - 2024-11-30

- Bumped maia-hdl to v0.6.1. Added configuration and changed to Vivado 2023.2.
- Bumped maia-httpd to v0.5.3. Added geolocation and HTTPS.
- Bumped maia-json to v0.5.0. Added geolocation.
- Bumped maia-wasm to v0.6.1. Added geolocation and HTTPS.

## [0.9.0] - 2024-10-12

- Bumped maia-httpd to v0.5.2. Move ichige to pm-remez.
- Bumped maia-wasm to v0.6.0. Added spectrum to waterfall.

## [0.8.1] - 2024-09-07

- Bumped maia-hdl to v0.6.0. Ported to Amaranth v0.5.2.
- Bumped maia-httpd to v0.5.1. Updated dependencies.
- Bumped maia-pac to v0.5.0. Regenerated with new svd2rust.
- Bumped maia-wasm to v0.5.1. Update dependencies.

## [0.8.0] - 2024-05-05

- Bumped maia-hdl to v0.5.0. Added DDC.
- Bumped maia-httpd to v0.5.0. Added DDC and refactor application state.
- Bumped maia-json to v0.4.0. Added DDC and error responses.
- Bumped maia-pac to v0.4.0. Updated to maia-hdl v0.5.0 SVD.
- Bumped maia-wasm to v0.5.0. Added DDC and settings dialog.

## [0.7.0] - 2024-02-23

- Bumped maia-httpd to v0.4.0. Added IQEngine.
- Bumped maia-httpd to v0.4.2. Added IQEngine.

## [0.6.1] - 2023-11-19

- Bumped maia-httpd to v0.3.1. Updated dependencies.
- Bumped maia-wasm to v0.4.1. Fixed waterfall crash.
- Changed adi-hdl to Vivado 2022.2 branch.

## [0.6.0] - 2023-09-29

- Bumped maia-hdl to v0.4.0. Added spectrometer peak detect.
- Bumped maia-httpd to v0.3.0. Added spectrometer mode control.
- Bumped maia-json to v0.3.0. Added spectrometer mode field.
- Bumped maia-pac to v0.2.0. Update with SVD from maia-hdl v0.4.0.
- Bumped maia-wasm to v0.4.0. Added spectrometer mode control.

## [0.5.1] - 2023-09-03

- Bumped maia-httpd to v0.2.3. Fixed SigMF formatting and updated dependencies.
- Bumped maia-wasm to v0.3.2. Updated dependencies.

## [0.5.0] - 2023-07-08

- Bumped maia-hdl to v0.3.0. Timing improvement.

## [0.4.0] - 2023-07-08

- Bumped maia-hdl to v0.2.0. Added support for Pluto+.

## [0.3.2] - 2023-06-10

- Bumped maia-httpd to v0.2.2. Fixed a panic in uClibc.

## [0.3.1] - 2023-06-10

- Bumped maia-hdl to v0.1.2. Fixed a warning.
- Bumped maia-httpd to v0.2.1. Updated dependencies.
- Bumped maia-wasm to v0.3.1. Updated dependencies.

## [0.3.0] - 2023-04-08

### Changed

- Bumped maia-httpd to v0.2.0. New recording features.
- Bumped maia-wasm to v0.3.0. New recording features and waterfall colormap.

## [0.2.1] - 2023-03-26

### Changed

- Bumped maia-hdl to v0.1.1. Updated adi-hdl submodule to Vivado 2021.2 branch.

## [0.2.0] - 2023-03-18

### Changed

- Bumped maia-httpd to v0.1.1. Only a minor security bugfix.
- Bumped maia-wasm to v0.2.0. API breaking change because of a refactor to improve
  reusability. Includes other minor changes.

## [0.1.0] - 2023-02-10

### Added

- Initial release of maia-sdr: support for the ADALM Pluto including a web
  interface with real-time waterfall display and IQ recording in SigMF format to
  the Pluto RAM.

[unreleased]: https://github.com/maia-sdr/maia-sdr/compare/v0.10.0...HEAD
[0.9.0]: https://github.com/maia-sdr/maia-sdr/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/maia-sdr/maia-sdr/compare/v0.8.1...v0.9.0
[0.8.1]: https://github.com/maia-sdr/maia-sdr/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/maia-sdr/maia-sdr/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/maia-sdr/maia-sdr/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/maia-sdr/maia-sdr/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/maia-sdr/maia-sdr/compare/v0.5.1...v0.6.0
[0.5.1]: https://github.com/maia-sdr/maia-sdr/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/maia-sdr/maia-sdr/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/maia-sdr/maia-sdr/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/maia-sdr/maia-sdr/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/maia-sdr/maia-sdr/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/maia-sdr/maia-sdr/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/maia-sdr/maia-sdr/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/maia-sdr/maia-sdr/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/maia-sdr/maia-sdr/releases/tag/v0.1.0
