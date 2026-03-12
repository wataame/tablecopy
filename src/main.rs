mod converter;
mod format;
mod hud;
mod image;
mod parser;

use clap::Parser;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process;
use std::time::SystemTime;

const CYCLE_TIMEOUT_SECS: u64 = 30;

#[derive(Clone, Copy, PartialEq)]
enum CycleFormat {
    Markdown,
    Image,
}

const FORMATS: [CycleFormat; 2] = [CycleFormat::Markdown, CycleFormat::Image];

#[derive(Parser)]
#[command(
    name = "tablecopy",
    version,
    about = "Convert Unicode box-drawing tables to Markdown / Image format"
)]
struct Cli {
    /// Read from stdin instead of clipboard (use "-" as value)
    #[arg(value_name = "STDIN")]
    stdin_flag: Option<String>,

    /// Install keyboard shortcut (macOS: Cmd+Ctrl+M, Windows: Ctrl+Alt+M)
    #[arg(long = "install")]
    install: bool,

    /// Uninstall keyboard shortcut
    #[arg(long = "uninstall")]
    uninstall: bool,

    /// Enable or disable HUD notification (on/off)
    #[arg(long = "hud", value_name = "on|off")]
    hud: Option<String>,
}

fn main() {
    // HUD subprocess mode (invoked by show_hud via env var)
    if hud::is_hud_mode() {
        hud::run_hud();
        return;
    }

    let cli = Cli::parse();

    if let Some(ref value) = cli.hud {
        match value.as_str() {
            "on" => {
                set_config("hud", "on");
                eprintln!("✓ HUD enabled");
            }
            "off" => {
                set_config("hud", "off");
                eprintln!("✓ HUD disabled");
            }
            _ => {
                eprintln!("Invalid value for --hud. Use 'on' or 'off'.");
                process::exit(1);
            }
        }
        return;
    }

    if cli.install {
        install_quick_action();
        return;
    }

    if cli.uninstall {
        uninstall_quick_action();
        return;
    }

    let use_stdin = cli.stdin_flag.as_deref() == Some("-");

    if use_stdin {
        run_stdin();
    } else {
        // Cycle mode (Markdown → Image → Markdown ...)
        run_cycle();
    }
}

/// Cycle through formats: Markdown → Image
fn run_cycle() {
    let mut clipboard = open_clipboard();
    // Clipboard may contain image (after Image cycle), so text read can fail
    let input = clipboard.get_text().ok();

    // Check if we have a saved original (within timeout)
    let state = load_state();

    let (original, format_index) = match state {
        Some((saved_original, last_index)) => {
            if input.as_ref().is_some_and(|t| has_unicode_table(t)) {
                // Clipboard has a new Unicode table — start fresh
                (input.unwrap(), 0)
            } else {
                // Cycle to next format (works even when clipboard has image data)
                let next_index = (last_index + 1) % FORMATS.len();
                (saved_original, next_index)
            }
        }
        None => {
            // First press: detect Unicode table
            let text = input.unwrap_or_default();
            if !has_unicode_table(&text) {
                eprintln!("No table found");
                hud::show_hud("No Table");
                return;
            }
            // Save original and start with Markdown
            (text, 0)
        }
    };

    let format = FORMATS[format_index];

    match format {
        CycleFormat::Markdown => {
            match converter::convert_to_markdown(&original) {
                Some(output) => {
                    let row_count = output.lines().count().saturating_sub(2);
                    clipboard.set_text(&output).unwrap_or_else(|e| {
                        eprintln!("Error writing to clipboard: {}", e);
                        process::exit(1);
                    });
                    save_state(&original, format_index);
                    eprintln!("✓ Converted to Markdown ({} rows)", row_count);
                    if is_hud_enabled() {
                        hud::show_hud("Markdown");
                    }
                }
                None => {
                    eprintln!("No table found");
                    hud::show_hud("No Table");
                }
            }
        }
        CycleFormat::Image => {
            let tables = parser::parse_tables(&original);
            match tables {
                Some(tables) => {
                    match image::render_table(&tables[0]) {
                        Some(img) => {
                            use std::borrow::Cow;
                            clipboard
                                .set_image(arboard::ImageData {
                                    width: img.width,
                                    height: img.height,
                                    bytes: Cow::Owned(img.rgba_data),
                                })
                                .unwrap_or_else(|e| {
                                    eprintln!("Error writing image to clipboard: {}", e);
                                    process::exit(1);
                                });
                            save_state(&original, format_index);
                            eprintln!("✓ Converted to Image ({} rows)", tables[0].rows.len());
                            if is_hud_enabled() {
                                hud::show_hud("Image");
                            }
                        }
                        None => {
                            eprintln!("Render error");
                            hud::show_hud("Error");
                        }
                    }
                }
                None => {
                    eprintln!("No table found");
                    hud::show_hud("No Table");
                }
            }
        }
    }
}

/// Run with stdin input (always Markdown)
fn run_stdin() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap_or_else(|e| {
        eprintln!("Error reading stdin: {}", e);
        process::exit(1);
    });

    match converter::convert_to_markdown(&input) {
        Some(output) => print!("{}", output),
        None => {
            eprintln!("No Unicode table found in input.");
            process::exit(1);
        }
    }
}

// --- State management ---

fn state_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
            std::env::var("USERPROFILE")
                .map(|p| format!("{}\\AppData\\Local", p))
                .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().to_string())
        });
        PathBuf::from(base).join("tablecopy").join("cache")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".cache").join("tablecopy")
    }
}

fn load_state() -> Option<(String, usize)> {
    let dir = state_dir();
    let original_path = dir.join("original");
    let index_path = dir.join("format_index");
    let ts_path = dir.join("timestamp");

    if let Ok(ts_str) = std::fs::read_to_string(&ts_path) {
        if let Ok(ts) = ts_str.trim().parse::<u64>() {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if now - ts > CYCLE_TIMEOUT_SECS {
                clear_state();
                return None;
            }
        }
    } else {
        return None;
    }

    let original = std::fs::read_to_string(&original_path).ok()?;
    let index_str = std::fs::read_to_string(&index_path).ok()?;
    let index = index_str.trim().parse::<usize>().ok()?;

    Some((original, index))
}

fn save_state(original: &str, format_index: usize) {
    let dir = state_dir();
    std::fs::create_dir_all(&dir).ok();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    std::fs::write(dir.join("original"), original).ok();
    std::fs::write(dir.join("format_index"), format_index.to_string()).ok();
    std::fs::write(dir.join("timestamp"), now.to_string()).ok();
}

fn clear_state() {
    let dir = state_dir();
    std::fs::remove_file(dir.join("original")).ok();
    std::fs::remove_file(dir.join("format_index")).ok();
    std::fs::remove_file(dir.join("timestamp")).ok();
}

// --- Config ---

/// Config lives alongside state to avoid extra macOS permission dialogs.
fn config_dir() -> PathBuf {
    // Use the same base directory as state_dir (without the "cache" subdirectory)
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
            std::env::var("USERPROFILE")
                .map(|p| format!("{}\\AppData\\Local", p))
                .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().to_string())
        });
        PathBuf::from(base).join("tablecopy")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".cache").join("tablecopy")
    }
}

fn get_config(key: &str) -> Option<String> {
    let path = config_dir().join("config");
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == key {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

fn set_config(key: &str, value: &str) {
    let dir = config_dir();
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("config");

    let mut entries = std::collections::HashMap::new();

    // Read existing config
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            if let Some((k, v)) = line.split_once('=') {
                entries.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }

    entries.insert(key.to_string(), value.to_string());

    let content: Vec<String> = entries.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    std::fs::write(&path, content.join("\n")).ok();
}

pub fn is_hud_enabled() -> bool {
    get_config("hud").is_none_or(|v| v != "off")
}

// --- Helpers ---

fn open_clipboard() -> arboard::Clipboard {
    arboard::Clipboard::new().unwrap_or_else(|e| {
        eprintln!("Error accessing clipboard: {}", e);
        process::exit(1);
    })
}

fn has_unicode_table(text: &str) -> bool {
    parser::parse_tables(text).is_some()
}

// --- Install ---

fn install_quick_action() {
    #[cfg(target_os = "windows")]
    {
        install_ahk_script();
        return;
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        eprintln!("--install is only supported on macOS and Windows.");
        process::exit(1);
    }

    #[cfg(target_os = "macos")]
    {
        let tablecopy_path = std::env::current_exe().unwrap_or_else(|e| {
            eprintln!("Error getting executable path: {}", e);
            process::exit(1);
        });

        let home = std::env::var("HOME").expect("HOME not set");
        let old_workflow =
            std::path::PathBuf::from(&home).join("Library/Services/TableCopy.workflow");
        if old_workflow.exists() {
            std::fs::remove_dir_all(&old_workflow).ok();
            eprintln!("✓ Removed old Automator workflow");
        }

        let shortcut_script = r#"
            tell application "Shortcuts Events"
                try
                    delete shortcut "TableCopy"
                end try
            end tell
            "#;
        std::process::Command::new("osascript")
            .arg("-e")
            .arg(shortcut_script)
            .output()
            .ok();

        let services_dir = std::path::PathBuf::from(&home).join("Library/Services");
        let workflow_dir = services_dir.join("TableCopy.workflow").join("Contents");

        std::fs::create_dir_all(&workflow_dir).unwrap_or_else(|e| {
            eprintln!("Error creating workflow directory: {}", e);
            process::exit(1);
        });

        let info_plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>NSServices</key>
	<array>
		<dict>
			<key>NSMenuItem</key>
			<dict>
				<key>default</key>
				<string>TableCopy</string>
			</dict>
			<key>NSMessage</key>
			<string>runWorkflowAsService</string>
		</dict>
	</array>
</dict>
</plist>"#;

        std::fs::write(workflow_dir.join("Info.plist"), info_plist).unwrap_or_else(|e| {
            eprintln!("Error writing Info.plist: {}", e);
            process::exit(1);
        });

        let document_wflow = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>AMApplicationBuild</key>
	<string>523</string>
	<key>AMApplicationVersion</key>
	<string>2.10</string>
	<key>AMDocumentVersion</key>
	<string>2</string>
	<key>actions</key>
	<array>
		<dict>
			<key>action</key>
			<dict>
				<key>AMAccepts</key>
				<dict>
					<key>Container</key>
					<string>List</string>
					<key>Optional</key>
					<true/>
					<key>Types</key>
					<array>
						<string>com.apple.cocoa.string</string>
					</array>
				</dict>
				<key>AMActionVersion</key>
				<string>2.0.3</string>
				<key>AMApplication</key>
				<array>
					<string>Automator</string>
				</array>
				<key>AMBundleIdentifier</key>
				<string>com.apple.RunShellScript</string>
				<key>AMCategory</key>
				<array>
					<string>AMCategoryUtilities</string>
				</array>
				<key>AMIconName</key>
				<string>Automator</string>
				<key>AMName</key>
				<string>Run Shell Script</string>
				<key>AMProvides</key>
				<dict>
					<key>Container</key>
					<string>List</string>
					<key>Types</key>
					<array>
						<string>com.apple.cocoa.string</string>
					</array>
				</dict>
				<key>ActionBundlePath</key>
				<string>/System/Library/Automator/Run Shell Script.action</string>
				<key>ActionName</key>
				<string>Run Shell Script</string>
				<key>ActionParameters</key>
				<dict>
					<key>COMMAND_STRING</key>
					<string>export PATH="/opt/homebrew/bin:/usr/local/bin:$PATH"; {}</string>
					<key>CheckedForUserDefaultShell</key>
					<true/>
					<key>inputMethod</key>
					<integer>0</integer>
					<key>shell</key>
					<string>/bin/sh</string>
					<key>source</key>
					<string></string>
				</dict>
				<key>BundleIdentifier</key>
				<string>com.apple.RunShellScript</string>
				<key>CFBundleVersion</key>
				<string>2.0.3</string>
				<key>CanShowSelectedItemsWhenRun</key>
				<false/>
				<key>CanShowWhenRun</key>
				<true/>
				<key>Category</key>
				<array>
					<string>AMCategoryUtilities</string>
				</array>
				<key>Class Name</key>
				<string>RunShellScriptAction</string>
				<key>InputUUID</key>
				<string>A1A1A1A1-B2B2-C3C3-D4D4-E5E5E5E5E5E5</string>
				<key>OutputUUID</key>
				<string>F6F6F6F6-A7A7-B8B8-C9C9-D0D0D0D0D0D0</string>
				<key>UUID</key>
				<string>11111111-2222-3333-4444-555555555555</string>
				<key>UnlocalizedApplications</key>
				<array>
					<string>Automator</string>
				</array>
			</dict>
		</dict>
	</array>
	<key>connectors</key>
	<dict/>
	<key>workflowMetaData</key>
	<dict>
		<key>serviceInputTypeIdentifier</key>
		<string>com.apple.Automator.nothing</string>
		<key>serviceOutputTypeIdentifier</key>
		<string>com.apple.Automator.nothing</string>
		<key>serviceProcessesInput</key>
		<integer>0</integer>
		<key>workflowTypeIdentifier</key>
		<string>com.apple.Automator.servicesMenu</string>
	</dict>
</dict>
</plist>"#,
            tablecopy_path.display()
        );

        std::fs::write(workflow_dir.join("document.wflow"), document_wflow).unwrap_or_else(|e| {
            eprintln!("Error writing document.wflow: {}", e);
            process::exit(1);
        });

        eprintln!("✓ Quick Action installed (optimized with /bin/sh)");

        let shortcut_result = std::process::Command::new("/bin/sh")
            .args([
                "-c",
                r#"defaults write pbs NSServicesStatus -dict-add '"(null) - TableCopy - runWorkflowAsService"' '{ "enabled" = 1; "key_equivalent" = "@^m"; }'"#,
            ])
            .output();

        std::process::Command::new("/System/Library/CoreServices/pbs")
            .arg("-flush")
            .output()
            .ok();

        match shortcut_result {
            Ok(output) if output.status.success() => {
                eprintln!("✓ Keyboard shortcut registered: Cmd+Ctrl+M");
            }
            _ => {
                eprintln!();
                eprintln!("⚠ Could not auto-register shortcut. Set it manually:");
                eprintln!(
                    "  System Settings → Keyboard → Keyboard Shortcuts → Services"
                );
            }
        }

        eprintln!();
        eprintln!("Usage: Press Cmd+Ctrl+M to cycle formats:");
        eprintln!("  1st press → Markdown (for Notion)");
        eprintln!("  2nd press → Image (for Slack)");
    }
}

fn uninstall_quick_action() {
    #[cfg(target_os = "windows")]
    {
        uninstall_ahk_script();
        return;
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        eprintln!("--uninstall is only supported on macOS and Windows.");
        process::exit(1);
    }

    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").expect("HOME not set");
        let workflow_path =
            std::path::PathBuf::from(home).join("Library/Services/TableCopy.workflow");

        if workflow_path.exists() {
            std::fs::remove_dir_all(&workflow_path).unwrap_or_else(|e| {
                eprintln!("Error removing workflow: {}", e);
                process::exit(1);
            });
            eprintln!("✓ Quick Action uninstalled: {}", workflow_path.display());
        } else {
            eprintln!("Quick Action not found at: {}", workflow_path.display());
        }

        clear_state();
    }
}

// --- Windows install/uninstall ---

#[cfg(target_os = "windows")]
fn install_ahk_script() {
    let tablecopy_path = std::env::current_exe().unwrap_or_else(|e| {
        eprintln!("Error getting executable path: {}", e);
        process::exit(1);
    });

    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| {
        eprintln!("APPDATA environment variable not set.");
        process::exit(1);
    });
    let install_dir = PathBuf::from(&appdata).join("TableCopy");
    std::fs::create_dir_all(&install_dir).unwrap_or_else(|e| {
        eprintln!("Error creating directory: {}", e);
        process::exit(1);
    });

    let ahk_content = format!(
        "; TableCopy - Keyboard Shortcut (Ctrl+Alt+M)\n\
         ; AutoHotkey v2 script\n\
         ; Generated by: tablecopy --install\n\
         \n\
         #Requires AutoHotkey v2.0\n\
         #SingleInstance Force\n\
         \n\
         ^!m::\n\
         {{\n\
             Run(\"{}\", , \"Hide\")\n\
         }}\n",
        tablecopy_path.display()
    );

    let ahk_path = install_dir.join("tablecopy.ahk");
    std::fs::write(&ahk_path, &ahk_content).unwrap_or_else(|e| {
        eprintln!("Error writing AHK script: {}", e);
        process::exit(1);
    });

    eprintln!("✓ AutoHotkey script created: {}", ahk_path.display());

    // Copy to Startup folder for auto-launch
    let startup_dir = PathBuf::from(&appdata)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Startup");

    if startup_dir.exists() {
        let startup_ahk = startup_dir.join("tablecopy.ahk");
        match std::fs::copy(&ahk_path, &startup_ahk) {
            Ok(_) => {
                eprintln!("✓ Copied to Startup folder (auto-launch on login)");
            }
            Err(_) => {
                eprintln!();
                eprintln!("⚠ Could not copy to Startup folder. To enable auto-start:");
                eprintln!("  Copy {} to:", ahk_path.display());
                eprintln!("  {}", startup_dir.display());
            }
        }
    }

    eprintln!();
    eprintln!("Prerequisites:");
    eprintln!("  AutoHotkey v2: https://www.autohotkey.com/");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  1. Install AutoHotkey v2 (if not already installed)");
    eprintln!("  2. Double-click the .ahk file to start");
    eprintln!("  3. Press Ctrl+Alt+M to cycle formats:");
    eprintln!("     1st press → Markdown (for Notion)");
    eprintln!("     2nd press → Image (for Slack)");
}

#[cfg(target_os = "windows")]
fn uninstall_ahk_script() {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| {
        eprintln!("APPDATA environment variable not set.");
        process::exit(1);
    });

    let ahk_path = PathBuf::from(&appdata).join("TableCopy").join("tablecopy.ahk");
    if ahk_path.exists() {
        std::fs::remove_file(&ahk_path).unwrap_or_else(|e| {
            eprintln!("Error removing AHK script: {}", e);
            process::exit(1);
        });
        eprintln!("✓ AHK script removed: {}", ahk_path.display());
    } else {
        eprintln!("AHK script not found at: {}", ahk_path.display());
    }

    // Remove from Startup folder
    let startup_ahk = PathBuf::from(&appdata)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Startup")
        .join("tablecopy.ahk");

    if startup_ahk.exists() {
        std::fs::remove_file(&startup_ahk).unwrap_or_else(|e| {
            eprintln!("Error removing Startup script: {}", e);
            process::exit(1);
        });
        eprintln!("✓ Startup script removed: {}", startup_ahk.display());
    }

    // Clean up directory if empty
    let install_dir = PathBuf::from(&appdata).join("TableCopy");
    if install_dir.exists() {
        if std::fs::read_dir(&install_dir).map_or(true, |mut d| d.next().is_none()) {
            std::fs::remove_dir(&install_dir).ok();
        }
    }

    clear_state();
}
