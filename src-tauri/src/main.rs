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

/* ===================== GitHub Releases ===================== */

const GITHUB_OWNER: &str = "Nuharji";

/// One downloadable file of a release.
#[derive(Serialize, Clone)]
struct Asset {
    name: String,
    /// API url: works for public and (with a token) private repositories.
    url: String,
    size: u64,
}

#[derive(Serialize, Clone)]
struct Release {
    version: String,
    name: String,
    notes: String,
    published: String,
    prerelease: bool,
    setup: Option<Asset>,
    portable: Option<Asset>,
}

/// Hides the console window of helper processes such as `gh`.
fn quiet(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

/// Optional GitHub token: lets the owner reach private repositories. Taken from GITHUB_TOKEN or the
/// GitHub CLI login (`gh auth token`). Public repositories work without one.
fn github_token() -> Option<String> {
    if let Ok(t) = std::env::var("GITHUB_TOKEN") {
        if !t.trim().is_empty() {
            return Some(t.trim().to_string());
        }
    }
    for gh in ["gh", r"C:\Program Files\GitHub CLI\gh.exe"] {
        if let Ok(out) = quiet(Command::new(gh).args(["auth", "token"])).output() {
            let t = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if out.status.success() && !t.is_empty() {
                return Some(t);
            }
        }
    }
    None
}

fn github_get(url: &str, accept: &str) -> Result<reqwest::blocking::Response, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("StudioHub")
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())?;
    let mut req = client.get(url).header("Accept", accept).header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(t) = github_token() {
        req = req.bearer_auth(t);
    }
    let resp = req.send().map_err(|e| e.to_string())?;
    match resp.status().as_u16() {
        200..=299 => Ok(resp),
        404 => Err("not found (repository is private or has no releases)".into()),
        403 | 429 => Err("GitHub rate limit reached — try again later".into()),
        code => Err(format!("GitHub answered {code}")),
    }
}

/// The architecture tag used in installer names: x64, x86 or arm64.
fn arch_tag() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86" => "x86",
        _ => "x64",
    }
}

/// All releases of an app's repository, newest first, with the installer for this PC.
#[tauri::command]
async fn github_releases(repo: String) -> Result<Vec<Release>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let url = format!("https://api.github.com/repos/{GITHUB_OWNER}/{repo}/releases?per_page=50");
        let text = github_get(&url, "application/vnd.github+json")?.text().map_err(|e| e.to_string())?;
        let list: Vec<serde_json::Value> = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let arch = arch_tag();
        let mut out: Vec<Release> = list
            .iter()
            .filter(|r| !r["draft"].as_bool().unwrap_or(false))
            .map(|r| {
                let assets: Vec<Asset> = r["assets"].as_array().map(|a| a.iter().map(|x| Asset {
                    name: x["name"].as_str().unwrap_or_default().to_string(),
                    url: x["url"].as_str().unwrap_or_default().to_string(),
                    size: x["size"].as_u64().unwrap_or(0),
                }).collect()).unwrap_or_default();
                // GitHub replaces spaces in file names with dots, so match on the ending only.
                let find = |suffix: String| assets.iter().find(|a| a.name.ends_with(&suffix)).cloned();
                let tag = r["tag_name"].as_str().unwrap_or_default();
                Release {
                    version: tag.trim_start_matches('v').to_string(),
                    name: r["name"].as_str().unwrap_or(tag).to_string(),
                    notes: r["body"].as_str().unwrap_or_default().to_string(),
                    published: r["published_at"].as_str().unwrap_or_default().to_string(),
                    prerelease: r["prerelease"].as_bool().unwrap_or(false),
                    setup: find(format!("_{arch}-setup.exe")),
                    portable: find(format!("_{arch}_portable.exe")),
                }
            })
            .collect();
        out.sort_by(|a, b| version_key(&b.version).cmp(&version_key(&a.version)));
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Serialize, Clone)]
struct Progress {
    id: String,
    done: u64,
    total: u64,
}

/// Downloads a release file into %LOCALAPPDATA%\StudioHub\downloads (portable apps stay there) and
/// returns its path. Progress is reported as "download-progress" events.
#[tauri::command]
async fn download_asset(app: AppHandle, id: String, url: String, name: String, size: u64) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use std::io::{Read, Write};
        let base = std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
        let dir = base.join("StudioHub").join("downloads");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let target = dir.join(&name);
        if target.metadata().map(|m| m.len() == size && size > 0).unwrap_or(false) {
            return Ok(target.to_string_lossy().into_owned());
        }
        let mut resp = github_get(&url, "application/octet-stream")?;
        let total = resp.content_length().unwrap_or(size);
        let part = dir.join(format!("{name}.part"));
        let mut file = fs::File::create(&part).map_err(|e| e.to_string())?;
        let (mut buf, mut done, mut last) = (vec![0u8; 64 * 1024], 0u64, 0u64);
        loop {
            let n = resp.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            done += n as u64;
            if done - last > 256 * 1024 || done == total {
                last = done;
                let _ = app.emit("download-progress", Progress { id: id.clone(), done, total });
            }
        }
        drop(file);
        fs::rename(&part, &target).map_err(|e| e.to_string())?;
        Ok(target.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| e.to_string())?
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
            github_releases,
            download_asset,
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
