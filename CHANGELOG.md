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

[unreleased]: https://github.com/maia-sdr/maia-sdr/compare/v0.3.2...HEAD
[0.3.2]: https://github.com/maia-sdr/maia-sdr/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/maia-sdr/maia-sdr/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/maia-sdr/maia-sdr/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/maia-sdr/maia-sdr/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/maia-sdr/maia-sdr/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/maia-sdr/maia-sdr/releases/tag/v0.1.0
