# Contributing to Photo Fix

Thank you for your interest in contributing! This guide will help you get started.

## Getting Started

1. Fork the repository and clone your fork
2. Install the Rust toolchain via [rustup](https://rustup.rs/)
3. Add the build target:
   ```bash
   rustup target add i686-pc-windows-gnu
   ```
4. Build the project:
   ```bash
   cargo build
   ```
5. Run tests:
   ```bash
   cargo test
   ```

## Architecture Overview

Photo Fix follows a simple two-thread architecture:

- **UI thread** — Handles window events and polls for worker messages via a 50ms `AnimationTimer`
- **Worker thread** — Performs all file I/O (scanning, EXIF reading, copying/moving) and sends progress back through an `mpsc` channel

Key design rules:
- **No blocking I/O on the UI thread** — all file operations happen on the worker thread
- The UI struct (`PhotoFixApp`) uses `RefCell` for interior mutability of runtime state
- NWG derive macros (`#[derive(NwgUi)]`) handle declarative UI control layout

### Code Layout

The entire application lives in `src/main.rs`:

| Section | Description |
|---------|------------|
| Top-level enums/structs | `ScanStatus`, `ScanResult`, `WorkerMsg`, `AppButtonState` |
| `PhotoFixApp` struct | UI controls and runtime state |
| `impl PhotoFixApp` | UI event handlers and polling logic |
| `mod worker` | Background scan and sort logic |
| `fn main()` | App initialization |

## Making Changes

### Pull Request Process

1. Create a feature branch from `master`
2. Make your changes
3. Ensure `cargo check` passes with no errors
4. Ensure `cargo test` passes
5. Submit a pull request with a clear description of what changed and why

### Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and address any warnings
- Use descriptive variable and function names
- Add comments for non-obvious logic
- Preserve existing comments unless they're directly affected by your change

### What Makes a Good Contribution

- Bug fixes with clear reproduction steps
- Performance improvements with before/after measurements
- Support for additional image formats
- Unit tests for existing logic (especially date parsing)
- Documentation improvements

## Reporting Issues

When filing a bug report, please include:
- Steps to reproduce
- Expected vs actual behavior
- Your OS version and Rust toolchain version (`rustup show`)
- The types of image files involved (if relevant)
