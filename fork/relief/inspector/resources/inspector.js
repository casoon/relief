// Relief. MIT-Lizenz wie das Relief-Repository.
//
// Relief-Panel: Befehlsleiste (oben) und Semantic Inspector. Der Inspector
// zeigt den Graph der Seite (Bereiche, Überschriften,
// Bedienelemente) mit Herkunft der Namen. Auswahl (Pfeiltasten) und
// Aktivierung („Im Dokument zeigen“) sind getrennt; Aktualisierungen der
// Seite werden gebündelt angesagt. Nur DOM-APIs, kein innerHTML (Trusted
// Types der WebUI).

import {addWebUiListener} from 'chrome://resources/js/cr.js';

const HERKUNFT = {
  known: 'gesichert',
  inferred: 'erschlossen',
  uncertain: 'unsicher',
};

// Höchstens so oft eine Ansage über Änderungen der Seite.
const ANSAGE_ABSTAND_MS = 3000;

const nodes = document.getElementById('nodes');
const details = document.getElementById('details');
const detailsEmpty = document.getElementById('details-empty');
const showButton = document.getElementById('show');
const statusLine = document.getElementById('status');
const announce = document.getElementById('announce');
const pageLine = document.getElementById('page');

let view = null;
let items = new Map();
let lastCounts = null;
let lastAnnouncement = 0;
let pendingAnnouncement = null;

function listLabel(item, focusKey) {
  let label = item.label;
  if (item.certainty !== 'known') {
    label += ` [Name ${HERKUNFT[item.certainty]}]`;
  }
  if (!item.reachable) {
    label += ' [gesperrt]';
  }
  if (item.findings.length > 0) {
    label += ` [${item.findings.length} ${item.findings.length === 1 ? 'Befund' : 'Befunde'}]`;
  }
  if (item.key === focusKey) {
    label += ' ← Position';
  }
  return label;
}

function group(label, list, focusKey) {
  const optgroup = document.createElement('optgroup');
  optgroup.label = `${label} (${list.length})`;
  for (const item of list) {
    const option = document.createElement('option');
    option.value = item.key;
    option.textContent = listLabel(item, focusKey);
    optgroup.append(option);
    items.set(item.key, item);
  }
  return optgroup;
}

function addDetail(term, value) {
  if (value === null || value === undefined || value === '') {
    return;
  }
  const li = document.createElement('li');
  const label = document.createElement('span');
  label.className = 'term';
  label.textContent = `${term}: `;
  li.append(label, value);
  details.append(li);
}

function renderDetails() {
  const item = items.get(nodes.value);
  details.replaceChildren();
  showButton.disabled = !item;
  details.hidden = !item;
  detailsEmpty.hidden = !!item;
  if (!item) {
    return;
  }
  addDetail('Rolle', item.role);
  addDetail(
      'Name',
      item.name === null ? `keiner (${HERKUNFT[item.certainty]})` :
                           `${item.name} (${HERKUNFT[item.certainty]})`);
  addDetail('Herkunft', item.origin);
  if (item.level) {
    addDetail('Ebene', String(item.level));
  }
  addDetail('Wert', item.value);
  addDetail('Bereich', item.region);
  addDetail(
      'Zustände', item.states.map(([key, value]) => `${key}=${value}`).join(', '));
  addDetail('Aktionen', item.actions.join(', '));
  for (const [relation, target] of item.relations) {
    addDetail(relation, target);
  }
  if (!item.reachable) {
    addDetail('Erreichbar', 'nein, ein modaler Dialog ist offen');
  }
  for (const finding of item.findings) {
    addDetail('Befund', findingText(finding));
  }
}

// Ergebnis und Schwere aus a11y-report in Worte.
const ERGEBNIS = {fail: 'Fehler', review: 'prüfen', untested: 'nicht geprüft'};

function findingText(f) {
  return `${f.message} (${ERGEBNIS[f.outcome] || f.outcome}, ${f.severity}, ${f.rule})`;
}

function renderChecks(checks) {
  const findings =
      [...view.regions, ...view.headings, ...view.controls]
          .reduce((n, item) => n + item.findings.length, 0) +
      checks.page.length;
  document.getElementById('checks-summary').textContent =
      `${checks.ran} Regeln auf dem Accessibility-Tree gelaufen, ${findings} ` +
      `Befunde; ${checks.not_run.length} Regeln nicht geprüft (brauchen ` +
      'Markup oder berechnete Stile).';
  const page = document.getElementById('checks-page');
  page.replaceChildren(...checks.page.map((f) => {
    const li = document.createElement('li');
    li.textContent = findingText(f);
    return li;
  }));
  const notRun = document.getElementById('checks-not-run');
  notRun.replaceChildren(...checks.not_run.map(([rule, reason]) => {
    const li = document.createElement('li');
    li.textContent = `${rule}: ${reason}`;
    return li;
  }));
}

function counts(v) {
  return {
    regions: v.regions.length,
    headings: v.headings.length,
    controls: v.controls.length,
  };
}

function difference(now, before, key, word) {
  const delta = now[key] - before[key];
  return delta === 0 ? null : `${word} ${delta > 0 ? '+' : ''}${delta}`;
}

// Änderungen bündeln: eine Ansage je Abstand, nur wenn sich die Zahlen
// geändert haben und die Ansage eingeschaltet ist.
function scheduleAnnouncement(now) {
  if (lastCounts === null) {
    lastCounts = now;
    statusLine.textContent =
        `${now.regions} Bereiche, ${now.headings} Überschriften, ` +
        `${now.controls} Bedienelemente.`;
    return;
  }
  const parts = [
    difference(now, lastCounts, 'regions', 'Bereiche'),
    difference(now, lastCounts, 'headings', 'Überschriften'),
    difference(now, lastCounts, 'controls', 'Bedienelemente'),
  ].filter(Boolean);
  if (parts.length === 0 || !announce.checked) {
    return;
  }
  const wait = Math.max(0, lastAnnouncement + ANSAGE_ABSTAND_MS - Date.now());
  clearTimeout(pendingAnnouncement);
  pendingAnnouncement = setTimeout(() => {
    const current = counts(view);
    const text = [
      difference(current, lastCounts, 'regions', 'Bereiche'),
      difference(current, lastCounts, 'headings', 'Überschriften'),
      difference(current, lastCounts, 'controls', 'Bedienelemente'),
    ].filter(Boolean);
    lastCounts = current;
    if (text.length > 0 && announce.checked) {
      statusLine.textContent = `Seite geändert: ${text.join(', ')}.`;
      lastAnnouncement = Date.now();
    }
  }, wait);
}

function render(json) {
  view = JSON.parse(json);
  pageLine.textContent = [view.title, view.page].filter(Boolean).join(' – ');
  const selected = nodes.value;
  items = new Map();
  nodes.replaceChildren(
      group('Bereiche', view.regions, view.focus),
      group('Überschriften', view.headings, view.focus),
      group('Bedienelemente', view.controls, view.focus));
  // Auswahl über Aktualisierungen hinweg halten, solange der Knoten besteht.
  if (items.has(selected)) {
    nodes.value = selected;
  }
  renderDetails();
  renderChecks(view.checks);
  renderSemantic();
  scheduleAnnouncement(counts(view));
}

function activate() {
  if (!items.has(nodes.value)) {
    return;
  }
  statusLine.textContent = 'Zeige im Dokument …';
  chrome.send('show', [nodes.value]);
}

nodes.addEventListener('change', renderDetails);
nodes.addEventListener('keydown', (event) => {
  if (event.key === 'Enter') {
    event.preventDefault();
    activate();
  }
});
nodes.addEventListener('dblclick', activate);
showButton.addEventListener('click', activate);

// ---------------------------------------------------------------------------
// Befehlsleiste: Eingabe → Runtime; Zustände sichtbar und als Statusmeldung
// (bereit, führe aus, Rückfrage, Bestätigung, fertig); Antworten im Log.

const cmdForm = document.getElementById('cmd-form');
const cmdInput = document.getElementById('cmd');
const cmdState = document.getElementById('cmd-state');
const cancelButton = document.getElementById('cancel');
const log = document.getElementById('log');

// Höchstens so viele Antworten im Log.
const LOG_MAX = 20;

let busy = false;
let pending = false;

function setState(text) {
  cmdState.textContent = text;
  cancelButton.disabled = !busy && !pending;
}

function stateAfter(answer) {
  if (answer.startsWith('Bestätigung nötig')) {
    pending = true;
    return 'Bestätigung erwartet: „ja“ oder „abbrechen“.';
  }
  if (answer.startsWith('Mehrdeutig') || answer.startsWith('Mehrere')) {
    pending = true;
    return 'Auswahl erwartet: Zahl oder Name, oder „abbrechen“.';
  }
  pending = false;
  return 'Fertig.';
}

function addLog(input, answer) {
  const entry = document.createElement('li');
  const asked = document.createElement('strong');
  asked.textContent = `${input}: `;
  entry.append(asked, answer);
  log.append(entry);
  while (log.children.length > LOG_MAX) {
    log.firstElementChild.remove();
  }
  entry.scrollIntoView({block: 'nearest'});
}

// Schickt die Eingabe; false, wenn noch ein Befehl läuft (die Eingabe
// bleibt dann stehen).
function send(text) {
  if (text.trim() === '') {
    return true;
  }
  if (busy) {
    setState('Noch beschäftigt: Eingabe bleibt stehen. Warten oder abbrechen.');
    return false;
  }
  busy = true;
  setState('Führe aus …');
  chrome.send('command', [text.trim()]);
  return true;
}

cmdForm.addEventListener('submit', (event) => {
  event.preventDefault();
  if (send(cmdInput.value)) {
    cmdInput.value = '';
  }
});

function cancel() {
  if (!busy && !pending) {
    return;
  }
  if (!busy) {
    // Offene Rückfrage: die Runtime verwirft sie und antwortet.
    busy = true;
    setState('Breche ab …');
  }
  chrome.send('cancel');
}
cancelButton.addEventListener('click', cancel);

// Escape: laufende Ausführung oder offene Rückfrage abbrechen, sonst das
// Panel schließen (Chromium gibt den Fokus an die Seite zurück).
document.addEventListener('keydown', (event) => {
  if (event.key !== 'Escape') {
    return;
  }
  event.preventDefault();
  if (busy || pending) {
    cancel();
  } else {
    chrome.send('close');
  }
});

// `input`: die Eingabe, wie der Host sie fürs Log liefert (Wert eines
// Ausfüll- oder Auswahlbefehls verdeckt).
addWebUiListener('answer', (answer, acted, input) => {
  busy = false;
  addLog(input, answer);
  if (semanticMode()) {
    showSemanticAnswer(answer);
  }
  setState(stateAfter(answer) + (acted ? ' Fokus liegt auf der Seite.' : ''));
});
addWebUiListener('focusCommand', () => cmdInput.focus());
// Antwort auf eine Eingabe außerhalb der Leiste (Sprungmarke): ins Log,
// Zustand wie bei eigenen Eingaben (z. B. Bestätigung erwartet).
addWebUiListener('externalAnswer', (input, answer, acted) => {
  addLog(input, answer);
  setState(stateAfter(answer) + (acted ? ' Fokus liegt auf der Seite.' : ''));
});

// ---------------------------------------------------------------------------
// Semantic View (Paket 29): die Seite aus dem Graph als lineare, bedienbare
// Ansicht. Jede Bedienung geht als validierte Aktion an die Originalseite
// (chrome.send('act')), riskante erst nach Rückfrage. Der Wechsel der
// Ansicht löst auf der Seite nichts aus und behält den Ort.

const semantic = document.getElementById('semantic');
const semanticBody = document.getElementById('semantic-body');
const semanticStatus = document.getElementById('semantic-status');
const semanticConfirm = document.getElementById('semantic-confirm');
const semanticQuestion = document.getElementById('semantic-question');
const inspectorPart = document.getElementById('inspector');

// Zuletzt gewählter Ort (Schlüssel), über Aktualisierungen und Wechsel.
let lastKey = null;
let lastLabel = null;
let semanticBusy = false;

function semanticMode() {
  return document.querySelector('input[name="mode"]:checked').value ===
      'semantic';
}

function describeAct(entry, kind) {
  const verb = {
    activate: 'aktiviere',
    set: 'fülle',
    select: 'wähle in',
    increment: 'erhöhe',
    decrement: 'verringere',
  }[kind];
  // Werte stehen nie im Log (wie bei Befehlen, Paket 76).
  return `Ansicht: ${verb} ${entry.text}`;
}

function act(entry, kind, value = '') {
  if (semanticBusy) {
    semanticStatus.textContent = 'Noch beschäftigt, bitte warten.';
    return;
  }
  semanticBusy = true;
  lastKey = entry.key;
  lastLabel = entry.text;
  semanticStatus.textContent = 'Führe aus …';
  chrome.send('act', [entry.key, kind, value, describeAct(entry, kind)]);
}

function nameText(entry) {
  let text = entry.text;
  if (entry.certainty && entry.certainty !== 'known') {
    text += ` (Name ${HERKUNFT[entry.certainty]})`;
  }
  if (!entry.reachable) {
    text += ' (gesperrt, ein Dialog ist offen)';
  }
  return text;
}

function controlElement(entry) {
  const off = entry.disabled || !entry.reachable;
  switch (entry.control) {
    case 'button':
    case 'link':
    case 'other': {
      const button = document.createElement('button');
      button.type = 'button';
      button.textContent =
          nameText(entry) + (entry.control === 'link' ? ' (Link)' : '');
      button.disabled = off;
      button.addEventListener('click', () => act(entry, 'activate'));
      return button;
    }
    case 'checkbox':
    case 'radio': {
      const label = document.createElement('label');
      label.className = 'choice';
      const input = document.createElement('input');
      input.type = entry.control;
      input.checked = entry.checked === true;
      input.disabled = off;
      input.addEventListener('change', () => {
        // Der Zustand kommt von der Seite zurück, nicht aus diesem Klick.
        input.checked = entry.checked === true;
        act(entry, 'activate');
      });
      label.append(input, nameText(entry));
      return label;
    }
    case 'select': {
      const label = document.createElement('label');
      label.append(nameText(entry));
      const select = document.createElement('select');
      select.disabled = off;
      for (const option of entry.options) {
        const o = document.createElement('option');
        o.textContent = option;
        o.value = option;
        select.append(o);
      }
      if (entry.selected !== null) {
        select.value = entry.selected;
      }
      select.addEventListener('change', () => act(entry, 'select', select.value));
      label.append(select);
      return label;
    }
    case 'number': {
      const wrap = document.createElement('div');
      const text = document.createElement('p');
      text.textContent = `${nameText(entry)}: ${entry.value ?? 'kein Wert'}`;
      const down = document.createElement('button');
      down.type = 'button';
      down.textContent = `${entry.text} verringern`;
      down.disabled = off;
      down.addEventListener('click', () => act(entry, 'decrement'));
      const up = document.createElement('button');
      up.type = 'button';
      up.textContent = `${entry.text} erhöhen`;
      up.disabled = off;
      up.addEventListener('click', () => act(entry, 'increment'));
      wrap.append(text, down, up);
      return wrap;
    }
    case 'textbox': {
      const label = document.createElement('label');
      label.append(nameText(entry));
      const input = document.createElement('input');
      input.type = entry.sensitive ? 'password' : 'text';
      input.autocomplete = 'off';
      input.value = entry.sensitive ? '' : (entry.value ?? '');
      if (entry.sensitive) {
        input.placeholder = '(verdeckt)';
      }
      input.disabled = off;
      // Übernehmen mit Eingabetaste oder beim Verlassen, nicht je Taste.
      input.addEventListener('change', () => act(entry, 'set', input.value));
      label.append(input);
      return label;
    }
  }
  return null;
}

function entryElement(entry) {
  if (entry.kind === 'heading') {
    // Unter „Relief“ (h1) und „Semantische Ansicht“ (h2).
    const h = document.createElement(`h${Math.min(6, entry.level + 2)}`);
    h.textContent = entry.text;
    return h;
  }
  if (entry.kind === 'text') {
    const p = document.createElement('p');
    p.textContent = entry.text;
    return p;
  }
  return controlElement(entry);
}

// Element eines Eintrags in der Ansicht (fokussierbar).
function focusTarget(key) {
  const holder = semanticBody.querySelector(`[data-key="${CSS.escape(key)}"]`);
  if (!holder) {
    return null;
  }
  return holder.matches('input, select, button') ?
      holder :
      holder.querySelector('input, select, button') ?? holder;
}

function renderSemantic() {
  if (!view || !semanticMode()) {
    return;
  }
  // Fokus und angefangene Eingabe über die Aktualisierung halten.
  const active = document.activeElement;
  const activeHolder = active && semanticBody.contains(active) ?
      active.closest('[data-key]') : null;
  const activeKey = activeHolder ? activeHolder.dataset.key : null;
  const typed = active && active.tagName === 'INPUT' &&
          (active.type === 'text' || active.type === 'password') ?
      active.value :
      null;

  const sections = [];
  let current = null;
  for (const entry of view.semantic) {
    if (!current || current.region !== entry.region) {
      current = {region: entry.region, element: document.createElement('section')};
      if (entry.region) {
        current.element.setAttribute('aria-label', entry.region);
        const label = document.createElement('p');
        label.className = 'region';
        label.textContent = entry.region;
        current.element.append(label);
      }
      sections.push(current);
    }
    const element = entryElement(entry);
    if (!element) {
      continue;
    }
    if (entry.key) {
      element.dataset.key = entry.key;
      if (entry.kind === 'heading') {
        element.tabIndex = -1;
      }
    }
    current.element.append(element);
  }
  semanticBody.replaceChildren(...sections.map((s) => s.element));

  if (activeKey) {
    const target = focusTarget(activeKey);
    if (target) {
      if (typed !== null && target.tagName === 'INPUT') {
        target.value = typed;
      }
      target.focus();
    }
  }
}

// Beim Wechsel in die Ansicht: an den Ort der Sitzung (Position) bzw. den
// zuletzt gewählten; ist er weg, das sagen und oben beginnen.
function enterSemantic() {
  semantic.hidden = false;
  inspectorPart.hidden = true;
  renderSemantic();
  const wanted = lastKey ?? view?.focus ?? null;
  const target = wanted ? focusTarget(wanted) : null;
  if (target) {
    target.focus();
    semanticStatus.textContent =
        `Semantische Ansicht. Ort: ${target.closest('[data-key]').textContent}.`;
  } else {
    document.getElementById('semantic-title').focus();
    semanticStatus.textContent = lastLabel ?
        `Semantische Ansicht. Der vorige Ort „${lastLabel}“ ist nicht mehr ` +
            'auf der Seite; weiter am Anfang.' :
        'Semantische Ansicht.';
  }
}

function leaveSemantic() {
  const active = document.activeElement?.closest?.('#semantic-body [data-key]');
  if (active) {
    lastKey = active.dataset.key;
  }
  semantic.hidden = true;
  inspectorPart.hidden = false;
  // Gleicher Ort in der Liste, sofern er dort steht; nichts auf der Seite.
  if (lastKey && items.has(lastKey)) {
    nodes.value = lastKey;
    renderDetails();
  }
  nodes.focus();
}

for (const radio of document.querySelectorAll('input[name="mode"]')) {
  radio.addEventListener('change', () => {
    if (semanticMode()) {
      enterSemantic();
    } else {
      leaveSemantic();
    }
  });
}

semanticBody.addEventListener('focusin', (event) => {
  const holder = event.target.closest('[data-key]');
  if (holder) {
    lastKey = holder.dataset.key;
    lastLabel = view?.semantic.find((e) => e.key === lastKey)?.text ?? null;
  }
});

function showSemanticAnswer(answer) {
  semanticStatus.textContent = answer;
  const question = answer.startsWith('Bestätigung nötig');
  semanticConfirm.hidden = !question;
  if (question) {
    semanticQuestion.textContent = answer;
    semanticQuestion.focus();
  }
}

document.getElementById('semantic-yes').addEventListener('click', () => {
  semanticConfirm.hidden = true;
  semanticBusy = true;
  semanticStatus.textContent = 'Führe aus …';
  chrome.send('viewCommand', ['ja']);
});
document.getElementById('semantic-no').addEventListener('click', () => {
  semanticConfirm.hidden = true;
  chrome.send('viewCommand', ['abbrechen']);
  const target = lastKey ? focusTarget(lastKey) : null;
  target?.focus();
});

addWebUiListener('viewAnswer', (answer, acted, shown) => {
  semanticBusy = false;
  addLog(shown, answer);
  // Gleicher Zustand wie in der Leiste: Escape bricht eine Rückfrage ab.
  setState(stateAfter(answer));
  showSemanticAnswer(answer);
  if (semanticConfirm.hidden && lastKey) {
    focusTarget(lastKey)?.focus();
  }
});

// ---------------------------------------------------------------------------
// Fähigkeitsprofil (Paket 41): einzelne Werte, global oder nur für die
// Website des Tabs; jede Änderung wirkt sofort, ist einzeln rücksetzbar,
// „Standard wiederherstellen“ setzt alles zurück.

const FAEHIGKEITEN = {
  visual_detail: ['Visuelles Detail',
    {full: 'voll', reduced: 'eingeschränkt', none: 'keines'}],
  text_scale: ['Textgröße (Faktor 1–3)', null],
  color_discrimination: ['Farbunterscheidung',
    {full: 'voll', reduced: 'eingeschränkt', none: 'keine'}],
  contrast: ['Kontrastbedarf',
    {standard: 'normal', increased: 'erhöht', maximum: 'maximal'}],
  motion_tolerance: ['Bewegung verträglich',
    {full: 'voll', reduced: 'wenig', none: 'keine'}],
  audio_output: ['Sprachausgabe',
    {preferred: 'bevorzugt', available: 'verfügbar', unavailable: 'nicht verfügbar'}],
  speech_input: ['Spracheingabe',
    {preferred: 'bevorzugt', available: 'verfügbar', unavailable: 'nicht verfügbar'}],
  keyboard_input: ['Tastatur',
    {preferred: 'bevorzugt', available: 'verfügbar', unavailable: 'nicht verfügbar'}],
  pointer_input: ['Zeigegerät',
    {preferred: 'bevorzugt', available: 'verfügbar', unavailable: 'nicht verfügbar'}],
  switch_input: ['Schalter',
    {preferred: 'bevorzugt', available: 'verfügbar', unavailable: 'nicht verfügbar'}],
  text_complexity: ['Textmenge je Antwort',
    {full: 'vollständig', reduced: 'kurz, Rest auf Nachfrage', none: 'sehr kurz'}],
};

// Fähigkeiten, die Relief noch nicht nutzt (sichtbar statt verschwiegen).
const OHNE_WIRKUNG = ['color_discrimination', 'speech_input', 'switch_input'];

const profileFields = document.getElementById('profile-fields');
const profileStatus = document.getElementById('profile-status');
const presetSelect = document.getElementById('preset');
let profileView = null;
// Hat die Nutzerin die Ansicht selbst gewählt, entscheidet das Profil nicht
// mehr über sie.
let modeChosen = false;

function scope() {
  return document.querySelector('input[name="scope"]:checked').value;
}

function valueText(field, value) {
  const names = FAEHIGKEITEN[field][1];
  return names ? names[value] : String(value).replace('.', ',');
}

function renderProfile() {
  if (!profileView) {
    return;
  }
  const site = profileView.site;
  document.getElementById('profile-site').textContent =
      site ? site : 'diese Website (Datei ohne Website)';
  document.getElementById('scope-site').disabled = !site;
  document.getElementById('profile-reset-site').disabled =
      !site || !profileView.fields.some((f) => f.site !== null);
  if (presetSelect.options.length === 0) {
    for (const name of profileView.presets) {
      const o = document.createElement('option');
      o.textContent = name;
      presetSelect.append(o);
    }
  }
  const focusedField = document.activeElement?.dataset?.field;
  const rows = profileView.fields.map((f) => {
    const [label, names] = FAEHIGKEITEN[f.field];
    const row = document.createElement('div');
    row.className = 'row';
    const id = `profile-${f.field}`;
    const l = document.createElement('label');
    l.htmlFor = id;
    l.textContent = label;
    let control;
    if (names) {
      control = document.createElement('select');
      for (const [value, text] of Object.entries(names)) {
        const o = document.createElement('option');
        o.value = value;
        o.textContent = text;
        control.append(o);
      }
      control.value = f.value;
    } else {
      control = document.createElement('input');
      control.type = 'number';
      control.min = '1';
      control.max = '3';
      control.step = '0.25';
      control.value = f.value;
    }
    control.id = id;
    control.dataset.field = f.field;
    const origin = document.createElement('span');
    origin.className = 'origin';
    origin.id = `${id}-origin`;
    origin.textContent = (f.site !== null ? ' (nur diese Website)' :
        f.global !== null ? ' (geändert, alle Websites)' : ' (Standard)') +
        (OHNE_WIRKUNG.includes(f.field) ? ' · noch ohne Wirkung' : '');
    control.setAttribute('aria-describedby', origin.id);
    control.addEventListener('change', () => {
      const value = names ? JSON.stringify(control.value) : control.value;
      chrome.send('profileSet', [scope(), f.field, value]);
    });
    const reset = document.createElement('button');
    reset.type = 'button';
    reset.textContent = `${label} zurücksetzen`;
    reset.disabled = scope() === 'site' ? f.site === null : f.global === null;
    reset.addEventListener('click', () => {
      chrome.send('profileReset', [scope(), f.field]);
    });
    row.append(l, control, origin, ' ', reset);
    return row;
  });
  profileFields.replaceChildren(...rows);
  if (focusedField) {
    document.getElementById(`profile-${focusedField}`)?.focus();
  }
}

for (const radio of document.querySelectorAll('input[name="scope"]')) {
  radio.addEventListener('change', renderProfile);
}
document.getElementById('preset-apply').addEventListener('click', () => {
  chrome.send('profilePreset', [presetSelect.value]);
});
document.getElementById('profile-reset-all').addEventListener('click', () => {
  chrome.send('profileReset', ['global', '']);
});
document.getElementById('profile-reset-site').addEventListener('click', () => {
  chrome.send('profileReset', ['site', '']);
});
for (const radio of document.querySelectorAll('input[name="mode"]')) {
  radio.addEventListener('change', () => {
    modeChosen = true;
  });
}

// Vorschlag aus den Systemeinstellungen (Paket 115): nur anbieten; bis
// zum Schließen des Panels abgelehnt, wenn „Nicht übernehmen“.
let suggestion = {};
let suggestionDismissed = false;

function renderSuggestion() {
  const box = document.getElementById('profile-suggestion');
  const entries = Object.entries(suggestion);
  box.hidden = suggestionDismissed || entries.length === 0;
  document.getElementById('profile-suggestion-text').textContent =
      'Vorschlag aus den Systemeinstellungen (alle Websites): ' +
      entries.map(([f, v]) =>
          `${FAEHIGKEITEN[f][0]} „${valueText(f, v)}“`).join(', ') + '.';
}

document.getElementById('suggestion-apply').addEventListener('click', () => {
  for (const [field, value] of Object.entries(suggestion)) {
    chrome.send('profileSet', ['global', field, JSON.stringify(value)]);
  }
  suggestionDismissed = true;
  renderSuggestion();
});
document.getElementById('suggestion-dismiss').addEventListener('click', () => {
  suggestionDismissed = true;
  renderSuggestion();
});

addWebUiListener('profile', (json, effects, suggested) => {
  profileView = JSON.parse(json);
  suggestion = JSON.parse(suggested || '{}');
  renderProfile();
  renderSuggestion();
  const root = document.documentElement;
  root.style.setProperty('--scale', String(effects.zoom));
  root.classList.toggle('contrast', effects.contrast);
  root.classList.toggle('calm', effects.calm);
  // Startansicht nach Profil, solange nicht selbst gewählt.
  if (!modeChosen && effects.semantic !== semanticMode()) {
    document.querySelector(
        `input[name="mode"][value="${effects.semantic ? 'semantic' : 'inspector'}"]`)
        .checked = true;
    if (effects.semantic) {
      enterSemantic();
    } else {
      leaveSemantic();
    }
  }
  profileStatus.textContent = `Wirkung: ${profileView.effects}.`;
});
addWebUiListener('profileStatus', (text) => {
  profileStatus.textContent = text;
});

addWebUiListener('graph', render);
addWebUiListener('status', (text) => {
  statusLine.textContent = text;
});
chrome.send('ready');
