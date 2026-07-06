#![windows_subsystem = "windows"]

use native_windows_gui as nwg;
use native_windows_derive::NwgUi;
use nwg::NativeUi;

use std::cell::RefCell;
use std::sync::mpsc;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

static IS_RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug)]
pub enum ScanStatus {
    PendingCopy { year: i32, month: u32, dest_path: PathBuf },
    PendingMove { year: i32, month: u32, dest_path: PathBuf },
    NoDateSkipped,
}

#[derive(Clone, Debug)]
pub struct ScanResult {
    pub src: PathBuf,
    pub status: ScanStatus,
}

/// Messages sent from the worker thread back to the UI
#[derive(Debug)]
pub enum WorkerMsg {
    ScanProgress { current: usize, total: usize, file: String },
    ScanDone(Vec<ScanResult>),
    SortProgress { current: usize, total: usize, file: String },
    SortDone { moved: usize, skipped: usize, errors: usize },
    Error(String),
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub enum AppButtonState {
    #[default]
    Scan,
    Sort,
}

#[derive(Default, NwgUi)]
pub struct PhotoFixApp {
    // ── Embedded Resources ──────────────────────────────────────
    #[nwg_resource]
    embed: nwg::EmbedResource,

    #[nwg_resource(source_embed: Some(&data.embed), source_embed_id: 1)]
    app_icon: nwg::Icon,

    // ── Main Window ──────────────────────────────────────────────
    #[nwg_control(
        size: (538, 324),
        position: (300, 200),
        title: "Photo Fix",
        flags: "WINDOW|VISIBLE",
        icon: Some(&data.app_icon)
    )]
    #[nwg_events(
        OnInit: [PhotoFixApp::on_init]
    )]
    window: nwg::Window,

    // ── Source Directory ─────────────────────────────────────────
    #[nwg_control(text: "Source Directory:", size: (514, 18), position: (12, 10))]
    lbl_src: nwg::Label,

    #[nwg_control(text: "", size: (419, 22), position: (12, 30), readonly: true)]
    inp_src: nwg::TextInput,

    #[nwg_control(text: "Browse...", size: (85, 24), position: (441, 29))]
    #[nwg_events(OnButtonClick: [PhotoFixApp::browse_source])]
    btn_src: nwg::Button,

    // ── Destination Directory ────────────────────────────────────
    #[nwg_control(text: "Destination Directory:", size: (514, 18), position: (12, 62))]
    lbl_dst: nwg::Label,

    #[nwg_control(text: "", size: (419, 22), position: (12, 82), readonly: true)]
    inp_dst: nwg::TextInput,

    #[nwg_control(text: "Browse...", size: (85, 24), position: (441, 81))]
    #[nwg_events(OnButtonClick: [PhotoFixApp::browse_dest])]
    btn_dst: nwg::Button,

    // ── Operation selector ───────────────────────────────────────
    #[nwg_control(text: "Operation:", size: (65, 18), position: (12, 118))]
    lbl_op: nwg::Label,

    #[nwg_control(size: (95, 200), position: (82, 115), collection: vec!["Copy Files", "Move Files"])]
    combo_op: nwg::ComboBox<&'static str>,

    // ── Structure selector ───────────────────────────────────────
    #[nwg_control(text: "Structure:", size: (65, 18), position: (192, 118))]
    lbl_structure: nwg::Label,

    #[nwg_control(size: (95, 200), position: (262, 115), collection: vec!["Year/Month", "Year Only"])]
    combo_structure: nwg::ComboBox<&'static str>,

    // ── Action button ────────────────────────────────────────────
    #[nwg_control(text: "Scan Folder", size: (154, 28), position: (372, 113))]
    #[nwg_events(
        OnButtonClick: [PhotoFixApp::on_action_click],
        OnButtonDoubleClick: [PhotoFixApp::on_action_click]
    )]
    btn_action: nwg::Button,

    // ── Progress bar ─────────────────────────────────────────────
    #[nwg_control(size: (514, 18), position: (12, 152), range: 0..1000)]
    progress: nwg::ProgressBar,

    // ── Status label ─────────────────────────────────────────────
    #[nwg_control(text: "Ready", size: (410, 18), position: (12, 184))]
    lbl_status: nwg::Label,

    // ── Log Toggle Button ────────────────────────────────────────
    #[nwg_control(text: "Expand Log", size: (94, 24), position: (432, 180))]
    #[nwg_events(
        OnButtonClick: [PhotoFixApp::toggle_log_view],
        OnButtonDoubleClick: [PhotoFixApp::toggle_log_view]
    )]
    btn_toggle_log: nwg::Button,

    // ── Log text box ─────────────────────────────────────────────
    #[nwg_control(
        text: "",
        size: (514, 98),
        position: (12, 214),
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
    scan_results: RefCell<Vec<ScanResult>>,
    btn_state: RefCell<AppButtonState>,
    log_lines: RefCell<Vec<String>>,
    log_expanded: RefCell<bool>,
    raw_handler: RefCell<Option<nwg::RawEventHandler>>,
}

impl PhotoFixApp {
    fn on_init(&self) {
        // Select first item in combo boxes
        self.combo_op.set_selection(Some(0));
        self.combo_structure.set_selection(Some(0));
        *self.btn_state.borrow_mut() = AppButtonState::Scan;
        self.btn_action.set_text("Scan Folder");
        *self.log_expanded.borrow_mut() = false;

        // Bind raw event handler to window to intercept WM_CLOSE (0x0010)
        let handler = nwg::bind_raw_event_handler(&self.window.handle, 0x10001, |_, msg, _, _| {
            if msg == 0x0010 { // WM_CLOSE
                if IS_RUNNING.load(Ordering::SeqCst) {
                    let params = nwg::MessageParams {
                        title: "Photo Fix",
                        content: "An operation is currently in progress. Exiting now may leave partially processed files or corrupt destination images.\n\nAre you sure you want to exit?",
                        buttons: nwg::MessageButtons::YesNo,
                        icons: nwg::MessageIcons::Warning,
                    };
                    if nwg::message(&params) == nwg::MessageChoice::No {
                        return Some(0); // Intercept and cancel closing
                    }
                }
                nwg::stop_thread_dispatch();
                return None; // Proceed to destroy window
            }
            None
        }).expect("Failed to bind raw event handler");

        *self.raw_handler.borrow_mut() = Some(handler);
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
                *self.btn_state.borrow_mut() = AppButtonState::Scan;
                self.btn_action.set_text("Scan Folder");
                self.btn_action.set_enabled(true);
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
                *self.btn_state.borrow_mut() = AppButtonState::Scan;
                self.btn_action.set_text("Scan Folder");
                self.btn_action.set_enabled(true);
            }
        }
    }

    fn log(&self, msg: &str) {
        self.log_batch(&[msg.to_string()]);
    }

    fn log_batch(&self, msgs: &[String]) {
        let mut lines = self.log_lines.borrow_mut();
        for msg in msgs {
            lines.push(msg.clone());
        }

        // Keep only the last 300 lines
        if lines.len() > 300 {
            let start = lines.len() - 300;
            *lines = lines[start..].to_vec();
        }

        let new_text = lines.join("\r\n");
        self.txt_log.set_text(&new_text);

        // Scroll to bottom
        let len = new_text.len() as u32;
        self.txt_log.set_selection(len..len);
    }

    fn toggle_log_view(&self) {
        let expanded = *self.log_expanded.borrow();
        if expanded {
            self.window.set_size(538, 324);
            self.txt_log.set_size(514, 98);
            self.btn_toggle_log.set_text("Expand Log");
            *self.log_expanded.borrow_mut() = false;
        } else {
            self.window.set_size(538, 564);
            self.txt_log.set_size(514, 338);
            self.btn_toggle_log.set_text("Collapse Log");
            *self.log_expanded.borrow_mut() = true;
        }
    }

    fn on_action_click(&self) {
        let state = *self.btn_state.borrow();
        match state {
            AppButtonState::Scan => self.on_scan(),
            AppButtonState::Sort => self.on_start(),
        }
    }

    fn on_scan(&self) {
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

        // Canonicalize paths to resolve relative segments, case discrepancies, and UNC formatting
        let canonical_src = std::fs::canonicalize(&src_path).unwrap_or_else(|_| src_path.clone());
        let canonical_dst = std::fs::canonicalize(&dst_path).unwrap_or_else(|_| dst_path.clone());

        // Perform a case-insensitive nesting check for Windows compatibility
        let src_lower = canonical_src.to_string_lossy().to_lowercase();
        let dst_lower = canonical_dst.to_string_lossy().to_lowercase();

        let has_separator = src_lower.ends_with('\\') || src_lower.ends_with('/');
        let src_prefix = if has_separator {
            src_lower.clone()
        } else {
            format!("{}\\", src_lower)
        };

        if src_lower == dst_lower || dst_lower.starts_with(&src_prefix) {
            nwg::modal_info_message(
                &self.window,
                "Photo Fix",
                "Destination directory cannot be the same as or inside the source directory.",
            );
            return;
        }

        let use_copy = self.combo_op.selection() == Some(0);
        let year_only = self.combo_structure.selection() == Some(1);

        self.scan_results.borrow_mut().clear();
        self.btn_src.set_enabled(false);
        self.btn_dst.set_enabled(false);
        self.combo_op.set_enabled(false);
        self.combo_structure.set_enabled(false);
        self.btn_action.set_enabled(false);
        self.progress.set_pos(0);
        self.lbl_status.set_text("Scanning...");
        self.txt_log.set_text("");
        self.log_lines.borrow_mut().clear();
        self.log(&format!("Scanning Source: {}", src));
        self.log("---");

        let (tx, rx) = mpsc::channel::<WorkerMsg>();
        *self.rx.borrow_mut() = Some(rx);
        *self.is_running.borrow_mut() = true;

        self.timer.start();

        std::thread::spawn(move || {
            crate::worker::run_scan(src_path, dst_path, use_copy, year_only, tx);
        });
    }

    fn on_start(&self) {
        if *self.is_running.borrow() {
            return;
        }

        let results = self.scan_results.borrow().clone();
        if results.is_empty() {
            nwg::modal_info_message(
                &self.window,
                "Photo Fix",
                "No scanned results found. Please run Scan Folder first.",
            );
            return;
        }

        self.btn_src.set_enabled(false);
        self.btn_dst.set_enabled(false);
        self.combo_op.set_enabled(false);
        self.combo_structure.set_enabled(false);
        self.btn_action.set_enabled(false);
        self.progress.set_pos(0);
        self.lbl_status.set_text("Sorting...");
        self.txt_log.set_text("");
        self.log_lines.borrow_mut().clear();
        self.log("Starting Sorting Execution...");
        self.log("---");

        let (tx, rx) = mpsc::channel::<WorkerMsg>();
        *self.rx.borrow_mut() = Some(rx);
        *self.is_running.borrow_mut() = true;

        self.timer.start();

        std::thread::spawn(move || {
            crate::worker::run_sort(results, tx);
        });
    }

    fn poll_worker(&self) {
        let rx_ref = self.rx.borrow();
        let rx = match rx_ref.as_ref() {
            Some(r) => r,
            None => return,
        };

        let mut logs = Vec::new();
        let mut last_scan_progress = None;
        let mut last_sort_progress = None;
        let mut scan_done_msg = None;
        let mut sort_done_msg = None;
        let mut error_msg = None;
        let mut disconnected = false;

        // Drain all pending messages, detecting worker crashes
        loop {
            match rx.try_recv() {
                Ok(msg) => match msg {
                    WorkerMsg::ScanProgress { current, total, file } => {
                        logs.push(format!("Scanned: {}", file));
                        last_scan_progress = Some((current, total));
                    }
                    WorkerMsg::ScanDone(results) => {
                        scan_done_msg = Some(results);
                    }
                    WorkerMsg::SortProgress { current, total, file } => {
                        logs.push(file);
                        last_sort_progress = Some((current, total));
                    }
                    WorkerMsg::SortDone { moved, skipped, errors } => {
                        sort_done_msg = Some((moved, skipped, errors));
                    }
                    WorkerMsg::Error(e) => {
                        error_msg = Some(e);
                    }
                },
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }

        // Apply all accumulated logs in one single batch update
        if !logs.is_empty() {
            self.log_batch(&logs);
        }

        // Update progress bar and label once using the latest values
        if let Some((current, total)) = last_scan_progress {
            if total > 0 {
                let pct = (current as u32 * 1000) / total as u32;
                self.progress.set_pos(pct);
            }
            self.lbl_status.set_text(&format!(
                "Scanning {}/{}...",
                current, total
            ));
        }

        if let Some((current, total)) = last_sort_progress {
            if total > 0 {
                let pct = (current as u32 * 1000) / total as u32;
                self.progress.set_pos(pct);
            }
            self.lbl_status.set_text(&format!(
                "Sorting {}/{}...",
                current, total
            ));
        }

        // Handle termination messages at the end
        if let Some(results) = scan_done_msg {
            self.progress.set_pos(1000);
            
            // Print complete plan preview logs
            let mut scan_logs = Vec::new();
            scan_logs.push("--- Scan Complete. Planned Actions: ---".to_string());
            
            let total_count = results.len();
            let mut pending_copy = 0;
            let mut pending_move = 0;
            let mut nodate = 0;

            for res in &results {
                let file_name = res.src.file_name().unwrap_or_default().to_string_lossy();
                match &res.status {
                    ScanStatus::PendingCopy { year, month, dest_path: _ } => {
                        pending_copy += 1;
                        scan_logs.push(format!("[copy] {} -> {}/{}", file_name, year, crate::worker::month_name(*month)));
                    }
                    ScanStatus::PendingMove { year, month, dest_path: _ } => {
                        pending_move += 1;
                        scan_logs.push(format!("[move] {} -> {}/{}", file_name, year, crate::worker::month_name(*month)));
                    }
                    ScanStatus::NoDateSkipped => {
                        nodate += 1;
                        scan_logs.push(format!("[skip-no-date] {}", file_name));
                    }
                }
            }

            let summary = format!(
                "Scan Done! Total: {}, Ready: {}, Missing Date Skips: {}",
                total_count, pending_copy + pending_move, nodate
            );
            scan_logs.push("---".to_string());
            scan_logs.push(summary.clone());
            
            self.log_batch(&scan_logs);
            self.lbl_status.set_text(&summary);
            self.timer.stop();
            if pending_copy + pending_move > 0 {
                *self.btn_state.borrow_mut() = AppButtonState::Sort;
                self.btn_action.set_text("Start Sorting");
            } else {
                *self.btn_state.borrow_mut() = AppButtonState::Scan;
                self.btn_action.set_text("Scan Folder");
            }
            self.btn_action.set_enabled(true);
            self.btn_src.set_enabled(true);
            self.btn_dst.set_enabled(true);
            self.combo_op.set_enabled(true);
            self.combo_structure.set_enabled(true);
            *self.scan_results.borrow_mut() = results;
            *self.is_running.borrow_mut() = false;
        } else if let Some((moved, skipped, errors)) = sort_done_msg {
            self.progress.set_pos(1000);
            let summary = format!(
                "Done! Sorted: {}, Skipped: {}, Errors: {}",
                moved, skipped, errors
            );
            self.lbl_status.set_text(&summary);
            self.log(&format!("---\r\n{}", summary));
            self.timer.stop();
            *self.btn_state.borrow_mut() = AppButtonState::Scan;
            self.btn_action.set_text("Scan Folder");
            self.btn_action.set_enabled(true);
            self.btn_src.set_enabled(true);
            self.btn_dst.set_enabled(true);
            self.combo_op.set_enabled(true);
            self.combo_structure.set_enabled(true);
            self.scan_results.borrow_mut().clear();
            *self.is_running.borrow_mut() = false;
        } else if let Some(e) = error_msg {
            self.lbl_status.set_text("Error!");
            self.log(&format!("ERROR: {}", e));
            self.timer.stop();
            *self.btn_state.borrow_mut() = AppButtonState::Scan;
            self.btn_action.set_text("Scan Folder");
            self.btn_action.set_enabled(true);
            self.btn_src.set_enabled(true);
            self.btn_dst.set_enabled(true);
            self.combo_op.set_enabled(true);
            self.combo_structure.set_enabled(true);
            *self.is_running.borrow_mut() = false;
        } else if disconnected {
            self.lbl_status.set_text("Error: worker stopped unexpectedly");
            self.log("ERROR: Worker thread terminated unexpectedly.");
            self.timer.stop();
            *self.btn_state.borrow_mut() = AppButtonState::Scan;
            self.btn_action.set_text("Scan Folder");
            self.btn_action.set_enabled(true);
            self.btn_src.set_enabled(true);
            self.btn_dst.set_enabled(true);
            self.combo_op.set_enabled(true);
            self.combo_structure.set_enabled(true);
            *self.is_running.borrow_mut() = false;
        }
    }
}

/// Worker module – performs the actual photo sorting off the UI thread
pub mod worker {
    use super::WorkerMsg;
    use super::ScanResult;
    use super::ScanStatus;
    use std::path::{Path, PathBuf};
    use std::sync::mpsc;

    const IMAGE_EXTENSIONS: &[&str] = &[
        "jpg", "jpeg", "png", "tif", "tiff", "bmp", "gif", "webp",
        "heic", "heif", "cr2", "nef", "arw", "dng", "orf", "rw2",
    ];

    /// Collect all image files from `src` recursively.
    fn collect_images(src: &Path) -> Vec<PathBuf> {
        let mut images = Vec::new();
        let mut dirs = vec![src.to_path_buf()];

        while let Some(dir) = dirs.pop() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    if entry.file_type().is_ok_and(|ft| ft.is_symlink()) {
                        continue;
                    }
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

    pub(crate) fn parse_exif_date(s: &str) -> Option<(i32, u32)> {
        let parts: Vec<&str> = s.split([':', ' ', '-', '/']).collect();
        if parts.len() >= 2 {
            let year_str = parts[0].trim().trim_matches('\0');
            let month_str = parts[1].trim().trim_matches('\0');
            if let (Ok(year), Ok(month)) = (year_str.parse::<i32>(), month_str.parse::<u32>()) {
                if (1901..2200).contains(&year) && (1..=12).contains(&month) {
                    return Some((year, month));
                }
            }
        }
        None
    }

    /// Extract capture/born date strictly from EXIF metadata, falling back to filesystem timestamps.
    fn get_date(path: &Path) -> Option<(i32, u32)> {
        use chrono::Datelike;

        // Try EXIF first
        if let Ok(file) = std::fs::File::open(path) {
            let mut buf_reader = std::io::BufReader::new(&file);
            if let Ok(exif) = exif::Reader::new().read_from_container(&mut buf_reader) {
                // 1. Try DateTimeOriginal
                if let Some(field) = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) {
                    if let exif::Value::Ascii(ref vec) = field.value {
                        if let Some(bytes) = vec.first() {
                            let s = String::from_utf8_lossy(bytes);
                            if let Some(date) = parse_exif_date(&s) {
                                return Some(date);
                            }
                        }
                    }
                }
                // 2. Try DateTimeDigitized
                if let Some(field) = exif.get_field(exif::Tag::DateTimeDigitized, exif::In::PRIMARY) {
                    if let exif::Value::Ascii(ref vec) = field.value {
                        if let Some(bytes) = vec.first() {
                            let s = String::from_utf8_lossy(bytes);
                            if let Some(date) = parse_exif_date(&s) {
                                return Some(date);
                            }
                        }
                    }
                }
                // 3. Try DateTime
                if let Some(field) = exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY) {
                    if let exif::Value::Ascii(ref vec) = field.value {
                        if let Some(bytes) = vec.first() {
                            let s = String::from_utf8_lossy(bytes);
                            if let Some(date) = parse_exif_date(&s) {
                                return Some(date);
                            }
                        }
                    }
                }
            }
        }

        // Fall back to filesystem metadata modified or created times
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(system_time) = metadata.modified().or_else(|_| metadata.created()) {
                let datetime: chrono::DateTime<chrono::Local> = system_time.into();
                let year = datetime.year();
                let month = datetime.month();
                if (1901..2200).contains(&year) && (1..=12).contains(&month) {
                    return Some((year, month));
                }
            }
        }

        None
    }

    /// Month number to abbreviated name
    pub fn month_name(month: u32) -> &'static str {
        match month {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            12 => "Dec",
            _ => "Unknown",
        }
    }

    pub fn run_scan(
        src: PathBuf,
        dst: PathBuf,
        use_copy: bool,
        year_only: bool,
        tx: mpsc::Sender<WorkerMsg>,
    ) {
        let images = collect_images(&src);
        let total = images.len();

        if total == 0 {
            let _ = tx.send(WorkerMsg::Error("No image files found in source directory.".into()));
            return;
        }

        // Parallel CPU parsing via Rayon
        use rayon::prelude::*;
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let tx_shared = std::sync::Arc::new(std::sync::Mutex::new(tx.clone()));

        let processed: Vec<(PathBuf, Option<(i32, u32)>)> = images
            .par_iter()
            .map(|img_path| {
                let date = get_date(img_path);
                
                let current = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                // Throttle UI updates: report every 50 files or on the last file
                if current.is_multiple_of(50) || current == total {
                    let file_name = img_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    if let Ok(lock) = tx_shared.lock() {
                        let _ = lock.send(WorkerMsg::ScanProgress {
                            current,
                            total,
                            file: file_name,
                        });
                    }
                }
                
                (img_path.clone(), date)
            })
            .collect();

        let mut results = Vec::new();

        for (img_path, date) in processed {
            let file_name = img_path.file_name().unwrap_or_default().to_string_lossy().to_string();

            let (year, month) = match date {
                Some(ym) => ym,
                None => {
                    results.push(ScanResult {
                        src: img_path,
                        status: ScanStatus::NoDateSkipped,
                    });
                    continue;
                }
            };

            let dest_path = if year_only {
                dst.join(format!("{}", year)).join(&file_name)
            } else {
                dst.join(format!("{}", year)).join(month_name(month)).join(&file_name)
            };

            let status = if use_copy {
                ScanStatus::PendingCopy { year, month, dest_path }
            } else {
                ScanStatus::PendingMove { year, month, dest_path }
            };

            results.push(ScanResult {
                src: img_path,
                status,
            });
        }

        let _ = tx.send(WorkerMsg::ScanDone(results));
    }

    fn generate_unique_dest_path(dest_path: &Path) -> PathBuf {
        if !dest_path.exists() {
            return dest_path.to_path_buf();
        }

        let parent = dest_path.parent().unwrap_or(dest_path);
        let stem = dest_path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = dest_path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();

        let mut counter = 1;
        loop {
            let candidate = parent.join(format!("{}_{}{}", stem, counter, ext));
            if !candidate.exists() {
                return candidate;
            }
            counter += 1;
        }
    }

    pub fn run_sort(
        results: Vec<ScanResult>,
        tx: mpsc::Sender<WorkerMsg>,
    ) {
        let total = results.len();
        let mut moved = 0usize;
        let mut skipped = 0usize;
        let mut errors = 0usize;

        for (i, res) in results.into_iter().enumerate() {
            let file_name = res.src.file_name().unwrap_or_default().to_string_lossy().to_string();

            match res.status {
                ScanStatus::PendingCopy { year, month, dest_path } => {
                    let final_dest = generate_unique_dest_path(&dest_path);
                    let final_file_name = final_dest.file_name().unwrap_or_default().to_string_lossy().to_string();

                    if let Some(parent) = final_dest.parent() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            errors += 1;
                            let _ = tx.send(WorkerMsg::SortProgress {
                                current: i + 1,
                                total,
                                file: format!("[err] {} - {}", file_name, e),
                            });
                            continue;
                        }
                    }

                    match std::fs::copy(&res.src, &final_dest) {
                        Ok(_) => {
                            moved += 1;
                            let log_msg = if final_file_name != file_name {
                                format!("Copied: {} -> {}/{} (renamed to {})", file_name, year, month_name(month), final_file_name)
                            } else {
                                format!("Copied: {} -> {}/{}", file_name, year, month_name(month))
                            };
                            let _ = tx.send(WorkerMsg::SortProgress {
                                current: i + 1,
                                total,
                                file: log_msg,
                            });
                        }
                        Err(e) => {
                            errors += 1;
                            let _ = tx.send(WorkerMsg::SortProgress {
                                current: i + 1,
                                total,
                                file: format!("[err] {} - {}", file_name, e),
                            });
                        }
                    }
                }
                ScanStatus::PendingMove { year, month, dest_path } => {
                    let final_dest = generate_unique_dest_path(&dest_path);
                    let final_file_name = final_dest.file_name().unwrap_or_default().to_string_lossy().to_string();

                    if let Some(parent) = final_dest.parent() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            errors += 1;
                            let _ = tx.send(WorkerMsg::SortProgress {
                                current: i + 1,
                                total,
                                file: format!("[err] {} - {}", file_name, e),
                            });
                            continue;
                        }
                    }

                    let result = std::fs::rename(&res.src, &final_dest).or_else(|_| {
                        std::fs::copy(&res.src, &final_dest)?;
                        std::fs::remove_file(&res.src)
                    });

                    match result {
                        Ok(_) => {
                            moved += 1;
                            let log_msg = if final_file_name != file_name {
                                format!("Moved: {} -> {}/{} (renamed to {})", file_name, year, month_name(month), final_file_name)
                            } else {
                                format!("Moved: {} -> {}/{}", file_name, year, month_name(month))
                            };
                            let _ = tx.send(WorkerMsg::SortProgress {
                                current: i + 1,
                                total,
                                file: log_msg,
                            });
                        }
                        Err(e) => {
                            errors += 1;
                            let _ = tx.send(WorkerMsg::SortProgress {
                                current: i + 1,
                                total,
                                file: format!("[err] {} - {}", file_name, e),
                            });
                        }
                    }
                }
                ScanStatus::NoDateSkipped => {
                    skipped += 1;
                    let _ = tx.send(WorkerMsg::SortProgress {
                        current: i + 1,
                        total,
                        file: format!("[skip-no-date] {}", file_name),
                    });
                }
            }
        }

        let _ = tx.send(WorkerMsg::SortDone { moved, skipped, errors });
    }
}

fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        let msg = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "An unknown panic occurred."
        };
        let loc = if let Some(location) = panic_info.location() {
            format!(" at {}:{}", location.file(), location.line())
        } else {
            "".to_string()
        };
        let detail = format!("Application panicked: {}{}\n\nPlease report this issue.", msg, loc);
        native_windows_gui::error_message("Photo Fix Panic", &detail);
    }));

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

#[cfg(test)]
mod tests {
    use super::worker::{parse_exif_date, month_name};

    #[test]
    fn test_parse_exif_date_colons() {
        assert_eq!(parse_exif_date("2024:05:12 12:34:56"), Some((2024, 5)));
    }

    #[test]
    fn test_parse_exif_date_dashes() {
        assert_eq!(parse_exif_date("2024-08-20 10:11:12"), Some((2024, 8)));
    }

    #[test]
    fn test_parse_exif_date_slashes() {
        assert_eq!(parse_exif_date("2024/11/05 08:00:00"), Some((2024, 11)));
    }

    #[test]
    fn test_parse_exif_date_invalid() {
        assert_eq!(parse_exif_date("invalid-date"), None);
        assert_eq!(parse_exif_date("1899:01:01"), None);
        assert_eq!(parse_exif_date("2024:13:01"), None);
    }

    #[test]
    fn test_month_name() {
        assert_eq!(month_name(1), "Jan");
        assert_eq!(month_name(12), "Dec");
        assert_eq!(month_name(13), "Unknown");
    }
}
