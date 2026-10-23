# Studio Hub

Launcher and installer for the Studio family apps (Rust / Tauri 2). Lists the apps, installs/updates them silently, opens and uninstalls them.

## The Studio apps

Every app works offline, without an account, in German and English — and Studio Hub installs and updates all of them.

| App | What it does | Downloads |
|---|---|---|
| [Markdown Studio](https://github.com/Nuharji/markdown-studio) | Markdown & LaTeX editor with live preview, diagrams, sources and Word / PDF export | [Releases](https://github.com/Nuharji/markdown-studio/releases) |
| [Data Studio](https://github.com/Nuharji/data-studio) | CSV / JSON / Excel viewer and editor with formulas, charts, pivot tables and function plots | [Releases](https://github.com/Nuharji/data-studio/releases) |
| [PDF Studio](https://github.com/Nuharji/pdf-studio) | PDF viewer and editor: annotate, reorder, merge, fill in forms, sign, recognize text | [Releases](https://github.com/Nuharji/pdf-studio/releases) |
| [Slides Studio](https://github.com/Nuharji/slides-studio) | Presentations with layouts, designs, tables, presenter view and PowerPoint export *(Pre-Alpha)* | [Releases](https://github.com/Nuharji/slides-studio/releases) |
| [Plan Studio](https://github.com/Nuharji/plan-studio) | School planner: timetable, homework, exams, grades, electives — also in your own calendar *(Pre-Alpha)* | [Releases](https://github.com/Nuharji/plan-studio/releases) |
| [Viewer Studio](https://github.com/Nuharji/viewer-studio) | Look at photos, videos, music and 3D models *(Pre-Alpha)* | [Releases](https://github.com/Nuharji/viewer-studio/releases) |
| [Code Studio](https://github.com/Nuharji/code-studio) | Code editor *(Pre-Alpha, on hold)* | [Releases](https://github.com/Nuharji/code-studio/releases) |

- UI: `ui/`, app catalog in `ui/app.js` (`CATALOG`)
- Native backend: `src-tauri/src/main.rs` (library scan, Windows uninstall registry, silent NSIS install/uninstall)

## Build (Windows x64)

```powershell
.\build-windows.ps1
```
