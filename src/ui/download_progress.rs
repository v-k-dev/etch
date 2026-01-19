use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Box as GtkBox, Button, Label, Orientation, ProgressBar};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct DownloadProgressWindow {
    dialog: ApplicationWindow,
    progress_bar: ProgressBar,
    speed_label: Label,
    eta_label: Label,
    size_label: Label,
    cancel_flag: Arc<AtomicBool>,
}

impl DownloadProgressWindow {
    pub fn new(parent: &ApplicationWindow, iso_name: &str) -> Self {
        let dialog = ApplicationWindow::builder()
            .transient_for(parent)
            .modal(true)
            .title("Downloading ISO")
            .default_width(500)
            .default_height(200)
            .decorated(false)
            .build();

        let main_box = GtkBox::new(Orientation::Vertical, 0);
        main_box.add_css_class("main-container");
        main_box.set_margin_top(20);
        main_box.set_margin_bottom(20);
        main_box.set_margin_start(24);
        main_box.set_margin_end(24);

        // Title
        let title_box = GtkBox::new(Orientation::Horizontal, 12);
        let title = Label::new(Some("DOWNLOADING ISO"));
        title.add_css_class("app-title");
        title.set_hexpand(true);
        title.set_halign(gtk4::Align::Start);
        title_box.append(&title);

        let close_btn = Button::new();
        close_btn.set_icon_name("window-close-symbolic");
        close_btn.add_css_class("menu-button");
        let dialog_clone = dialog.clone();
        close_btn.connect_clicked(move |_| {
            dialog_clone.close();
        });
        title_box.append(&close_btn);
        main_box.append(&title_box);

        // ISO name label
        let name_label = Label::new(Some(iso_name));
        name_label.add_css_class("section-label");
        name_label.set_margin_top(16);
        name_label.set_halign(gtk4::Align::Start);
        main_box.append(&name_label);

        // Progress bar
        let progress_bar = ProgressBar::new();
        progress_bar.set_margin_top(12);
        progress_bar.set_show_text(true);
        progress_bar.set_text(Some("Starting download..."));
        main_box.append(&progress_bar);

        // Info grid
        let info_box = GtkBox::new(Orientation::Horizontal, 24);
        info_box.set_margin_top(12);
        info_box.set_homogeneous(true);

        // Speed
        let speed_box = GtkBox::new(Orientation::Vertical, 4);
        let speed_label_title = Label::new(Some("Speed"));
        speed_label_title.add_css_class("field-label");
        speed_label_title.set_halign(gtk4::Align::Start);
        let speed_label = Label::new(Some("-- MB/s"));
        speed_label.add_css_class("field-value");
        speed_label.set_halign(gtk4::Align::Start);
        speed_box.append(&speed_label_title);
        speed_box.append(&speed_label);
        info_box.append(&speed_box);

        // ETA
        let eta_box = GtkBox::new(Orientation::Vertical, 4);
        let eta_label_title = Label::new(Some("ETA"));
        eta_label_title.add_css_class("field-label");
        eta_label_title.set_halign(gtk4::Align::Start);
        let eta_label = Label::new(Some("Calculating..."));
        eta_label.add_css_class("field-value");
        eta_label.set_halign(gtk4::Align::Start);
        eta_box.append(&eta_label_title);
        eta_box.append(&eta_label);
        info_box.append(&eta_box);

        // Size
        let size_box = GtkBox::new(Orientation::Vertical, 4);
        let size_label_title = Label::new(Some("Size"));
        size_label_title.add_css_class("field-label");
        size_label_title.set_halign(gtk4::Align::Start);
        let size_label = Label::new(Some("0 MB / 0 MB"));
        size_label.add_css_class("field-value");
        size_label.set_halign(gtk4::Align::Start);
        size_box.append(&size_label_title);
        size_box.append(&size_label);
        info_box.append(&size_box);

        main_box.append(&info_box);

        // Cancel button
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_btn = Button::with_label("Cancel Download");
        cancel_btn.add_css_class("button-primary");
        cancel_btn.set_margin_top(20);
        let cancel_flag_clone = cancel_flag.clone();
        let dialog_clone = dialog.clone();
        cancel_btn.connect_clicked(move |_| {
            cancel_flag_clone.store(true, Ordering::Relaxed);
            dialog_clone.close();
        });
        main_box.append(&cancel_btn);

        dialog.set_child(Some(&main_box));

        Self {
            dialog,
            progress_bar,
            speed_label,
            eta_label,
            size_label,
            cancel_flag,
        }
    }

    pub fn show(&self) {
        self.dialog.present();
    }

    pub fn update_progress(&self, bytes_downloaded: u64, total_bytes: u64, speed_bps: f64) {
        let fraction = if total_bytes > 0 {
            bytes_downloaded as f64 / total_bytes as f64
        } else {
            0.0
        };
        
        self.progress_bar.set_fraction(fraction);
        
        // Update percentage text
        let percent = (fraction * 100.0) as u32;
        self.progress_bar.set_text(Some(&format!("{}%", percent)));

        // Update speed
        let speed_mbps = speed_bps / 1_048_576.0;
        self.speed_label.set_text(&format!("{:.2} MB/s", speed_mbps));

        // Update size
        let mb_downloaded = bytes_downloaded as f64 / 1_048_576.0;
        let mb_total = total_bytes as f64 / 1_048_576.0;
        self.size_label.set_text(&format!("{:.1} MB / {:.1} MB", mb_downloaded, mb_total));

        // Calculate ETA
        if speed_bps > 0.0 {
            let remaining_bytes = total_bytes.saturating_sub(bytes_downloaded);
            let eta_seconds = remaining_bytes as f64 / speed_bps;
            
            if eta_seconds < 60.0 {
                self.eta_label.set_text(&format!("{:.0} seconds", eta_seconds));
            } else if eta_seconds < 3600.0 {
                let minutes = eta_seconds / 60.0;
                self.eta_label.set_text(&format!("{:.1} minutes", minutes));
            } else {
                let hours = eta_seconds / 3600.0;
                self.eta_label.set_text(&format!("{:.1} hours", hours));
            }
        } else {
            self.eta_label.set_text("Calculating...");
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    pub fn close(&self) {
        self.dialog.close();
    }
}
