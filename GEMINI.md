<!-- GSD:project-start source:PROJECT.md -->
## Project

**Photo Fix**

A high-performance Windows XP-style photo sorter desktop utility built in Rust. It scans a source folder for images, extracts DateTimeOriginal from EXIF headers or falls back to filesystem timestamps, and automatically organizes them into folders sorted by Year/Month.

**Core Value:** To achieve raw, hardware-saturating photo sorting speeds with an extremely lightweight footprint (< 2MB binary, < 15MB RAM) and an authentic Windows XP-style UI.

### Constraints

- **Binary Size**: Production binary must compile to < 2MB (requires optimization flags, panic=abort, strip=true, and LTO).
- **RAM Footprint**: Idle memory consumption must be < 15MB RAM.
- **UI Thread Safety**: Do not perform block/disk I/O on the UI main thread.
<!-- GSD:project-end -->

<!-- GSD:stack-start source:STACK.md -->
## Technology Stack

- **Language:** Rust (stable-i686-pc-windows-gnu, rustc 1.96.1)
- **GUI:** native-windows-gui 1.0.12 + native-windows-derive 1.0.3 (pure Win32 API via winapi crate)
- **EXIF:** kamadak-exif 0.6
- **Date/Time:** chrono 0.4 (clock + std features only)
- **Parallelism:** rayon 1.10 (available, not yet used in worker)
- **Toolchain:** i686-pc-windows-gnu (32-bit MinGW target)
- **Release binary:** ~457 KB with opt-level="z", LTO, strip, panic=abort
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

- Single `src/main.rs` file with inline `worker` module for sorting logic.
- `WorkerMsg` enum for typed worker→UI communication.
- NWG derive macros (`#[derive(NwgUi)]`) for declarative UI control layout.
- `RefCell` for interior mutability of runtime state in the UI struct.
- All I/O happens on background threads; UI thread only polls via `AnimationTimer`.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

```
main() → nwg::init() → PhotoFixApp::build_ui() → dispatch_thread_events()
                              │
                    ┌─────────┴─────────┐
                    │   PhotoFixApp      │
                    │  (NwgUi struct)    │
                    │                   │
                    │  on_start() ──────┼──→ std::thread::spawn(worker::run_sort)
                    │                   │         │
                    │  poll_worker() ←──┼─────────┘ (mpsc channel)
                    │  (AnimationTimer) │
                    └───────────────────┘
```

- **UI thread**: Handles window events, polls `mpsc::Receiver<WorkerMsg>` via 50ms timer.
- **Worker thread**: Scans directories, reads EXIF, copies/moves files, sends progress via `mpsc::Sender`.
- **No blocking I/O on UI thread** — all file operations run in the worker.
<!-- GSD:architecture-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
