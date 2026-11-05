# Changelog

## 1.15.0 — 2026-11-05
- New: six themes — Midnight (navy, blue), Dusk (plum, violet), Graphite (neutral gray, orange), High contrast (black, white, yellow — easy to read), Sand (warm beige, terracotta) and Rosé (soft pink, raspberry).
- New: two layouts — **Focus** (roomy, no status bar, bars and panels without lines, the tool rail fades until you point at it) and **Mirrored** (panels on the left, tool rail on the right). Choose them in the settings menu (Theme / Layout) and in Theme per app.

## 1.14.0 — 2026-11-05
- New: CLI theme and CLI layout. Theme "CLI (Terminal)": black, phosphor green, monospace, square corners. Layout "CLI": monospace font, compact, square and flat (no shadows) — goes with every theme. Choose them in the settings menu (Theme / Layout) and in Theme per app. The layout "Custom" also gets the new option Font → Monospace.

## 1.13.0 — 2026-11-05
- New: **Theme per app** (settings menu): each Studio app can look like the hub or have its own theme and layout. Apps pick up a change when their window comes to the front.
- Every app has the same switch in its settings (Appearance → "Like Studio Hub" / "Only for this app"). The language stays shared.

## 1.12.2 — 2026-11-05
- New descriptions: Agent Studio is now for building apps with AI; Plan Studio has your school files and Claude for school.

## 1.12.1 — 2026-11-03
- Fix (macOS): the app no longer asks again and again for access to your Documents folder (or Desktop, Downloads). The app is now signed as a whole, so macOS can remember your answer. After an update, macOS may ask once more.
- Installing an older app version that isn't signed this way: the Hub signs it on the Mac after copying, so it doesn't keep asking either.

## 1.12.0 — 2026-11-03
- New: MCP server. Started with `--mcp`, Studio Hub opens no window and lets Claude, ChatGPT and other MCP clients use it: `list_apps`, `open_app`, `shared_settings`. Installing and removing apps stay with the window; the shared settings are only read. Setup: see README → MCP.

## 1.11.0 — 2026-10-30
- New: Layouts, next to the themes (settings menu; shared by all Studio apps, any layout goes with any theme). **Adobe** is the look so far; **Modern** has rounded, floating panels and more room; **Custom** is yours: panels and tool rail left or right, density, corners, floating panels, status bar.

## 1.10.0 — 2026-10-24
- The Agent Studio card shows what needs you, what runs and what was done today (needs Agent Studio 0.8 or later).

## 1.9.0 — 2026-10-24
- Your own order: drag an app card to another place, or use "Move up" / "Move down" in its ⋯ menu (also on right-click).
  "Reset order" goes back to the standard order. New apps join at the end.
- Pin apps: the pin next to an app's name (it shows when the mouse is over the card) or "Pin to the top" in the menu puts
  the app in a "Pinned" group at the top. Drag apps into or out of that group to pin or unpin them; pinned apps can be
  ordered too. Order and pins are kept on this computer.

## 1.8.1 — 2026-10-24
- Agent Studio shows its icon (it was missing); the icons of Slides, Plan and Viewer Studio are the new ones (fading from
  dark at the top left to lighter at the bottom right).

## 1.8.0 — 2026-10-24
- New in the catalog: Agent Studio (Pre-Alpha) — Claude as your assistant with routines, your Markdown to-do list and
  approvals, under your control. Unlike the other apps it needs the internet and a Claude account.

## 1.7.1 — 2026-10-24
- Plan Studio is stable (1.0) and no longer marked Pre-Alpha.

## 1.7.0 — 2026-10-24
- New: sync folder for two computers (settings menu → "Sync folder for two computers…"). Choose the same folder — e.g. in
  iCloud Drive, OneDrive or Dropbox — on both computers: Plan Studio keeps its plan there (from 0.9.0), so both computers
  have the same timetable, homework and grades. Tip: put the work folder there too, then your files are on both as well.

## 1.6.2 — 2026-10-23
- Fixed: "GitHub rate limit reached". GitHub allows 60 requests an hour per internet connection without signing in, and
  the Hub asked again for every app each time its window came to the front. Now the release lists are kept on disk: a
  list younger than 10 minutes is used as it is, an older one is only checked for changes (an unchanged answer doesn't
  count against the limit), and when GitHub refuses, the last list is shown instead of an error. Without any list the
  Hub says when GitHub answers again.
- Downloads use GitHub's normal download links, which don't count against the limit either.
- A signed-in GitHub CLI is found on macOS too (Homebrew), so the Hub can use its higher limit.

## 1.6.1 — 2026-10-23
- Only app versions (tags like v1.2.3) count as releases, so other downloads in an app's repository (Viewer Studio's FFmpeg) never show up as a version.

## 1.6.0 — 2026-10-18
- New in the catalog: **Viewer Studio** (Pre-Alpha) — look at photos, videos, music and 3D models (STL, OBJ, GLB, 3MF), with sizes and volume for 3D printing.
- Fixed: the “Pre-Alpha” label broke into two lines next to longer app names.

## 1.5.2 — 2026-10-15
- App icon with a picture of what the app does, like Slides and Plan Studio; the window and taskbar icon is now the same picture.
- The cards show each app's icon.

## 1.5.1 — 2026-10-15
- Every app before version 1.0 is marked Pre-Alpha on its card (Slides Studio, Plan Studio, Code Studio).

## 1.5.0 — 2026-10-15
- New apps in the family: Slides Studio (presentations) and Plan Studio (school planner). Apps without a release yet show “no release on GitHub yet” instead of an error.
- Code Studio is marked Pre-Alpha.
- New: work folder for all apps (settings menu ⋯): every Studio app opens and saves in it.
- Theme and language are shared with all Studio apps: changing them here (or in any app) changes them everywhere.

## 1.4.0 — 2026-10-11
- New: Auto-hide header (settings menu ⋯). The header slides in when the mouse touches the top edge of the window.

## 1.3.0 — 2026-10-11
- “Run portable” on Windows now downloads the portable .zip where a release has one (program plus the spell-check dictionaries), unpacks it safely into its own folder and starts the program from there. Older releases still use the bare .exe.

## 1.2.6 — 2026-10-11
- The English and German texts are now language files (src-tauri/i18n) delivered by the app's Rust core instead of being part of the JavaScript.

## 1.2.5 — 2026-10-10
- Everything now comes from GitHub (github.com/Nuharji): installs, updates, portable versions and older versions. The local library folder is gone — no setup needed, and no GitHub login since the repositories are public.
- If GitHub can't be reached, the app cards say so.

## 1.2.4 — 2026-10-10
- Icon fades swapped: dark themes now show the dark fade and light themes the light fade — in the menu bar, on the start screen and in the taskbar.

## 1.2.3 — 2026-10-10
- New color for Studio Hub: violet, so it can't be mistaken for Markdown Studio's blue.


## 1.2.2 — 2026-10-10
- New app in the list: Code Studio, the IDE of the Studio family.

## 1.2.1 — 2026-10-10
- The app tiles use the new Studio icon colors (dark-to-bright fade, like the Hub icon).

## 1.2.0 — 2026-10-10
- Studio Hub now runs on macOS and Linux too: installs apps from GitHub (.dmg into Applications, AppImage with a menu entry), updates and uninstalls them.
- The Hub can update itself on every platform.

## 1.1.0 — 2026-10-10
- Install and update apps directly from GitHub Releases — no local files needed.
- Change log for every app, and install older versions from the version history.

## 1.0.0 — 2026-10-10
- First release: lists the Studio apps, install, update, open, uninstall and try without installing.