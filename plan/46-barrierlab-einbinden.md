# 46 · barrierlab: Relief als Konsument, Reader ersetzen

**Umgebung:** Cloud (Repo `casoon/barrierlab`) · **Phase:** Querschnitt · **Abhängig von:** —

## Ziel

Relief ist in barrierlab als Konsument eingetragen und ersetzt dort den
geplanten Reader-Host (→
[spezifikation/12](spezifikation/12-produktumfang.md#relief-ersetzt-den-barrierlab-reader-entscheidung-2026-09-30)).

## Schritte

1. `barrierlab/docs/consumers.md`: Zeile „Reader-Host“ durch Relief
   (`casoon/relief`) ersetzen; heute `a11y-perception`, künftig
   `accname`, `a11y-rules`, `a11y-report`.
2. Die Reader-Planung in barrierlab als „ersetzt durch Relief“ schließen.
3. Liste führen, was aus Relief nach barrierlab wandern kann (→ 12,
   „barrierlab einbinden“), und bei jedem Kandidaten den zweiten Konsumenten
   nennen.

## Fertig, wenn

- PR in barrierlab mit den Änderungen an `consumers.md`.
