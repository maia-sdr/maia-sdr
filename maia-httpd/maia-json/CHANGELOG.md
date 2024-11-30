# Changelog

All notable changes to maia-json will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.5.0 - 2024-11-30

### Added

- Geolocation in recording metadata
- Device geolocation

## 0.4.0 - 2024-05-05

### Added

- DDC
- Spectrometer input
- Recorder 16-bit mode
- Recorder stopping state
- Error responses

## 0.3.0 - 2023-09-29

### Added

- Field in Spectrometer for mode (average or peak detect).

## 0.2.0 - 2023-04-07

### Added

- Field in Recorder for timestamp prepending.
- Field in Recorder for maximum duration.

## 0.1.1 - 2023-03-18

### Fixed

- Omit null fields in JSON PATCH requests.

## 0.1.0 - 2023-02-10

### Added

- Initial release of maia-json: supports the spectrometer, ad9361, recording,
  and time.
