#!/usr/bin/env python3
# Relief. MIT-Lizenz wie das Relief-Repository.
"""Bettet die Inspector-Ressourcen (HTML, JS, CSS) als Raw-String-Literale in
einen Header ein. Ersetzt grit, damit der Inspector ohne Eintrag in
Chromiums resource_ids (ein Patch) auskommt.

Aufruf: embed_resources.py <ausgabe.h> <datei>...
"""

import os
import sys

MIME = {".html": "text/html", ".js": "text/javascript", ".css": "text/css"}
DELIMITER = "RELIEF_RESOURCE"


def main():
    out, files = sys.argv[1], sys.argv[2:]
    lines = [
        "// Erzeugt von //relief/inspector/embed_resources.py, nicht bearbeiten.",
        "#ifndef RELIEF_INSPECTOR_INSPECTOR_RESOURCES_H_",
        "#define RELIEF_INSPECTOR_INSPECTOR_RESOURCES_H_",
        "",
        "namespace relief::inspector_resources {",
        "",
        "struct Resource {",
        "  const char* path;",
        "  const char* data;",
        "};",
        "",
        "inline constexpr Resource kResources[] = {",
    ]
    for path in files:
        name = os.path.basename(path)
        if os.path.splitext(name)[1] not in MIME:
            sys.exit(f"unbekannte Endung: {name}")
        with open(path, encoding="utf-8") as f:
            data = f.read()
        if f"){DELIMITER}\"" in data:
            sys.exit(f"Begrenzer kommt in {name} vor")
        lines.append(f'    {{"{name}", R"{DELIMITER}({data}){DELIMITER}"}},')
    lines += ["};", "", "}  // namespace relief::inspector_resources", "",
              "#endif  // RELIEF_INSPECTOR_INSPECTOR_RESOURCES_H_", ""]
    with open(out, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))


if __name__ == "__main__":
    main()
