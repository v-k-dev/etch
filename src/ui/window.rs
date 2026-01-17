use gtk4::gio::ListStore;
use gtk4::prelude::*;
use gtk4::{
    glib, Application, ApplicationWindow, Box as GtkBox, Button, ButtonsType, DropDown,
    FileChooserAction, FileChooserDialog, Image, Label, MessageDialog, MessageType, Orientation,
    PolicyType, ProgressBar, ResponseType, ScrolledWindow, TextBuffer, TextView,
};
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

    // Menu button
    let menu_button = Button::new();
    menu_button.add_css_class("menu-button");
    menu_button.set_icon_name("open-menu-symbolic");
    menu_button.set_valign(gtk4::Align::Center);
    
    let window_for_menu = window.clone();
    menu_button.connect_clicked(move |_| {
        let dialog = MessageDialog::new(
            Some(&window_for_menu),
            gtk4::DialogFlags::MODAL,
            MessageType::Info,
            ButtonsType::Ok,
            "Etch v0.1.0",
        );
        dialog.set_secondary_text(Some(
            "A minimal ISO to USB writer with verification.\n\n\
             Features:\n\
             • Direct block device writing via pkexec (polkit)\n\
             • Automatic byte-by-byte verification after write\n\
             • Removable device detection and validation\n\
             • Real-time progress tracking with speed metrics\n\
             • Nothing OS inspired minimalist design\n\n\
             Technology Stack:\n\
             • Rust for safety and performance\n\
             • GTK4 for modern native UI\n\
             • Separate privileged helper binary (etch-helper)\n\n\
             GitHub: github.com/yourusername/etch\n\
             License: MIT"
        ));
        dialog.connect_response(|dialog, _| dialog.close());
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

    let iso_button = build_icon_button("Choose File", "document-open-symbolic", "button-compact");
    iso_section.append(&iso_button);

    content_box.append(&iso_section);

    // Device Selection Section
    let device_section = GtkBox::new(Orientation::Vertical, 6);
    device_section.add_css_class("section-compact");
    device_section.set_vexpand(true);

    let device_section_title = Label::new(Some("TARGET"));
    device_section_title.add_css_class("section-title-compact");
    device_section_title.set_halign(gtk4::Align::Start);
    device_section.append(&device_section_title);

    let device_label = Label::new(None);
    device_label.add_css_class("file-label-compact");
    device_label.set_halign(gtk4::Align::Start);
    device_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    device_label.set_max_width_chars(30);
    device_label.set_vexpand(true);
    device_label.set_valign(gtk4::Align::Start);
    device_section.append(&device_label);

    let devices = Rc::new(crate::io::devices::list_removable_devices().unwrap_or_default());
    let device_store = ListStore::new::<glib::BoxedAnyObject>();

    if devices.is_empty() {
        device_label.set_text("No removable devices detected");
    } else {
        for device in devices.iter() {
            let obj = glib::BoxedAnyObject::new(device.path.clone());
            device_store.append(&obj);
        }
        device_label.set_text("Select device");
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
            .iter()
            .find(|d| d.path == path)
            .map(|d| {
                format!(
                    "{} · {} {} · {}",
                    d.path.display(),
                    d.vendor,
                    d.model,
                    d.capacity_human()
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
    device_dropdown.set_sensitive(!devices.is_empty());
    device_dropdown.add_css_class("dropdown-compact");
    device_section.append(&device_dropdown);

    content_box.append(&device_section);
    main_box.append(&content_box);

    // Action Section - Horizontal
    let action_box = GtkBox::new(Orientation::Horizontal, 12);
    action_box.set_margin_top(6);

    let write_button = build_icon_button("Write", "media-floppy-symbolic", "write-button-compact");
    write_button.set_sensitive(false);
    write_button.set_size_request(75, 26);
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

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        let filename = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown");
                        iso_label.set_text(filename);
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
    let state_clone = state;
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
                    .iter()
                    .find(|d| d.path.as_path() == requested_path);
                match device {
                    Some(device) => (iso, device.clone()),
                    None => {
                        append_message(&message_buffer_clone, "BLOCKED: select ISO and target");
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
            },
        );
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
        gtk4::DialogFlags::MODAL,
        MessageType::Warning,
        ButtonsType::None,
        "Confirm Destructive Operation",
    );

    dialog.set_secondary_text(Some(&message));
    dialog.add_button("Cancel", ResponseType::Cancel);
    dialog.add_button("ERASE & WRITE", ResponseType::Accept);

    dialog.connect_response(move |dialog, response| {
        if response == ResponseType::Accept {
            append_message(&ui.message_buffer, "User confirmed operation");
            dialog.close();

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
        }
        if response == ResponseType::Cancel {
            append_message(&ui.message_buffer, "Operation cancelled by user");
        }
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
                return;
            }
            Err(e) => {
                let _ = tx.send(WorkMessage::Error(format!("Failed to wait for helper: {e}")));
                return;
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
                        ui.speed_label.set_text(&format!(
                            "{mb_written:.0}/{mb_total:.0} MB · {mb_per_sec:.1} MB/s"
                        ));
                    }
                    Ok(WorkMessage::WriteComplete) => {
                        append_message(
                            &ui.message_buffer,
                            "Write phase complete, starting verification",
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
                        ui.speed_label.set_text(&format!(
                            "{mb_verified:.0}/{mb_total:.0} MB · {mb_per_sec:.1} MB/s"
                        ));
                    }
                    Ok(WorkMessage::VerifyComplete) => {
                        append_message(&ui.message_buffer, "Verification complete");
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
                        rx_opt = None;
                        *rx_holder.borrow_mut() = rx_opt;
                        return glib::ControlFlow::Break;
                    }
                    Ok(WorkMessage::Error(err)) => {
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
                        append_message(&ui.message_buffer, &format!("✗ {err}"));
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
    let mut end_iter = buffer.end_iter();
    if buffer.char_count() > 0 {
        buffer.insert(&mut end_iter, "\n");
        end_iter = buffer.end_iter();
    }
    buffer.insert(&mut end_iter, message);
    let final_iter = buffer.end_iter();
    buffer.place_cursor(&final_iter);

    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/etch-ui.log")
    {
        use std::io::Write;
        let _ = writeln!(file, "{message}");
    }
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
    devices: &Rc<Vec<crate::core::models::BlockDevice>>,
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

    let mut label_text = if devices.is_empty() {
        "No removable devices detected".to_string()
    } else {
        "Select device".to_string()
    };
    let mut current_path: Option<String> = None;

    if index != gtk4::INVALID_LIST_POSITION {
        if let Some(device) = devices.get(index as usize) {
            label_text = format!(
                "{} · {} {} · {}",
                device.path.display(),
                device.vendor,
                device.model,
                device.capacity_human()
            );
            current_path = Some(device.path.to_string_lossy().into_owned());
        }
    }

    let mut state_ref = state.borrow_mut();
    let selection_changed = state_ref.selected_device_path != current_path;
    state_ref.selected_device_path = current_path.clone();
    recompute_action_state(&mut state_ref);
    let action_state = state_ref.action_state.clone();
    drop(state_ref);

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
