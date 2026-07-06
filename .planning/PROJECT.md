# Photo Fix

## What This Is

A high-performance Windows XP-style photo sorter desktop utility built in Rust. It scans a source folder for images, extracts DateTimeOriginal from EXIF headers or falls back to filesystem timestamps, and automatically organizes them into folders sorted by Year/Month.

## Core Value

To achieve raw, hardware-saturating photo sorting speeds with an extremely lightweight footprint (< 2MB binary, < 15MB RAM) and an authentic Windows XP-style UI.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] **Scan & Ingestion**: Scan source folder for image files (JPEG, TIFF, etc.) eagerly into memory on a background thread.
- [ ] **Metadata Extraction**: Extract `DateTimeOriginal` EXIF tag from image headers without decoding the full pixel array.
- [ ] **Fallback Ingestion**: Query OS filesystem creation/modification timestamps if EXIF metadata is missing.
- [ ] **XP-Style UI**: Build a lightweight, beveled Windows XP/Classic-style GUI using FLTK.
- [ ] **Non-Blocking UI**: UI thread remains at 60 FPS by using non-blocking channels (`std::sync::mpsc`) to communicate with the background worker.
- [ ] **High-Speed Execution**: Maximize multi-core CPU usage via Rayon for parallel reading, parsing, and writing.
- [ ] **Collision Handling**: Resolve filename collisions in the destination directory by auto-renaming (e.g., appending `_1`, `_2`).
- [ ] **Footprint Optimizations**: Apply Cargo optimization profiles to achieve < 2MB binary and < 15MB idle RAM.

### Out of Scope

- **Web UI Runtimes**: Tauri, Electron, Slint, or web-view solutions — Excluded to meet binary size (< 2MB) and RAM (< 15MB) limits.
- **Image Editing/Manipulation**: Modifying pixel arrays or writing EXIF data back — Excluded to keep scope focused on organization.
- **Full Image Decoding**: Loading entire image files into memory — Excluded to maintain blazing fast speed and low RAM footprint.

## Context

- Target OS is Windows, requiring clean Win32-style window widgets and native directory dialogs.
- Low-overhead crates are utilized: `fltk` for native-like rendering, `rayon` for CPU-bound data parallelism, `exif` for zero-allocation header reading, and `chrono` for date/time conversion.

## Constraints

- **Binary Size**: Production binary must compile to < 2MB (requires optimization flags, panic=abort, strip=true, and LTO).
- **RAM Footprint**: Idle memory consumption must be < 15MB RAM.
- **UI Thread Safety**: Do not perform block/disk I/O on the UI main thread.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Auto-rename collisions | Numeric suffix renaming (e.g., `_1`, `_2`) prevents data loss without prompting the user. | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-07-06 after initialization*
