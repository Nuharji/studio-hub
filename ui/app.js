'use strict';
const $ = s => document.querySelector(s), $$ = s => [...document.querySelectorAll(s)];
const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : null;
const esc = s => String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const STORE = 'studio-hub.v1';

/* The family. `repo` is the GitHub repository (owner is set in the Rust backend).
   Everything is downloaded from the apps' GitHub releases (github.com/Nuharji). */
const CATALOG = {
  'Markdown Studio': {code: 'Md', g: ['#1B2F6B', '#378EF0'], gl: ['#378EF0', '#9CCBFF'], repo: 'markdown-studio',
    en: 'Markdown & LaTeX editor with live preview, diagrams and a full syntax guide.',
    de: 'Markdown- & LaTeX-Editor mit Live-Vorschau, Diagrammen und Syntax-Anleitung.'},
  'PDF Studio': {code: 'Pd', g: ['#6B1420', '#F0485C'], gl: ['#E8394F', '#FF9EAA'], repo: 'pdf-studio',
    en: 'View, search, annotate, reorder, merge and split PDF files.',
    de: 'PDFs ansehen, durchsuchen, kommentieren, sortieren, zusammenführen und teilen.'},
  'Data Studio': {code: 'Dt', g: ['#0A4535', '#1FBF8F'], gl: ['#14A97A', '#86EBC8'], repo: 'data-studio',
    en: 'CSV, JSON and Excel: filter, sort, chart, edit and convert.',
    de: 'CSV, JSON und Excel: filtern, sortieren, Diagramme, bearbeiten und umwandeln.'},
  'Slides Studio': {code: 'Sl', g: ['#5C0B45', '#E0369A'], gl: ['#D6338E', '#FFA6D6'], repo: 'slides-studio', stage: 'preAlpha',
    en: 'Presentations with layouts, designs, shapes and tables; present full screen with a presenter view, export to PowerPoint and PDF.',
    de: 'Präsentationen mit Layouts, Designs, Formen und Tabellen; Vollbild mit Referentenansicht, Export als PowerPoint und PDF.'},
  'Plan Studio': {code: 'Pl', g: ['#5A4400', '#E6B800'], gl: ['#E0A800', '#FFE27A'], repo: 'plan-studio', stage: 'preAlpha',
    en: 'School planner: timetable, homework, exams and deadlines — with a calendar export.',
    de: 'Schulplaner: Stundenplan, Hausaufgaben, Klausuren und Termine — mit Kalender-Export.'},
  'Viewer Studio': {code: 'Vw', g: ['#3B1D7A', '#8B5CF6'], gl: ['#7C3AED', '#C4B5FD'], repo: 'viewer-studio', stage: 'preAlpha',
    en: 'Look at photos, videos, music and 3D models (STL, OBJ, GLB, 3MF) — with sizes and volume for 3D printing.',
    de: 'Fotos, Videos, Musik und 3D-Modelle (STL, OBJ, GLB, 3MF) ansehen — mit Maßen und Volumen für den 3D-Druck.'},
  'Code Studio': {code: 'Cd', g: ['#5A2A05', '#F58A1F'], gl: ['#EE7A12', '#FFC27E'], repo: 'code-studio', stage: 'preAlpha',
    en: 'IDE in the JetBrains style: projects, Search Everywhere, find in files, terminal, local history.',
    de: 'IDE im JetBrains-Stil: Projekte, Überall suchen, In Dateien suchen, Terminal, lokaler Verlauf.'}
};
const SOON = [];   // apps in development, e.g. {name, code, g: [from, to], en, de}
const SELF = 'Studio Hub';

/* The texts (English / German) live in src-tauri/i18n/*.json and are loaded from Rust at start. */
let I18N = {en: {}, de: {}};
let settings = {lang: 'auto', theme: 'dark'}, lang = 'en', ownVersion = '', platform = 'windows';
try { Object.assign(settings, JSON.parse(localStorage.getItem(STORE)) || {}); } catch (e) {}
const save = () => { try { localStorage.setItem(STORE, JSON.stringify(settings)); } catch (e) {} };
function t(k, vars) {
  let s = I18N[lang][k] ?? I18N.en[k] ?? k;
  if (vars) s = s.replace(/\{(\w+)\}/g, (m, x) => vars[x] ?? m);
  return s;
}
const cmpVer = (a, b) => {
  const x = String(a).split('.').map(Number), y = String(b).split('.').map(Number);
  for (let i = 0; i < Math.max(x.length, y.length); i++) { const d = (x[i] || 0) - (y[i] || 0); if (d) return Math.sign(d); }
  return 0;
};
let toastT;
function toast(msg, error = false) {
  const el = $('#toast'); el.textContent = msg; el.className = 'toast show' + (error ? ' error' : '');
  clearTimeout(toastT); toastT = setTimeout(() => el.classList.remove('show'), error ? 6000 : 3000);
}

/* ================= State =================
   releases: GitHub releases per app · installed: what is installed on this computer */
let releases = {}, githubErrors = {}, installed = {}, busy = {};
const repoOf = name => (CATALOG[name] || (name === SELF ? {repo: 'studio-hub'} : {})).repo;
async function fetchReleases() {
  const names = [...Object.keys(CATALOG), SELF];
  await Promise.all(names.map(async n => {
    try { releases[n] = await invoke('github_releases', {repo: repoOf(n)}); delete githubErrors[n]; }
    catch (e) { releases[n] = []; if (/not found/i.test(String(e))) delete githubErrors[n]; else githubErrors[n] = String(e); }
  }));
}
async function scan() {
  const btn = $('#refreshBtn'); btn.classList.add('spin');
  try {
    if (invoke) {
      const [inst] = await Promise.all([invoke('installed_apps'), fetchReleases()]);
      const names = new Set(Object.keys(CATALOG));
      installed = Object.fromEntries(inst.filter(i => names.has(i.product)).map(i => [i.product, i]));
    }
  } catch (e) { toast(String(e), true); }
  btn.classList.remove('spin');
  render();
}
/* The newest installable version: the latest GitHub release with an installer for this system. */
function best(name) {
  const rel = (releases[name] || []).find(r => r.setup && !r.prerelease);
  return rel ? {version: rel.version, release: rel} : null;
}
function stateOf(name) {
  const b = best(name), inst = installed[name];
  if (busy[name]) return 'busy';
  if (inst && b && cmpVer(b.version, inst.version) > 0) return 'update';
  if (inst) return 'installed';
  if (b) return 'available';
  return 'missing';
}

/* ================= Rendering ================= */
/* The app's own icon (ui/icons/<code>.png, the same picture as the installed app); apps without one get a letter tile. */
function tile(name, c = CATALOG[name] || {code: name.replace(/[^A-Za-z]/g, '').slice(0, 2), g: ['#555', '#888']}) {
  if (CATALOG[name]) return `<img class="tile icon" src="icons/${esc(c.code.toLowerCase())}.png" alt="">`;
  const code = c.code.charAt(0).toUpperCase() + c.code.slice(1).toLowerCase();
  // Dark themes show the dark fade, light themes the light fade (same as each app's own logo).
  const g = c.gl && (settings.theme === 'light' || settings.theme === 'github') ? c.gl : c.g;
  return `<div class="tile" style="background:linear-gradient(135deg,${g[0]},${g[1]})">${esc(code)}</div>`;
}
/* Apps before version 1.0 are marked Pre-Alpha (from the newest release, else the installed version);
   the catalog can set it too. */
function stageOf(name) {
  const c = CATALOG[name] || {}, b = best(name), inst = installed[name];
  const v = (b && b.version) || (inst && inst.version);
  return c.stage || (v && cmpVer(v, '1.0.0') < 0 ? 'preAlpha' : null);
}
function card(name) {
  const st = stateOf(name), b = best(name), inst = installed[name], c = CATALOG[name] || {};
  const src = '';
  const status = {
    busy: `<span class="dot busy"></span>${esc(busy[name] && busy[name].text || '')}`,
    update: `<span class="dot update"></span>${esc(t('updateAvail', {v: inst && inst.version, n: b && b.version}) + src)}`,
    installed: `<span class="dot ok"></span>${esc(t('installed', {v: inst && inst.version}))}`,
    available: `<span class="dot"></span>${esc(t('available', {v: b && b.version}) + src)}`,
    missing: `<span class="dot"></span>${esc(t(githubErrors[name] ? 'offline' : 'missing'))}`
  }[st];
  const btn = (act, label, primary = false, disabled = false) =>
    `<button class="btn${primary ? ' primary' : ''}" data-act="${act}" data-name="${esc(name)}"${disabled ? ' disabled' : ''}>${esc(t(label))}</button>`;
  const portable = b && b.release.portable;
  let actions = '';
  if (st === 'busy') actions = `<button class="btn primary" disabled>${esc(busy[name].text)}</button>`;
  else if (st === 'update') actions = btn('install', 'update', true) + `<div class="row">${btn('open', 'open', false, !inst.exe)}${btn('more', '⋯')}</div>`;
  else if (st === 'installed') actions = btn('open', 'open', true, !inst.exe) + `<div class="row">${btn('more', '⋯')}</div>`;
  else if (st === 'available') actions = btn('install', 'install', true) + `<div class="row">${btn('portable', 'portable', false, !portable)}${btn('more', '⋯')}</div>`;
  else actions = `<button class="btn primary" disabled>${esc(t('install'))}</button><div class="row">${btn('more', '⋯')}</div>`;
  return `<article class="card" data-name="${esc(name)}">${tile(name)}
    <div class="info"><h2>${esc(name)}${stageOf(name) ? ` <span class="tag stage">${esc(t(stageOf(name)))}</span>` : ''}</h2><p>${esc(c[lang] || c.en || '')}</p><div class="status">${status}</div></div>
    <div class="actions">${actions}</div></article>`;
}
function render() {
  document.documentElement.lang = lang;
  document.documentElement.dataset.theme = settings.theme;
  $('#tagline').textContent = t('tagline');
  $('#refreshBtn').title = t('refresh'); $('#menuBtn').title = t('menu');
  $('#appsTitle').textContent = t('apps'); $('#soonTitle').textContent = t('soon');
  const names = Object.keys(CATALOG).filter(n => n !== SELF);
  $('#apps').innerHTML = names.length ? names.map(card).join('') : `<div class="empty">${t('noApps')}</div>`;
  const soon = SOON.filter(s => !best(s.name));
  $('#soonTitle').hidden = !soon.length;
  $('#soon').innerHTML = soon.map(s => `<article class="card soon">${tile(s.name, s)}
    <div class="info"><h2>${esc(s.name)}</h2><p>${esc(s[lang] || s.en)}</p><div class="status"><span class="dot"></span>${esc(t('devStatus'))}</div></div><div></div></article>`).join('');
  // Newer Studio Hub on GitHub?
  const selfRel = (releases[SELF] || []).find(r => r.setup && !r.prerelease);
  const banner = $('#selfUpdate');
  banner.hidden = !(selfRel && ownVersion && cmpVer(selfRel.version, ownVersion) > 0);
  if (!banner.hidden) banner.innerHTML = `<span>${esc(t('selfUpdate', {v: selfRel.version}))}</span>
    <button class="btn small" data-act="history" data-name="${SELF}">${esc(t('selfHistory'))}</button>
    <button class="btn small primary" data-act="selfUpdate" data-name="${SELF}">${esc(t('update'))}</button>`;
}

/* ================= Downloads & installs ================= */
let dlSeq = 0;
const progress = {};
if (invoke) window.__TAURI__.event.listen('download-progress', ev => {
  const {id, done, total} = ev.payload, name = progress[id];
  if (!name || !busy[name]) return;
  busy[name].text = t('downloading', {p: total ? Math.round(done / total * 100) : '…'});
  const s = $(`.card[data-name="${CSS.escape(name)}"] .status`), b = $(`.card[data-name="${CSS.escape(name)}"] .actions .btn`);
  if (s) s.innerHTML = `<span class="dot busy"></span>${esc(busy[name].text)}`;
  if (b) b.textContent = busy[name].text;
});
async function download(name, asset) {
  const id = String(++dlSeq); progress[id] = name;
  try { return await invoke('download_asset', {id, url: asset.url, name: asset.name, size: asset.size}); }
  finally { delete progress[id]; }
}
/* Installs the newest version, or a specific GitHub release (older versions from the history). */
async function doInstall(name, release = null) {
  const b = release ? {version: release.version, release} : best(name); if (!b) return;
  const wasInstalled = !!installed[name];
  busy[name] = {text: t(wasInstalled ? 'updating' : 'installing')}; render();
  try {
    const setup = await download(name, b.release.setup);
    busy[name].text = t(wasInstalled ? 'updating' : 'installing'); render();
    await invoke('install', {setup, product: name, version: b.version});
    toast(wasInstalled ? t('updated', {name, v: b.version}) : t('done', {name, v: b.version}));
  } catch (e) { toast(t('failed', {name, e}), true); }
  delete busy[name]; await scan();
}
async function doPortable(name) {
  const b = best(name); if (!b) return;
  try {
    busy[name] = {text: t('downloading', {p: 0})}; render();
    const path = await download(name, b.release.portable);
    delete busy[name]; render();
    await start(name, path);
  } catch (e) { delete busy[name]; render(); toast(t('failed', {name, e}), true); }
}
/* Updating the Hub itself. Windows: the installer has to replace this running program, so it runs visibly
   and the Hub closes. macOS/Linux: the new .app / AppImage is put in place; it takes effect on the next start. */
async function selfUpdate() {
  const rel = (releases[SELF] || []).find(r => r.setup && !r.prerelease); if (!rel) return;
  try {
    busy[SELF] = {text: ''}; toast(t('downloading', {p: '…'}));
    const path = await download(SELF, rel.setup);
    if (platform === 'windows') {
      await invoke('launch', {path});
      window.__TAURI__.window.getCurrentWindow().close();
    } else {
      await invoke('install', {setup: path, product: SELF, version: rel.version});
      toast(t('restartHub', {v: rel.version}));
    }
  } catch (e) { toast(t('failed', {name: SELF, e}), true); }
  delete busy[SELF];
}
async function doUninstall(name) {
  const inst = installed[name]; if (!inst) return;
  const ok = await invoke('confirm', {title: t('confirmTitle'), message: t('confirmMsg', {name}), okLabel: t('confirmOk'), cancelLabel: t('cancel')});
  if (!ok) return;
  busy[name] = {text: t('removing')}; render();
  try { await invoke('uninstall', {command: inst.uninstall, product: name}); toast(t('removed', {name})); }
  catch (e) { toast(t('failed', {name, e}), true); }
  delete busy[name]; await scan();
}
async function start(name, path) {
  try { await invoke('launch', {path}); } catch (e) { toast(t('openFail', {name, e}), true); }
}

/* ================= Changelog & version history ================= */
function notesHtml(md) {
  if (!md || !md.trim()) return `<p class="dim">${esc(t('noNotes'))}</p>`;
  const inline = s => esc(s).replace(/\*\*(.+?)\*\*/g, '<b>$1</b>').replace(/`([^`]+)`/g, '<code>$1</code>');
  let html = '', list = false;
  for (const raw of md.replace(/\r/g, '').split('\n')) {
    const line = raw.trim();
    const item = /^[-*] (.*)/.exec(line);
    if (item) { if (!list) { html += '<ul>'; list = true; } html += `<li>${inline(item[1])}</li>`; continue; }
    if (list) { html += '</ul>'; list = false; }
    if (!line) continue;
    const h = /^#{1,6} (.*)/.exec(line);
    html += h ? `<h4>${inline(h[1])}</h4>` : `<p>${inline(line)}</p>`;
  }
  return html + (list ? '</ul>' : '');
}
function showHistory(name) {
  const list = releases[name] || [], inst = installed[name], err = githubErrors[name];
  const dateOf = s => s ? new Date(s).toLocaleDateString(lang, {dateStyle: 'medium'}) : '';
  const items = list.map((r, i) => {
    const isCur = inst && cmpVer(inst.version, r.version) === 0;
    const tags = (i === 0 ? `<span class="tag">${esc(t('latest'))}</span>` : '') + (isCur ? `<span class="tag ok">${esc(t('current'))}</span>` : '');
    const btn = r.setup && name !== SELF ? `<button class="btn small${isCur ? '' : ' primary'}" data-ver="${esc(r.version)}">${esc(t(isCur ? 'reinstall' : 'installThis'))}</button>` : '';
    return `<section class="ver"><div class="ver-head"><b>${esc(r.version)}</b>${tags}<span class="dim">${esc(dateOf(r.published))}</span><span class="flex"></span>${btn}</div>${notesHtml(r.notes)}</section>`;
  }).join('');
  $('#modalTitle').textContent = t('historyTitle', {name});
  $('#modalBody').innerHTML = items || `<p class="dim">${esc(err ? t('githubError', {e: err}) : t('noReleases'))}</p>`;
  $('#modalBody').onclick = e => {
    const b = e.target.closest('[data-ver]'); if (!b) return;
    const rel = list.find(r => r.version === b.dataset.ver);
    closeModal(); doInstall(name, rel);
  };
  $('#modal').hidden = false;
}
const closeModal = () => { $('#modal').hidden = true; };
$('#modalClose').onclick = closeModal;
$('#modal').addEventListener('mousedown', e => { if (e.target.id === 'modal') closeModal(); });
window.addEventListener('keydown', e => { if (e.key === 'Escape') closeModal(); });

/* ================= Menus & actions ================= */
function moreMenu(name) {
  const inst = installed[name], b = best(name);
  const portable = b && b.release.portable;
  return [
    {id: `open|${name}`, label: t('open'), enabled: !!(inst && inst.exe)},
    {id: `install|${name}`, label: t(inst ? 'update' : 'install'), enabled: !!b && (!inst || cmpVer(b.version, inst.version) > 0)},
    {id: `portable|${name}`, label: t('portable'), enabled: !!portable}, null,
    {id: `history|${name}`, label: t('history'), enabled: !!repoOf(name)}, null,
    {id: `reveal|${name}`, label: t('reveal'), enabled: !!(inst && inst.location)},
    null,
    {id: `uninstall|${name}`, label: t('uninstall'), enabled: !!inst}
  ];
}
function settingsMenu() {
  const th = [['dark', 'themeDark'], ['light', 'themeLight'], ['github', 'themeGithub'], ['github-dark', 'themeGithubDark']];
  return [
    {id: 'lang|auto', label: `${t('language')}: ${t('sysLang')}`, checked: settings.lang === 'auto'},
    {id: 'lang|en', label: `${t('language')}: English`, checked: settings.lang === 'en'},
    {id: 'lang|de', label: `${t('language')}: Deutsch`, checked: settings.lang === 'de'}, null,
    ...th.map(([v, k]) => ({id: `theme|${v}`, label: `${t('theme')}: ${t(k)}`, checked: settings.theme === v})), null,
    {id: 'autoHide|', label: t('autoHide'), checked: !!settings.autoHide}, null,
    {id: 'workFolder|', label: workFolder() ? t('workFolderIs', {p: workFolder()}) : t('workFolderPick')},
    {id: 'workFolderClear|', label: t('workFolderClear'), enabled: !!workFolder()}, null,
    {id: 'github|', label: 'github.com/Nuharji'}, {id: `history|${SELF}`, label: t('selfHistory')}, {id: 'about|', label: t('about')}
  ];
}
async function act(action, name) {
  const inst = installed[name];
  switch (action) {
    case 'open': if (inst && inst.exe) await start(name, inst.exe); break;
    case 'install': await doInstall(name); break;
    case 'portable': await doPortable(name); break;
    case 'uninstall': await doUninstall(name); break;
    case 'history': showHistory(name); break;
    case 'selfUpdate': await selfUpdate(); break;
    case 'reveal': if (inst) invoke('reveal', {path: inst.exe || inst.location}); break;

    case 'lang': settings.lang = name; applyLang(); save(); render(); sharePush(); break;
    case 'theme': settings.theme = name; save(); render(); sharePush(); break;
    case 'workFolder': {
      const p = await invoke('pick_folder');
      if (p) { window.Shared && Shared.push({workFolder: p}); toast(t('workFolderSet', {p})); }
      break;
    }
    case 'workFolderClear': window.Shared && Shared.push({workFolder: null}); toast(t('workFolderCleared')); break;
    case 'autoHide': settings.autoHide = !settings.autoHide; save(); if (window.AutoHide) AutoHide.set(settings.autoHide); break;
    case 'github': invoke('open_url', {url: 'https://github.com/Nuharji'}); break;
    case 'about': toast(t('aboutText', {v: ownVersion})); break;
  }
}
document.addEventListener('click', e => {
  const b = e.target.closest('[data-act]');
  if (!b || b.disabled) return;
  if (b.dataset.act === 'more') return showMenu(moreMenu(b.dataset.name));
  act(b.dataset.act, b.dataset.name);
});
$('#lib').onclick = () => act('github');
$('#refreshBtn').onclick = scan;
$('#menuBtn').onclick = () => showMenu(settingsMenu());
function showMenu(items) {
  if (!invoke) return;
  invoke('context_menu', {items: items.map(i => i ? {id: i.id, label: i.label, enabled: i.enabled !== false, checked: i.checked, accel: null} : {})});
}
if (window.Native) {
  Native.setProvider(e => { const c = e.target.closest('.card[data-name]'); return c ? moreMenu(c.dataset.name) : null; });
  Native.setHandler(id => { const [a, n] = id.split('|'); act(a, n); });
}
/* Theme, language and work folder are shared by all Studio apps (shared.js / shared.rs). */
const workFolder = () => (window.Shared && Shared.values.workFolder) || '';
const sharePush = () => { if (window.Shared) Shared.push({theme: settings.theme, lang: settings.lang}); };
function applyLang() {
  lang = settings.lang === 'auto' ? ((navigator.language || 'en').toLowerCase().startsWith('de') ? 'de' : 'en') : settings.lang;
}
// Re-check when the window comes back to the front (an app may have been installed or removed meanwhile).
let lastFocusScan = 0;
window.addEventListener('focus', () => {
  if (Object.keys(busy).length || Date.now() - lastFocusScan < 60000) return;
  lastFocusScan = Date.now(); scan();
});

// Auto-hide: the header slides in at the top edge (autohide.js).
if (window.AutoHide) { AutoHide.init({top: ['header']}); AutoHide.set(settings.autoHide); }

(async () => {
  if (invoke) I18N = await invoke('texts');
  if (window.Shared) await Shared.init({apply: sh => {
    if (sh.theme) settings.theme = sh.theme;
    if (sh.lang) settings.lang = sh.lang;
    applyLang(); save(); render();
  }});
  applyLang(); render(); sharePush();
  if (invoke) {
    ownVersion = await window.__TAURI__.app.getVersion().catch(() => '');
    platform = await invoke('platform').catch(() => 'windows');
  }
  lastFocusScan = Date.now(); scan();
})();
window.__hub = {scan, act, doInstall, state: () => ({releases, githubErrors, installed, busy})};
