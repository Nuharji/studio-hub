// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod shared;

use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::{AppHandle, Emitter};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_opener::OpenerExt;

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

/* ===================== Platform ===================== */

#[tauri::command]
fn platform() -> &'static str {
    std::env::consts::OS
}

/// Ending of this platform's installer file: "…_x64-setup.exe", "…_universal.dmg" or "…_amd64.AppImage".
fn setup_suffix() -> String {
    match std::env::consts::OS {
        "windows" => format!("_{}-setup.exe", arch_tag()),
        "macos" => "_universal.dmg".into(),
        _ => format!("_{}.AppImage", if std::env::consts::ARCH == "aarch64" { "aarch64" } else { "amd64" }),
    }
}

/// Endings of a build that runs without installing, best first: on Windows the portable .zip (program plus the
/// spell-check dictionaries), else the bare portable .exe of older releases; on Linux the AppImage itself.
fn portable_suffixes() -> Vec<String> {
    match std::env::consts::OS {
        "windows" => vec![format!("_{}_portable.zip", arch_tag()), format!("_{}_portable.exe", arch_tag())],
        "linux" => vec![setup_suffix()],
        _ => vec![],
    }
}

/// Unpacks a portable .zip into a folder of the same name next to it and returns the program inside.
/// Entries can't leave that folder; an earlier unpacked copy of the same file is reused.
fn unpack_portable(zip_path: &Path) -> Result<PathBuf, String> {
    let dir = zip_path.with_extension("");
    let exe_in = |d: &Path| fs::read_dir(d).ok()?.flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("exe")));
    if let Some(exe) = exe_in(&dir) { return Ok(exe); }
    let tmp = dir.with_extension("unpacking");
    let _ = fs::remove_dir_all(&tmp);
    let mut archive = zip::ZipArchive::new(fs::File::open(zip_path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let Some(rel) = entry.enclosed_name() else { continue };
        let out = tmp.join(rel);
        if entry.is_dir() { fs::create_dir_all(&out).map_err(|e| e.to_string())?; continue; }
        if let Some(p) = out.parent() { fs::create_dir_all(p).map_err(|e| e.to_string())?; }
        let mut f = fs::File::create(&out).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut f).map_err(|e| e.to_string())?;
    }
    let _ = fs::remove_dir_all(&dir);
    fs::rename(&tmp, &dir).map_err(|e| e.to_string())?;
    exe_in(&dir).ok_or_else(|| "no program in the portable .zip".into())
}

fn home() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir())
}

/// $XDG_DATA_HOME, usually ~/.local/share (Linux).
fn xdg_data() -> PathBuf {
    std::env::var("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|_| home().join(".local/share"))
}

/// Where the Hub keeps downloads (and, on Linux, the installed apps).
fn data_dir() -> PathBuf {
    match std::env::consts::OS {
        "windows" => std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir()).join("StudioHub"),
        "macos" => home().join("Library/Application Support/StudioHub"),
        _ => xdg_data().join("StudioHub"),
    }
}


#[cfg(windows)]
fn unquote(s: &str) -> String {
    s.trim().trim_matches('"').to_string()
}

/// The app's main exe in its install folder (anything but the uninstaller).
#[cfg(windows)]
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

/// macOS: the .app bundles in /Applications and ~/Applications, with their bundle version.
#[cfg(target_os = "macos")]
#[tauri::command]
fn installed_apps() -> Vec<Installed> {
    let plist_value = |plist: &str, key: &str| -> Option<String> {
        let rest = &plist[plist.find(&format!("<key>{key}</key>"))?..];
        let start = rest.find("<string>")? + "<string>".len();
        let end = rest[start..].find("</string>")?;
        Some(rest[start..start + end].trim().to_string())
    };
    let mut out = Vec::new();
    for dir in [PathBuf::from("/Applications"), home().join("Applications")] {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for p in entries.flatten().map(|e| e.path()) {
            if p.extension().map_or(true, |x| x != "app") {
                continue;
            }
            let plist = fs::read_to_string(p.join("Contents/Info.plist")).unwrap_or_default();
            let path = p.to_string_lossy().into_owned();
            out.push(Installed {
                product: p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
                version: plist_value(&plist, "CFBundleShortVersionString").unwrap_or_default(),
                location: path.clone(),
                uninstall: path.clone(),
                exe: Some(path),
            });
        }
    }
    out
}

/// Linux: the Hub keeps its own list of the AppImages it installed.
#[cfg(all(unix, not(target_os = "macos")))]
#[derive(Serialize, Deserialize)]
struct LinuxEntry {
    product: String,
    version: String,
    path: String,
    desktop: String,
}

#[cfg(all(unix, not(target_os = "macos")))]
fn read_manifest() -> Vec<LinuxEntry> {
    fs::read_to_string(data_dir().join("installed.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

#[cfg(all(unix, not(target_os = "macos")))]
fn write_manifest(list: &[LinuxEntry]) -> Result<(), String> {
    fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    fs::write(data_dir().join("installed.json"), json).map_err(|e| e.to_string())
}

#[cfg(all(unix, not(target_os = "macos")))]
#[tauri::command]
fn installed_apps() -> Vec<Installed> {
    read_manifest()
        .into_iter()
        .filter(|e| Path::new(&e.path).is_file())
        .map(|e| Installed { product: e.product, version: e.version, location: e.path.clone(), uninstall: e.path.clone(), exe: Some(e.path) })
        .collect()
}

fn run_and_wait(mut cmd: Command) -> Result<(), String> {
    let status = cmd.status().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("exit code {}", status.code().unwrap_or(-1)))
    }
}

/// Installs (or updates) an app from a downloaded or library installer file.
#[tauri::command]
async fn install(setup: String, product: String, version: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || install_impl(&setup, &product, &version))
        .await
        .map_err(|e| e.to_string())?
}

/// Windows: the app's NSIS installer, silently (per-user install, no admin prompt).
#[cfg(windows)]
fn install_impl(setup: &str, _product: &str, _version: &str) -> Result<(), String> {
    let mut cmd = Command::new(setup);
    cmd.arg("/S");
    run_and_wait(cmd)
}

/// macOS: mount the .dmg, copy the .app into /Applications (or ~/Applications), unmount.
#[cfg(target_os = "macos")]
fn install_impl(setup: &str, _product: &str, _version: &str) -> Result<(), String> {
    let writable = |dir: &Path| {
        let probe = dir.join(".studiohub-write-test");
        let ok = fs::write(&probe, b"").is_ok();
        let _ = fs::remove_file(&probe);
        ok
    };
    let mount = std::env::temp_dir().join(format!("studiohub-{}", std::process::id()));
    fs::create_dir_all(&mount).map_err(|e| e.to_string())?;
    let mut attach = Command::new("hdiutil");
    attach.args(["attach", "-nobrowse", "-readonly", "-noautoopen", "-mountpoint"]).arg(&mount).arg(setup);
    run_and_wait(attach)?;
    let result = (|| {
        let app = fs::read_dir(&mount)
            .map_err(|e| e.to_string())?
            .flatten()
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|x| x == "app"))
            .ok_or("the disk image contains no app")?;
        let mut target_dir = PathBuf::from("/Applications");
        if !writable(&target_dir) {
            target_dir = home().join("Applications");
            fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
        }
        let target = target_dir.join(app.file_name().ok_or("bad app name")?);
        if target.exists() {
            fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
        }
        let mut copy = Command::new("ditto");
        copy.arg(&app).arg(&target);
        run_and_wait(copy)
    })();
    let _ = Command::new("hdiutil").args(["detach", "-quiet"]).arg(&mount).status();
    let _ = fs::remove_dir(&mount);
    result
}

/// Linux: keep the AppImage in the Hub's app folder and add it to the application menu.
#[cfg(all(unix, not(target_os = "macos")))]
fn install_impl(setup: &str, product: &str, version: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let dir = data_dir().join("apps");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let target = dir.join(format!("{}.AppImage", product.replace(' ', "-")));
    if Path::new(setup) != target {
        fs::copy(setup, &target).map_err(|e| e.to_string())?;
    }
    fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    let menu = xdg_data().join("applications");
    fs::create_dir_all(&menu).map_err(|e| e.to_string())?;
    let desktop = menu.join(format!("studiohub-{}.desktop", product.to_lowercase().replace(' ', "-")));
    fs::write(&desktop, format!("[Desktop Entry]\nType=Application\nName={product}\nExec=\"{}\" %F\nTerminal=false\nCategories=Office;Utility;\n", target.display()))
        .map_err(|e| e.to_string())?;
    let mut list = read_manifest();
    list.retain(|e| e.product != product);
    list.push(LinuxEntry {
        product: product.into(),
        version: version.into(),
        path: target.to_string_lossy().into_owned(),
        desktop: desktop.to_string_lossy().into_owned(),
    });
    write_manifest(&list)
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

/// macOS: the uninstall "command" is the .app bundle; remove it.
#[cfg(target_os = "macos")]
fn run_uninstaller(command: &str) -> Result<(), String> {
    fs::remove_dir_all(command).map_err(|e| e.to_string())
}

/// Linux: the uninstall "command" is the AppImage; remove it, its menu entry and its list entry.
#[cfg(all(unix, not(target_os = "macos")))]
fn run_uninstaller(command: &str) -> Result<(), String> {
    let mut list = read_manifest();
    if let Some(entry) = list.iter().find(|e| e.path == command) {
        let _ = fs::remove_file(&entry.desktop);
    }
    let _ = fs::remove_file(command);
    list.retain(|e| e.path != command);
    write_manifest(&list)
}

/// Windows: the uninstall command from the registry, made silent.
#[cfg(windows)]
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
        let (setup_end, portable_ends) = (setup_suffix(), portable_suffixes());
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
                let find = |suffix: &str| assets.iter().find(|a| a.name.ends_with(suffix)).cloned();
                let tag = r["tag_name"].as_str().unwrap_or_default();
                Release {
                    version: tag.trim_start_matches('v').to_string(),
                    name: r["name"].as_str().unwrap_or(tag).to_string(),
                    notes: r["body"].as_str().unwrap_or_default().to_string(),
                    published: r["published_at"].as_str().unwrap_or_default().to_string(),
                    prerelease: r["prerelease"].as_bool().unwrap_or(false),
                    setup: find(&setup_end),
                    portable: portable_ends.iter().find_map(|s| find(s)),
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

/// Downloads a release file into the Hub's data folder (…/StudioHub/downloads; portable apps stay there)
/// and returns its path — for a portable .zip, the program in the unpacked folder. Progress is reported as
/// "download-progress" events.
#[tauri::command]
async fn download_asset(app: AppHandle, id: String, url: String, name: String, size: u64) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use std::io::{Read, Write};
        let dir = data_dir().join("downloads");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let target = dir.join(&name);
        let done_path = |p: &Path| -> Result<String, String> {
            let p = if p.extension().is_some_and(|x| x == "zip") { unpack_portable(p)? } else { p.to_path_buf() };
            Ok(p.to_string_lossy().into_owned())
        };
        if target.metadata().map(|m| m.len() == size && size > 0).unwrap_or(false) {
            return done_path(&target);
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
        // A newer download of the same file replaces an earlier unpacked copy.
        let _ = fs::remove_dir_all(target.with_extension(""));
        done_path(&target)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Starts an app (installed exe, .app bundle, AppImage or portable build) without waiting for it.
#[tauri::command]
fn launch(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if cfg!(target_os = "macos") {
        return Command::new("open").arg(&p).spawn().map(|_| ()).map_err(|e| format!("{path}: {e}"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o755));   // downloaded AppImages aren't executable yet
    }
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
    crate::shared::file_dialog(&app)
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

/// The app's texts in English and German (i18n/*.json, built into the program).
#[tauri::command]
fn texts() -> serde_json::Value {
    let parse = |s: &str| serde_json::from_str::<serde_json::Value>(s).unwrap_or_default();
    serde_json::json!({"en": parse(include_str!("../i18n/en.json")), "de": parse(include_str!("../i18n/de.json"))})
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
            shared::shared_get,
            shared::shared_set,
            texts,
            platform,
            context_menu,
            clipboard_read,
            clipboard_write,
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

#[cfg(test)]
mod portable_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn unpacks_portable_zip_safely() {
        let dir = std::env::temp_dir().join(format!("hub-zip-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("Markdown-Studio_9.9.9_x64_portable.zip");
        {
            let mut w = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
            let o = zip::write::SimpleFileOptions::default();
            w.start_file("Markdown-Studio_9.9.9_x64_portable.exe", o).unwrap();
            w.write_all(b"MZ").unwrap();
            w.start_file("dictionaries/en/index.dic", o).unwrap();
            w.write_all(b"1\nhouse\n").unwrap();
            w.start_file("../evil.txt", o).unwrap();
            w.write_all(b"x").unwrap();
            w.finish().unwrap();
        }
        let exe = unpack_portable(&zip_path).unwrap();
        assert_eq!(exe, dir.join("Markdown-Studio_9.9.9_x64_portable").join("Markdown-Studio_9.9.9_x64_portable.exe"));
        assert!(dir.join("Markdown-Studio_9.9.9_x64_portable/dictionaries/en/index.dic").exists());
        assert!(!dir.join("evil.txt").exists() && !std::env::temp_dir().join("evil.txt").exists());
        // A second call reuses the unpacked folder.
        assert_eq!(unpack_portable(&zip_path).unwrap(), exe);
        let _ = fs::remove_dir_all(&dir);
    }
}

