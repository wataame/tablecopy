/// Show a HUD overlay on screen. This spawns a child process to avoid
/// blocking the main CLI and to satisfy macOS's main-thread requirement.
pub fn show_hud(format_name: &str) {
    #[cfg(target_os = "macos")]
    {
        let exe = std::env::current_exe().unwrap_or_default();
        std::process::Command::new(exe)
            .env("TABLECOPY_HUD_FORMAT", format_name)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok();
    }

    #[cfg(target_os = "windows")]
    {
        show_hud_windows(format_name);
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = format_name;
    }
}

/// Check if we're being invoked as the HUD subprocess.
pub fn is_hud_mode() -> bool {
    std::env::var("TABLECOPY_HUD_FORMAT").is_ok()
}

/// Run the HUD display (called in the child process).
pub fn run_hud() {
    #[cfg(target_os = "macos")]
    {
        let format_name = std::env::var("TABLECOPY_HUD_FORMAT").unwrap_or_default();
        run_hud_macos(&format_name);
    }
}

#[cfg(target_os = "macos")]
fn run_hud_macos(format_name: &str) {
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSColor, NSScreen,
        NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindow,
        NSWindowStyleMask,
    };
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    let mtm = unsafe { MainThreadMarker::new_unchecked() };

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

    let width: f64 = 220.0;
    let height: f64 = 64.0;

    // Get screen center
    let (cx, cy) = {
        if let Some(screen) = NSScreen::mainScreen(mtm) {
            let frame = screen.frame();
            (
                frame.origin.x + (frame.size.width - width) / 2.0,
                frame.origin.y + (frame.size.height - height) / 2.0 + 80.0,
            )
        } else {
            (400.0, 400.0)
        }
    };

    let frame = NSRect::new(NSPoint::new(cx, cy), NSSize::new(width, height));

    let window = unsafe {
        let w = NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            frame,
            NSWindowStyleMask::Borderless,
            objc2_app_kit::NSBackingStoreType(2), // NSBackingStoreBuffered
            false,
        );
        w.setOpaque(false);
        w.setBackgroundColor(Some(&NSColor::clearColor()));
        // Floating window level
        let _: () = objc2::msg_send![&w, setLevel: 25_i64];
        w.setHasShadow(false);
        w.setIgnoresMouseEvents(true);
        w.setAlphaValue(0.0);
        w
    };

    // Visual effect background
    let effect_frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));
    let effect_view = unsafe {
        let v = NSVisualEffectView::initWithFrame(
            NSVisualEffectView::alloc(mtm),
            effect_frame,
        );
        v.setMaterial(NSVisualEffectMaterial::Popover);
        v.setState(NSVisualEffectState::Active);
        v.setWantsLayer(true);
        if let Some(layer) = v.layer() {
            let _: () = objc2::msg_send![&layer, setCornerRadius: 20.0_f64];
            let _: () = objc2::msg_send![&layer, setMasksToBounds: true];
            let _: () = objc2::msg_send![&layer, setBorderWidth: 0.0_f64];
        }
        v
    };

    // Format label
    let icon = if format_name == "No Table" || format_name == "Error" { "⚠" } else { "✓" };
    let title_text = format!("{} {}", icon, format_name);
    let title_label = create_label(mtm, &title_text, 16.0, 0.0, 14.0, width, 28.0);

    effect_view.addSubview(&title_label);

    if let Some(content) = window.contentView() {
        content.addSubview(&effect_view);
    }
    window.makeKeyAndOrderFront(None);

    // Show immediately
    let _: () = unsafe { objc2::msg_send![&window, setAlphaValue: 1.0_f64] };

    // Schedule exit after 0.8s
    let window_clone = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(800));
        let _ = window_clone;
        std::process::exit(0);
    });

    app.run();
}

#[cfg(target_os = "macos")]
fn create_label(
    mtm: objc2::MainThreadMarker,
    text: &str,
    font_size: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> objc2::rc::Retained<objc2_app_kit::NSTextField> {
    use objc2_app_kit::{NSFont, NSTextField, NSColor};
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

    let ns_text = NSString::from_str(text);
    let label = NSTextField::labelWithString(&ns_text, mtm);
    let font = NSFont::boldSystemFontOfSize(font_size);
    label.setFont(Some(&font));
    let text_color = NSColor::labelColor();
    label.setTextColor(Some(&text_color));
    unsafe {
        let _: () = objc2::msg_send![&label, setAlignment: 1_i64]; // NSTextAlignmentCenter
    }
    label.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(width, height)));
    label
}

#[cfg(target_os = "windows")]
fn show_hud_windows(format_name: &str) {
    let script = format!(
        concat!(
            "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null;",
            "[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom, ContentType = WindowsRuntime] | Out-Null;",
            "$xml = New-Object Windows.Data.Xml.Dom.XmlDocument;",
            "$xml.LoadXml('<toast duration=\"short\"><visual><binding template=\"ToastGeneric\">",
            "<text>tablecopy</text><text>{}</text>",
            "</binding></visual><audio silent=\"true\"/></toast>');",
            "$toast = [Windows.UI.Notifications.ToastNotification]::new($xml);",
            "[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('tablecopy').Show($toast)",
        ),
        format_name
    );

    std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok();
}
