# Changelog

## 0.1.1 - 2026-06-29

### Fixed

- Fixed Phase 0 database resolution so the MVP production database is preferred
  over an older `CAPSULE_HOME` fallback database.

## 0.1.0 - 2026-06-29

### Added

- Scaffolded the Tauri 2, React, TypeScript, and Vite desktop app.
- Added read-only Capsule database status inspection.
- Added Capsule-compatible backup listing and manual backup creation.
- Added JSON backup manifests and backup verification.
- Added frontend mock runtime for browser-only Vite development.
- Added Rust tests for backup path generation and database status inspection.
