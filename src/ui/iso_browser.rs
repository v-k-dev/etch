use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Box as GtkBox, Button, Image, Label, Orientation, PolicyType, ScrolledWindow, ProgressBar, MessageDialog, ButtonsType, MessageType};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crate::download::{DistrosCatalog, ISOFetcher};
use crate::download::catalog::DistroCategory;

use super::window::{AppState, recompute_action_state_export, update_action_area_export};
use super::download_progress::DownloadProgressWindow;

/// Show an error dialog with a user-friendly message
fn show_error_dialog(parent: &ApplicationWindow, title: &str, message: &str) {
    let dialog = MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(MessageType::Error)
        .buttons(ButtonsType::Ok)
        .text(title)
        .secondary_text(message)
        .build();
    
    dialog.connect_response(|dialog, _| {
        dialog.close();
    });
    
    dialog.present();
}

/// Get embedded SVG icon data for a distro
fn get_distro_icon_svg(distro_id: &str) -> Option<&'static [u8]> {
    let id_lower = distro_id.to_lowercase();
    
    // Ubuntu family
    if id_lower.contains("ubuntu") && !id_lower.contains("kubuntu") && !id_lower.contains("xubuntu") && !id_lower.contains("lubuntu") && !id_lower.contains("mate") {
        Some(include_bytes!("icons/ubuntu.svg"))
    } else if id_lower.contains("kubuntu") {
        Some(include_bytes!("icons/kubuntu.svg"))
    } else if id_lower.contains("xubuntu") {
        Some(include_bytes!("icons/xubuntu.svg"))
    } else if id_lower.contains("lubuntu") {
        Some(include_bytes!("icons/lubuntu.svg"))
    } else if id_lower.contains("mate") {
        Some(include_bytes!("icons/ubuntumate.svg"))
    } else if id_lower.contains("pop") {
        Some(include_bytes!("icons/popos.svg"))
    } else if id_lower.contains("zorin") {
        Some(include_bytes!("icons/zorin.svg"))
    } else if id_lower.contains("elementary") {
        Some(include_bytes!("icons/elementary.svg"))
    } else if id_lower.contains("bodhi") {
        Some(include_bytes!("icons/ubuntu.svg")) // fallback
    } else if id_lower.contains("trisquel") {
        Some(include_bytes!("icons/ubuntu.svg")) // fallback
    
    // Fedora/RHEL family  
    } else if id_lower.contains("fedora") {
        Some(include_bytes!("icons/fedora.svg"))
    } else if id_lower.contains("centos") {
        Some(include_bytes!("icons/centos.svg"))
    } else if id_lower.contains("rocky") {
        Some(include_bytes!("icons/rockylinux.svg"))
    } else if id_lower.contains("alma") {
        Some(include_bytes!("icons/almalinux.svg"))
    } else if id_lower.contains("rhel") {
        Some(include_bytes!("icons/fedora.svg")) // fallback
    } else if id_lower.contains("oracle-linux") || id_lower.contains("oracle") {
        Some(include_bytes!("icons/fedora.svg")) // fallback
    } else if id_lower.contains("eurolinux") {
        Some(include_bytes!("icons/fedora.svg")) // fallback
    } else if id_lower.contains("scientific-linux") {
        Some(include_bytes!("icons/fedora.svg")) // fallback
    } else if id_lower.contains("nst-") {
        Some(include_bytes!("icons/fedora.svg")) // fallback
    
    // Debian family
    } else if id_lower.contains("debian") {
        Some(include_bytes!("icons/debian.svg"))
    } else if id_lower.contains("mx") {
        Some(include_bytes!("icons/mxlinux.svg"))
    } else if id_lower.contains("antix") {
        Some(include_bytes!("icons/debian.svg")) // fallback
    } else if id_lower.contains("devuan") {
        Some(include_bytes!("icons/debian.svg")) // fallback
    } else if id_lower.contains("sparky") {
        Some(include_bytes!("icons/debian.svg")) // fallback
    } else if id_lower.contains("peppermint") {
        Some(include_bytes!("icons/debian.svg")) // fallback
    } else if id_lower.contains("parrot") {
        Some(include_bytes!("icons/parrotsecurity.svg"))
    } else if id_lower.contains("vanilla") {
        Some(include_bytes!("icons/debian.svg")) // fallback
    } else if id_lower.contains("bugtraq") {
        Some(include_bytes!("icons/debian.svg")) // fallback
    } else if id_lower.contains("backbox") {
        Some(include_bytes!("icons/ubuntu.svg")) // Ubuntu-based
    } else if id_lower.contains("caine") {
        Some(include_bytes!("icons/ubuntu.svg")) // Ubuntu-based
    } else if id_lower.contains("remnux") {
        Some(include_bytes!("icons/ubuntu.svg")) // Ubuntu-based
    } else if id_lower.contains("deft") {
        Some(include_bytes!("icons/ubuntu.svg")) // Ubuntu-based
    
    // Arch family
    } else if id_lower.contains("arch") && !id_lower.contains("black") {
        Some(include_bytes!("icons/archlinux.svg"))
    } else if id_lower.contains("manjaro") {
        Some(include_bytes!("icons/manjaro.svg"))
    } else if id_lower.contains("endeavour") {
        Some(include_bytes!("icons/endeavouros.svg"))
    } else if id_lower.contains("garuda") {
        Some(include_bytes!("icons/garudalinux.svg"))
    } else if id_lower.contains("artix") {
        Some(include_bytes!("icons/artixlinux.svg"))
    } else if id_lower.contains("cachy") {
        Some(include_bytes!("icons/archlinux.svg")) // fallback
    } else if id_lower.contains("arco") {
        Some(include_bytes!("icons/archlinux.svg")) // fallback
    } else if id_lower.contains("xero") {
        Some(include_bytes!("icons/archlinux.svg")) // fallback
    } else if id_lower.contains("archcraft") {
        Some(include_bytes!("icons/archlinux.svg")) // fallback
    } else if id_lower.contains("crystal") {
        Some(include_bytes!("icons/crystal.svg"))
    } else if id_lower.contains("reborn") {
        Some(include_bytes!("icons/archlinux.svg")) // fallback
    } else if id_lower.contains("biglinux") {
        Some(include_bytes!("icons/archlinux.svg")) // fallback
    } else if id_lower.contains("parabola") {
        Some(include_bytes!("icons/archlinux.svg")) // fallback
    } else if id_lower.contains("blend") {
        Some(include_bytes!("icons/archlinux.svg")) // fallback
    } else if id_lower.contains("blackarch") {
        Some(include_bytes!("icons/archlinux.svg")) // fallback
    } else if id_lower.contains("athena") {
        Some(include_bytes!("icons/archlinux.svg")) // fallback
    } else if id_lower.contains("archstrike") {
        Some(include_bytes!("icons/archlinux.svg")) // fallback
    
    // Mint
    } else if id_lower.contains("mint") {
        Some(include_bytes!("icons/linuxmint.svg"))
    
    // SUSE family
    } else if id_lower.contains("opensuse") || id_lower.contains("suse") {
        Some(include_bytes!("icons/opensuse.svg"))
    
    // Gentoo family
    } else if id_lower.contains("gentoo") {
        Some(include_bytes!("icons/gentoo.svg"))
    } else if id_lower.contains("pentoo") {
        Some(include_bytes!("icons/gentoo.svg")) // fallback
    
    // Security/Hacking/Forensics
    } else if id_lower.contains("kali") {
        Some(include_bytes!("icons/kalilinux.svg"))
    } else if id_lower.contains("tails") {
        Some(include_bytes!("icons/tails.svg"))
    } else if id_lower.contains("qubes") {
        Some(include_bytes!("icons/qubesos.svg"))
    } else if id_lower.contains("security-onion") {
        Some(include_bytes!("icons/kalilinux.svg")) // fallback
    } else if id_lower.contains("dracos") {
        Some(include_bytes!("icons/kalilinux.svg")) // fallback
    } else if id_lower.contains("wifislax") {
        Some(include_bytes!("icons/kalilinux.svg")) // fallback
    } else if id_lower.contains("samurai") {
        Some(include_bytes!("icons/kalilinux.svg")) // fallback
    
    // Raspberry Pi & Embedded
    } else if id_lower.contains("raspios") || id_lower.contains("raspberry") {
        Some(include_bytes!("icons/raspberrypi.svg"))
    } else if id_lower.contains("dietpi") {
        Some(include_bytes!("icons/raspberrypi.svg")) // fallback
    } else if id_lower.contains("libreelec") {
        Some(include_bytes!("icons/raspberrypi.svg")) // fallback
    } else if id_lower.contains("retropie") {
        Some(include_bytes!("icons/raspberrypi.svg")) // fallback
    } else if id_lower.contains("recalbox") {
        Some(include_bytes!("icons/raspberrypi.svg")) // fallback
    } else if id_lower.contains("batocera") {
        Some(include_bytes!("icons/raspberrypi.svg")) // fallback
    } else if id_lower.contains("volumio") {
        Some(include_bytes!("icons/raspberrypi.svg")) // fallback
    } else if id_lower.contains("home-assistant") {
        Some(include_bytes!("icons/raspberrypi.svg")) // fallback
    
    // Android
    } else if id_lower.contains("android") {
        Some(include_bytes!("icons/android.svg"))
    } else if id_lower.contains("bliss") {
        Some(include_bytes!("icons/android.svg")) // fallback
    } else if id_lower.contains("prime") && id_lower.contains("os") {
        Some(include_bytes!("icons/android.svg")) // fallback
    
    // Gaming
    } else if id_lower.contains("bazzite") {
        Some(include_bytes!("icons/fedora.svg")) // Fedora immutable
    } else if id_lower.contains("nobara") {
        Some(include_bytes!("icons/fedora.svg")) // Fedora gaming
    } else if id_lower.contains("chimera") {
        Some(include_bytes!("icons/archlinux.svg")) // Arch gaming
    } else if id_lower.contains("steam") && id_lower.contains("os") {
        Some(include_bytes!("icons/archlinux.svg")) // Arch-based
    
    // Independent
    } else if id_lower.contains("popos") || id_lower.contains("pop_os") || id_lower.contains("pop!") {
        Some(include_bytes!("icons/popos.svg"))
    } else if id_lower.contains("elementary") {
        Some(include_bytes!("icons/elementary.svg"))
    } else if id_lower.contains("zorin") {
        Some(include_bytes!("icons/zorin.svg"))
    } else if id_lower.contains("linux-lite") || id_lower.contains("linux lite") {
        Some(include_bytes!("icons/ubuntu.svg")) // Ubuntu-based
    } else if id_lower.contains("nixos") {
        Some(include_bytes!("icons/nixos.svg"))
    } else if id_lower.contains("alpine") {
        Some(include_bytes!("icons/alpinelinux.svg"))
    } else if id_lower.contains("void") {
        Some(include_bytes!("icons/voidlinux.svg"))
    } else if id_lower.contains("solus") {
        Some(include_bytes!("icons/solus.svg"))
    } else if id_lower.contains("clear") {
        Some(include_bytes!("icons/intel.svg")) // Clear Linux is by Intel
    } else if id_lower.contains("slackware") {
        Some(include_bytes!("icons/slackware.svg"))
    } else if id_lower.contains("mageia") {
        Some(include_bytes!("icons/linux.svg")) // fallback
    } else if id_lower.contains("pclinux") {
        Some(include_bytes!("icons/linux.svg")) // fallback
    } else if id_lower.contains("tiny") || id_lower.contains("tinycore") {
        Some(include_bytes!("icons/linux.svg")) // fallback
    } else if id_lower.contains("armbian") {
        Some(include_bytes!("icons/raspberrypi.svg")) // ARM SBC
    } else if id_lower.contains("openwrt") {
        Some(include_bytes!("icons/linux.svg")) // Router OS
    } else if id_lower.contains("coreos") {
        Some(include_bytes!("icons/fedora.svg")) // Fedora-based
    } else if id_lower.contains("photon") {
        Some(include_bytes!("icons/linux.svg")) // VMware minimal
    } else if id_lower.contains("proxmox") {
        Some(include_bytes!("icons/debian.svg")) // Debian-based
    } else if id_lower.contains("truenas") {
        Some(include_bytes!("icons/freebsd.svg")) // FreeBSD-based
    } else if id_lower.contains("opnsense") || id_lower.contains("pfsense") {
        Some(include_bytes!("icons/freebsd.svg")) // FreeBSD-based
    } else if id_lower.contains("centos") {
        Some(include_bytes!("icons/centos.svg"))
    } else if id_lower.contains("vanilla") {
        Some(include_bytes!("icons/linux.svg")) // Immutable
    } else if id_lower.contains("microos") {
        Some(include_bytes!("icons/opensuse.svg")) // openSUSE-based
    
    // BSD
    } else if id_lower.contains("freebsd") {
        Some(include_bytes!("icons/freebsd.svg"))
    } else if id_lower.contains("openbsd") {
        Some(include_bytes!("icons/openbsd.svg"))
    
    // Exotic/Alternative
    } else if id_lower.contains("haiku") {
        Some(include_bytes!("icons/linux.svg")) // fallback
    } else if id_lower.contains("redox") {
        Some(include_bytes!("icons/redox.svg"))
    } else if id_lower.contains("serenity") {
        Some(include_bytes!("icons/linux.svg")) // fallback
    } else if id_lower.contains("react") && id_lower.contains("os") {
        Some(include_bytes!("icons/reactos.svg"))
    } else if id_lower.contains("freedos") {
        Some(include_bytes!("icons/linux.svg")) // fallback
    } else if id_lower.contains("guix") {
        Some(include_bytes!("icons/gnu.svg"))
    } else if id_lower.contains("hyperbola") {
        Some(include_bytes!("icons/gnu.svg"))
    } else if id_lower.contains("temple") {
        Some(include_bytes!("icons/linux.svg")) // fallback
    } else if id_lower.contains("kolibri") {
        Some(include_bytes!("icons/linux.svg")) // fallback
    } else if id_lower.contains("menuet") {
        Some(include_bytes!("icons/linux.svg")) // fallback
    
    } else {
        None
    }
}

pub fn show_iso_browser_window(
    parent: &ApplicationWindow,
    iso_label: Label,
    state: Rc<RefCell<AppState>>,
    platform_box: GtkBox,
    platform_icon: Image,
    platform_label: Label,
    write_button: Button,
    progress_label: Label,
    progress_bar: ProgressBar,
    speed_label: Label,
    status_dot: GtkBox,
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
    titlebar.set_margin_top(10);
    titlebar.set_margin_bottom(8);
    titlebar.set_margin_start(12);
    titlebar.set_margin_end(12);

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

    // Search bar and filters container
    let search_box = GtkBox::new(Orientation::Vertical, 8);
    search_box.set_margin_start(16);
    search_box.set_margin_end(16);
    search_box.set_margin_bottom(8);

    let search_entry = gtk4::SearchEntry::builder()
        .placeholder_text("Search distributions...")
        .hexpand(true)
        .build();
    search_entry.add_css_class("search-entry");
    search_box.append(&search_entry);

    // Filter system
    let filter_scroll = ScrolledWindow::new();
    filter_scroll.set_policy(PolicyType::Never, PolicyType::Never);
    filter_scroll.set_hexpand(true);
    filter_scroll.set_margin_top(0);
    filter_scroll.set_margin_bottom(6);
    
    let filter_box = GtkBox::new(Orientation::Horizontal, 5);
    filter_box.set_halign(gtk4::Align::Start);

    let categories = vec![
        ("All", None),
        ("Popular", Some(DistroCategory::Other)),
        ("Gaming", Some(DistroCategory::Gaming)),
        ("Ubuntu", Some(DistroCategory::Ubuntu)),
        ("Arch", Some(DistroCategory::Arch)),
        ("Mint", Some(DistroCategory::Mint)),
        ("Debian", Some(DistroCategory::Debian)),
        ("Fedora", Some(DistroCategory::Fedora)),
        ("Raspberry Pi", Some(DistroCategory::Raspberry)),
        ("Security", Some(DistroCategory::Debian)),
    ];

    let active_category: Rc<RefCell<Option<DistroCategory>>> = Rc::new(RefCell::new(None));
    let filter_buttons: Rc<RefCell<Vec<Button>>> = Rc::new(RefCell::new(Vec::new()));

    filter_scroll.set_child(Some(&filter_box));
    search_box.append(&filter_scroll);
    main_box.append(&search_box);

    // Content
    let scroll = ScrolledWindow::new();
    scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
    scroll.set_vexpand(true);
    scroll.set_hexpand(true);
    scroll.set_margin_start(8);
    scroll.set_margin_end(8);
    scroll.set_margin_bottom(8);

    let distros_box = GtkBox::new(Orientation::Vertical, 2);
    distros_box.set_hexpand(true);

    // Track all download buttons to disable during download
    let download_buttons: Rc<RefCell<Vec<Button>>> = Rc::new(RefCell::new(Vec::new()));

    // Load and display distros function
    let dialog_for_closure = dialog.clone();
    let load_distros = {
        let distros_box = distros_box.clone();
        let download_buttons = download_buttons.clone();
        let iso_label = iso_label.clone();
        let state = state.clone();
        let platform_box = platform_box.clone();
        let platform_icon = platform_icon.clone();
        let platform_label = platform_label.clone();
        let write_button = write_button.clone();
        let progress_label = progress_label.clone();
        let progress_bar = progress_bar.clone();
        let speed_label = speed_label.clone();
        let status_dot = status_dot.clone();
        let parent = parent.clone();
        
        move |query: Option<String>, category: Option<DistroCategory>| {
            // Clear existing distros
            while let Some(child) = distros_box.first_child() {
                distros_box.remove(&child);
            }
            download_buttons.borrow_mut().clear();

            // Fetch distros based on filters
            let distros = if let Some(q) = query {
                DistrosCatalog::search(&q).unwrap_or_default()
            } else if let Some(cat) = category {
                DistrosCatalog::by_category(&cat).unwrap_or_default()
            } else {
                DistrosCatalog::fetch().map(|c| c.distros).unwrap_or_default()
            };

            if distros.is_empty() {
                let no_results = Label::new(Some("No distributions found"));
                no_results.add_css_class("iso-meta-label");
                no_results.set_margin_top(40);
                distros_box.append(&no_results);
                return;
            }

            for distro in &distros {
                let row = GtkBox::new(Orientation::Horizontal, 8);
                row.set_margin_top(0);
                row.set_margin_bottom(0);
                row.set_margin_start(1);
                row.set_margin_end(1);
                row.add_css_class("iso-row");

                // Check if already downloaded and verify integrity
                let download_dir = crate::download::ISOFetcher::default_download_dir();
                let expected_filename = format!("{}.iso", distro.id);
                let iso_path = download_dir.join(&expected_filename);
                let already_downloaded = iso_path.exists();
                
                // Verify integrity if file exists
                let is_verified = if already_downloaded {
                    use crate::download::verification::quick_verify;
                    quick_verify(&iso_path, &distro.sha256)
                } else {
                    false
                };

                if is_verified {
                    row.add_css_class("iso-row-downloaded");
                } else if already_downloaded {
                    // File exists but not verified - add different style
                    row.add_css_class("iso-row-unverified");
                }

                // Logo - use embedded colored SVG
                let logo_box = GtkBox::new(Orientation::Vertical, 0);
                logo_box.set_size_request(22, 22);
                logo_box.set_halign(gtk4::Align::Center);
                logo_box.set_valign(gtk4::Align::Center);
                logo_box.add_css_class("iso-logo-container");
                
                if let Some(svg_data) = get_distro_icon_svg(&distro.id) {
                    // Write SVG to temp file and load it
                    use std::io::Write;
                    let temp_path = format!("/tmp/etch_icon_{}.svg", distro.id);
                    if let Ok(mut file) = std::fs::File::create(&temp_path) {
                        let _ = file.write_all(svg_data);
                        let pic = gtk4::Picture::for_filename(&temp_path);
                        pic.set_size_request(20, 20);
                        logo_box.append(&pic);
                    }
                } else {
                    // Fallback icon with subtle background
                    let fallback_icon = Image::from_icon_name("media-optical-symbolic");
                    fallback_icon.set_icon_size(gtk4::IconSize::Normal);
                    fallback_icon.add_css_class("iso-fallback-icon");
                    logo_box.append(&fallback_icon);
                }
                
                row.append(&logo_box);

                let info = GtkBox::new(Orientation::Vertical, 0);
                info.set_hexpand(true);
                info.set_halign(gtk4::Align::Start);
                info.set_valign(gtk4::Align::Center);

                let name_box = GtkBox::new(Orientation::Horizontal, 6);
                
                let name = Label::new(Some(&distro.name));
                name.set_halign(gtk4::Align::Start);
                name.add_css_class("iso-name-label");
                name_box.append(&name);
                
                // Add LTS/version badge for certain distros
                if distro.name.contains("LTS") || distro.version.contains("LTS") {
                    let lts_badge = Label::new(Some("LTS"));
                    lts_badge.add_css_class("version-badge");
                    lts_badge.add_css_class("badge-lts");
                    name_box.append(&lts_badge);
                } else if distro.name.contains("Server") || distro.description.contains("server") {
                    let server_badge = Label::new(Some("SRV"));
                    server_badge.add_css_class("version-badge");
                    server_badge.add_css_class("badge-server");
                    name_box.append(&server_badge);
                } else if distro.name.contains("Pro") || distro.description.contains("pro") || distro.description.contains("enterprise") {
                    let pro_badge = Label::new(Some("PRO"));
                    pro_badge.add_css_class("version-badge");
                    pro_badge.add_css_class("badge-pro");
                    name_box.append(&pro_badge);
                }

                // Add verification status badge
                if is_verified {
                    let check_icon = Image::from_icon_name("emblem-default-symbolic");
                    check_icon.set_icon_size(gtk4::IconSize::Normal);
                    check_icon.add_css_class("iso-verified-badge");
                    check_icon.set_tooltip_text(Some("✓ Downloaded & verified"));
                    name_box.append(&check_icon);
                } else if already_downloaded {
                    let warn_icon = Image::from_icon_name("dialog-warning-symbolic");
                    warn_icon.set_icon_size(gtk4::IconSize::Normal);
                    warn_icon.add_css_class("iso-unverified-badge");
                    warn_icon.set_tooltip_text(Some("⚠ Downloaded but not verified"));
                    name_box.append(&warn_icon);
                }
                
                info.append(&name_box);

                let meta = Label::new(Some(&format!("{} • {}", distro.version, distro.size_human)));
                meta.set_halign(gtk4::Align::Start);
                meta.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                meta.set_max_width_chars(38);
                meta.add_css_class("iso-meta-label");
                info.append(&meta);

                row.append(&info);

                let dl_btn = Button::new();
                
                // Change button behavior if already verified
                if is_verified {
                    // Use "write" icon instead of download
                    let write_icon = Image::from_icon_name("media-floppy-symbolic");
                    write_icon.set_icon_size(gtk4::IconSize::Normal);
                    dl_btn.set_child(Some(&write_icon));
                    dl_btn.set_tooltip_text(Some(&format!("Write {} to USB", distro.name)));
                } else {
                    let dl_icon = Image::from_icon_name("folder-download-symbolic");
                    dl_icon.set_icon_size(gtk4::IconSize::Normal);
                    dl_btn.set_child(Some(&dl_icon));
                    dl_btn.set_tooltip_text(Some(&format!("Download {} ({})", distro.name, distro.size_human)));
                }
                
                dl_btn.add_css_class("download-button-compact");
                dl_btn.set_size_request(36, 28);
                dl_btn.set_halign(gtk4::Align::End);
                dl_btn.set_valign(gtk4::Align::Center);
                dl_btn.set_hexpand(false);

                let distro_clone = distro.clone();
                let iso_label_clone = iso_label.clone();
                let state_clone = state.clone();
                let platform_box_clone = platform_box.clone();
                let platform_icon_clone = platform_icon.clone();
                let platform_label_clone = platform_label.clone();
                let dialog_clone2 = dialog_for_closure.clone();
                let parent_clone = parent.clone();
                let buttons_for_click = download_buttons.clone();
                let write_button_clone = write_button.clone();
                let progress_label_clone = progress_label.clone();
                let progress_bar_clone = progress_bar.clone();
                let speed_label_clone = speed_label.clone();
                let status_dot_clone = status_dot.clone();
                let iso_path_clone = iso_path.clone();
                let is_verified_clone = is_verified;

                dl_btn.connect_clicked(move |_btn| {
                    // If already verified, skip download and go straight to write mode
                    if is_verified_clone {
                        println!("✓ ISO already verified, skipping to write mode");
                        
                        // Update state with the ISO
                        let mut state_ref = state_clone.borrow_mut();
                        state_ref.selected_iso = Some(iso_path_clone.clone());
                        recompute_action_state_export(&mut state_ref);
                        let action_state = state_ref.action_state.clone();
                        drop(state_ref);
                        
                        // Update UI
                        iso_label_clone.set_text(&iso_path_clone.file_name().unwrap().to_string_lossy());
                        
                        let platform = crate::core::platforms::Platform::from_iso_path(&iso_path_clone);
                        platform_box_clone.set_visible(true);
                        platform_icon_clone.set_icon_name(Some(platform.icon_name()));
                        platform_label_clone.set_text(platform.display_name());
                        
                        // Update write button state
                        update_action_area_export(
                            &action_state,
                            &write_button_clone,
                            &progress_label_clone,
                            &progress_bar_clone,
                            &speed_label_clone,
                            &status_dot_clone,
                        );
                        
                        // Close browser dialog
                        dialog_clone2.close();
                        return;
                    }
                    
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
                    let write_button_for_poll = write_button_clone.clone();
                    let progress_label_for_poll = progress_label_clone.clone();
                    let progress_bar_for_poll = progress_bar_clone.clone();
                    let speed_label_for_poll = speed_label_clone.clone();
                    let status_dot_for_poll = status_dot_clone.clone();
                    let parent_for_poll = parent_clone.clone();
                    let parent_for_err = parent_clone.clone();
                    let distro_name_for_err = distro_clone.name.clone();

                    // Poll for progress updates
                    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                        // Check if progress window was closed (cancel button)
                        if progress_window.is_cancelled() {
                            println!("✗ Download cancelled by user");
                            // Clear global download flag on cancel
                            state_for_poll.borrow().download_in_progress.store(false, Ordering::Relaxed);
                            // Re-enable ALL buttons after cancel
                            for button in buttons_for_poll.borrow().iter() {
                                button.set_sensitive(true);
                            }
                            return glib::ControlFlow::Break;
                        }

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
                                // Clear global download flag
                                state_for_poll.borrow().download_in_progress.store(false, Ordering::Relaxed);

                                match result {
                                    Ok(path) => {
                                        println!("✓ Download complete: {}", path.display());
                                        
                                        // Show completion state in progress window
                                        progress_window.show_complete();
                                        
                                        // Update state with downloaded ISO
                                        let mut state_ref = state_for_poll.borrow_mut();
                                        state_ref.selected_iso = Some(path.clone());
                                        recompute_action_state_export(&mut state_ref);
                                        let action_state = state_ref.action_state.clone();
                                        drop(state_ref);
                                        
                                        // Update UI
                                        iso_label_for_poll.set_text(&path.file_name().unwrap().to_string_lossy());

                                        let platform = crate::core::platforms::Platform::from_iso_path(&path);
                                        platform_box_for_poll.set_visible(true);
                                        platform_icon_for_poll.set_icon_name(Some(platform.icon_name()));
                                        platform_label_for_poll.set_text(platform.display_name());
                                        
                                        // Update write button state
                                        update_action_area_export(
                                            &action_state,
                                            &write_button_for_poll,
                                            &progress_label_for_poll,
                                            &progress_bar_for_poll,
                                            &speed_label_for_poll,
                                            &status_dot_for_poll,
                                        );

                                        // Re-enable ALL buttons after success
                                        for button in buttons_for_poll.borrow().iter() {
                                            button.set_sensitive(true);
                                        }
                                        
                                        // Show success notification
                                        let success_dialog = gtk4::MessageDialog::builder()
                                            .transient_for(&parent_for_poll)
                                            .modal(true)
                                            .message_type(gtk4::MessageType::Info)
                                            .buttons(gtk4::ButtonsType::Ok)
                                            .text("Download Complete")
                                            .secondary_text(&format!("Successfully downloaded {}\n\nThe ISO is ready to be written to a USB device.", path.file_name().unwrap().to_string_lossy()))
                                            .build();
                                        
                                        let progress_window_clone = progress_window.clone();
                                        let dialog_clone = dialog_for_poll.clone();
                                        success_dialog.connect_response(move |dialog, _| {
                                            dialog.close();
                                            progress_window_clone.close();
                                            dialog_clone.close();
                                        });
                                        success_dialog.present();
                                    }
                                    Err(e) => {
                                        let error_msg = e.to_string();
                                        println!("✗ Download failed: {}", error_msg);
                                        
                                        // Close progress window
                                        progress_window.close();
                                        
                                        // Show user-friendly error dialog
                                        let error_title = "Download Failed";
                                        let error_details = if error_msg.contains("Network") || error_msg.contains("connection") {
                                            format!("Network error: Could not connect to server.\n\nPlease check your internet connection and try again.")
                                        } else if error_msg.contains("disk") || error_msg.contains("space") {
                                            format!("Not enough disk space.\n\nPlease free up some space and try again.")
                                        } else if error_msg.contains("hash") || error_msg.contains("SHA256") {
                                            format!("File verification failed.\n\nThe downloaded file is corrupted or tampered with.\nPlease try downloading again.")
                                        } else {
                                            format!("Failed to download {}.\n\nError: {}", distro_name_for_err, error_msg)
                                        };
                                        
                                        show_error_dialog(&parent_for_err, error_title, &error_details);
                                        
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
    };

    // Initial load - all distros
    load_distros(None, None);

    // Create category filter buttons with modern styling
    for (label, category) in categories {
        let btn = Button::with_label(label);
        btn.add_css_class("filter-button");
        btn.add_css_class("filter-chip");
        if label == "All" {
            btn.add_css_class("filter-active");
        }
        
        let active_category_clone = active_category.clone();
        let filter_buttons_clone = filter_buttons.clone();
        let load_distros_clone = load_distros.clone();
        let search_entry_clone = search_entry.clone();
        
        btn.connect_clicked(move |clicked_btn| {
            // Update active category
            *active_category_clone.borrow_mut() = category.clone();
            
            // Update button styles - clear all first
            for button in filter_buttons_clone.borrow().iter() {
                button.remove_css_class("filter-active");
            }
            clicked_btn.add_css_class("filter-active");
            
            // Clear search and reload with category filter
            search_entry_clone.set_text("");
            load_distros_clone(None, category.clone());
        });
        
        filter_buttons.borrow_mut().push(btn.clone());
        filter_box.append(&btn);
    }

    // Search functionality
    let load_distros_for_search = load_distros.clone();
    let active_category_for_search = active_category.clone();
    search_entry.connect_search_changed(move |entry| {
        let query = entry.text().to_string();
        if query.is_empty() {
            // Show category filter if active, otherwise all
            let cat = active_category_for_search.borrow().clone();
            load_distros_for_search(None, cat);
        } else {
            // Search ignores category filter
            load_distros_for_search(Some(query), None);
        }
    });

    scroll.set_child(Some(&distros_box));
    main_box.append(&scroll);

    dialog.set_child(Some(&main_box));
    dialog.present();
}
