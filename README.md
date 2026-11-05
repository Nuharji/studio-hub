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
| [Plan Studio](https://github.com/Nuharji/plan-studio) | School planner: timetable, homework, exams, grades, electives, study plans — also in your own calendar and on two computers | [Releases](https://github.com/Nuharji/plan-studio/releases) |
| [Viewer Studio](https://github.com/Nuharji/viewer-studio) | Look at photos, videos, music and 3D models *(Pre-Alpha)* | [Releases](https://github.com/Nuharji/viewer-studio/releases) |
| [Agent Studio](https://github.com/Nuharji/agent-studio) | Claude as your assistant: routines, your Markdown to-do list, approvals, live log and emergency stop *(Pre-Alpha, needs a Claude account)* | [Releases](https://github.com/Nuharji/agent-studio/releases) |
| [Code Studio](https://github.com/Nuharji/code-studio) | Code editor *(Pre-Alpha, on hold)* | [Releases](https://github.com/Nuharji/code-studio/releases) |

- UI: `ui/`, app catalog in `ui/app.js` (`CATALOG`)
- Native backend: `src-tauri/src/main.rs` (library scan, Windows uninstall registry, silent NSIS install/uninstall)

## MCP
Studio Hub is also an MCP server, so Claude (Desktop or Code), ChatGPT and other MCP clients can use what it can do:
started with `--mcp`, the program opens no window and speaks MCP on stdin / stdout until the client closes it.

- `list_apps` — The Studio apps installed on this computer, with their versions.
- `open_app` — Starts an installed Studio app, or brings it to the front.
- `shared_settings` — The settings all Studio apps share: theme, layout, language, work folder, sync folder (and `apps`: the apps with their own theme / layout).

Installing and removing apps stay with the window; the shared settings are only read.

Claude Code: `claude mcp add studio-hub -- "/Applications/Studio Hub.app/Contents/MacOS/studio-hub" --mcp`.
Claude Desktop (`claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "studio-hub": {
      "command": "/Applications/Studio Hub.app/Contents/MacOS/studio-hub",
      "args": [
        "--mcp"
      ]
    }
  }
}
```
The program is `/Applications/Studio Hub.app/Contents/MacOS/studio-hub` on macOS, `studio-hub.exe` in the install folder on
Windows, and `studio-hub` (or the AppImage) on Linux.

## Build (Windows x64)

```powershell
.\build-windows.ps1
```
