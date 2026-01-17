use gtk4::prelude::*;
use gtk4::{
    glib, Application, ApplicationWindow, Box as GtkBox, Button, ButtonsType, DropDown,
    FileChooserAction, FileChooserDialog, Image, Label, MessageDialog, MessageType, Orientation,
    ProgressBar, ResponseType, StringList,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;

#[derive(Clone)]
struct AppState {
    selected_iso: Option<PathBuf>,
    selected_device: Option<crate::core::models::BlockDevice>,
    is_working: bool,
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
}

#[derive(Debug, Clone)]
enum WorkMessage {
    WriteProgress(u64, u64, u64),  // bytes, total, bps
    VerifyProgress(u64, u64, u64), // bytes, total, bps
    WriteComplete,
    VerifyComplete,
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

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Etch")
        .default_width(760)
        .default_height(480)
        .resizable(false)
        .build();

    let state = Rc::new(RefCell::new(AppState {
        selected_iso: None,
        selected_device: None,
        is_working: false,
    }));

    let main_box = GtkBox::new(Orientation::Vertical, 0);
    main_box.add_css_class("main-container");
    main_box.set_margin_top(24);
    main_box.set_margin_bottom(24);
    main_box.set_margin_start(24);
    main_box.set_margin_end(24);

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

    main_box.append(&title_box);

    // Warning - Compact
    let warning = Label::new(Some("All data on the target will be permanently erased"));
    warning.add_css_class("warning-compact");
    warning.set_halign(gtk4::Align::Start);
    main_box.append(&warning);

    // Main Content - Horizontal Layout
    let content_box = GtkBox::new(Orientation::Horizontal, 32);
    content_box.set_homogeneous(true);
    content_box.set_hexpand(true);

    // ISO Selection Section
    let iso_section = GtkBox::new(Orientation::Vertical, 8);
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
    let device_section = GtkBox::new(Orientation::Vertical, 8);
    device_section.add_css_class("section-compact");
    device_section.set_vexpand(true);

    let device_section_title = Label::new(Some("TARGET"));
    device_section_title.add_css_class("section-title-compact");
    device_section_title.set_halign(gtk4::Align::Start);
    device_section.append(&device_section_title);

    // Get list of removable devices
    let devices = crate::io::devices::list_removable_devices().unwrap_or_default();
    let device_strings = StringList::new(&[]);

    if devices.is_empty() {
        device_strings.append("No removable devices detected");
    } else {
        for device in &devices {
            let display = format!(
                "{} · {} {} · {}",
                device.path.display(),
                device.vendor,
                device.model,
                device.capacity_human()
            );
            device_strings.append(&display);
        }
    }

    let device_dropdown = DropDown::new(Some(device_strings), None::<gtk4::Expression>);
    device_dropdown.set_sensitive(!devices.is_empty());
    device_dropdown.add_css_class("dropdown-compact");
    device_section.append(&device_dropdown);

    content_box.append(&device_section);
    main_box.append(&content_box);

    // Action Section - Horizontal
    let action_box = GtkBox::new(Orientation::Horizontal, 12);
    action_box.set_margin_top(8);

    let write_button = build_icon_button("Write", "media-floppy-symbolic", "write-button-compact");
    write_button.set_sensitive(false);
    write_button.set_size_request(120, -1);
    action_box.append(&write_button);

    // Progress Section - Compact
    let progress_box = GtkBox::new(Orientation::Vertical, 4);
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

    window.set_child(Some(&main_box));

    // Connect ISO button
    let iso_label_clone = iso_label;
    let state_clone = state.clone();
    let write_button_clone = write_button.clone();
    let devices_clone = devices.clone();
    let device_dropdown_clone = device_dropdown.clone();

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
        let devices = devices_clone.clone();
        let device_dropdown = device_dropdown_clone.clone();

        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file() {
                    if let Some(path) = file.path() {
                        let filename = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown");
                        iso_label.set_text(filename);
                        state.borrow_mut().selected_iso = Some(path);

                        // Enable write button if device also selected
                        let device_selected = !devices.is_empty()
                            && device_dropdown.selected() != gtk4::INVALID_LIST_POSITION;
                        write_button.set_sensitive(device_selected && !state.borrow().is_working);
                    }
                }
            }
            dialog.close();
        });

        dialog.show();
    });

    // Connect device dropdown
    let state_clone = state.clone();
    let write_button_clone = write_button.clone();
    let devices_clone = devices;

    device_dropdown.connect_selected_notify(move |dropdown| {
        let selected = dropdown.selected();
        if selected != gtk4::INVALID_LIST_POSITION && (selected as usize) < devices_clone.len() {
            state_clone.borrow_mut().selected_device =
                Some(devices_clone[selected as usize].clone());

            // Enable write button if ISO also selected
            let state_ref = state_clone.borrow();
            let iso_selected = state_ref.selected_iso.is_some();
            write_button_clone.set_sensitive(iso_selected && !state_ref.is_working);
        }
    });

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

    write_button.connect_clicked(move |_| {
        let state = state_clone.borrow();
        if let (Some(iso), Some(device)) = (&state.selected_iso, &state.selected_device) {
            let iso = iso.clone();
            let device = device.clone();
            drop(state);

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
                },
            );
        }
    });

    window.present();
}

fn build_icon_button(label: &str, icon_name: &str, class_name: &str) -> Button {
    let button = Button::new();
    button.add_css_class(class_name);

    let content_box = GtkBox::new(Orientation::Horizontal, 8);
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
            // Validate device before starting
            if let Err(e) = crate::io::devices::validate_device(&device.path) {
                show_error_dialog(dialog, &format!("Cannot write to device:\n\n{e}"));
                dialog.close();
                return;
            }

            state.borrow_mut().is_working = true;
            ui.write_button.set_sensitive(false);
            ui.iso_button.set_sensitive(false);
            ui.device_dropdown.set_sensitive(false);
            
            // Activate status dot
            ui.status_dot.remove_css_class("idle");
            ui.status_dot.add_css_class("active");

            start_write_operation(iso.clone(), device.clone(), state.clone(), ui.clone());
        }
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
        // Write phase
        let tx_clone = tx.clone();
        let write_result =
            crate::io::writer::write_iso(&iso, &device.path, move |bytes, total, bps| {
                // Channel send errors are not critical during progress updates
                // If channel is closed, UI thread has terminated
                let _ = tx_clone.send(WorkMessage::WriteProgress(bytes, total, bps));
            });

        if let Err(e) = write_result {
            // Error notification is critical - if this fails, log to stderr
            if tx
                .send(WorkMessage::Error(format!("Write failed: {e}")))
                .is_err()
            {
                eprintln!("CRITICAL: Write failed but UI channel closed: {e}");
            }
            return;
        }

        if tx.send(WorkMessage::WriteComplete).is_err() {
            eprintln!("WARNING: Write completed but UI channel closed");
            return;
        }

        // Verification phase
        let tx_clone = tx.clone();
        let verify_result = crate::core::verification::verify_write(
            &iso,
            &device.path,
            move |bytes, total, bps| {
                let _ = tx_clone.send(WorkMessage::VerifyProgress(bytes, total, bps));
            },
        );

        if let Err(e) = verify_result {
            if tx
                .send(WorkMessage::Error(format!("Verification failed: {e}")))
                .is_err()
            {
                eprintln!("CRITICAL: Verification failed but UI channel closed: {e}");
            }
            return;
        }

        if tx.send(WorkMessage::VerifyComplete).is_err() {
            eprintln!("WARNING: Verification completed but UI channel closed");
        }
    });

    // Handle messages from worker thread
    glib::spawn_future_local(async move {
        loop {
            match rx.recv() {
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
                    ui.progress_label.set_text("Verifying");
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
                    ui.progress_bar.set_fraction(1.0);
                    ui.progress_bar.set_text(Some("100%"));
                    ui.progress_label.set_text("Complete");
                    ui.progress_label.add_css_class("success-text");
                    ui.speed_label.set_text("");
                    
                    // Success status dot
                    ui.status_dot.remove_css_class("active");
                    ui.status_dot.add_css_class("success");

                    state.borrow_mut().is_working = false;
                    ui.write_button.set_sensitive(true);
                    ui.iso_button.set_sensitive(true);
                    ui.device_dropdown.set_sensitive(true);
                    break;
                }
                Ok(WorkMessage::Error(err)) => {
                    ui.progress_label.set_text(&format!("Error: {err}"));
                    ui.progress_label.add_css_class("error-text");
                    ui.progress_bar.set_fraction(0.0);
                    ui.speed_label.set_text("");
                    
                    // Error status - back to idle
                    ui.status_dot.remove_css_class("active");
                    ui.status_dot.add_css_class("idle");

                    state.borrow_mut().is_working = false;
                    ui.write_button.set_sensitive(true);
                    ui.iso_button.set_sensitive(true);
                    ui.device_dropdown.set_sensitive(true);
                    break;
                }
                Err(_) => {
                    // Channel closed, worker thread finished
                    break;
                }
            }
        }
    });
}

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
