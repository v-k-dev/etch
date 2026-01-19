use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Box as GtkBox, Button, Image, Label, Orientation, PolicyType, ScrolledWindow};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crate::download::{DistrosCatalog, ISOFetcher};

use super::window::AppState;
use super::download_progress::DownloadProgressWindow;

pub fn show_iso_browser_window(
    parent: &ApplicationWindow,
    iso_label: Label,
    state: Rc<RefCell<AppState>>,
    platform_box: GtkBox,
    platform_icon: Image,
    platform_label: Label,
) {
    let dialog = ApplicationWindow::builder()
        .transient_for(parent)
        .modal(true)
        .title("Browse ISOs")
        .default_width(680)
        .default_height(440)
        .decorated(false)
        .build();

    let main_box = GtkBox::new(Orientation::Vertical, 0);
    main_box.add_css_class("main-container");

    // Title bar
    let titlebar = GtkBox::new(Orientation::Horizontal, 12);
    titlebar.add_css_class("title-section");
    titlebar.set_margin_top(14);
    titlebar.set_margin_bottom(10);
    titlebar.set_margin_start(16);
    titlebar.set_margin_end(16);

    let title = Label::new(Some("BROWSE ISOs"));
    title.add_css_class("app-title");
    title.set_hexpand(true);
    title.set_halign(gtk4::Align::Start);
    titlebar.append(&title);

    let close_btn = Button::new();
    close_btn.set_icon_name("window-close-symbolic");
    close_btn.add_css_class("menu-button");
    let dialog_clone = dialog.clone();
    close_btn.connect_clicked(move |_| {
        dialog_clone.close();
    });
    titlebar.append(&close_btn);

    main_box.append(&titlebar);

    // Content
    let scroll = ScrolledWindow::new();
    scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    scroll.set_vexpand(true);
    scroll.set_hexpand(true);
    scroll.set_margin_start(12);
    scroll.set_margin_end(12);
    scroll.set_margin_bottom(12);

    let distros_box = GtkBox::new(Orientation::Vertical, 4);
    distros_box.set_hexpand(true);

    // Track all download buttons to disable during download
    let download_buttons: Rc<RefCell<Vec<Button>>> = Rc::new(RefCell::new(Vec::new()));

    // Load catalog
    let catalog_result = DistrosCatalog::fetch();

    match catalog_result {
        Ok(catalog) => {
            for distro in &catalog.distros {
                let row = GtkBox::new(Orientation::Horizontal, 8);
                row.set_margin_top(3);
                row.set_margin_bottom(3);
                row.add_css_class("iso-row");

                // Logo icon with colored background
                let logo_box = GtkBox::new(Orientation::Vertical, 0);
                logo_box.set_size_request(36, 36);
                logo_box.set_halign(gtk4::Align::Center);
                logo_box.set_valign(gtk4::Align::Center);
                
                // Add category-specific background color
                use crate::download::catalog::DistroCategory;
                match &distro.category {
                    DistroCategory::Ubuntu => logo_box.add_css_class("logo-ubuntu"),
                    DistroCategory::Fedora => logo_box.add_css_class("logo-fedora"),
                    DistroCategory::Debian => logo_box.add_css_class("logo-debian"),
                    DistroCategory::Arch => logo_box.add_css_class("logo-arch"),
                    DistroCategory::Mint => logo_box.add_css_class("logo-mint"),
                    DistroCategory::Raspberry => logo_box.add_css_class("logo-raspberry"),
                    DistroCategory::Suse => logo_box.add_css_class("logo-suse"),
                    _ => logo_box.add_css_class("logo-default"),
                }
                
                let logo_icon = Image::from_icon_name("drive-harddisk-symbolic");
                logo_icon.set_icon_size(gtk4::IconSize::Normal);
                logo_box.append(&logo_icon);
                row.append(&logo_box);

                let info = GtkBox::new(Orientation::Vertical, 1);
                info.set_hexpand(false);
                info.set_halign(gtk4::Align::Start);
                info.set_size_request(360, -1);

                let name = Label::new(Some(&distro.name));
                name.set_halign(gtk4::Align::Start);
                name.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                name.set_max_width_chars(35);
                name.add_css_class("iso-name-label");
                info.append(&name);

                let meta = Label::new(Some(&format!("{} • {}", distro.version, distro.size_human)));
                meta.set_halign(gtk4::Align::Start);
                meta.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                meta.set_max_width_chars(35);
                meta.add_css_class("iso-meta-label");
                info.append(&meta);

                row.append(&info);

                let dl_btn = Button::new();
                let dl_icon = Image::from_icon_name("folder-download-symbolic");
                dl_icon.set_icon_size(gtk4::IconSize::Normal);
                dl_btn.set_child(Some(&dl_icon));
                dl_btn.add_css_class("download-button-compact");
                dl_btn.set_tooltip_text(Some(&format!("Download {} ({})", distro.name, distro.size_human)));
                dl_btn.set_size_request(40, 32);
                dl_btn.set_halign(gtk4::Align::End);
                dl_btn.set_valign(gtk4::Align::Center);
                dl_btn.set_hexpand(false);

                let distro_clone = distro.clone();
                let iso_label_clone = iso_label.clone();
                let state_clone = state.clone();
                let platform_box_clone = platform_box.clone();
                let platform_icon_clone = platform_icon.clone();
                let platform_label_clone = platform_label.clone();
                let dialog_clone2 = dialog.clone();
                let parent_clone = parent.clone();
                let buttons_for_click = download_buttons.clone();

                dl_btn.connect_clicked(move |_btn| {
                    // Check global download flag - prevent concurrent downloads
                    if state_clone.borrow().download_in_progress.load(Ordering::Relaxed) {
                        println!("⚠ Download already in progress, ignoring click");
                        return;
                    }

                    // Set global flag FIRST
                    state_clone.borrow().download_in_progress.store(true, Ordering::Relaxed);

                    // Disable ALL download buttons
                    for button in buttons_for_click.borrow().iter() {
                        button.set_sensitive(false);
                    }

                    let download_dir = ISOFetcher::default_download_dir();
                    
                    println!("→ Starting download: {}", distro_clone.name);
                    println!("→ Download location: {}", download_dir.display());

                    // Create progress window
                    let progress_window = DownloadProgressWindow::new(&parent_clone, &distro_clone.name);
                    progress_window.show();

                    let (tx, rx) = mpsc::channel();
                    let (progress_tx, progress_rx) = mpsc::channel();
                    let distro_for_thread = distro_clone.clone();
                    let download_dir_clone = download_dir.clone();
                    let cancel_flag = Arc::new(AtomicBool::new(false));
                    let cancel_flag_thread = cancel_flag.clone();

                    // Spawn download thread
                    thread::spawn(move || {
                        let result = ISOFetcher::download(
                            &distro_for_thread, 
                            &download_dir_clone, 
                            Some(progress_tx),
                            Some(cancel_flag_thread),
                        );
                        tx.send(result).ok();
                    });

                    let iso_label_for_poll = iso_label_clone.clone();
                    let buttons_for_poll = buttons_for_click.clone();
                    let state_for_poll = state_clone.clone();
                    let platform_box_for_poll = platform_box_clone.clone();
                    let platform_icon_for_poll = platform_icon_clone.clone();
                    let platform_label_for_poll = platform_label_clone.clone();
                    let dialog_for_poll = dialog_clone2.clone();

                    // Poll for progress updates
                    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                        // Update progress if available
                        while let Ok(progress) = progress_rx.try_recv() {
                            use crate::download::DownloadProgress;
                            match progress {
                                DownloadProgress::Progress { bytes, total, bps } => {
                                    progress_window.update_progress(bytes, total, bps as f64);
                                }
                                _ => {}
                            }
                        }

                        // Check if download finished
                        match rx.try_recv() {
                            Ok(result) => {
                                progress_window.close();
                                
                                // Clear global download flag
                                state_for_poll.borrow().download_in_progress.store(false, Ordering::Relaxed);

                                match result {
                                    Ok(path) => {
                                        println!("✓ Download complete: {}", path.display());
                                        state_for_poll.borrow_mut().selected_iso = Some(path.clone());
                                        iso_label_for_poll.set_text(&path.file_name().unwrap().to_string_lossy());

                                        let platform = crate::core::platforms::Platform::from_iso_path(&path);
                                        platform_box_for_poll.set_visible(true);
                                        platform_icon_for_poll.set_icon_name(Some(platform.icon_name()));
                                        platform_label_for_poll.set_text(platform.display_name());

                                        // Re-enable ALL buttons after success
                                        for button in buttons_for_poll.borrow().iter() {
                                            button.set_sensitive(true);
                                        }
                                        
                                        dialog_for_poll.close();
                                    }
                                    Err(e) => {
                                        println!("✗ Download failed: {}", e);
                                        
                                        // Re-enable ALL buttons after failure
                                        for button in buttons_for_poll.borrow().iter() {
                                            button.set_sensitive(true);
                                        }
                                    }
                                }
                                glib::ControlFlow::Break
                            }
                            Err(_) => glib::ControlFlow::Continue,
                        }
                    });
                });

                // Add button to tracking list BEFORE appending to row
                download_buttons.borrow_mut().push(dl_btn.clone());
                row.append(&dl_btn);
                distros_box.append(&row);
            }
        }
        Err(e) => {
            let err = Label::new(Some(&format!("Failed to load catalog:\n{}", e)));
            err.add_css_class("warning-compact");
            err.set_wrap(true);
            err.set_margin_top(20);
            distros_box.append(&err);
        }
    }

    scroll.set_child(Some(&distros_box));
    main_box.append(&scroll);

    dialog.set_child(Some(&main_box));
    dialog.present();
}
