# 34 · Bridge-Variante B: Runtime im Utility-Prozess

**Umgebung:** Cloud schreibt, M4 baut und misst · **Phase:** vor 4 · **Abhängig von:** 19; fällig, bevor Code mit Absturz- oder Missbrauchsrisiko (KI-Adapter, Heuristiken auf Seiteninhalt) in die Runtime kommt

## Ziel

Die Rust-Runtime aus dem Browser-Prozess in einen sandboxbaren
Utility-Prozess hinter Mojo verlegen und die Kosten messen, die in 17 offen
blieben (Entscheidung „jetzt A, Ziel B“ in
[spezifikation/02](spezifikation/02-rust-core-und-bridge.md#entscheidung-entscheidung-in-17)).

## Schritte

1. `crates/relief-bridge/mojom/relief_runtime.mojom` in `//relief/` bauen,
   Service nach Vorbild `services/data_decoder` (Rust mit `cxx` im
   Utility-Prozess).
2. Mojo-Traits Mojo ↔ `cxx`-Strukturen; `RuntimeHost` wird zum Client.
3. Der Browser prüft jeden `ActionPlan` aus dem Utility-Prozess vor dem
   Senden erneut (Knoten existiert, Aktion gemeldet).
4. Messen: Mojo-Serialisierung der Delta in C++, IPC-Latenz p50/p95 bis zur
   Antwort, Kaltstart des Utility-Prozesses (eine Instanz je Tab oder
   geteilt), Speicher; Vergleich mit den A-Zahlen in spezifikation/09.

## Fertig, wenn

- Messwerte in spezifikation/02, Entscheidung (Wechsel, Zeitpunkt) in
  `docs/decisions.md`.

## Nicht Teil

KI-Adapter selbst (→ 27 ff.).
