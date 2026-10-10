// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{AppHandle, Emitter};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_opener::OpenerExt;

/// An installer found in the library folder (the newest version per app).
#[derive(Serialize, Clone)]
struct Package {
    product: String,
    version: String,
    setup: String,
    portable: Option<String>,
}

/// An app registered in Windows' list of installed programs.
#[derive(Serialize, Default)]
struct Installed {
    product: String,
    version: String,
    location: String,
    uninstall: String,
    exe: Option<String>,
}

/// "1.10.2" sorts after "1.9.0".
fn version_key(v: &str) -> Vec<u64> {
    v.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

/// "Markdown Studio_1.3.1_x64-setup.exe" -> ("Markdown Studio", "1.3.1")
fn parse_installer(name: &str) -> Option<(String, String)> {
    let stem = name.strip_suffix("_x64-setup.exe")?;
    let (product, version) = stem.rsplit_once('_')?;
    Some((product.to_string(), version.to_string()))
}

/// Looks for installers in the folder, its sub-folders and their `dist` folders.
#[tauri::command]
fn scan_library(dir: String) -> Vec<Package> {
    let root = PathBuf::from(&dir);
    let mut places = vec![root.clone()];
    if let Ok(entries) = fs::read_dir(&root) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                places.push(p.join("dist"));
                places.push(p);
            }
        }
    }
    let mut best: HashMap<String, Package> = HashMap::new();
    for place in places {
        let Ok(entries) = fs::read_dir(&place) else { continue };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let Some((product, version)) = parse_installer(&name) else { continue };
            let newer = best.get(&product).map_or(true, |b| version_key(&version) > version_key(&b.version));
            if newer {
                let portable = place.join(format!("{}_{}_x64_portable.exe", product.replace(' ', "-"), version));
                best.insert(
                    product.clone(),
                    Package {
                        product,
                        version,
                        setup: e.path().to_string_lossy().into_owned(),
                        portable: portable.is_file().then(|| portable.to_string_lossy().into_owned()),
                    },
                );
            }
        }
    }
    let mut list: Vec<Package> = best.into_values().collect();
    list.sort_by(|a, b| a.product.cmp(&b.product));
    list
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches('"').to_string()
}

/// The app's main exe in its install folder (anything but the uninstaller).
fn main_exe(location: &Path) -> Option<String> {
    fs::read_dir(location).ok()?.flatten().map(|e| e.path()).find(|p| {
        p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"))
            && !p.file_name().is_some_and(|n| n.to_string_lossy().to_lowercase().starts_with("uninstall"))
    }).map(|p| p.to_string_lossy().into_owned())
}

#[cfg(windows)]
#[tauri::command]
fn installed_apps() -> Vec<Installed> {
    use winreg::{enums::*, RegKey};
    const KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
    const KEY32: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";
    let mut out = Vec::new();
    for (hive, path) in [(HKEY_CURRENT_USER, KEY), (HKEY_LOCAL_MACHINE, KEY), (HKEY_LOCAL_MACHINE, KEY32)] {
        let Ok(root) = RegKey::predef(hive).open_subkey(path) else { continue };
        for name in root.enum_keys().flatten() {
            let Ok(k) = root.open_subkey(&name) else { continue };
            let Ok(product) = k.get_value::<String, _>("DisplayName") else { continue };
            let location = unquote(&k.get_value::<String, _>("InstallLocation").unwrap_or_default());
            out.push(Installed {
                exe: (!location.is_empty()).then(|| main_exe(Path::new(&location))).flatten(),
                product,
                version: k.get_value("DisplayVersion").unwrap_or_default(),
                location,
                uninstall: k.get_value("UninstallString").unwrap_or_default(),
            });
        }
    }
    out
}

#[cfg(not(windows))]
#[tauri::command]
fn installed_apps() -> Vec<Installed> {
    Vec::new()
}

fn run_and_wait(mut cmd: Command) -> Result<(), String> {
    let status = cmd.status().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("exit code {}", status.code().unwrap_or(-1)))
    }
}

/// Runs an app's NSIS installer silently (per-user install, no admin prompt). Also used for updates.
#[tauri::command]
async fn install(setup: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut cmd = Command::new(&setup);
        cmd.arg("/S");
        run_and_wait(cmd)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Runs the uninstall command Windows has registered for the app, silently, and waits until
/// the app is really gone (NSIS uninstallers hand off to a temp copy and return immediately).
#[tauri::command]
async fn uninstall(command: String, product: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_uninstaller(&command)?;
        for _ in 0..120 {
            if !installed_apps().iter().any(|i| i.product == product) {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        Err("the uninstaller did not finish within 60 seconds".into())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn run_uninstaller(command: &str) -> Result<(), String> {
    let command = command.trim().to_string();
    {
        let cmd = if command.to_lowercase().starts_with("msiexec") {
            // MSI installs: "MsiExec.exe /I{GUID}" -> uninstall quietly.
            let guid = command.split_whitespace().skip(1).map(|a| a.trim_start_matches(['/', '-']).trim_start_matches(['I', 'i', 'X', 'x'])).collect::<Vec<_>>().join(" ");
            let mut c = Command::new("msiexec");
            c.args(["/x", guid.trim(), "/qn"]);
            c
        } else {
            let (exe, rest) = match command.strip_prefix('"') {
                Some(r) => r.split_once('"').map(|(e, rest)| (e.to_string(), rest.trim().to_string())).unwrap_or((r.to_string(), String::new())),
                None => (command.clone(), String::new()),
            };
            let mut c = Command::new(exe);
            c.args(rest.split_whitespace());
            c.arg("/S");
            c
        };
        run_and_wait(cmd)
    }
}

/// Starts an app (installed exe or portable build) without waiting for it.
#[tauri::command]
fn launch(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    let mut cmd = Command::new(&p);
    if let Some(dir) = p.parent() {
        cmd.current_dir(dir);
    }
    cmd.spawn().map(|_| ()).map_err(|e| format!("{path}: {e}"))
}

#[tauri::command]
fn reveal(app: AppHandle, path: String) -> Result<(), String> {
    app.opener().reveal_item_in_dir(path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn pick_folder(app: AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .blocking_pick_folder()
        .and_then(|f| f.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
async fn confirm(app: AppHandle, title: String, message: String, ok_label: String, cancel_label: String) -> bool {
    app.dialog()
        .message(message)
        .title(title)
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(ok_label, cancel_label))
        .blocking_show()
}

#[tauri::command]
fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    let allowed = ["https://", "http://", "mailto:"].iter().any(|p| url.starts_with(p));
    if !allowed {
        return Err(format!("Refusing to open {url}"));
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

/// One entry of a native context menu; an entry without `id` is a separator.
#[derive(Deserialize)]
struct MenuEntry {
    id: Option<String>,
    label: Option<String>,
    enabled: Option<bool>,
    checked: Option<bool>,
    accel: Option<String>,
}

/// Shows a native (OS) context menu at the mouse position. The chosen item's id
/// reaches the UI as a "context-menu" event (see `on_menu_event` in `main`).
#[tauri::command]
fn context_menu(app: AppHandle, window: tauri::Window, items: Vec<MenuEntry>) -> Result<(), String> {
    use tauri::menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder};
    let mut menu = MenuBuilder::new(&app);
    for entry in items {
        let Some(id) = entry.id else {
            menu = menu.separator();
            continue;
        };
        let label = entry.label.unwrap_or_default();
        let enabled = entry.enabled.unwrap_or(true);
        if let Some(checked) = entry.checked {
            let item = CheckMenuItemBuilder::with_id(id, label)
                .enabled(enabled)
                .checked(checked)
                .build(&app)
                .map_err(|e| e.to_string())?;
            menu = menu.item(&item);
            continue;
        }
        let plain = || MenuItemBuilder::with_id(id.clone(), label.clone()).enabled(enabled);
        let item = match entry.accel.as_deref() {
            Some(a) => plain().accelerator(a).build(&app).or_else(|_| plain().build(&app)),
            None => plain().build(&app),
        }
        .map_err(|e| e.to_string())?;
        menu = menu.item(&item);
    }
    let menu = menu.build().map_err(|e| e.to_string())?;
    window.popup_menu(&menu).map_err(|e| e.to_string())
}

#[tauri::command]
fn clipboard_read(app: AppHandle) -> String {
    app.clipboard().read_text().unwrap_or_default()
}

#[tauri::command]
fn clipboard_write(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard().write_text(text).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .on_menu_event(|app, event| {
            let _ = app.emit("context-menu", event.id().as_ref());
        })
        .invoke_handler(tauri::generate_handler![
            context_menu,
            clipboard_read,
            clipboard_write,
            scan_library,
            installed_apps,
            install,
            uninstall,
            launch,
            reveal,
            pick_folder,
            confirm,
            open_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running Studio Hub");
}
