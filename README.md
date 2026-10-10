# Studio Hub

Launcher and installer for the Studio family apps (Rust / Tauri 2). Lists the apps, installs/updates them silently, opens and uninstalls them.

- UI: `ui/`, app catalog in `ui/app.js` (`CATALOG`)
- Native backend: `src-tauri/src/main.rs` (library scan, Windows uninstall registry, silent NSIS install/uninstall)

## Build (Windows x64)

```powershell
.\build-windows.ps1
```
