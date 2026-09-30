// @ts-check
import casoonPages from '@casoon/pages-theme';
import { defineConfig } from 'astro/config';

// Projektseite: https://casoon.github.io/relief/ — `base` ist der GitHub-Pages-Pfad.
export default defineConfig({
  site: 'https://casoon.github.io/relief',
  base: '/relief/',
  integrations: [
    casoonPages({
      name: 'Relief',
      description:
        'Research browser built on Chromium that understands web pages through the accessibility tree and adapts operation to people’s abilities.',
      repo: 'casoon/relief',
      license: 'MIT',
      changelog: false,
      showcase: false,
      docsGroups: {
        project: 'Project',
      },
    }),
  ],
});
