# 75 · Formularziel und HTML-`autocomplete` im Fork

**Umgebung:** Cloud schreibt, M4 baut · **Phase:** quer · **Abhängig von:** 58 · **Entscheidung nötig**

## Ziel

Im Fork nennt und bindet die Rückfrage zu einem Absenden-Button das
Formularziel wie im CDP-Host (`extra["formAction"]`,
→ spezifikation/07, „Bestätigungstoken“), und sensible Felder mit
`autocomplete` für Zahlungs-/Identitätsdaten werden verdeckt.

## Kontext

Belegt (Chromium 154.0.8037.58): Blink serialisiert weder `action` noch
`formaction` (`AXNodeObject::Url` in `ax_node_object.cc` liefert nur
Link-Ziel, Dokument- und Bildadresse), und `kAutoComplete` ist
`aria-autocomplete` bzw. „list“ (`AXNodeObject::AutoComplete`), nicht das
HTML-Attribut. `kInputType` kommt an und wird genutzt.

## Schritte

1. Weg entscheiden, mit Kosten: (a) Blink-Patch, der für Absenden-Buttons
   das aufgelöste Formularziel und für Felder HTML-`autocomplete` als
   String-Attribut serialisiert (Patch-Serie wächst, → spezifikation/01,
   Fork-Strategie); (b) Anfrage an den Renderer nur bei einer Rückfrage
   (eigener Mojo-Weg, asynchron; die Bindung entsteht heute synchron in
   `Session::handle`); (c) im Fork bewusst ohne Formularziel, Rückfrage sagt
   „Formularziel unbekannt“.
2. Umsetzen; Mirror legt die Angaben unter denselben Schlüsseln ab
   (`relief_interaction::security::{FORM_ACTION, HTML_AUTOCOMPLETE}`).
3. Browser-Test: Rückfrage nennt das Formularziel, geändertes `action`
   zwischen Rückfrage und „ja“ verlangt neue Bestätigung.

## Fertig, wenn

- Eine Rückfrage im Fork nennt das Formularziel (oder sagt nach Entscheidung
  (c) ausdrücklich, dass es unbekannt ist), und ein anderes Ziel verlangt
  neue Bestätigung (`relief_browsertests`).

## Nicht Teil

- Formularziel im CDP-Host (umgesetzt, 58).
