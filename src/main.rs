#![windows_subsystem = "windows"]

use native_windows_gui as nwg;
use native_windows_derive::NwgUi;
use nwg::NativeUi;

use std::cell::RefCell;
use std::sync::mpsc;
use std::path::PathBuf;

/// Messages sent from the worker thread back to the UI
#[derive(Debug)]
pub enum WorkerMsg {
    Progress { current: usize, total: usize, file: String },
    Done { moved: usize, skipped: usize, errors: usize },
    Error(String),
}

#[derive(Default, NwgUi)]
pub struct PhotoFixApp {
    // ── Main Window ──────────────────────────────────────────────
    #[nwg_control(
        size: (440, 310),
        position: (300, 200),
        title: "Photo Fix",
        flags: "WINDOW|VISIBLE"
    )]
    #[nwg_events(
        OnWindowClose: [PhotoFixApp::on_exit],
        OnInit: [PhotoFixApp::on_init]
    )]
    window: nwg::Window,

    // ── Source Directory ─────────────────────────────────────────
    #[nwg_control(text: "Source Directory:", size: (410, 18), position: (12, 10))]
    lbl_src: nwg::Label,

    #[nwg_control(text: "", size: (310, 22), position: (12, 30), readonly: true)]
    inp_src: nwg::TextInput,

    #[nwg_control(text: "Browse...", size: (85, 24), position: (332, 29))]
    #[nwg_events(OnButtonClick: [PhotoFixApp::browse_source])]
    btn_src: nwg::Button,

    // ── Destination Directory ────────────────────────────────────
    #[nwg_control(text: "Destination Directory:", size: (410, 18), position: (12, 62))]
    lbl_dst: nwg::Label,

    #[nwg_control(text: "", size: (310, 22), position: (12, 82), readonly: true)]
    inp_dst: nwg::TextInput,

    #[nwg_control(text: "Browse...", size: (85, 24), position: (332, 81))]
    #[nwg_events(OnButtonClick: [PhotoFixApp::browse_dest])]
    btn_dst: nwg::Button,

    // ── Operation selector ───────────────────────────────────────
    #[nwg_control(text: "Operation:", size: (70, 18), position: (12, 118))]
    lbl_op: nwg::Label,

    #[nwg_control(size: (130, 200), position: (90, 115), collection: vec!["Copy Files", "Move Files"])]
    combo_op: nwg::ComboBox<&'static str>,

    // ── Start button ─────────────────────────────────────────────
    #[nwg_control(text: "Start Sorting", size: (130, 28), position: (290, 113))]
    #[nwg_events(OnButtonClick: [PhotoFixApp::on_start])]
    btn_start: nwg::Button,

    // ── Progress bar ─────────────────────────────────────────────
    #[nwg_control(size: (410, 18), position: (12, 152), range: 0..1000)]
    progress: nwg::ProgressBar,

    // ── Status label ─────────────────────────────────────────────
    #[nwg_control(text: "Ready", size: (410, 18), position: (12, 176))]
    lbl_status: nwg::Label,

    // ── Log text box ─────────────────────────────────────────────
    #[nwg_control(
        text: "",
        size: (410, 100),
        position: (12, 200),
        readonly: true,
        flags: "VISIBLE|VSCROLL|AUTOVSCROLL|TAB_STOP"
    )]
    txt_log: nwg::TextBox,

    // ── Timer for polling worker messages ────────────────────────
    #[nwg_control(interval: std::time::Duration::from_millis(50), active: false)]
    #[nwg_events(OnTimerTick: [PhotoFixApp::poll_worker])]
    timer: nwg::AnimationTimer,

    // ── Runtime state (not UI controls) ──────────────────────────
    rx: RefCell<Option<mpsc::Receiver<WorkerMsg>>>,
    is_running: RefCell<bool>,
}

impl PhotoFixApp {
    fn on_init(&self) {
        // Select first item in combo box
        self.combo_op.set_selection(Some(0));
    }

    fn on_exit(&self) {
        nwg::stop_thread_dispatch();
    }

    fn browse_source(&self) {
        let mut dlg = Default::default();
        nwg::FileDialog::builder()
            .title("Select Source Directory")
            .action(nwg::FileDialogAction::OpenDirectory)
            .build(&mut dlg)
            .expect("Failed to build source folder dialog");

        if dlg.run(Some(&self.window)) {
            if let Ok(path) = dlg.get_selected_item() {
                self.inp_src.set_text(&path.to_string_lossy());
            }
        }
    }

    fn browse_dest(&self) {
        let mut dlg = Default::default();
        nwg::FileDialog::builder()
            .title("Select Destination Directory")
            .action(nwg::FileDialogAction::OpenDirectory)
            .build(&mut dlg)
            .expect("Failed to build destination folder dialog");

        if dlg.run(Some(&self.window)) {
            if let Ok(path) = dlg.get_selected_item() {
                self.inp_dst.set_text(&path.to_string_lossy());
            }
        }
    }

    fn log(&self, msg: &str) {
        self.log_batch(&[msg.to_string()]);
    }

    fn log_batch(&self, msgs: &[String]) {
        let current = self.txt_log.text();
        let mut lines: Vec<&str> = current.split("\r\n").filter(|s| !s.is_empty()).collect();
        
        for msg in msgs {
            lines.push(msg);
        }

        // Keep only the last 300 lines
        if lines.len() > 300 {
            let start = lines.len() - 300;
            lines = lines[start..].to_vec();
        }

        let new_text = lines.join("\r\n");
        self.txt_log.set_text(&new_text);

        // Scroll to bottom
        let len = new_text.len() as u32;
        self.txt_log.set_selection(len..len);
    }

    fn on_start(&self) {
        if *self.is_running.borrow() {
            return;
        }

        let src = self.inp_src.text();
        let dst = self.inp_dst.text();

        if src.is_empty() || dst.is_empty() {
            nwg::modal_info_message(
                &self.window,
                "Photo Fix",
                "Please select both source and destination directories.",
            );
            return;
        }

        let src_path = PathBuf::from(&src);
        let dst_path = PathBuf::from(&dst);

        if !src_path.is_dir() {
            nwg::modal_info_message(
                &self.window,
                "Photo Fix",
                "Source directory does not exist.",
            );
            return;
        }

        let use_copy = self.combo_op.selection() == Some(0);

        // Create channel for worker → UI communication
        let (tx, rx) = mpsc::channel::<WorkerMsg>();
        *self.rx.borrow_mut() = Some(rx);
        *self.is_running.borrow_mut() = true;

        // Update UI state
        self.btn_start.set_enabled(false);
        self.progress.set_pos(0);
        self.lbl_status.set_text("Scanning...");
        self.txt_log.set_text("");
        self.log(&format!("Source: {}", src));
        self.log(&format!("Destination: {}", dst));
        self.log(&format!("Operation: {}", if use_copy { "Copy" } else { "Move" }));
        self.log("---");

        // Start the timer to poll worker messages
        self.timer.start();

        // Spawn worker thread
        std::thread::spawn(move || {
            crate::worker::run_sort(src_path, dst_path, use_copy, tx);
        });
    }

    fn poll_worker(&self) {
        let rx_ref = self.rx.borrow();
        let rx = match rx_ref.as_ref() {
            Some(r) => r,
            None => return,
        };

        let mut logs = Vec::new();
        let mut last_progress = None;
        let mut done_msg = None;
        let mut error_msg = None;

        // Drain all pending messages
        while let Ok(msg) = rx.try_recv() {
            match msg {
                WorkerMsg::Progress { current, total, file } => {
                    logs.push(format!("  {}", file));
                    last_progress = Some((current, total));
                }
                WorkerMsg::Done { moved, skipped, errors } => {
                    done_msg = Some((moved, skipped, errors));
                }
                WorkerMsg::Error(e) => {
                    error_msg = Some(e);
                }
            }
        }

        // Apply all accumulated logs in one single batch update
        if !logs.is_empty() {
            self.log_batch(&logs);
        }

        // Update progress bar and label once using the latest values
        if let Some((current, total)) = last_progress {
            if total > 0 {
                let pct = (current as u32 * 1000) / total as u32;
                self.progress.set_pos(pct);
            }
            self.lbl_status.set_text(&format!(
                "Processing {}/{}...",
                current, total
            ));
        }

        // Handle termination messages at the end
        if let Some((moved, skipped, errors)) = done_msg {
            self.progress.set_pos(1000);
            let summary = format!(
                "Done! Processed: {}, Skipped: {}, Errors: {}",
                moved, skipped, errors
            );
            self.lbl_status.set_text(&summary);
            self.log(&format!("---\r\n{}", summary));
            self.timer.stop();
            self.btn_start.set_enabled(true);
            *self.is_running.borrow_mut() = false;
        } else if let Some(e) = error_msg {
            self.lbl_status.set_text("Error!");
            self.log(&format!("ERROR: {}", e));
            self.timer.stop();
            self.btn_start.set_enabled(true);
            *self.is_running.borrow_mut() = false;
        }
    }
}

/// Worker module – performs the actual photo sorting off the UI thread
pub mod worker {
    use super::WorkerMsg;
    use std::path::PathBuf;
    use std::sync::mpsc;

    const IMAGE_EXTENSIONS: &[&str] = &[
        "jpg", "jpeg", "png", "tif", "tiff", "bmp", "gif", "webp",
        "heic", "heif", "cr2", "nef", "arw", "dng", "orf", "rw2",
    ];

    /// Collect all image files from `src` recursively.
    fn collect_images(src: &PathBuf) -> Vec<PathBuf> {
        let mut images = Vec::new();
        let mut dirs = vec![src.clone()];

        while let Some(dir) = dirs.pop() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        dirs.push(path);
                    } else if let Some(ext) = path.extension() {
                        let ext_lower = ext.to_string_lossy().to_lowercase();
                        if IMAGE_EXTENSIONS.contains(&ext_lower.as_str()) {
                            images.push(path);
                        }
                    }
                }
            }
        }
        images
    }

    /// Extract DateTimeOriginal from EXIF, or fall back to file modified time.
    fn get_date(path: &PathBuf) -> Option<(i32, u32)> {
        // Try EXIF first
        if let Ok(file) = std::fs::File::open(path) {
            let mut buf_reader = std::io::BufReader::new(&file);
            if let Ok(exif) = exif::Reader::new().read_from_container(&mut buf_reader) {
                if let Some(field) = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) {
                    if let exif::Value::Ascii(ref vec) = field.value {
                        if let Some(bytes) = vec.first() {
                            let s = String::from_utf8_lossy(bytes);
                            // Format: "YYYY:MM:DD HH:MM:SS"
                            let parts: Vec<&str> = s.split(|c| c == ':' || c == ' ').collect();
                            if parts.len() >= 2 {
                                if let (Ok(year), Ok(month)) =
                                    (parts[0].parse::<i32>(), parts[1].parse::<u32>())
                                {
                                    if year > 1900 && year < 2200 && month >= 1 && month <= 12 {
                                        return Some((year, month));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback: file creation time first on Windows, then modified time
        if let Ok(meta) = std::fs::metadata(path) {
            if let Ok(created) = meta.created() {
                let datetime: chrono::DateTime<chrono::Local> = created.into();
                use chrono::Datelike;
                if datetime.year() > 1900 && datetime.year() < 2200 {
                    return Some((datetime.year(), datetime.month()));
                }
            }
            if let Ok(modified) = meta.modified() {
                let datetime: chrono::DateTime<chrono::Local> = modified.into();
                use chrono::Datelike;
                if datetime.year() > 1900 && datetime.year() < 2200 {
                    return Some((datetime.year(), datetime.month()));
                }
            }
        }

        None
    }

    /// Month number to abbreviated name
    fn month_name(month: u32) -> &'static str {
        match month {
            1 => "01-Jan",
            2 => "02-Feb",
            3 => "03-Mar",
            4 => "04-Apr",
            5 => "05-May",
            6 => "06-Jun",
            7 => "07-Jul",
            8 => "08-Aug",
            9 => "09-Sep",
            10 => "10-Oct",
            11 => "11-Nov",
            12 => "12-Dec",
            _ => "00-Unknown",
        }
    }

    pub fn run_sort(
        src: PathBuf,
        dst: PathBuf,
        use_copy: bool,
        tx: mpsc::Sender<WorkerMsg>,
    ) {
        // 1. Scan for images
        let images = collect_images(&src);
        let total = images.len();

        if total == 0 {
            let _ = tx.send(WorkerMsg::Error("No image files found in source directory.".into()));
            return;
        }

        let mut moved = 0usize;
        let mut skipped = 0usize;
        let mut errors = 0usize;

        // Stage 1: Parallel date extraction
        use rayon::prelude::*;
        let processed: Vec<(PathBuf, Option<(i32, u32)>)> = images
            .par_iter()
            .map(|img_path| {
                let date = get_date(img_path);
                (img_path.clone(), date)
            })
            .collect();

        // Stage 2: Sequential copy/move
        for (i, (img_path, date)) in processed.into_iter().enumerate() {
            let file_name = img_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            // Get year/month
            let (year, month) = match date {
                Some(ym) => ym,
                None => {
                    skipped += 1;
                    let _ = tx.send(WorkerMsg::Progress {
                        current: i + 1,
                        total,
                        file: format!("[skip-no-date] {}", file_name),
                    });
                    continue;
                }
            };

            // Build destination: dst/YYYY/MM-Mon/filename
            let dest_dir = dst
                .join(format!("{}", year))
                .join(month_name(month));

            if let Err(e) = std::fs::create_dir_all(&dest_dir) {
                errors += 1;
                let _ = tx.send(WorkerMsg::Progress {
                    current: i + 1,
                    total,
                    file: format!("[err] {} - {}", file_name, e),
                });
                continue;
            }

            let src_size = std::fs::metadata(&img_path).map(|m| m.len()).unwrap_or(0);
            let mut dest_file = dest_dir.join(&file_name);

            if dest_file.exists() {
                let mut resolved = true;
                // Check if it's a duplicate (size matches)
                let dest_size = std::fs::metadata(&dest_file).map(|m| m.len()).unwrap_or(0);
                if dest_size == src_size {
                    skipped += 1;
                    let _ = tx.send(WorkerMsg::Progress {
                        current: i + 1,
                        total,
                        file: format!("[exists-dup] {}", file_name),
                    });
                    continue;
                }

                // If sizes differ, resolve the name collision using hyphen suffix format (photo-1.jpg)
                let stem = img_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                let ext = img_path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
                
                let mut suffix = 1;
                loop {
                    let new_name = if ext.is_empty() {
                        format!("{}-{}", stem, suffix)
                    } else {
                        format!("{}-{}.{}", stem, suffix, ext)
                    };
                    
                    let candidate = dest_dir.join(&new_name);
                    if candidate.exists() {
                        let candidate_size = std::fs::metadata(&candidate).map(|m| m.len()).unwrap_or(0);
                        if candidate_size == src_size {
                            // Found an existing file that matches the source content exactly, treat as duplicate and skip
                            skipped += 1;
                            let _ = tx.send(WorkerMsg::Progress {
                                current: i + 1,
                                total,
                                file: format!("[exists-dup] {}", new_name),
                            });
                            resolved = false;
                            break;
                        }
                        suffix += 1;
                    } else {
                        dest_file = candidate;
                        resolved = true;
                        break;
                    }
                }
                
                // If resolving determined it was a duplicate and skipped
                if !resolved {
                    continue;
                }
            }

            let result = if use_copy {
                std::fs::copy(&img_path, &dest_file).map(|_| ())
            } else {
                std::fs::rename(&img_path, &dest_file).or_else(|_| {
                    // rename fails across drives, fall back to copy+delete
                    std::fs::copy(&img_path, &dest_file)?;
                    std::fs::remove_file(&img_path)
                })
            };

            match result {
                Ok(()) => {
                    moved += 1;
                    let _ = tx.send(WorkerMsg::Progress {
                        current: i + 1,
                        total,
                        file: format!("{} -> {}/{}", file_name, year, month_name(month)),
                    });
                }
                Err(e) => {
                    errors += 1;
                    let _ = tx.send(WorkerMsg::Progress {
                        current: i + 1,
                        total,
                        file: format!("[err] {} - {}", file_name, e),
                    });
                }
            }
        }

        let _ = tx.send(WorkerMsg::Done { moved, skipped, errors });
    }
}

fn main() {
    nwg::init().expect("Failed to initialize Native Windows GUI");

    // Set the classic Windows system font globally
    let mut font = nwg::Font::default();
    nwg::Font::builder()
        .size(16)
        .family("MS Shell Dlg 2")
        .build(&mut font)
        .expect("Failed to build font");
    nwg::Font::set_global_default(Some(font));

    let _app = PhotoFixApp::build_ui(Default::default())
        .expect("Failed to build Photo Fix UI");

    nwg::dispatch_thread_events();
}
