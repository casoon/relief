// Paket 45: ein Playwright-Test nutzt Relief.* und schlägt beim kaputten
// Formular fehl (spike/fixtures/form-clean.html vs. form-broken.html).

import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { test } from '@playwright/test';
import { pageModel, reliefAssert, reliefExpect } from './relief.mjs';

const fixture = (name) =>
  pathToFileURL(resolve(import.meta.dirname, '../../spike/fixtures', name)).href;

// Der AXTree kommt kurz nach dem Laden; bis dahin ist das Modell leer.
async function openWithModel(page, name) {
  await page.goto(fixture(name));
  await reliefExpect
    .poll(async () => (await pageModel(page)).controls, { timeout: 10_000 })
    .toBeGreaterThan(0);
}

test('sauberes Formular besteht', async ({ page }) => {
  await openWithModel(page, 'form-clean.html');
  const model = await pageModel(page);
  reliefExpect(model.groups.some((g) => g.kind === 'Form')).toBe(true);
  await reliefExpect(page).toPassReliefForm();
});

test('kaputtes Formular fällt durch', async ({ page }) => {
  await openWithModel(page, 'form-broken.html');
  const { failed, findings } = await reliefAssert(page, 'feldnamen');
  reliefExpect(failed).toBeGreaterThan(0);
  reliefExpect(findings.map((f) => f.rule_id)).toContain('form/field-name');
  await reliefExpect(page).not.toPassReliefForm();
});
