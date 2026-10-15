// Contrôle de concordance des ids entre index.html, main.ts et style.css.
//
// `tsc --noEmit` ne voit pas le HTML : un `id="map"` qui ne correspondrait plus
// à `container: "map"` laisserait la recette au vert et donnerait une page
// blanche. Ici chaque id du HTML doit être visé par le code ou le style, et
// chaque cible du code ou du style doit exister dans le HTML.
//
// Exécuté par `npm run check`. Hors ligne, sans dépendance.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const APP = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

export function htmlIds(html) {
  return new Set([...html.matchAll(/\bid\s*=\s*"([^"]+)"/g)].map((m) => m[1]));
}

export function tsTargets(ts) {
  const code = ts.replace(/\/\*[\s\S]*?\*\//g, "").replace(/^\s*\/\/.*$/gm, "");
  const out = new Set();
  for (const m of code.matchAll(/["'`]#([A-Za-z_][\w-]*)["'`]/g)) out.add(m[1]);
  for (const m of code.matchAll(/\bcontainer\s*:\s*["'`]([^"'`]+)["'`]/g)) out.add(m[1]);
  for (const m of code.matchAll(/getElementById\(\s*["'`]([^"'`]+)["'`]\s*\)/g)) out.add(m[1]);
  return out;
}

export function cssTargets(css) {
  let text = css.replace(/\/\*[\s\S]*?\*\//g, "");
  // Ne garde que les sélecteurs : retire les corps de règles, du plus profond au plus haut.
  let previous;
  do {
    previous = text;
    text = text.replace(/\{[^{}]*\}/g, " ");
  } while (text !== previous);
  return new Set([...text.matchAll(/#([A-Za-z_][\w-]*)/g)].map((m) => m[1]));
}

/** Rend la liste des erreurs, en français, vide quand tout concorde. */
export function checkIds({ html, ts, css }) {
  const ids = htmlIds(html);
  const tsSet = tsTargets(ts);
  const cssSet = cssTargets(css);
  const errors = [];
  for (const id of ids) {
    if (!tsSet.has(id) && !cssSet.has(id)) {
      errors.push(`id mort : « ${id} » est dans index.html mais ni main.ts ni style.css ne le vise`);
    }
  }
  for (const [file, set] of [["main.ts", tsSet], ["style.css", cssSet]]) {
    for (const id of set) {
      if (!ids.has(id)) errors.push(`id orphelin : « ${id} » est visé par ${file} mais absent de index.html`);
    }
  }
  return errors;
}

function selfTest() {
  const html = '<div id="map"></div><div id="status"></div>';
  const ts = 'document.querySelector("#status"); new M({ container: "map" });';
  const css = ":root { --c: #e8e8e4; }\n#map { x: 1 }\n#status { y: 2 }";
  const cases = [
    ["ids concordants", { html, ts, css }, 0],
    ["id orphelin", { html, ts: ts.replace('"map"', '"carte"'), css }, 1],
    ["id mort", { html: html + '<p id="inutile"></p>', ts, css }, 1],
  ];
  const problems = [];
  for (const [name, input, expected] of cases) {
    const found = checkIds(input);
    if (found.length !== expected) problems.push(`autocontrôle « ${name} » : ${found.length} erreur(s), ${expected} attendue(s)`);
  }
  const orphan = checkIds(cases[1][1]).join("\n");
  if (!orphan.includes("carte")) problems.push("autocontrôle : l'id orphelin « carte » n'est pas nommé");
  if (!checkIds(cases[2][1]).join("\n").includes("inutile")) problems.push("autocontrôle : l'id mort n'est pas nommé");
  return problems;
}

const self = selfTest();
if (self.length > 0) {
  console.error("Contrôle des ids : ÉCHEC, le contrôle lui-même est faux.\n  " + self.join("\n  "));
  process.exit(1);
}
const errors = checkIds({
  html: readFileSync(path.join(APP, "index.html"), "utf8"),
  ts: readFileSync(path.join(APP, "src", "main.ts"), "utf8"),
  css: readFileSync(path.join(APP, "src", "style.css"), "utf8"),
});
if (errors.length > 0) {
  console.error("Contrôle des ids : ÉCHEC.\n  " + errors.join("\n  "));
  process.exit(1);
}
console.log("Contrôle des ids : OK (index.html, main.ts et style.css concordent).");
