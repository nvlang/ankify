# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/nvlang/ankify/releases/tag/v0.1.0) - 2026-05-18

### Added

- honour the document's ankiconnect-url, verbose, cache, and checks
- make SVG cards theme-aware (transparent bg, currentColor)
- make card image scale configurable
- query module

### Fixed

- reject unknown card formats instead of defaulting to PNG
- resolve the Typst --root once for query and compile
- preserve note-ID alignment on partial addNotes failure
- remove underline artifact between README badges
- correct note-field-to-image mapping and harden sync
- remove bad function call in generated typst code
- actually run typst setup function

### Other

- Merge branch 'claude/awesome-goldberg-bd8488'
- declare MSRV, align README CLI options, refresh stale sync docs
- address low-severity audit findings
- add CONTRIBUTING and SECURITY, fix the authors field
- add unit tests for Format parsing, theme_svg, and Sha256
- correct note()/configure() documentation
- cover the field-to-image mapping with an empty field
- trim dead code from the published library API
- reword package descriptions to "generate and sync"
- correct the crate description and drop a dead error variant
- add alpha-stage disclaimers to the READMEs
- set the crate version to 0.1.0
- write a proper crate README
- declare the CLI crate's license as MIT
- apply rustfmt and resolve clippy lints
- remove scratch files and build artifacts
- cover per-field formats, per-note decks, and the no-notes case
- remove unused dependencies
- replace integration suite with real, hermetic coverage
- wip
- wip
- wip
- cleanup
- cache
- compile
- `generate` module
- improve typst plugin
- wip
