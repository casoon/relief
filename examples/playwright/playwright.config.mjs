import { defineConfig } from '@playwright/test';
import { reliefLaunchOptions } from './relief.mjs';

export default defineConfig({
  testDir: '.',
  testMatch: '*.spec.mjs',
  workers: 1,
  use: {
    browserName: 'chromium',
    launchOptions: reliefLaunchOptions(),
  },
});
