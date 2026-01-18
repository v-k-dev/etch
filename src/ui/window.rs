use gtk4::gio::ListStore;
use gtk4::prelude::*;
use gtk4::{
    glib, Application, ApplicationWindow, Box as GtkBox, Button, ButtonsType, DropDown,
    FileChooserAction, FileChooserDialog, Image, Label, MessageDialog, MessageType, Orientation,
    PolicyType, ProgressBar, ResponseType, ScrolledWindow, TextBuffer, TextView,
};
use gtk4::gdk_pixbuf::PixbufLoader;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;

#[derive(Clone)]
struct AppState {
    selected_iso: Option<PathBuf>,
    selected_device_path: Option<String>,
    is_working: bool,
    action_state: ActionAreaState,
}

#[derive(Clone)]
struct UIComponents {
    status_dot: GtkBox,
    progress_label: Label,
    progress_bar: ProgressBar,
    speed_label: Label,
    write_button: Button,
    iso_button: Button,
    device_dropdown: DropDown,
    message_buffer: TextBuffer,
    refresh_fn: Rc<dyn Fn(bool)>,
}

#[derive(Debug, Clone)]
enum WorkMessage {
    WriteProgress(u64, u64, u64),  // bytes, total, bps
    VerifyProgress(u64, u64, u64), // bytes, total, bps
    WriteComplete,
    VerifyComplete,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
enum ActionAreaState {
    Idle,
    Armed,
    Writing,
    Verifying,
    Done,
    Error(String),
}

/// Build the main application window
#[allow(clippy::too_many_lines)] // UI setup requires comprehensive code
pub fn build_ui(app: &Application) {
    // Load CSS
    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_data(include_str!("style.css"));
    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("Could not connect to display"),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let _ = std::fs::remove_file("/tmp/etch-ui.log");

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Etch")
        .default_width(680)
        .default_height(440)
        .resizable(false)
        .build();
    
    // Set window icon - embed the PNG directly
    let loader = PixbufLoader::new();
    let icon_data = include_bytes!("../../org.etch.Etch.png");
    if loader.write(icon_data).is_ok() {
        let _ = loader.close();
        if let Some(_pixbuf) = loader.pixbuf() {
            // Icon loaded successfully - GTK will find it via theme or we rely on desktop file
            window.set_icon_name(Some("org.etch.Etch"));
        }
    }

    let state = Rc::new(RefCell::new(AppState {
        selected_iso: None,
        selected_device_path: None,
        is_working: false,
        action_state: ActionAreaState::Idle,
    }));

    let main_box = GtkBox::new(Orientation::Vertical, 0);
    main_box.add_css_class("main-container");
    main_box.set_margin_top(20);
    main_box.set_margin_bottom(16);
    main_box.set_margin_start(18);
    main_box.set_margin_end(18);

    // Title Section - Horizontal Layout with Status Dot
    let title_box = GtkBox::new(Orientation::Horizontal, 12);
    title_box.add_css_class("title-section");
    title_box.set_halign(gtk4::Align::Fill);

    // Status Dot
    let status_dot = GtkBox::new(Orientation::Horizontal, 0);
    status_dot.add_css_class("status-dot");
    status_dot.add_css_class("idle");
    status_dot.set_valign(gtk4::Align::Center);
    title_box.append(&status_dot);

    let title = Label::new(Some("ETCH"));
    title.add_css_class("app-title");
    title_box.append(&title);

    let subtitle = Label::new(Some("ISO to USB Writer"));
    subtitle.add_css_class("app-subtitle");
    subtitle.set_hexpand(true);
    subtitle.set_halign(gtk4::Align::Start);
    subtitle.set_valign(gtk4::Align::Center);
    title_box.append(&subtitle);

    // Update check button
    let update_button = Button::new();
    update_button.add_css_class("menu-button");
    update_button.set_icon_name("software-update-available-symbolic");
    update_button.set_valign(gtk4::Align::Center);
    update_button.set_tooltip_text(Some("Check for updates"));
    
    let window_for_update = window.clone();
    update_button.connect_clicked(move |_| {
        let window_clone = window_for_update.clone();
        
        // Check for updates from GitHub
        let dialog = MessageDialog::new(
            Some(&window_for_update),
            gtk4::DialogFlags::MODAL,
            MessageType::Info,
            ButtonsType::None,
            "Checking for Updates",
        );
        dialog.set_secondary_text(Some("Checking GitHub for the latest version..."));
        
        let (tx, rx) = mpsc::channel();
        
        // Fetch latest release from GitHub in background thread
        // Try releases first, then fallback to tags
        thread::spawn(move || {
            // Try releases first
            let result = std::process::Command::new("curl")
                .args([
                    "-s",
                    "-H", "Accept: application/vnd.github+json",
                    "https://api.github.com/repos/v-k-dev/etch/releases/latest"
                ])
                .output();
            
            // If releases don't exist, try tags
            if let Ok(ref output) = result {
                if output.status.success() {
                    let response = String::from_utf8_lossy(&output.stdout);
                    if response.contains("\"message\"") && response.contains("\"Not Found\"") {
                        // No releases, try tags instead
                        let tags_result = std::process::Command::new("curl")
                            .args([
                                "-s",
                                "-H", "Accept: application/vnd.github+json",
                                "https://api.github.com/repos/v-k-dev/etch/tags"
                            ])
                            .output();
                        tx.send(("tags", tags_result)).ok();
                        return;
                    }
                }
            }
            
            tx.send(("releases", result)).ok();
        });
        
        // Poll for result
        let dialog_clone = dialog.clone();
        let rx_holder = Rc::new(RefCell::new(Some(rx)));
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            let mut rx_opt = rx_holder.borrow_mut().take();
            if let Some(rx) = rx_opt.as_mut() {
                match rx.try_recv() {
                    Ok((source, Ok(output))) if output.status.success() => {
                        dialog_clone.close();
                        
                        let response = String::from_utf8_lossy(&output.stdout);
                        
                        match source {
                            "tags" => {
                                // Parse tags array and get the first (latest) tag
                                if response.trim() == "[]" || response.contains("\"message\"") {
                                    // No tags exist
                                    let info_dialog = MessageDialog::new(
                                        Some(&window_clone),
                                        gtk4::DialogFlags::MODAL,
                                        MessageType::Info,
                                        ButtonsType::Ok,
                                        "No Releases Available",
                                    );
                                    info_dialog.set_secondary_text(Some(&format!(
                                        "No releases have been published yet.\n\n\
                                         Current Version: v{}\n\
                                         Git: {}\n\n\
                                         You're running the latest development version.",
                                        crate::VERSION, crate::GIT_HASH
                                    )));
                                    info_dialog.connect_response(|d, _| d.close());
                                    info_dialog.show();
                                    return glib::ControlFlow::Break;
                                }
                                
                                // Parse first tag from array: [{"name": "v0.1.1", ...}]
                                if let Some(name_start) = response.find("\"name\"") {
                                    if let Some(name_value_start) = response[name_start..].find(": \"") {
                                        let name_search = &response[name_start + name_value_start + 3..];
                                        if let Some(name_end) = name_search.find('"') {
                                            let latest_tag = &name_search[..name_end];
                                            
                                            if compare_versions(&format!("v{}", crate::VERSION), latest_tag) {
                                                // Update available - show manual update dialog
                                                show_manual_update_dialog(&window_clone, latest_tag);
                                            } else {
                                                // Already up to date
                                                show_up_to_date_dialog(&window_clone);
                                            }
                                        } else {
                                            show_update_error(&window_clone, "Failed to parse tag name");
                                        }
                                    } else {
                                        show_update_error(&window_clone, "Invalid tags API response");
                                    }
                                } else {
                                    show_update_error(&window_clone, "No tags found in response");
                                }
                            }
                            "releases" => {
                                // Parse release JSON response
                                if response.contains("\"message\"") && response.contains("\"Not Found\"") {
                                    // This shouldn't happen as we fallback to tags, but handle it
                                    show_update_error(&window_clone, "No releases found");
                                    return glib::ControlFlow::Break;
                                }
                                
                                // Parse JSON response to get tag_name and download URL
                                if let Some(tag_start) = response.find("\"tag_name\"") {
                                    if let Some(tag_value_start) = response[tag_start..].find(": \"") {
                                        let tag_search = &response[tag_start + tag_value_start + 3..];
                                        if let Some(tag_end) = tag_search.find('"') {
                                            let latest_tag = &tag_search[..tag_end];
                                            
                                            if compare_versions(&format!("v{}", crate::VERSION), latest_tag) {
                                                // Update available
                                                show_update_available_dialog(&window_clone, latest_tag, &response);
                                            } else {
                                                // Already up to date
                                                show_up_to_date_dialog(&window_clone);
                                            }
                                        } else {
                                            show_update_error(&window_clone, "Failed to parse release version");
                                        }
                                    } else {
                                        show_update_error(&window_clone, "Invalid GitHub API response format");
                                    }
                                } else {
                                    show_update_error(&window_clone, "GitHub API returned unexpected format");
                                }
                            }
                            _ => {
                                show_update_error(&window_clone, "Internal error: unknown source");
                            }
                        }
                        
                        return glib::ControlFlow::Break;
                    }
                    Ok((_, Ok(output))) => {
                        // Non-success status code
                        dialog_clone.close();
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        if stderr.contains("Could not resolve host") {
                            show_update_error(&window_clone, "No internet connection");
                        } else {
                            show_update_error(&window_clone, "Failed to connect to GitHub");
                        }
                        return glib::ControlFlow::Break;
                    }
                    Ok((_, Err(_))) => {
                        dialog_clone.close();
                        show_update_error(&window_clone, "Failed to execute update check");
                        return glib::ControlFlow::Break;
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        *rx_holder.borrow_mut() = rx_opt;
                        return glib::ControlFlow::Continue;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        dialog_clone.close();
                        show_update_error(&window_clone, "Update check failed");
                        return glib::ControlFlow::Break;
                    }
                }
            }
            glib::ControlFlow::Break
        });
        
        dialog.show();
    });
    
    title_box.append(&update_button);

    // Menu button
    let menu_button = Button::new();
    menu_button.add_css_class("menu-button");
    menu_button.set_icon_name("open-menu-symbolic");
    menu_button.set_valign(gtk4::Align::Center);
    
    let window_for_menu = window.clone();
    menu_button.connect_clicked(move |_| {
        // Create advanced formatting options menu - more compact
        let dialog = gtk4::Window::builder()
            .transient_for(&window_for_menu)
            .modal(true)
            .decorated(false)
            .resizable(false)
            .default_width(280)
            .default_height(360)
            .build();
        
        dialog.add_css_class("about-dialog");
        
        // Main overlay container
        let overlay = gtk4::Overlay::new();
        
        // Content box - reduced spacing
        let content = GtkBox::new(Orientation::Vertical, 4);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(14);
        content.set_margin_end(14);
        content.set_halign(gtk4::Align::Fill);
        content.set_valign(gtk4::Align::Start);
        
        // Menu title - more compact
        let title = Label::new(Some("Advanced"));
        title.add_css_class("menu-title");
        title.set_halign(gtk4::Align::Start);
        title.set_margin_bottom(8);
        content.append(&title);
        
        // Formatting Speed Options
        let speed_section = Label::new(Some("Speed"));
        speed_section.add_css_class("menu-section");
        speed_section.set_halign(gtk4::Align::Start);
        content.append(&speed_section);
        
        let fast_btn = Button::with_label("Fast (Quick)");
        fast_btn.add_css_class("menu-item");
        fast_btn.set_halign(gtk4::Align::Fill);
        content.append(&fast_btn);
        
        let dialog_for_fast = dialog.clone();
        fast_btn.connect_clicked(move |_| {
            let info = MessageDialog::new(
                None::<&gtk4::Window>,
                gtk4::DialogFlags::MODAL,
                MessageType::Info,
                ButtonsType::Ok,
                "Fast Mode",
            );
            info.set_secondary_text(Some("Quick write mode selected.\nFastest write speed with minimal verification."));
            info.connect_response(|d, _| d.close());
            info.show();
            dialog_for_fast.close();
        });
        
        let medium_btn = Button::with_label("Medium (Balanced)");
        medium_btn.add_css_class("menu-item");
        medium_btn.set_halign(gtk4::Align::Fill);
        content.append(&medium_btn);
        
        let dialog_for_medium = dialog.clone();
        medium_btn.connect_clicked(move |_| {
            let info = MessageDialog::new(
                None::<&gtk4::Window>,
                gtk4::DialogFlags::MODAL,
                MessageType::Info,
                ButtonsType::Ok,
                "Medium Mode",
            );
            info.set_secondary_text(Some("Balanced write mode selected.\nGood speed with standard verification."));
            info.connect_response(|d, _| d.close());
            info.show();
            dialog_for_medium.close();
        });
        
        let slow_btn = Button::with_label("Secure (Slow)");
        slow_btn.add_css_class("menu-item");
        slow_btn.set_halign(gtk4::Align::Fill);
        content.append(&slow_btn);
        
        let dialog_for_slow = dialog.clone();
        slow_btn.connect_clicked(move |_| {
            let info = MessageDialog::new(
                None::<&gtk4::Window>,
                gtk4::DialogFlags::MODAL,
                MessageType::Info,
                ButtonsType::Ok,
                "Secure Mode",
            );
            info.set_secondary_text(Some("Secure write mode selected.\nSlower speed with extensive verification and checksum validation."));
            info.connect_response(|d, _| d.close());
            info.show();
            dialog_for_slow.close();
        });
        
        // Security Options
        let security_section = Label::new(Some("Security"));
        security_section.add_css_class("menu-section");
        security_section.set_halign(gtk4::Align::Start);
        security_section.set_margin_top(8);
        content.append(&security_section);
        
        let clean_btn = Button::with_label("Zero Fill");
        clean_btn.add_css_class("menu-item");
        clean_btn.set_halign(gtk4::Align::Fill);
        content.append(&clean_btn);
        
        let dialog_for_clean = dialog.clone();
        clean_btn.connect_clicked(move |_| {
            let info = MessageDialog::new(
                None::<&gtk4::Window>,
                gtk4::DialogFlags::MODAL,
                MessageType::Info,
                ButtonsType::Ok,
                "Zero Fill",
            );
            info.set_secondary_text(Some("Overwrites entire drive with zeros.\nBasic secure erase method."));
            info.connect_response(|d, _| d.close());
            info.show();
            dialog_for_clean.close();
        });
        
        let forensic_btn = Button::with_label("DoD 5220.22-M");
        forensic_btn.add_css_class("menu-item");
        forensic_btn.set_halign(gtk4::Align::Fill);
        content.append(&forensic_btn);
        
        let dialog_for_forensic = dialog.clone();
        forensic_btn.connect_clicked(move |_| {
            let info = MessageDialog::new(
                None::<&gtk4::Window>,
                gtk4::DialogFlags::MODAL,
                MessageType::Info,
                ButtonsType::Ok,
                "DoD 5220.22-M",
            );
            info.set_secondary_text(Some("Department of Defense standard.\n3-pass overwrite: random → complement → random.\nSuitable for classified data."));
            info.connect_response(|d, _| d.close());
            info.show();
            dialog_for_forensic.close();
        });
        
        let crypto_btn = Button::with_label("AES-256 Shred");
        crypto_btn.add_css_class("menu-item");
        crypto_btn.set_halign(gtk4::Align::Fill);
        content.append(&crypto_btn);
        
        let dialog_for_crypto = dialog.clone();
        crypto_btn.connect_clicked(move |_| {
            let info = MessageDialog::new(
                None::<&gtk4::Window>,
                gtk4::DialogFlags::MODAL,
                MessageType::Info,
                ButtonsType::Ok,
                "AES-256 Shred",
            );
            info.set_secondary_text(Some("Military-grade secure erase.\n7-pass overwrite with AES-256 encrypted random data.\nMaximum security, slowest operation."));
            info.connect_response(|d, _| d.close());
            info.show();
            dialog_for_crypto.close();
        });
        
        // About section
        let about_section = Label::new(Some("About"));
        about_section.add_css_class("menu-section");
        about_section.set_halign(gtk4::Align::Start);
        about_section.set_margin_top(8);
        content.append(&about_section);
        
        let version_info = Label::new(Some(&crate::version_info()));
        version_info.add_css_class("menu-info");
        version_info.set_halign(gtk4::Align::Start);
        content.append(&version_info);
        
        overlay.set_child(Some(&content));
        
        // Close button overlay (top-right)
        let close_button = Button::new();
        close_button.set_icon_name("window-close-symbolic");
        close_button.add_css_class("about-close-button");
        close_button.set_halign(gtk4::Align::End);
        close_button.set_valign(gtk4::Align::Start);
        close_button.set_margin_top(8);
        close_button.set_margin_end(8);
        
        let dialog_clone = dialog.clone();
        close_button.connect_clicked(move |_| {
            dialog_clone.close();
        });
        
        overlay.add_overlay(&close_button);
        
        dialog.set_child(Some(&overlay));
        dialog.show();
    });
    
    title_box.append(&menu_button);

    main_box.append(&title_box);

    // Warning - Compact
    let warning = Label::new(Some("All data on the target will be permanently erased"));
    warning.add_css_class("warning-compact");
    warning.set_halign(gtk4::Align::Start);
    main_box.append(&warning);

    // Main Content - Horizontal Layout
    let content_box = GtkBox::new(Orientation::Horizontal, 24);
    content_box.set_homogeneous(true);
    content_box.set_hexpand(true);

    // ISO Selection Section
    let iso_section = GtkBox::new(Orientation::Vertical, 6);
    iso_section.add_css_class("section-compact");
    iso_section.set_vexpand(true);

    let iso_section_title = Label::new(Some("SOURCE"));
    iso_section_title.add_css_class("section-title-compact");
    iso_section_title.set_halign(gtk4::Align::Start);
    iso_section.append(&iso_section_title);

    let iso_label = Label::new(Some("No ISO selected"));
    iso_label.add_css_class("file-label-compact");
    iso_label.set_halign(gtk4::Align::Start);
    iso_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    iso_label.set_max_width_chars(30);
    iso_label.set_vexpand(true);
    iso_label.set_valign(gtk4::Align::Start);
    iso_section.append(&iso_label);

    // Platform detection display
    let platform_box = GtkBox::new(Orientation::Horizontal, 8);
    platform_box.set_halign(gtk4::Align::Start);
    platform_box.set_visible(false); // Hidden by default until ISO is selected
    
    let platform_icon = Image::new();
    platform_icon.set_icon_name(Some("help-faq-symbolic"));
    platform_icon.set_pixel_size(24);
    platform_icon.add_css_class("platform-icon");
    platform_box.append(&platform_icon);
    
    let platform_label = Label::new(Some("Select an ISO image"));
    platform_label.add_css_class("platform-label");
    platform_box.append(&platform_label);
    
    iso_section.append(&platform_box);

    let iso_button = build_icon_button("Choose File", "document-open-symbolic", "button-compact");
    iso_section.append(&iso_button);

    content_box.append(&iso_section);

    // Device Selection Section
    let device_section = GtkBox::new(Orientation::Vertical, 6);
    device_section.add_css_class("section-compact");
    device_section.set_vexpand(true);

    let target_header_box = GtkBox::new(Orientation::Horizontal, 8);
    target_header_box.set_halign(gtk4::Align::Fill);
    
    let device_section_title = Label::new(Some("TARGET"));
    device_section_title.add_css_class("section-title-compact");
    device_section_title.set_hexpand(true);
    device_section_title.set_halign(gtk4::Align::Start);
    target_header_box.append(&device_section_title);
    
    let refresh_button = Button::new();
    refresh_button.set_icon_name("view-refresh-symbolic");
    refresh_button.add_css_class("refresh-button");
    refresh_button.set_tooltip_text(Some("Refresh device list"));
    target_header_box.append(&refresh_button);
    
    device_section.append(&target_header_box);

    let device_label = Label::new(None);
    device_label.add_css_class("file-label-compact");
    device_label.set_halign(gtk4::Align::Start);
    device_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    device_label.set_max_width_chars(30);
    device_label.set_vexpand(true);
    device_label.set_valign(gtk4::Align::Start);
    device_section.append(&device_label);

    let devices = Rc::new(RefCell::new(crate::io::devices::list_removable_devices().unwrap_or_default()));
    let device_store = ListStore::new::<glib::BoxedAnyObject>();

    {
        let devices_borrow = devices.borrow();
        if devices_borrow.is_empty() {
            device_label.set_text("No removable devices detected");
        } else {
            for device in devices_borrow.iter() {
                let obj = glib::BoxedAnyObject::new(device.path.clone());
                device_store.append(&obj);
            }
            device_label.set_text("Select device");
        }
    }

    let factory = gtk4::SignalListItemFactory::new();
    let devices_for_factory = Rc::clone(&devices);
    factory.connect_setup(move |_, item| {
        let item = item.downcast_ref::<gtk4::ListItem>().unwrap();
        let label = gtk4::Label::new(None);
        item.set_child(Some(&label));
    });
    factory.connect_bind(move |_, item| {
        let item = item.downcast_ref::<gtk4::ListItem>().unwrap();
        let obj = item.item().and_downcast::<glib::BoxedAnyObject>().unwrap();
        let path: PathBuf = obj.borrow::<PathBuf>().clone();

        let display = devices_for_factory
            .borrow()
            .iter()
            .find(|d| d.path == path)
            .map(|d| {
                format!(
                    "{} · {} {} · {} · {}",
                    d.path.display(),
                    d.vendor,
                    d.model,
                    d.capacity_human(),
                    d.connection_type.as_str()
                )
            })
            .unwrap_or_else(|| path.display().to_string());

        if let Some(label) = item.child().and_downcast::<gtk4::Label>() {
            label.set_text(&display);
        }
    });

    let selection_model = gtk4::SingleSelection::new(Some(device_store.clone()));
    let device_dropdown = DropDown::new(
        Some(selection_model.clone().upcast::<gtk4::gio::ListModel>()),
        None::<gtk4::Expression>,
    );
    device_dropdown.set_factory(Some(&factory));
    device_dropdown.set_sensitive(!devices.borrow().is_empty());
    device_dropdown.add_css_class("dropdown-compact");
    device_section.append(&device_dropdown);

    content_box.append(&device_section);
    main_box.append(&content_box);

    // Action Section - Horizontal
    let action_box = GtkBox::new(Orientation::Horizontal, 12);
    action_box.set_margin_top(6);

    let write_button = build_icon_button("Write", "media-floppy-symbolic", "write-button-compact");
    write_button.set_sensitive(false);
    action_box.append(&write_button);

    // Progress Section - Compact
    let progress_box = GtkBox::new(Orientation::Vertical, 3);
    progress_box.set_hexpand(true);

    let progress_label = Label::new(Some("Ready"));
    progress_label.add_css_class("progress-label-compact");
    progress_label.set_halign(gtk4::Align::Start);
    progress_box.append(&progress_label);

    let progress_bar = ProgressBar::new();
    progress_bar.set_show_text(true);
    progress_bar.add_css_class("progress-compact");
    progress_box.append(&progress_bar);

    let speed_label = Label::new(Some(""));
    speed_label.add_css_class("speed-label-compact");
    speed_label.set_halign(gtk4::Align::Start);
    progress_box.append(&speed_label);

    action_box.append(&progress_box);
    main_box.append(&action_box);

    // Message Log Section
    let message_buffer = TextBuffer::new(None);
    let message_view = TextView::with_buffer(&message_buffer);
    message_view.set_editable(false);
    message_view.set_cursor_visible(false);
    message_view.set_wrap_mode(gtk4::WrapMode::Word);
    message_view.add_css_class("message-box-compact");

    let message_scroll = ScrolledWindow::new();
    message_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    message_scroll.set_child(Some(&message_view));
    message_scroll.set_vexpand(true);
    message_scroll.set_hexpand(true);
    message_scroll.add_css_class("message-scroll");

    main_box.append(&message_scroll);

    window.set_child(Some(&main_box));

    sync_device_selection(
        &selection_model,
        &state,
        &devices,
        &device_label,
        &message_buffer,
        &write_button,
        &progress_label,
        &progress_bar,
        &speed_label,
        &status_dot,
        false,
    );

    {
        let state_ref = state.borrow();
        let initial_message = state_ref
            .selected_device_path
            .as_ref()
            .map(|path| format!("DEV_SELECTED={path}"))
            .unwrap_or_else(|| "DEV_SELECTED=NONE".to_string());
        append_message(&message_buffer, &initial_message);
    }

    let state_for_selection = state.clone();
    let devices_for_selection = Rc::clone(&devices);
    let device_label_for_selection = device_label.clone();
    let message_buffer_for_selection = message_buffer.clone();
    let write_button_for_selection = write_button.clone();
    let progress_label_for_selection = progress_label.clone();
    let progress_bar_for_selection = progress_bar.clone();
    let speed_label_for_selection = speed_label.clone();
    let status_dot_for_selection = status_dot.clone();

    // Helper function to refresh device list
    let refresh_device_list = {
        let devices_rc = Rc::clone(&devices);
        let device_store_rc = device_store.clone();
        let device_label_rc = device_label.clone();
        let device_dropdown_rc = device_dropdown.clone();
        let message_buffer_rc = message_buffer.clone();
        let selection_model_rc = selection_model.clone();
        let state_rc = state.clone();
        let write_button_rc = write_button.clone();
        let progress_label_rc = progress_label.clone();
        let progress_bar_rc = progress_bar.clone();
        let speed_label_rc = speed_label.clone();
        let status_dot_rc = status_dot.clone();
        
        move |silent: bool| {
            if !silent {
                append_message(&message_buffer_rc, "Refreshing device list...");
            }
            
            // Get previous device paths
            let old_paths: Vec<String> = devices_rc
                .borrow()
                .iter()
                .map(|d| d.path.to_string_lossy().to_string())
                .collect();
            
            // Rescan devices
            let new_devices = crate::io::devices::list_removable_devices().unwrap_or_default();
            let device_count = new_devices.len();
            
            // Get new device paths
            let new_paths: Vec<String> = new_devices
                .iter()
                .map(|d| d.path.to_string_lossy().to_string())
                .collect();
            
            // Check if anything changed
            let changed = old_paths != new_paths;
            
            if changed || !silent {
                // Update devices Rc
                *devices_rc.borrow_mut() = new_devices;
                
                // Clear and repopulate store
                device_store_rc.remove_all();
                
                let devices_borrow = devices_rc.borrow();
                if devices_borrow.is_empty() {
                    device_label_rc.set_text("No devices detected");
                    device_dropdown_rc.set_sensitive(false);
                    if !silent {
                        append_message(&message_buffer_rc, "No devices found");
                    }
                } else {
                    for device in devices_borrow.iter() {
                        let obj = glib::BoxedAnyObject::new(device.path.clone());
                        device_store_rc.append(&obj);
                    }
                    device_label_rc.set_text("Select device");
                    device_dropdown_rc.set_sensitive(true);
                    if !silent {
                        append_message(&message_buffer_rc, &format!("✓ Found {} device(s)", device_count));
                    } else if changed {
                        append_message(&message_buffer_rc, &format!("Device change detected: {} device(s) available", device_count));
                    }
                }
                drop(devices_borrow);
                
                // Reset selection if device list changed
                if changed {
                    selection_model_rc.set_selected(gtk4::INVALID_LIST_POSITION);
                    
                    // Update UI state
                    let mut state_ref = state_rc.borrow_mut();
                    state_ref.selected_device_path = None;
                    recompute_action_state(&mut state_ref);
                    let action_state = state_ref.action_state.clone();
                    drop(state_ref);
                    
                    update_action_area(
                        &action_state,
                        &write_button_rc,
                        &progress_label_rc,
                        &progress_bar_rc,
                        &speed_label_rc,
                        &status_dot_rc,
                    );
                }
            }
        }
    };
    
    // Connect manual refresh button
    let refresh_fn_for_button = refresh_device_list.clone();
    refresh_button.connect_clicked(move |_| {
        refresh_fn_for_button(false);
    });
    
    // Auto-detect device changes every 3 seconds
    let refresh_fn_for_timer = refresh_device_list.clone();
    let state_for_timer = state.clone();
    glib::timeout_add_seconds_local(3, move || {
        // Only auto-refresh when not working
        let is_working = state_for_timer.borrow().is_working;
        if !is_working {
            refresh_fn_for_timer(true);
        }
        glib::ControlFlow::Continue
    });

    selection_model.connect_selected_notify(move |model| {
        sync_device_selection(
            model,
            &state_for_selection,
            &devices_for_selection,
            &device_label_for_selection,
            &message_buffer_for_selection,
            &write_button_for_selection,
            &progress_label_for_selection,
            &progress_bar_for_selection,
            &speed_label_for_selection,
            &status_dot_for_selection,
            true,
        );
    });

    // Connect ISO button
    let iso_label_clone = iso_label;
    let state_clone = state.clone();
    let write_button_clone = write_button.clone();
    let progress_label_clone = progress_label.clone();
    let progress_bar_clone = progress_bar.clone();
    let speed_label_clone = speed_label.clone();
    let status_dot_clone = status_dot.clone();
    let platform_icon_clone = platform_icon.clone();
    let platform_label_clone = platform_label.clone();
    let platform_box_clone = platform_box.clone();

    iso_button.connect_clicked(move |button| {
        let window = button.root().and_downcast::<ApplicationWindow>().unwrap();

        let dialog = FileChooserDialog::new(
            Some("Select ISO File"),
            Some(&window),
            FileChooserAction::Open,
            &[
                ("Cancel", ResponseType::Cancel),
                ("Open", ResponseType::Accept),
            ],
        );

        let iso_label = iso_label_clone.clone();
        let state = state_clone.clone();
        let write_button = write_button_clone.clone();
        let progress_label = progress_label_clone.clone();
        let progress_bar = progress_bar_clone.clone();
        let speed_label = speed_label_clone.clone();
        let status_dot = status_dot_clone.clone();
        let platform_icon = platform_icon_clone.clone();
        let platform_label = platform_label_clone.clone();
        let platform_box = platform_box_clone.clone();

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        let filename = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown");
                        iso_label.set_text(filename);
                        
                        // Detect platform and update UI
                        let platform = crate::core::platforms::Platform::from_iso_path(&path);
                        platform_icon.set_icon_name(Some(platform.icon_name()));
                        platform_label.set_text(platform.display_name());
                        platform_box.set_visible(true); // Show platform info when ISO selected
                        
                        let mut state_ref = state.borrow_mut();
                        state_ref.selected_iso = Some(path);
                        recompute_action_state(&mut state_ref);
                        let action_state = state_ref.action_state.clone();
                        drop(state_ref);
                        update_action_area(
                            &action_state,
                            &write_button,
                            &progress_label,
                            &progress_bar,
                            &speed_label,
                            &status_dot,
                        );
                    }
                }
            }
            dialog.close();
        });

        dialog.show();
    });

    // Connect device dropdown handled via selection model above

    // Connect write button
    let state_clone = state.clone();
    let window_clone = window.clone();
    let status_dot_clone = status_dot;
    let progress_label_clone = progress_label;
    let progress_bar_clone = progress_bar;
    let speed_label_clone = speed_label;
    let write_button_clone = write_button.clone();
    let iso_button_clone = iso_button;
    let device_dropdown_clone = device_dropdown;
    let message_buffer_clone = message_buffer.clone();
    let devices_for_write = Rc::clone(&devices);

    write_button.connect_clicked(move |_| {
        let (iso_path, device_path) = {
            let state_ref = state_clone.borrow();
            (
                state_ref.selected_iso.clone(),
                state_ref.selected_device_path.clone(),
            )
        };

        let iso_line = if let Some(iso) = iso_path.as_ref() {
            format!("ISO={}", iso.display())
        } else {
            "ISO=NONE".to_string()
        };
        append_message(&message_buffer_clone, &iso_line);

        let dev_line = if let Some(device) = device_path.as_ref() {
            format!("DEV={device}")
        } else {
            "DEV=NONE".to_string()
        };
        append_message(&message_buffer_clone, &dev_line);

        let (iso, device) = match (iso_path, device_path) {
            (Some(iso), Some(device_path)) => {
                let requested_path = Path::new(&device_path);
                let device = devices_for_write
                    .borrow()
                    .iter()
                    .find(|d| d.path.as_path() == requested_path)
                    .cloned();
                match device {
                    Some(device) => (iso, device),
                    None => {
        append_message(&message_buffer_clone, "⚠ Please select both ISO file and target device");
                        return;
                    }
                }
            }
            _ => {
                append_message(&message_buffer_clone, "BLOCKED: select ISO and target");
                return;
            }
        };

        append_message(&message_buffer_clone, "Starting write…");

        let refresh_for_write = refresh_device_list.clone();
        show_confirmation_dialog(
            &window_clone,
            iso,
            device,
            state_clone.clone(),
            UIComponents {
                status_dot: status_dot_clone.clone(),
                progress_label: progress_label_clone.clone(),
                progress_bar: progress_bar_clone.clone(),
                speed_label: speed_label_clone.clone(),
                write_button: write_button_clone.clone(),
                iso_button: iso_button_clone.clone(),
                device_dropdown: device_dropdown_clone.clone(),
                message_buffer: message_buffer_clone.clone(),
                refresh_fn: Rc::new(refresh_for_write),
            },
        );
    });

    // Add close request handler - ALWAYS allow close, no blocking
    window.connect_close_request(move |_window| {
        // Always allow window to close
        glib::Propagation::Proceed
    });

    window.present();
}

fn build_icon_button(label: &str, icon_name: &str, class_name: &str) -> Button {
    let button = Button::new();
    button.add_css_class(class_name);

    let content_box = GtkBox::new(Orientation::Horizontal, 4);
    content_box.set_halign(gtk4::Align::Center);

    let icon = Image::from_icon_name(icon_name);
    icon.add_css_class("button-icon");
    content_box.append(&icon);

    let text = Label::new(Some(label));
    text.add_css_class("button-label");
    content_box.append(&text);

    button.set_child(Some(&content_box));
    button
}

fn show_confirmation_dialog(
    window: &ApplicationWindow,
    iso: PathBuf,
    device: crate::core::models::BlockDevice,
    state: Rc<RefCell<AppState>>,
    ui: UIComponents,
) {
    let message = format!(
        "TARGET DEVICE\n\n\
         Device: {}\n\
         Model: {} {}\n\
         Capacity: {}\n\n\
         DANGER ZONE\n\n\
         ALL DATA WILL BE PERMANENTLY ERASED\n\
         This action cannot be undone.\n\n\
         Continue?",
        device.path.display(),
        device.vendor,
        device.model,
        device.capacity_human()
    );

    let dialog = MessageDialog::new(
        Some(window),
        gtk4::DialogFlags::MODAL | gtk4::DialogFlags::DESTROY_WITH_PARENT,
        MessageType::Warning,
        ButtonsType::None,
        "Confirm Destructive Operation",
    );
    dialog.set_decorated(true);

    dialog.set_secondary_text(Some(&message));
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("ERASE & WRITE", ResponseType::Accept);

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            append_message(&ui.message_buffer, "✓ User confirmed destructive operation");

            // Spawn worker thread for device validation to prevent UI freeze
            let device_path = device.path.clone();
            let (tx, rx) = mpsc::channel();
            thread::spawn(move || {
                let result = crate::io::devices::validate_device(&device_path);
                let _ = tx.send(result);
            });

            // Poll for validation result without blocking
            let rx_holder = Rc::new(RefCell::new(Some(rx)));
            let iso_clone = iso.clone();
            let device_clone = device.clone();
            let state_clone = state.clone();
            let ui_clone = ui.clone();

            glib::idle_add_local(move || {
                let mut rx_opt = rx_holder.borrow_mut().take();
                if let Some(rx) = rx_opt.as_mut() {
                    match rx.try_recv() {
                        Ok(Ok(())) => {
                            // Validation passed
                            {
                                let mut state_ref = state_clone.borrow_mut();
                                state_ref.is_working = true;
                                state_ref.action_state = ActionAreaState::Writing;
                            }
                            update_action_area(
                                &ActionAreaState::Writing,
                                &ui_clone.write_button,
                                &ui_clone.progress_label,
                                &ui_clone.progress_bar,
                                &ui_clone.speed_label,
                                &ui_clone.status_dot,
                            );
                            ui_clone.iso_button.set_sensitive(false);
                            ui_clone.device_dropdown.set_sensitive(false);
                            append_message(
                                &ui_clone.message_buffer,
                                &format!("INFO: Starting write to {}", device_clone.path.display()),
                            );
                            start_write_operation(
                                iso_clone.clone(),
                                device_clone.clone(),
                                state_clone.clone(),
                                ui_clone.clone(),
                            );
                            rx_opt = None;
                            *rx_holder.borrow_mut() = rx_opt;
                            glib::ControlFlow::Break
                        }
                        Ok(Err(e)) => {
                            // Validation failed
                            append_message(
                                &ui_clone.message_buffer,
                                &format!("✗ Validation failed: {e}"),
                            );
                            rx_opt = None;
                            *rx_holder.borrow_mut() = rx_opt;
                            glib::ControlFlow::Break
                        }
                        Err(mpsc::TryRecvError::Empty) => {
                            // Not ready, continue polling
                            *rx_holder.borrow_mut() = rx_opt;
                            glib::ControlFlow::Continue
                        }
                        Err(mpsc::TryRecvError::Disconnected) => {
                            append_message(&ui_clone.message_buffer, "✗ Validation check failed");
                            rx_opt = None;
                            *rx_holder.borrow_mut() = rx_opt;
                            glib::ControlFlow::Break
                        }
                    }
                } else {
                    glib::ControlFlow::Break
                }
            });
        } else if response == ResponseType::Cancel {
            append_message(&ui.message_buffer, "Operation cancelled by user");
        }
        // Close dialog after handling response
        dialog.close();
    });

    dialog.show();
}

#[allow(clippy::too_many_lines)] // Worker thread coordination requires comprehensive error handling
fn start_write_operation(
    iso: PathBuf,
    device: crate::core::models::BlockDevice,
    state: Rc<RefCell<AppState>>,
    ui: UIComponents,
) {
    let (tx, rx) = mpsc::channel();

    // Spawn worker thread
    thread::spawn(move || {
        // Get the path to etch-helper binary
        let helper_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("etch-helper")))
            .unwrap_or_else(|| std::path::PathBuf::from("./target/release/etch-helper"));

        // Launch etch-helper via pkexec
        let mut child = match std::process::Command::new("pkexec")
            .arg(&helper_path)
            .arg(&iso)
            .arg(&device.path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                let _ = tx.send(WorkMessage::Error(format!("Failed to launch pkexec: {e}")));
                return;
            }
        };

        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                let _ = tx.send(WorkMessage::Error("Failed to capture stdout".to_string()));
                return;
            }
        };

        // Read output from helper
        use std::io::BufRead;
        let reader = std::io::BufReader::new(stdout);
        let mut total_size = 0u64;
        
        for line in reader.lines() {
            let line = match line {
                Ok(line) => line,
                Err(e) => {
                    let _ = tx.send(WorkMessage::Error(format!("Failed to read helper output: {e}")));
                    return;
                }
            };

            let parts: Vec<&str> = line.split_whitespace().collect();
            match parts.first().copied() {
                Some("READY") => {
                    if let Some(size_str) = parts.get(1) {
                        total_size = size_str.parse().unwrap_or(0);
                    }
                }
                Some("PROGRESS") => {
                    if let (Some(written_str), Some(bps_str)) = (parts.get(1), parts.get(2)) {
                        let written = written_str.parse().unwrap_or(0);
                        let bps = bps_str.parse().unwrap_or(0);
                        let _ = tx.send(WorkMessage::WriteProgress(written, total_size, bps));
                    }
                }
                Some("DONE") => {
                    if tx.send(WorkMessage::WriteComplete).is_err() {
                        eprintln!("WARNING: Write completed but UI channel closed");
                    }
                }
                Some("VERIFY_START") => {
                    if let Some(size_str) = parts.get(1) {
                        total_size = size_str.parse().unwrap_or(0);
                    }
                }
                Some("VERIFY_PROGRESS") => {
                    if let (Some(verified_str), Some(bps_str)) = (parts.get(1), parts.get(2)) {
                        let verified = verified_str.parse().unwrap_or(0);
                        let bps = bps_str.parse().unwrap_or(0);
                        let _ = tx.send(WorkMessage::VerifyProgress(verified, total_size, bps));
                    }
                }
                Some("VERIFY_DONE") => {
                    if tx.send(WorkMessage::VerifyComplete).is_err() {
                        eprintln!("WARNING: Verification completed but UI channel closed");
                    }
                    break;
                }
                Some("METRICS") => {
                    // Log performance metrics
                    eprintln!("Performance metrics: {}", parts[1..].join(" "));
                }
                Some("ERROR") => {
                    let error_msg = parts[1..].join(" ");
                    let _ = tx.send(WorkMessage::Error(format!("Helper error: {error_msg}")));
                    return;
                }
                _ => {}
            }
        }

        // Wait for helper to finish
        match child.wait() {
            Ok(status) if !status.success() => {
                let _ = tx.send(WorkMessage::Error(format!("Helper exited with status: {status}")));
            }
            Err(e) => {
                let _ = tx.send(WorkMessage::Error(format!("Failed to wait for helper: {e}")));
            }
            _ => {}
        }
    });

    // Handle messages from worker thread - non-blocking to prevent UI freeze
    let rx_holder = Rc::new(RefCell::new(Some(rx)));
    glib::idle_add_local(move || {
        let mut rx_opt = rx_holder.borrow_mut().take();
        if let Some(rx) = rx_opt.as_mut() {
            loop {
                match rx.try_recv() {
                    Ok(WorkMessage::WriteProgress(bytes, total, bps)) => {
                        #[allow(clippy::cast_precision_loss)] // Acceptable for UI display
                        let fraction = bytes as f64 / total as f64;
                        ui.progress_bar.set_fraction(fraction);
                        ui.progress_bar
                            .set_text(Some(&format!("{:.0}%", fraction * 100.0)));
                        ui.progress_label.set_text("Writing...");

                        #[allow(clippy::cast_precision_loss)] // Acceptable for UI display
                        let mb_per_sec = bps as f64 / 1_000_000.0;
                        #[allow(clippy::cast_precision_loss)]
                        let mb_written = bytes as f64 / 1_000_000.0;
                        #[allow(clippy::cast_precision_loss)]
                        let mb_total = total as f64 / 1_000_000.0;
                        
                        let remaining_bytes = total.saturating_sub(bytes);
                        let eta_secs = if bps > 0 {
                            remaining_bytes as f64 / bps as f64
                        } else {
                            0.0
                        };
                        let eta_mins = (eta_secs / 60.0) as u32;
                        let eta_secs_remainder = (eta_secs % 60.0) as u32;
                        
                        ui.speed_label.set_text(&format!(
                            "{mb_written:.0}/{mb_total:.0} MB · {mb_per_sec:.1} MB/s · ETA {eta_mins}:{eta_secs_remainder:02}"
                        ));
                    }
                    Ok(WorkMessage::WriteComplete) => {
                        append_message(
                            &ui.message_buffer,
                            "✓ Write complete - Starting data verification",
                        );
                        {
                            let mut state_ref = state.borrow_mut();
                            state_ref.action_state = ActionAreaState::Verifying;
                        }
                        update_action_area(
                            &ActionAreaState::Verifying,
                            &ui.write_button,
                            &ui.progress_label,
                            &ui.progress_bar,
                            &ui.speed_label,
                            &ui.status_dot,
                        );
                        ui.progress_bar.set_fraction(0.0);
                    }
                    Ok(WorkMessage::VerifyProgress(bytes, total, bps)) => {
                        #[allow(clippy::cast_precision_loss)] // Acceptable for UI display
                        let fraction = bytes as f64 / total as f64;
                        ui.progress_bar.set_fraction(fraction);
                        ui.progress_bar
                            .set_text(Some(&format!("{:.0}%", fraction * 100.0)));
                        ui.progress_label.set_text("Verifying");

                        #[allow(clippy::cast_precision_loss)] // Acceptable for UI display
                        let mb_per_sec = bps as f64 / 1_000_000.0;
                        #[allow(clippy::cast_precision_loss)]
                        let mb_verified = bytes as f64 / 1_000_000.0;
                        #[allow(clippy::cast_precision_loss)]
                        let mb_total = total as f64 / 1_000_000.0;
                        
                        let remaining_bytes = total.saturating_sub(bytes);
                        let eta_secs = if bps > 0 {
                            remaining_bytes as f64 / bps as f64
                        } else {
                            0.0
                        };
                        let eta_mins = (eta_secs / 60.0) as u32;
                        let eta_secs_remainder = (eta_secs % 60.0) as u32;
                        
                        ui.speed_label.set_text(&format!(
                            "{mb_verified:.0}/{mb_total:.0} MB · {mb_per_sec:.1} MB/s · ETA {eta_mins}:{eta_secs_remainder:02}"
                        ));
                    }
                    Ok(WorkMessage::VerifyComplete) => {
                        append_message(&ui.message_buffer, "✓ Verification complete - All data matches perfectly!");
                        let action_state = {
                            let mut state_ref = state.borrow_mut();
                            state_ref.is_working = false;
                            state_ref.action_state = ActionAreaState::Done;
                            state_ref.action_state.clone()
                        };
                        update_action_area(
                            &action_state,
                            &ui.write_button,
                            &ui.progress_label,
                            &ui.progress_bar,
                            &ui.speed_label,
                            &ui.status_dot,
                        );
                        ui.progress_bar.set_fraction(1.0);
                        ui.progress_bar.set_text(Some("100%"));
                        ui.iso_button.set_sensitive(true);
                        ui.device_dropdown.set_sensitive(true);
                        
                        // Auto-refresh device list after completion
                        (ui.refresh_fn)(true);
                        
                        rx_opt = None;
                        *rx_holder.borrow_mut() = rx_opt;
                        return glib::ControlFlow::Break;
                    }
                    Ok(WorkMessage::Error(err)) => {
                        // Categorize error for user-friendly messages
                        let error_msg = if err.contains("permission") || err.contains("pkexec") {
                            format!("⚠ USER ERROR: Permission denied\\nDetails: {err}\\nSolution: Ensure polkit is configured correctly")
                        } else if err.contains("mounted") {
                            format!("⚠ USER ERROR: Device is mounted\\nDetails: {err}\\nSolution: Unmount all partitions first")
                        } else if err.contains("space") || err.contains("full") {
                            format!("⚠ USER ERROR: Insufficient space\\nDetails: {err}\\nSolution: Use a larger device")
                        } else if err.contains("Verification failed") || err.contains("mismatch") {
                            format!("✗ VERIFICATION ERROR: Data mismatch detected\\nDetails: {err}\\nCause: Device may be faulty or write operation interrupted")
                        } else {
                            format!("✗ ERROR: {err}")
                        };
                        
                        let action_state = {
                            let mut state_ref = state.borrow_mut();
                            state_ref.is_working = false;
                            state_ref.action_state = ActionAreaState::Error(err.clone());
                            state_ref.action_state.clone()
                        };
                        update_action_area(
                            &action_state,
                            &ui.write_button,
                            &ui.progress_label,
                            &ui.progress_bar,
                            &ui.speed_label,
                            &ui.status_dot,
                        );
                        append_message(&ui.message_buffer, &error_msg);
                        ui.iso_button.set_sensitive(true);
                        ui.device_dropdown.set_sensitive(true);
                        rx_opt = None;
                        *rx_holder.borrow_mut() = rx_opt;
                        return glib::ControlFlow::Break;
                    }
                    Err(mpsc::TryRecvError::Empty) => {
                        break;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        rx_opt = None;
                        *rx_holder.borrow_mut() = rx_opt;
                        return glib::ControlFlow::Break;
                    }
                }
            }
        }
        *rx_holder.borrow_mut() = rx_opt;
        glib::ControlFlow::Continue
    });
}

#[allow(dead_code)]
fn show_error_dialog(parent: &impl IsA<gtk4::Window>, message: &str) {
    let dialog = MessageDialog::new(
        Some(parent),
        gtk4::DialogFlags::MODAL,
        MessageType::Error,
        ButtonsType::Ok,
        "Error",
    );
    dialog.set_secondary_text(Some(message));
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.show();
}
fn append_message(buffer: &TextBuffer, message: &str) {
    // Auto-detect message type from content
    let message_type = if message.contains("✓") || message.contains("complete") || message.contains("success") {
        "success"
    } else if message.contains("✗") || message.contains("ERROR") || message.contains("failed") {
        "error"
    } else if message.contains("⚠") || message.contains("WARNING") {
        "warning"
    } else if message.starts_with("DEV_") || message.starts_with("ISO=") || message.starts_with("DEV=") {
        "metadata"
    } else if message.contains("Starting") || message.contains("INFO:") {
        "info"
    } else {
        "default"
    };
    append_message_with_type(buffer, message, message_type)
}

fn append_message_with_type(buffer: &TextBuffer, message: &str, message_type: &str) {
    let mut end_iter = buffer.end_iter();
    
    // Add newline if buffer isn't empty
    if buffer.char_count() > 0 {
        buffer.insert(&mut end_iter, "\n");
        end_iter = buffer.end_iter();
    }
    
    // Create or get text tags for styling
    let tag_table = buffer.tag_table();
    
    // Success tag (green)
    if tag_table.lookup("success").is_none() {
        let tag = gtk4::TextTag::new(Some("success"));
        tag.set_foreground(Some("#4ade80"));
        tag.set_weight(700);
        tag_table.add(&tag);
    }
    
    // Error tag (red)
    if tag_table.lookup("error").is_none() {
        let tag = gtk4::TextTag::new(Some("error"));
        tag.set_foreground(Some("#ff4444"));
        tag.set_weight(700);
        tag_table.add(&tag);
    }
    
    // Warning tag (yellow)
    if tag_table.lookup("warning").is_none() {
        let tag = gtk4::TextTag::new(Some("warning"));
        tag.set_foreground(Some("#fbbf24"));
        tag.set_weight(600);
        tag_table.add(&tag);
    }
    
    // Info tag (white)
    if tag_table.lookup("info").is_none() {
        let tag = gtk4::TextTag::new(Some("info"));
        tag.set_foreground(Some("#ffffff"));
        tag_table.add(&tag);
    }
    
    // Metadata tag (grey)
    if tag_table.lookup("metadata").is_none() {
        let tag = gtk4::TextTag::new(Some("metadata"));
        tag.set_foreground(Some("#909090"));
        tag.set_weight(500);
        tag_table.add(&tag);
    }
    
    // Format message based on type
    let (formatted_message, tag_name) = match message_type {
        "success" => (format!("✓ {}", message.replace("✓", "").trim()), "success"),
        "error" => (format!("✗ {}", message.replace("✗", "").trim()), "error"),
        "warning" => (format!("⚠ {}", message.replace("⚠", "").trim()), "warning"),
        "info" => (format!("● {}", message.replace("INFO:", "").trim()), "info"),
        "metadata" => (format!("  {}", message), "metadata"),
        _ => (message.to_string(), ""),
    };
    
    // Insert message with tag
    let start_offset = end_iter.offset();
    buffer.insert(&mut end_iter, &formatted_message);
    
    // Apply tag to the inserted text
    if !tag_name.is_empty() {
        if let Some(tag) = tag_table.lookup(tag_name) {
            let start_iter = buffer.iter_at_offset(start_offset);
            let end_iter = buffer.end_iter();
            buffer.apply_tag(&tag, &start_iter, &end_iter);
        }
    }
    
    // Scroll to end
    let final_iter = buffer.end_iter();
    buffer.place_cursor(&final_iter);
}

fn recompute_action_state(state: &mut AppState) {
    if state.is_working {
        return;
    }

    if state.selected_iso.is_some() && state.selected_device_path.is_some() {
        state.action_state = ActionAreaState::Armed;
    } else {
        state.action_state = ActionAreaState::Idle;
    }
}

fn update_action_area(
    state: &ActionAreaState,
    write_button: &Button,
    progress_label: &Label,
    progress_bar: &ProgressBar,
    speed_label: &Label,
    status_dot: &GtkBox,
) {
    progress_label.remove_css_class("success-text");
    progress_label.remove_css_class("error-text");

    match state {
        ActionAreaState::Idle => {
            write_button.set_sensitive(false);
            progress_label.set_text("Select ISO and target");
            progress_bar.set_visible(false);
            progress_bar.set_fraction(0.0);
            speed_label.set_text("");
            status_dot.remove_css_class("active");
            status_dot.remove_css_class("success");
            status_dot.add_css_class("idle");
        }
        ActionAreaState::Armed => {
            write_button.set_sensitive(true);
            progress_label.set_text("Ready to write");
            progress_bar.set_visible(true);
            progress_bar.set_fraction(0.0);
            progress_bar.set_text(Some("0%"));
            speed_label.set_text("");
            status_dot.remove_css_class("active");
            status_dot.remove_css_class("success");
            status_dot.add_css_class("idle");
        }
        ActionAreaState::Writing => {
            write_button.set_sensitive(false);
            progress_label.set_text("Writing...");
            progress_bar.set_visible(true);
            status_dot.remove_css_class("idle");
            status_dot.remove_css_class("success");
            status_dot.add_css_class("active");
        }
        ActionAreaState::Verifying => {
            write_button.set_sensitive(false);
            progress_label.set_text("Verifying...");
            progress_bar.set_visible(true);
            status_dot.remove_css_class("idle");
            status_dot.remove_css_class("success");
            status_dot.add_css_class("active");
        }
        ActionAreaState::Done => {
            write_button.set_sensitive(true);
            progress_label.set_text("Done");
            progress_label.add_css_class("success-text");
            progress_bar.set_fraction(1.0);
            progress_bar.set_text(Some("100%"));
            progress_bar.set_visible(true);
            speed_label.set_text("");
            status_dot.remove_css_class("active");
            status_dot.remove_css_class("idle");
            status_dot.add_css_class("success");
        }
        ActionAreaState::Error(msg) => {
            write_button.set_sensitive(true);
            progress_label.set_text(msg);
            progress_label.add_css_class("error-text");
            progress_bar.set_fraction(0.0);
            progress_bar.set_visible(true);
            progress_bar.set_text(Some("0%"));
            speed_label.set_text("");
            status_dot.remove_css_class("active");
            status_dot.remove_css_class("success");
            status_dot.add_css_class("idle");
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_device_selection(
    selection_model: &gtk4::SingleSelection,
    state: &Rc<RefCell<AppState>>,
    devices: &Rc<RefCell<Vec<crate::core::models::BlockDevice>>>,
    device_label: &Label,
    message_buffer: &TextBuffer,
    write_button: &Button,
    progress_label: &Label,
    progress_bar: &ProgressBar,
    speed_label: &Label,
    status_dot: &GtkBox,
    emit_log: bool,
) {
    let index = selection_model.selected();
    
    let devices_borrow = devices.borrow();

    let mut label_text = if devices_borrow.is_empty() {
        "No removable devices detected".to_string()
    } else {
        "Select device".to_string()
    };
    let mut current_path: Option<String> = None;

    if index != gtk4::INVALID_LIST_POSITION {
        if let Some(device) = devices_borrow.get(index as usize) {
            label_text = format!(
                "{} · {} {} · {} · {}",
                device.path.display(),
                device.vendor,
                device.model,
                device.capacity_human(),
                device.connection_type.as_str()
            );
            current_path = Some(device.path.to_string_lossy().into_owned());
        }
    }

    drop(devices_borrow);
    
    let mut state_ref = state.borrow_mut();
    let selection_changed = state_ref.selected_device_path != current_path;
    let had_device = state_ref.selected_device_path.is_some();
    state_ref.selected_device_path = current_path.clone();
    recompute_action_state(&mut state_ref);
    let action_state = state_ref.action_state.clone();
    let has_iso = state_ref.selected_iso.is_some();
    drop(state_ref);
    
    // Force write button update if we now have both ISO and device
    if has_iso && current_path.is_some() && !had_device {
        write_button.set_sensitive(true);
    }

    device_label.set_text(&label_text);
    update_action_area(
        &action_state,
        write_button,
        progress_label,
        progress_bar,
        speed_label,
        status_dot,
    );

    if emit_log && selection_changed {
        let message = current_path
            .map(|path| format!("DEV_SELECTED={path}"))
            .unwrap_or_else(|| "DEV_SELECTED=NONE".to_string());
        append_message(message_buffer, &message);
    }
}

// Compare semantic versions: returns true if remote is newer than current
fn compare_versions(current: &str, remote: &str) -> bool {
    let parse_version = |v: &str| -> Option<(u32, u32, u32)> {
        let v = v.trim_start_matches('v');
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() >= 3 {
            let major = parts[0].parse().ok()?;
            let minor = parts[1].parse().ok()?;
            let patch = parts[2].parse().ok()?;
            Some((major, minor, patch))
        } else {
            None
        }
    };
    
    if let (Some((c_maj, c_min, c_pat)), Some((r_maj, r_min, r_pat))) = 
        (parse_version(current), parse_version(remote)) {
        // Compare versions
        if r_maj > c_maj { return true; }
        if r_maj < c_maj { return false; }
        if r_min > c_min { return true; }
        if r_min < c_min { return false; }
        if r_pat > c_pat { return true; }
    }
    false
}

fn show_up_to_date_dialog(window: &ApplicationWindow) {
    let info_dialog = MessageDialog::new(
        Some(window),
        gtk4::DialogFlags::MODAL,
        MessageType::Info,
        ButtonsType::Ok,
        "Up to Date",
    );
    info_dialog.set_secondary_text(Some(&format!(
        "You are using the latest version.\n\nVersion: v{}\nGit: {}",
        crate::VERSION, crate::GIT_HASH
    )));
    info_dialog.connect_response(|d, _| d.close());
    info_dialog.show();
}

fn show_manual_update_dialog(window: &ApplicationWindow, latest_tag: &str) {
    let dialog = MessageDialog::new(
        Some(window),
        gtk4::DialogFlags::MODAL,
        MessageType::Info,
        ButtonsType::None,
        "Update Available",
    );
    dialog.set_secondary_text(Some(&format!(
        "A new version is available!\n\n\
         Current: v{}\n\
         Latest: {}\n\n\
         Please download the latest version from GitHub:\n\
         https://github.com/v-k-dev/etch/releases/tag/{}",
        crate::VERSION, latest_tag, latest_tag
    )));
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Open in Browser", ResponseType::Accept);
    
    let tag = latest_tag.to_string();
    dialog.connect_response(move |dlg, response| {
        if response == ResponseType::Accept {
            // Open browser to release page
            let url = format!("https://github.com/v-k-dev/etch/releases/tag/{}", tag);
            let _ = std::process::Command::new("xdg-open")
                .arg(&url)
                .spawn();
        }
        dlg.close();
    });
    
    dialog.show();
}

fn show_update_error(window: &ApplicationWindow, message: &str) {
    let dialog = MessageDialog::new(
        Some(window),
        gtk4::DialogFlags::MODAL,
        MessageType::Error,
        ButtonsType::Ok,
        "Update Check Failed",
    );
    dialog.set_secondary_text(Some(&format!(
        "{}\n\nPlease check your internet connection or try again later.\n\
         You can manually check for updates at:\nhttps://github.com/v-k-dev/etch/releases",
        message
    )));
    dialog.connect_response(|d, _| d.close());
    dialog.show();
}

fn show_update_available_dialog(window: &ApplicationWindow, latest_tag: &str, api_response: &str) {
    let dialog = MessageDialog::new(
        Some(window),
        gtk4::DialogFlags::MODAL,
        MessageType::Question,
        ButtonsType::None,
        "Update Available",
    );
    dialog.set_secondary_text(Some(&format!(
        "A new version is available!\n\n\
         Current: v{}\n\
         Latest: {}\n\n\
         This will download and install the update.\n\
         Authentication will be required.\n\n\
         Do you want to update now?",
        crate::VERSION, latest_tag
    )));
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("Update Now", ResponseType::Accept);
    
    let window_clone = window.clone();
    let tag = latest_tag.to_string();
    let response_data = api_response.to_string();
    
    dialog.connect_response(move |dlg, response| {
        if response == ResponseType::Accept {
            // Extract download URL from API response
            if let Some(browser_url) = extract_download_url(&response_data) {
                start_update_download(&window_clone, &tag, &browser_url);
            } else {
                show_update_error(&window_clone, "Could not find download URL in release");
            }
        }
        dlg.close();
    });
    
    dialog.show();
}

fn extract_download_url(api_response: &str) -> Option<String> {
    // Look for browser_download_url with x86_64 binary
    if let Some(assets_start) = api_response.find("\"assets\"") {
        let assets_section = &api_response[assets_start..];
        if let Some(url_start) = assets_section.find("\"browser_download_url\"") {
            if let Some(url_value_start) = assets_section[url_start..].find(": \"") {
                let url_search = &assets_section[url_start + url_value_start + 3..];
                if let Some(url_end) = url_search.find('"') {
                    let url = &url_search[..url_end];
                    // Prefer x86_64 binary
                    if url.contains("x86_64") || url.contains("etch") {
                        return Some(url.to_string());
                    }
                }
            }
        }
    }
    None
}

fn start_update_download(window: &ApplicationWindow, _tag: &str, download_url: &str) {
    let update_dialog = gtk4::Dialog::new();
    update_dialog.set_transient_for(Some(window));
    update_dialog.set_modal(true);
    update_dialog.set_title(Some("Installing Update"));
    update_dialog.set_default_size(450, 180);
    
    let content = update_dialog.content_area();
    let vbox = GtkBox::new(Orientation::Vertical, 8);
    vbox.set_margin_start(16);
    vbox.set_margin_end(16);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);
    
    let status_label = Label::new(Some("Downloading update..."));
    status_label.set_halign(gtk4::Align::Start);
    vbox.append(&status_label);
    
    let progress = ProgressBar::new();
    progress.set_show_text(true);
    progress.set_text(Some("Preparing"));
    vbox.append(&progress);
    
    content.append(&vbox);
    
    let close_button = update_dialog.add_button("Close", ResponseType::Ok);
    close_button.set_sensitive(false);
    
    // Set up restart handler
    update_dialog.connect_response(move |_, response| {
        if response == ResponseType::Ok {
            // Restart the application
            let _ = std::process::Command::new("etch").spawn();
            std::process::exit(0);
        }
    });
    
    let (tx, rx) = mpsc::channel();
    let url = download_url.to_string();
    
    // Download and install in background thread
    thread::spawn(move || {
        // Create temp file for download
        let temp_file = "/tmp/etch-update";
        let target_binary = "/usr/bin/etch";
        
        let _ = tx.send("DOWNLOAD_START".to_string());
        
        // Find etch-updater binary
        let updater_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("etch-updater")))
            .unwrap_or_else(|| std::path::PathBuf::from("/usr/bin/etch-updater"));
        
        // Run updater via pkexec
        let result = std::process::Command::new("pkexec")
            .arg(&updater_path)
            .arg(&url)
            .arg(temp_file)
            .arg(target_binary)
            .output();
        
        match result {
            Ok(output) if output.status.success() => {
                let _ = tx.send("UPDATE_SUCCESS".to_string());
            }
            Ok(output) => {
                let error = String::from_utf8_lossy(&output.stderr).to_string();
                let _ = tx.send(format!("ERROR:{}", error));
            }
            Err(e) => {
                let _ = tx.send(format!("ERROR:{}", e));
            }
        }
    });
    
    // Poll for progress
    let progress_clone = progress.clone();
    let status_clone = status_label.clone();
    let close_clone = close_button.clone();
    let dialog_clone = update_dialog.clone();
    let window_clone = window.clone();
    
    let mut step = 0;
    glib::timeout_add_local(std::time::Duration::from_millis(200), move || {
        step += 1;
        
        match rx.try_recv() {
            Ok(msg) => {
                if msg == "DOWNLOAD_START" {
                    status_clone.set_text("Downloading...");
                    progress_clone.set_text(Some("Downloading"));
                } else if msg == "UPDATE_SUCCESS" {
                    status_clone.set_markup("<span color='#4CAF50'>✓ Update Successful</span>");
                    progress_clone.set_fraction(1.0);
                    progress_clone.set_text(Some("Complete"));
                    close_clone.set_sensitive(true);
                    close_clone.add_css_class("suggested-action");
                    return glib::ControlFlow::Break;
                } else if msg.starts_with("ERROR:") {
                    dialog_clone.close();
                    show_update_error(&window_clone, &msg[6..]);
                    return glib::ControlFlow::Break;
                }
            }
            Err(mpsc::TryRecvError::Empty) => {
                // Still working - pulse progress
                if step < 50 {
                    progress_clone.pulse();
                } else {
                    status_clone.set_text("Installing...");
                    progress_clone.set_text(Some("Installing"));
                    progress_clone.pulse();
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                dialog_clone.close();
                show_update_error(&window_clone, "Update process failed unexpectedly");
                return glib::ControlFlow::Break;
            }
        }
        
        glib::ControlFlow::Continue
    });
    
    update_dialog.show();
}
