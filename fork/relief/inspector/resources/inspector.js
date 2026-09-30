// Relief. MIT-Lizenz wie das Relief-Repository.
//
// Semantic Inspector: zeigt den Graph der Seite (Bereiche, Überschriften,
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

addWebUiListener('graph', render);
addWebUiListener('status', (text) => {
  statusLine.textContent = text;
});
chrome.send('ready');
