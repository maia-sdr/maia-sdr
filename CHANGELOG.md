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

### Changed

- Bumped maia-httpd to v0.1.1. Only a minor security bugfix.
- Bumped maia-wasm to v0.2.0. API breaking change because of a refactor to improve
  reusability. Includes other minor changes.

## [0.1.0] - 2023-02-10

### Added

- Initial release of maia-sdr: support for the ADALM Pluto including a web
  interface with real-time waterfall display and IQ recording in SigMF format to
  the Pluto RAM.
