#![windows_subsystem = "windows"]

use native_windows_gui as nwg;
use native_windows_derive::NwgUi;
use nwg::NativeUi;

use std::cell::RefCell;
use std::sync::mpsc;
use std::path::PathBuf;

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

    // ── Scan button ──────────────────────────────────────────────
    #[nwg_control(text: "Scan Folder", size: (96, 28), position: (226, 113))]
    #[nwg_events(OnButtonClick: [PhotoFixApp::on_scan])]
    btn_scan: nwg::Button,

    // ── Start button ─────────────────────────────────────────────
    #[nwg_control(text: "Start Sorting", size: (96, 28), position: (327, 113), enabled: false)]
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
    scan_results: RefCell<Vec<ScanResult>>,
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

        let use_copy = self.combo_op.selection() == Some(0);

        self.scan_results.borrow_mut().clear();
        self.btn_scan.set_enabled(false);
        self.btn_start.set_enabled(false);
        self.progress.set_pos(0);
        self.lbl_status.set_text("Scanning...");
        self.txt_log.set_text("");
        self.log(&format!("Scanning Source: {}", src));
        self.log("---");

        let (tx, rx) = mpsc::channel::<WorkerMsg>();
        *self.rx.borrow_mut() = Some(rx);
        *self.is_running.borrow_mut() = true;

        self.timer.start();

        std::thread::spawn(move || {
            crate::worker::run_scan(src_path, dst_path, use_copy, tx);
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

        self.btn_scan.set_enabled(false);
        self.btn_start.set_enabled(false);
        self.progress.set_pos(0);
        self.lbl_status.set_text("Sorting...");
        self.txt_log.set_text("");
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

        // Drain all pending messages
        while let Ok(msg) = rx.try_recv() {
            match msg {
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
            self.btn_scan.set_enabled(true);
            self.btn_start.set_enabled(pending_copy + pending_move > 0);
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
            self.btn_scan.set_enabled(true);
            self.btn_start.set_enabled(false);
            self.scan_results.borrow_mut().clear();
            *self.is_running.borrow_mut() = false;
        } else if let Some(e) = error_msg {
            self.lbl_status.set_text("Error!");
            self.log(&format!("ERROR: {}", e));
            self.timer.stop();
            self.btn_scan.set_enabled(true);
            self.btn_start.set_enabled(false);
            *self.is_running.borrow_mut() = false;
        }
    }
}

/// Worker module – performs the actual photo sorting off the UI thread
pub mod worker {
    use super::WorkerMsg;
    use super::ScanResult;
    use super::ScanStatus;
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

    fn parse_exif_date(s: &str) -> Option<(i32, u32)> {
        let parts: Vec<&str> = s.split(|c| c == ':' || c == ' ' || c == '-').collect();
        if parts.len() >= 2 {
            if let (Ok(year), Ok(month)) = (parts[0].parse::<i32>(), parts[1].parse::<u32>()) {
                if year > 1900 && year < 2200 && month >= 1 && month <= 12 {
                    return Some((year, month));
                }
            }
        }
        None
    }

    fn parse_date_from_filename(filename: &str) -> Option<(i32, u32)> {
        let chars: Vec<char> = filename.chars().collect();
        // 1. Scan for YYYY-MM-DD or YYYY_MM_DD patterns (10 chars)
        if chars.len() >= 10 {
            for i in 0..=chars.len() - 10 {
                let win = &chars[i..i+10];
                if win[0].is_ascii_digit() && win[1].is_ascii_digit() && win[2].is_ascii_digit() && win[3].is_ascii_digit()
                    && (win[4] == '-' || win[4] == '_')
                    && win[5].is_ascii_digit() && win[6].is_ascii_digit()
                    && (win[7] == '-' || win[7] == '_')
                    && win[8].is_ascii_digit() && win[9].is_ascii_digit()
                {
                    let year_str: String = win[0..4].iter().collect();
                    let month_str: String = win[5..7].iter().collect();
                    let day_str: String = win[8..10].iter().collect();
                    if let (Ok(year), Ok(month), Ok(day)) = (year_str.parse::<i32>(), month_str.parse::<u32>(), day_str.parse::<u32>()) {
                        if year > 1900 && year < 2200 && month >= 1 && month <= 12 && day >= 1 && day <= 31 {
                            return Some((year, month));
                        }
                    }
                }
            }
        }

        // 2. Scan for YYYYMMDD patterns (8 consecutive digits)
        if chars.len() >= 8 {
            for i in 0..=chars.len() - 8 {
                let win = &chars[i..i+8];
                if win.iter().all(|c| c.is_ascii_digit()) {
                    let year_str: String = win[0..4].iter().collect();
                    let month_str: String = win[4..6].iter().collect();
                    let day_str: String = win[6..8].iter().collect();
                    if let (Ok(year), Ok(month), Ok(day)) = (year_str.parse::<i32>(), month_str.parse::<u32>(), day_str.parse::<u32>()) {
                        if year > 1900 && year < 2200 && month >= 1 && month <= 12 && day >= 1 && day <= 31 {
                            return Some((year, month));
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract capture/born date strictly from EXIF metadata or filename fallback.
    fn get_date(path: &PathBuf) -> Option<(i32, u32)> {
        // 1. Try EXIF
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

        // 2. Fallback: Parse date from filename (for screenshots)
        if let Some(file_name) = path.file_name() {
            let name_str = file_name.to_string_lossy();
            if let Some(date) = parse_date_from_filename(&name_str) {
                return Some(date);
            }
        }

        None
    }

    /// Month number to abbreviated name
    pub fn month_name(month: u32) -> &'static str {
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

    pub fn run_scan(
        src: PathBuf,
        dst: PathBuf,
        use_copy: bool,
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
                let file_name = img_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                if let Ok(lock) = tx_shared.lock() {
                    let _ = lock.send(WorkerMsg::ScanProgress {
                        current,
                        total,
                        file: file_name,
                    });
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

            let dest_path = dst.join(format!("{}", year)).join(month_name(month)).join(&file_name);

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
                    if dest_path.exists() {
                        skipped += 1;
                        let _ = tx.send(WorkerMsg::SortProgress {
                            current: i + 1,
                            total,
                            file: format!("[exists] {}", file_name),
                        });
                        continue;
                    }

                    if let Some(parent) = dest_path.parent() {
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

                    match std::fs::copy(&res.src, &dest_path) {
                        Ok(_) => {
                            moved += 1;
                            let _ = tx.send(WorkerMsg::SortProgress {
                                current: i + 1,
                                total,
                                file: format!("Copied: {} -> {}/{}", file_name, year, month_name(month)),
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
                    if dest_path.exists() {
                        skipped += 1;
                        let _ = tx.send(WorkerMsg::SortProgress {
                            current: i + 1,
                            total,
                            file: format!("[exists] {}", file_name),
                        });
                        continue;
                    }

                    if let Some(parent) = dest_path.parent() {
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

                    let result = std::fs::rename(&res.src, &dest_path).or_else(|_| {
                        std::fs::copy(&res.src, &dest_path)?;
                        std::fs::remove_file(&res.src)
                    });

                    match result {
                        Ok(_) => {
                            moved += 1;
                            let _ = tx.send(WorkerMsg::SortProgress {
                                current: i + 1,
                                total,
                                file: format!("Moved: {} -> {}/{}", file_name, year, month_name(month)),
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
