#!/usr/bin/env node
// MCP-Server für Relief (Paket 142): Assistenten wie Claude lesen das
// Seitenmodell und schicken Befehle in Sprache an den aktiven Tab, über die
// CDP-Domäne `Relief.*` (→ crates/relief-bridge/src/devtools.rs).
//
// Relief löst auf, prüft, stuft das Risiko ein und fragt zurück. Eine
// Rückfrage bestätigt nur die Nutzerin oder der Nutzer in Relief (Panel oder
// Sprache); über diesen Weg geht weder „ja“ noch „!“, und solange eine
// Rückfrage offen ist, nur „abbrechen“ (→ plan/spezifikation/07).
//
// Relief mit `--remote-debugging-port=9222` starten, dann im MCP-Client:
//   { "command": "node", "args": ["<repo>/examples/mcp/server.mjs"] }
// `RELIEF_CDP` setzt den Endpunkt (Standard http://127.0.0.1:9222).

import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { z } from 'zod';

const ENDPOINT = process.env.RELIEF_CDP ?? 'http://127.0.0.1:9222';

// Aktiver Tab: Chromium listet das zuletzt aktive Ziel zuerst; Relief-Panel
// und andere interne Seiten zählen nicht.
async function activeTab() {
  const response = await fetch(`${ENDPOINT}/json/list`);
  const targets = await response.json();
  const tab = targets.find(
    (t) => t.type === 'page' && !/^(chrome|devtools|chrome-untrusted):/.test(t.url),
  );
  if (!tab) {
    throw new Error(`Kein Tab unter ${ENDPOINT} (Relief mit --remote-debugging-port starten).`);
  }
  return tab;
}

// Ein Aufruf der Domäne `Relief.*` an den aktiven Tab, eigene Verbindung je
// Aufruf. Fehlerantworten von Relief werden zu Fehlern mit deren Text.
async function relief(method, params = {}) {
  const tab = await activeTab();
  const socket = new WebSocket(tab.webSocketDebuggerUrl);
  try {
    await new Promise((resolve, reject) => {
      socket.addEventListener('open', resolve, { once: true });
      socket.addEventListener('error', () => reject(new Error('CDP nicht erreichbar')), {
        once: true,
      });
    });
    const reply = new Promise((resolve) => {
      socket.addEventListener('message', (event) => {
        const message = JSON.parse(event.data);
        if (message.id === 1) resolve(message);
      });
    });
    socket.send(JSON.stringify({ id: 1, method, params }));
    const message = await reply;
    if (message.error) throw new Error(message.error.message);
    return message.result;
  } finally {
    socket.close();
  }
}

// Werkzeugergebnis; Fehler gehen als Text an das Modell, nicht als Abbruch.
async function answer(call) {
  try {
    return { content: [{ type: 'text', text: await call() }] };
  } catch (error) {
    return { content: [{ type: 'text', text: error.message }], isError: true };
  }
}

const server = new McpServer({ name: 'relief', version: '0.0.1' });

server.registerTool(
  'relief_page_model',
  {
    title: 'Seitenmodell',
    description:
      'Seitentyp, Bereiche mit ihren Bedienelementen und primäre Aktion des aktiven Tabs in Relief. ' +
      'Jede Aussage trägt ihre Herkunft (certainty: known, inferred, uncertain); ' +
      'inferred und uncertain sind Vermutungen und so wiederzugeben. Nur lesend.',
    annotations: { readOnlyHint: true },
  },
  () => answer(async () => JSON.stringify(await relief('Relief.getPageModel'), null, 2)),
);

server.registerTool(
  'relief_command',
  {
    title: 'Befehl an Relief',
    description:
      'Befehl in Sprache an den aktiven Tab, wie in Reliefs Befehlsleiste: „was ist hier“, ' +
      '„überschriften“, „gehe zu …“, „fülle … mit …“, „wähle …“, „klicke …“, „was fehlt noch“, ' +
      '„hilfe“ (vollständige Liste). Relief prüft jede Aktion. Riskante Aktionen (Absenden, Kaufen, ' +
      'unsichere Namen) beantwortet Relief mit einer Rückfrage (confirmationPending: true): Die ' +
      'bestätigt nur die Nutzerin oder der Nutzer in Relief selbst (Panel oder Sprache), nicht dieses ' +
      'Werkzeug. Dann die Person bitten, in Relief zu antworten, und bis dahin nur „abbrechen“ senden; ' +
      'das Ergebnis danach mit relief_page_model oder „was ist hier“ prüfen.',
    inputSchema: { command: z.string().describe('Befehl in Sprache, z. B. „fülle Name mit Erika“') },
  },
  ({ command }) =>
    answer(async () => {
      const result = await relief('Relief.runCommand', { command });
      const lines = [result.answer];
      if (result.acted) lines.push('(auf der Seite ausgeführt)');
      if (result.confirmationPending) {
        lines.push('(Rückfrage offen: bestätigt wird in Relief, nicht über dieses Werkzeug)');
      }
      return lines.join('\n');
    }),
);

server.registerTool(
  'relief_assert',
  {
    title: 'Zusicherung prüfen',
    description:
      'Prüft eine Zusicherung zur Barrierefreiheit des aktuellen Formulars, z. B. „feldnamen“, ' +
      '„fehler-verknüpft“, „fokus-auf-erstem-fehler“, „bestätigungsdialog“. Antwort: Befunde ' +
      '(Regel-ID, Ergebnis, Meldung) und Zahl der Fehler. Nur lesend.',
    inputSchema: { assertion: z.string().describe('Zusicherung wie in Reliefs Aufgabendateien') },
    annotations: { readOnlyHint: true },
  },
  ({ assertion }) =>
    answer(async () => JSON.stringify(await relief('Relief.assert', { assertion }), null, 2)),
);

await server.connect(new StdioServerTransport());
