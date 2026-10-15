# Changelog

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