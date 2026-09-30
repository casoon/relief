# 30 · CDP-Host: barrierlab-Release übernehmen

**Umgebung:** Cloud · **Phase:** quer · **Abhängig von:** barrierlab-Release

Nachladende Seiten (Netzwerk-Ruhe) sind erledigt, siehe
[spezifikation/09](spezifikation/09-phasen-und-kriterien.md#nachtrag-netzwerk-ruhe-2026-09-25-belegt).

## Ziel

`checked`/`pressed` ist auf barrierlab `main` (7ea448b), aber nicht im
Release: `a11y-perception` 0.1.1 (neueste auf crates.io, geprüft 2026-09-25)
verfolgt nur `expanded`, `hidden`, `selected`, `invalid`, `modal`. Nach dem
nächsten Release `a11y-perception` anheben und `respond::target_change`
prüfen bzw. entfernen.

## Fertig, wenn

- `cargo info a11y-perception` zeigt eine Version, deren `TRACKED_PROPERTIES`
  `checked` und `pressed` enthält, und der Workspace nutzt sie;
- `respond::target_change` ist entfernt oder begründet behalten;
- Prüfbefehle und Standardaufgaben 01–05 ohne Fehler.
