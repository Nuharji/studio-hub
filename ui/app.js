'use strict';
const $ = s => document.querySelector(s), $$ = s => [...document.querySelectorAll(s)];
const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : null;
const esc = s => String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const STORE = 'studio-hub.v1';

/* The family. Apps found in the library that aren't listed here still show up, with a generic tile. */
const CATALOG = {
  'Markdown Studio': {code: 'Md', g: ['#378EF0', '#7B4DFF'],
    en: 'Markdown & LaTeX editor with live preview, diagrams and a full syntax guide.',
    de: 'Markdown- & LaTeX-Editor mit Live-Vorschau, Diagrammen und Syntax-Anleitung.'},
  'PDF Studio': {code: 'Pd', g: ['#E5384F', '#FF6B5A'],
    en: 'View, search, annotate, reorder, merge and split PDF files.',
    de: 'PDFs ansehen, durchsuchen, kommentieren, sortieren, zusammenführen und teilen.'},
  'Data Studio': {code: 'Dt', g: ['#12A06B', '#22B8B0'],
    en: 'CSV, JSON and Excel: filter, sort, chart, edit and convert.',
    de: 'CSV, JSON und Excel: filtern, sortieren, Diagramme, bearbeiten und umwandeln.'}
};
const SOON = [
  {name: 'Code Studio', code: 'Cd', g: ['#E8590C', '#F59F00'],
    en: 'Code editor (IDE) with folder tree, search, terminal and Git.', de: 'Code-Editor (IDE) mit Ordnerbaum, Suche, Terminal und Git.'}
];
const SELF = 'Studio Hub';

const I18N = {
  en: {
    tagline: 'Install, update and open your Studio apps', library: 'Library', change: 'Change…', refresh: 'Check again',
    menu: 'Settings', apps: 'Your apps', soon: 'In development', open: 'Open', install: 'Install', update: 'Update',
    uninstall: 'Uninstall', portable: 'Try without installing', reveal: 'Show in folder', revealSetup: 'Show installer',
    installed: 'Installed · {v}', updateAvail: 'Installed {v} · update to {n} available', available: 'Not installed · {v} ready to install',
    missing: 'No installer in the library', installing: 'Installing…', updating: 'Updating…', removing: 'Uninstalling…',
    devStatus: 'Coming soon', noApps: 'No Studio apps found in this folder.<br>Choose the folder that contains the app projects (with their <b>dist</b> folders).',
    done: '{name} installed', updated: '{name} updated to {v}', removed: '{name} uninstalled', failed: '{name}: {e}',
    confirmTitle: 'Uninstall', confirmMsg: 'Uninstall {name}? Your documents are not touched.', confirmOk: 'Uninstall', cancel: 'Cancel',
    language: 'Language', theme: 'Theme', sysLang: 'System language', themeDark: 'Dark', themeLight: 'Light', themeGithub: 'GitHub',
    themeGithubDark: 'GitHub Dark', about: 'About Studio Hub', aboutText: 'Studio Hub {v} — part of the Studio family.', openFail: 'Could not start {name}: {e}'
  },
  de: {
    tagline: 'Studio-Apps installieren, aktualisieren und öffnen', library: 'Bibliothek', change: 'Ändern…', refresh: 'Erneut prüfen',
    menu: 'Einstellungen', apps: 'Deine Apps', soon: 'In Entwicklung', open: 'Öffnen', install: 'Installieren', update: 'Aktualisieren',
    uninstall: 'Deinstallieren', portable: 'Ohne Installation testen', reveal: 'Im Ordner anzeigen', revealSetup: 'Installationsdatei anzeigen',
    installed: 'Installiert · {v}', updateAvail: 'Installiert {v} · Update auf {n} verfügbar', available: 'Nicht installiert · {v} bereit',
    missing: 'Kein Installationsprogramm in der Bibliothek', installing: 'Wird installiert…', updating: 'Wird aktualisiert…',
    removing: 'Wird deinstalliert…', devStatus: 'Demnächst',
    noApps: 'In diesem Ordner wurden keine Studio-Apps gefunden.<br>Wähle den Ordner mit den App-Projekten (mit ihren <b>dist</b>-Ordnern).',
    done: '{name} installiert', updated: '{name} auf {v} aktualisiert', removed: '{name} deinstalliert', failed: '{name}: {e}',
    confirmTitle: 'Deinstallieren', confirmMsg: '{name} deinstallieren? Deine Dokumente bleiben erhalten.', confirmOk: 'Deinstallieren',
    cancel: 'Abbrechen', language: 'Sprache', theme: 'Design', sysLang: 'Systemsprache', themeDark: 'Dunkel', themeLight: 'Hell',
    themeGithub: 'GitHub', themeGithubDark: 'GitHub Dunkel', about: 'Über Studio Hub', aboutText: 'Studio Hub {v} – Teil der Studio-Familie.',
    openFail: '{name} konnte nicht gestartet werden: {e}'
  }
};
let settings = {lang: 'auto', theme: 'dark', library: 'C:\\ai-stuff'}, lang = 'en';
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

/* ================= State ================= */
let packages = {}, installed = {}, busy = {};
async function scan() {
  const btn = $('#refreshBtn'); btn.classList.add('spin');
  try {
    const [pk, inst] = invoke ? await Promise.all([invoke('scan_library', {dir: settings.library}), invoke('installed_apps')]) : [[], []];
    packages = Object.fromEntries(pk.map(p => [p.product, p]));
    const names = new Set([...Object.keys(CATALOG), ...Object.keys(packages)]);
    installed = Object.fromEntries(inst.filter(i => names.has(i.product)).map(i => [i.product, i]));
  } catch (e) { toast(String(e), true); }
  btn.classList.remove('spin');
  render();
}
function stateOf(name) {
  const pkg = packages[name], inst = installed[name];
  if (busy[name]) return 'busy';
  if (inst && pkg && cmpVer(pkg.version, inst.version) > 0) return 'update';
  if (inst) return 'installed';
  if (pkg) return 'available';
  return 'missing';
}

/* ================= Rendering ================= */
function tile(name, c = CATALOG[name] || {code: name.replace(/[^A-Za-z]/g, '').slice(0, 2), g: ['#555', '#888']}) {
  const code = c.code.charAt(0).toUpperCase() + c.code.slice(1).toLowerCase();
  return `<div class="tile" style="background:linear-gradient(135deg,${c.g[0]},${c.g[1]})">${esc(code)}</div>`;
}
function card(name) {
  const st = stateOf(name), pkg = packages[name], inst = installed[name], c = CATALOG[name] || {};
  const status = {
    busy: [`<span class="dot busy"></span>${esc(t(busy[name]))}`],
    update: [`<span class="dot update"></span>${esc(t('updateAvail', {v: inst && inst.version, n: pkg && pkg.version}))}`],
    installed: [`<span class="dot ok"></span>${esc(t('installed', {v: inst && inst.version}))}`],
    available: [`<span class="dot"></span>${esc(t('available', {v: pkg && pkg.version}))}`],
    missing: [`<span class="dot"></span>${esc(t('missing'))}`]
  }[st];
  const b = (act, label, primary = false, disabled = false) =>
    `<button class="btn${primary ? ' primary' : ''}" data-act="${act}" data-name="${esc(name)}"${disabled ? ' disabled' : ''}>${esc(t(label))}</button>`;
  let actions = '';
  if (st === 'busy') actions = b('none', busy[name], true, true);
  else if (st === 'update') actions = b('install', 'update', true) + `<div class="row">${b('open', 'open', false, !inst.exe)}${b('more', '⋯')}</div>`;
  else if (st === 'installed') actions = b('open', 'open', true, !inst.exe) + `<div class="row">${b('more', '⋯')}</div>`;
  else if (st === 'available') actions = b('install', 'install', true) + `<div class="row">${b('portable', 'portable', false, !pkg.portable)}</div>`;
  else actions = b('none', 'install', true, true);
  return `<article class="card" data-name="${esc(name)}">${tile(name)}
    <div class="info"><h2>${esc(name)}</h2><p>${esc(c[lang] || c.en || '')}</p><div class="status">${status}</div></div>
    <div class="actions">${actions}</div></article>`;
}
function render() {
  document.documentElement.lang = lang;
  document.documentElement.dataset.theme = settings.theme;
  $('#tagline').textContent = t('tagline');
  $('#libPath').textContent = settings.library; $('#lib').title = `${t('library')}: ${settings.library}`;
  $('#libBtn').textContent = t('change'); $('#refreshBtn').title = t('refresh'); $('#menuBtn').title = t('menu');
  $('#appsTitle').textContent = t('apps'); $('#soonTitle').textContent = t('soon');
  const names = [...new Set([...Object.keys(CATALOG), ...Object.keys(packages), ...Object.keys(installed)])].filter(n => n !== SELF);
  const shown = names.filter(n => CATALOG[n] || packages[n] || installed[n]);
  $('#apps').innerHTML = shown.length && (Object.keys(packages).length || Object.keys(installed).length)
    ? shown.map(card).join('') : `<div class="empty">${t('noApps')}</div>`;
  const soon = SOON.filter(s => !packages[s.name]);
  $('#soonTitle').hidden = !soon.length;
  $('#soon').innerHTML = soon.map(s => `<article class="card soon">${tile(s.name, s)}
    <div class="info"><h2>${esc(s.name)}</h2><p>${esc(s[lang] || s.en)}</p><div class="status"><span class="dot"></span>${esc(t('devStatus'))}</div></div><div></div></article>`).join('');
}

/* ================= Actions ================= */
async function doInstall(name) {
  const pkg = packages[name], wasInstalled = !!installed[name];
  busy[name] = wasInstalled ? 'updating' : 'installing'; render();
  try {
    await invoke('install', {setup: pkg.setup});
    toast(wasInstalled ? t('updated', {name, v: pkg.version}) : t('done', {name}));
  } catch (e) { toast(t('failed', {name, e}), true); }
  delete busy[name]; await scan();
}
async function doUninstall(name) {
  const inst = installed[name]; if (!inst) return;
  const ok = await invoke('confirm', {title: t('confirmTitle'), message: t('confirmMsg', {name}), okLabel: t('confirmOk'), cancelLabel: t('cancel')});
  if (!ok) return;
  busy[name] = 'removing'; render();
  try { await invoke('uninstall', {command: inst.uninstall, product: name}); toast(t('removed', {name})); }
  catch (e) { toast(t('failed', {name, e}), true); }
  delete busy[name]; await scan();
}
async function start(name, path) {
  try { await invoke('launch', {path}); } catch (e) { toast(t('openFail', {name, e}), true); }
}
function moreMenu(name) {
  const inst = installed[name], pkg = packages[name];
  return [
    {id: `open|${name}`, label: t('open'), enabled: !!(inst && inst.exe)},
    {id: `install|${name}`, label: t(inst ? 'update' : 'install'), enabled: !!pkg && (!inst || cmpVer(pkg.version, inst.version) > 0)},
    {id: `portable|${name}`, label: t('portable'), enabled: !!(pkg && pkg.portable)}, null,
    {id: `reveal|${name}`, label: t('reveal'), enabled: !!(inst && inst.location)},
    {id: `revealSetup|${name}`, label: t('revealSetup'), enabled: !!pkg}, null,
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
    {id: 'library|', label: `${t('library')}: ${t('change')}`}, {id: 'about|', label: t('about')}
  ];
}
async function act(action, name) {
  const inst = installed[name], pkg = packages[name];
  switch (action) {
    case 'open': if (inst && inst.exe) start(name, inst.exe); break;
    case 'install': if (pkg) await doInstall(name); break;
    case 'portable': if (pkg && pkg.portable) await start(name, pkg.portable); break;
    case 'uninstall': await doUninstall(name); break;
    case 'reveal': if (inst) invoke('reveal', {path: inst.exe || inst.location}); break;
    case 'revealSetup': if (pkg) invoke('reveal', {path: pkg.setup}); break;
    case 'lang': settings.lang = name; applyLang(); save(); render(); break;
    case 'theme': settings.theme = name; save(); render(); break;
    case 'library': {
      const dir = await invoke('pick_folder');
      if (dir) { settings.library = dir; save(); scan(); }
      break;
    }
    case 'about': {
      const v = await window.__TAURI__.app.getVersion().catch(() => '');
      toast(t('aboutText', {v})); break;
    }
  }
}
document.addEventListener('click', e => {
  const b = e.target.closest('[data-act]');
  if (!b || b.disabled) return;
  if (b.dataset.act === 'more') return showMenu(moreMenu(b.dataset.name));
  act(b.dataset.act, b.dataset.name);
});
$('#libBtn').onclick = () => act('library');
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
function applyLang() {
  lang = settings.lang === 'auto' ? ((navigator.language || 'en').toLowerCase().startsWith('de') ? 'de' : 'en') : settings.lang;
}
// Re-check when the window comes back to the front (an app may have been installed or removed meanwhile).
window.addEventListener('focus', () => { if (!Object.keys(busy).length) scan(); });

applyLang(); render(); scan();
window.__hub = {scan, act, state: () => ({packages, installed, busy})};
