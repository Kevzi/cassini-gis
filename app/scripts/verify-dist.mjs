// Contrôle de `dist/` : chaque fichier que le paquet construit désigne existe.
//
// `npm run build` construit sans ouvrir le résultat : un asset nommé mais absent
// (le worker MapLibre, un jour) ne se voit qu'à l'ouverture de la fenêtre. Ici
// `dist/index.html` est lu, chaque adresse `/assets/...` est cherchée dans
// `dist/`, puis chaque `.js`, `.mjs` et `.css` trouvé est lu à son tour.
//
// À lancer après `npm run build` : `npm run verify:dist`. Hors ligne.

import { readFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const APP = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const DIST = path.join(APP, "dist");

/** Adresses `/assets/...` nommées par un texte. */
export function assetRefs(text) {
  return [...text.matchAll(/\/assets\/[A-Za-z0-9_.\-/]+\.[A-Za-z0-9]+/g)].map((m) => m[0]);
}

/**
 * `read(adresse)` rend le texte du fichier, ou null s'il n'existe pas.
 * Rend la liste des adresses manquantes.
 */
export function missingAssets(indexHtml, read) {
  const missing = [];
  const seen = new Set();
  const queue = assetRefs(indexHtml);
  while (queue.length > 0) {
    const ref = queue.shift();
    if (seen.has(ref)) continue;
    seen.add(ref);
    const content = read(ref);
    if (content === null) {
      missing.push(ref);
    } else if (/\.(js|mjs|css)$/.test(ref)) {
      queue.push(...assetRefs(content));
    }
  }
  return missing;
}

function selfTest() {
  const files = {
    "/assets/a.js": 'import "/assets/b.mjs"; new URL("/assets/w.mjs")',
    "/assets/b.mjs": "",
    "/assets/w.mjs": "",
    "/assets/s.css": "",
  };
  const index = '<script src="/assets/a.js"></script><link href="/assets/s.css">';
  const problems = [];
  if (missingAssets(index, (r) => files[r] ?? null).length !== 0) problems.push("dist complet refusé à tort");
  const without = { ...files };
  delete without["/assets/w.mjs"];
  const miss = missingAssets(index, (r) => without[r] ?? null);
  if (miss.join() !== "/assets/w.mjs") problems.push("asset manquant non nommé : " + miss.join());
  return problems;
}

const self = selfTest();
if (self.length > 0) {
  console.error("Contrôle de dist : ÉCHEC, le contrôle lui-même est faux.\n  " + self.join("\n  "));
  process.exit(1);
}
const indexPath = path.join(DIST, "index.html");
if (!existsSync(indexPath)) {
  console.error("Contrôle de dist : ÉCHEC, dist/index.html est absent. Lance d'abord « npm run build ».");
  process.exit(1);
}
const missing = missingAssets(readFileSync(indexPath, "utf8"), (ref) => {
  const file = path.join(DIST, ref.replace(/^\//, ""));
  return existsSync(file) ? readFileSync(file, "utf8") : null;
});
if (missing.length > 0) {
  console.error("Contrôle de dist : ÉCHEC, fichiers nommés mais absents de dist/ :\n  " + missing.join("\n  "));
  process.exit(1);
}
console.log("Contrôle de dist : OK (chaque asset nommé existe dans dist/).");
