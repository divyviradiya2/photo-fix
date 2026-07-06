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

Technology stack not yet documented. Will populate after codebase mapping or first phase.
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
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
