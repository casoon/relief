# 10 · Cloud-Umgebung und CI: Nachweis unter Linux

**Umgebung:** Cloud · **Phase:** 0 · **Abhängig von:** GitHub-Actions-Abrechnung

## Ziel

CI-Workflow und Setup-Skript liegen bei (`.github/workflows/ci.yml`,
`scripts/cloud-setup.sh`, beschrieben in
[spezifikation/10](spezifikation/10-teststrategie.md#ci-und-cloud-umgebung)
und `docs/project-state.md`). Offen ist, dass beides unter Linux tatsächlich
läuft; damit beantwortet sich auch, ob der CDP-Host unter Linux läuft.

## Kontext

Die ersten Läufe (Runs 36083451531, 36083580484) starteten keinen Job:
„recent account payments have failed or your spending limit needs to be
increased“. Solange das Repo privat ist, sind Actions-Minuten
kostenpflichtig; für öffentliche Repos entfällt das.

## Schritte

1. Abrechnung/Ausgabenlimit für GitHub Actions im Konto `casoon` klären
   (Nutzer).
2. CI auf dem Branch neu anstoßen (`gh run rerun` oder Push), bis beide Jobs
   grün sind. Erwartete Stolperstellen im Browser-Job: Chrome-Sandbox unter
   Ubuntu 24.04 (AppArmor, `kernel.apparmor_restrict_unprivileged_userns`,
   der Job gibt den Wert aus). Abhilfe dann bevorzugt im Workflow
   (`sysctl … =0`), `--no-sandbox` nur begründet.
3. Unter Linux gefundene Fehler des Hosts beheben.
4. `scripts/cloud-setup.sh` in einer Cloud-Umgebung eintragen, eine Session
   starten und dort `relief-cdp run spike/tasks/01`–`05` laufen lassen.
5. Ergebnis in spezifikation/10 und 31 eintragen, dieses Paket löschen.

## Fertig, wenn

- CI läuft auf dem PR grün, beide Jobs.
- Der Browser-Job erfüllt alle 68 Erwartungen in `spike/tasks/01`–`05`.
- Eine Cloud-Session mit dem Setup-Skript erfüllt dieselben Erwartungen.

## Nicht Teil

Windows (kein Host; → 31). Tests gegen echte Websites im CI.
