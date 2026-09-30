# 10 · Cloud-Setup: Nachweis in einer Cloud-Session

**Umgebung:** Cloud · **Phase:** 0 · **Abhängig von:** Zugang zu Cloud-Sessions (bisher HTTP 403)

## Ziel

Die CI läuft unter Linux grün (→
[spezifikation/10](spezifikation/10-teststrategie.md#ci-und-cloud-umgebung),
„Linux-Nachweis“). Offen ist nur, dass `scripts/cloud-setup.sh` in einer
Cloud-Session auf claude.ai funktioniert.

## Schritte

1. `scripts/cloud-setup.sh` in einer Cloud-Umgebung eintragen, eine Session
   starten und dort `relief-cdp run spike/tasks/01`–`05` laufen lassen.
2. Ergebnis in spezifikation/10 eintragen, dieses Paket löschen.

## Fertig, wenn

- Eine Cloud-Session mit dem Setup-Skript erfüllt alle 68 Erwartungen in
  `spike/tasks/01`–`05`.

## Nicht Teil

Windows (kein Host; → 31). Tests gegen echte Websites im CI.
