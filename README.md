# Photo Fix

A lightweight, high-performance photo sorting utility for Windows. Scans a source folder for images, extracts date information from EXIF metadata, and automatically organizes them into `Year/Month` folders.

Built in Rust with a native Win32 UI — no Electron, no web runtimes, no bloat.

## Features

- **EXIF-Based Sorting** — Reads `DateTimeOriginal`, `DateTimeDigitized`, and `DateTime` tags from image headers
- **Parallel Processing** — Multi-core EXIF parsing via [Rayon](https://github.com/rayon-rs/rayon)
- **Native Win32 UI** — Classic Windows interface using [native-windows-gui](https://github.com/gabdube/native-windows-gui)
- **Two-Phase Workflow** — Scan first to preview planned actions, then sort to execute
- **Copy or Move** — Choose whether to copy or move your files
- **Flexible Structure** — Organize by `Year/Month` or `Year` only
- **Tiny Footprint** — ~457 KB release binary, < 15 MB RAM usage

## Supported Formats

| Category | Extensions |
|----------|-----------|
| Common | JPG, JPEG, PNG, BMP, GIF, WebP |
| High-Efficiency | HEIC, HEIF |
| RAW | CR2, NEF, ARW, DNG, ORF, RW2 |
| TIFF | TIF, TIFF |

## Building

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- Target: `i686-pc-windows-gnu` (32-bit MinGW)

```bash
rustup target add i686-pc-windows-gnu
```

### Compile

```bash
# Debug build
cargo build

# Release build (optimized, ~457 KB)
cargo build --release
```

The release binary will be at `target/release/photo-fix.exe`.

### Run

```bash
cargo run --release
```

## Usage

1. Click **Browse...** to select a **source directory** containing your photos
2. Click **Browse...** to select a **destination directory** for the sorted output
3. Pick an operation: **Copy Files** or **Move Files**
4. Pick a folder structure: **Year/Month** or **Year Only**
5. Click **Scan Folder** to preview the planned actions in the log
6. Review the scan results
7. Click **Start Sorting** to execute

## Project Structure

```
photo-fix/
├── src/
│   └── main.rs          # Application code (UI + worker module)
├── Cargo.toml           # Dependencies and release profile
├── Cargo.lock           # Locked dependency versions
├── README.md
├── LICENSE
└── CONTRIBUTING.md
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
