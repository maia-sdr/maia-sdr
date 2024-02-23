# Changelog

All notable changes to maia-hdl will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.4.0 - 2023-09-23

### Added

- Peak detect in spectrum integrator.

## 0.3.0 - 2023-07-08

### Changed

- Register output of ClkNxCommonEdge to improve timing (API-breaking change).

## 0.2.0 - 2023-07-08

### Added

- Vivado project for Pluto+.

## 0.1.2 - 2023-06-10

### Fixed

- Warning in spectrum integrator counter reset.

## 0.1.1 - 2023-03-26

### Changed

- Updated adi-hdl submodule to branch used in ADI Pluto firmware v0.36, which is uses
  Vivado 2021.2.

## 0.1.0 - 2023-02-10

### Added

- Initial release of maia-hdl: includes an FFT implementation, a spectrometer,
  an IQ recorder to RAM, and a Vivado project for the ADALM Pluto.
