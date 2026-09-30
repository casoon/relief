import { docsCollection } from '@casoon/pages-theme/content';

// Quellen: pages/ im Repo. `docs/` ist die lebende Projektdoku, keine Seiteninhalte.
export const collections = { docs: docsCollection({ base: '../pages' }) };
