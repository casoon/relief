// Relief für Playwright (Paket 45): Relief als Browser starten und das
// Seitenmodell über die CDP-Domäne `Relief.*` abfragen
// (→ crates/relief-bridge/src/devtools.rs).
//
//   import { reliefLaunchOptions, pageModel, reliefAssert, reliefExpect } from 'relief-playwright';
//   const browser = await chromium.launch(reliefLaunchOptions());
//   await reliefExpect(page).toPassReliefAssertion('feldnamen');

import { homedir } from 'node:os';
import { join } from 'node:path';
import { expect } from '@playwright/test';

/** Startoptionen: eigener Build (`RELIEF`, sonst ~/chromium/src/out/Relief). */
export function reliefLaunchOptions(extra = {}) {
  return {
    executablePath:
      process.env.RELIEF ??
      join(homedir(), 'chromium/src/out/Relief/Relief.app/Contents/MacOS/Relief'),
    ...extra,
  };
}

async function send(page, method, params = {}) {
  const session = await page.context().newCDPSession(page);
  try {
    return await session.send(method, params);
  } finally {
    await session.detach();
  }
}

/** Seitentyp mit Herkunft, Gruppen, primäre Aktion (`Relief.getPageModel`). */
export function pageModel(page) {
  return send(page, 'Relief.getPageModel');
}

/**
 * Zusicherung wie `assert:` in Aufgabendateien (`Relief.assert`):
 * `{ findings, failed }`, Befunde im Format von a11y-report.
 */
export function reliefAssert(page, assertion) {
  return send(page, 'Relief.assert', { assertion });
}

/** `expect` mit `toPassReliefAssertion` und `toPassReliefForm`. */
export const reliefExpect = expect.extend({
  async toPassReliefAssertion(page, assertion) {
    const { findings, failed } = await reliefAssert(page, assertion);
    const text = findings.map((f) => `${f.rule_id}: ${f.message}`).join('\n');
    return {
      pass: failed === 0,
      name: 'toPassReliefAssertion',
      message: () =>
        this.isNot
          ? `„${assertion}“ ohne Fehler, erwartet waren welche.`
          : `„${assertion}“: ${failed} Fehler\n${text}`,
    };
  },
  // Formular ohne Ablauf davor: Feldnamen, verknüpfte Fehler, Fokus.
  async toPassReliefForm(page) {
    const lines = [];
    let failed = 0;
    for (const assertion of ['feldnamen', 'fehler-verknüpft', 'fokus-auf-erstem-fehler']) {
      const result = await reliefAssert(page, assertion);
      failed += result.failed;
      for (const f of result.findings) {
        if (f.outcome === 'fail') lines.push(`${assertion} → ${f.rule_id}: ${f.message}`);
      }
    }
    return {
      pass: failed === 0,
      name: 'toPassReliefForm',
      message: () =>
        this.isNot ? 'Formular ohne Fehler, erwartet waren welche.' : lines.join('\n'),
    };
  },
});
