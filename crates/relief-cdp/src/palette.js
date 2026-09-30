// Relief-Befehlsleiste. Wird vom Host in jedes neue Dokument eingefügt
// (Page.addScriptToEvaluateOnNewDocument) und spricht über die Binding
// `reliefCommand` mit ihm. Nur im obersten Frame.
//
// Die Leiste ist ein modaler <dialog>: Nur so liegt sie über einem offenen
// modalen Dialog der Seite, der alles andere inert macht. Beim Schließen
// gibt der Browser den Fokus an das vorher fokussierte Element zurück.
(() => {
  if (window.top !== window || window.__reliefPalette) return;
  window.__reliefPalette = true;

  const NAME = 'Relief-Befehlsleiste';
  const HINT = 'Enter ausführen · Escape schließen · „hilfe“ zeigt die Befehle';
  let host, dlg, input, out, hint, toast, toastTimer, keys = 0, busy = false;

  // Alle Tastendrücke auf der Seite zählen (Messgröße der Studie).
  addEventListener('keydown', (e) => {
    keys++;
    if (e.ctrlKey && e.shiftKey && e.code === 'Space') {
      e.preventDefault();
      e.stopImmediatePropagation();
      open();
    }
  }, true);

  function build() {
    host = document.createElement('relief-palette');
    const root = host.attachShadow({ mode: 'open' });
    root.innerHTML = `
      <style>
        dialog, .toast { box-sizing: border-box; width: min(46rem, calc(100vw - 2rem));
          margin: auto auto 1.5rem; background: #fff; color: #111; border: 3px solid #111;
          border-radius: 10px; padding: 1rem 1.2rem; font: 18px/1.45 system-ui, sans-serif;
          box-shadow: 0 6px 24px rgba(0,0,0,.35); }
        dialog::backdrop { background: rgba(0,0,0,.15); }
        label { display: block; font-weight: 700; margin-bottom: .4rem; }
        input { width: 100%; box-sizing: border-box; font: inherit; padding: .5rem .6rem;
          border: 2px solid #111; border-radius: 6px; color: #111; background: #fff; }
        input:focus { outline: 4px solid #0a58ca; outline-offset: 2px; }
        .hint { margin: .4rem 0 0; font-size: 15px; color: #333; }
        .out { white-space: pre-wrap; margin-top: .7rem; max-height: 40vh; overflow: auto; }
        .out:empty { display: none; }
        .toast { inset: auto 0 0 0; white-space: pre-wrap; max-height: 30vh; overflow: auto; }
        @media (prefers-color-scheme: dark) {
          dialog, .toast, input { background: #111; color: #f5f5f5; border-color: #f5f5f5; }
          .hint { color: #ccc; }
        }
      </style>
      <dialog aria-label="${NAME}">
        <form>
          <label for="i">Relief: Was möchtest du tun?</label>
          <input id="i" autocomplete="off" spellcheck="false" aria-describedby="h">
          <p class="hint" id="h">${HINT}</p>
        </form>
        <div class="out" role="log" aria-live="polite"></div>
      </dialog>
      <div class="toast" popover="manual" role="status" aria-label="Relief-Ergebnis"></div>`;
    dlg = root.querySelector('dialog');
    input = root.getElementById('i');
    out = root.querySelector('.out');
    hint = root.getElementById('h');
    toast = root.querySelector('.toast');
    root.querySelector('form').addEventListener('submit', (e) => {
      e.preventDefault();
      const text = input.value.trim();
      if (!text || busy) return;
      busy = true;
      input.value = '';
      out.textContent = 'Einen Moment …';
      window.reliefCommand(JSON.stringify({ text, keys }));
    });
    document.documentElement.appendChild(host);
  }

  function open() {
    if (!host || !host.isConnected) build();
    hideToast();
    if (!dlg.open) dlg.showModal();
    input.focus();
  }

  function hideToast() {
    clearTimeout(toastTimer);
    if (toast && toast.matches(':popover-open')) toast.hidePopover();
  }

  // Vom Host aufgerufen.
  // Vor einer Aktion: Leiste schließen, Fokus zurück an die Seite.
  window.__reliefHide = () => { if (dlg && dlg.open) dlg.close(); };
  // Ergebnis in der Leiste (Abfrage, Rückfrage, Bestätigung) …
  window.__reliefShow = (text, confirm) => {
    busy = false;
    open();
    out.textContent = text;
    hint.textContent = confirm ? '„ja“ bestätigt, „nein“ bricht ab' : HINT;
  };
  // … oder nach einer ausgeführten Aktion als Hinweis, ohne den Fokus zu
  // nehmen. Strg+Umschalt+Leertaste öffnet die Leiste wieder.
  window.__reliefToast = (text) => {
    busy = false;
    if (!host || !host.isConnected) build();
    toast.textContent = text;
    if (!toast.matches(':popover-open')) toast.showPopover();
    clearTimeout(toastTimer);
    toastTimer = setTimeout(hideToast, 10000);
  };
})();
