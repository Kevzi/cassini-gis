// Garde « hors ligne total » (CLAUDE.md §2.3), côté interface.
//
// Échoue si le style du fond de carte contient la moindre adresse réseau. Le
// moteur applique la même règle de son côté (core/src/basemap.rs), parce qu'un
// style peut être fourni par l'utilisateur comme par l'application.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ici = dirname(fileURLToPath(import.meta.url));
const chemin = join(ici, "..", "src", "basemap.style.json");
const style = JSON.parse(readFileSync(chemin, "utf8"));

const fautes = [];

function parcourir(valeur, trace) {
  if (typeof valeur === "string") {
    const bas = valeur.toLowerCase();
    if (bas.includes("://") || bas.startsWith("//")) {
      fautes.push(`${trace} = ${valeur}`);
    }
    return;
  }
  if (Array.isArray(valeur)) {
    valeur.forEach((v, i) => parcourir(v, `${trace}[${i}]`));
    return;
  }
  if (valeur && typeof valeur === "object") {
    for (const [cle, v] of Object.entries(valeur)) parcourir(v, `${trace}.${cle}`);
  }
}

parcourir(style, "style");

// `glyphs` et `sprite` sont les deux portes de sortie classiques d'un style
// MapLibre : si elles apparaissent un jour, elles doivent être locales.
for (const cle of ["glyphs", "sprite"]) {
  if (cle in style) fautes.push(`style.${cle} est défini : vérifier qu'il est local (${style[cle]})`);
}

if (fautes.length > 0) {
  console.error("Garde hors ligne : échec, le style désigne le réseau.");
  for (const f of fautes) console.error(`  - ${f}`);
  process.exit(1);
}

console.log(`Garde hors ligne : OK, ${Object.keys(style.sources).length} source(s), aucune adresse réseau.`);
