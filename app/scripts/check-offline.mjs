// Garde « hors ligne total » (CLAUDE.md §2.3), côté interface.
//
// Échoue si le style du fond de carte, ou n'importe quel fichier servi à la
// fenêtre, contient la moindre adresse réseau. Une seule exception, nommée : le
// schéma `pmtiles://` suivi de la clé locale attendue.
//
// Le crible couvre `app/index.html` et tout `app/src`, sous-dossiers compris :
// une feuille de style est un chemin de sortie aussi direct qu'un `fetch`, parce
// qu'`@import url(https://…)` et `url(//…)` sont de vraies requêtes réseau.
//
// Pourquoi la clé et pas seulement le schéma : le `Protocol` de la bibliothèque
// pmtiles traite ce qui suit `pmtiles://` comme une clé de dictionnaire. Une clé
// qu'il ne connaît pas le fait construire un `PMTiles` sur un `FetchSource`,
// c'est-à-dire une requête réseau, sans un mot dans la console ; et
// `pmtiles://https://exemple.fr/f.pmtiles` lui est une adresse parfaitement
// valable. Reconnaître le schéma ne garde donc rien.
//
// La clé attendue n'est pas écrite ici : elle est relue dans
// `app/src/basemap.source.ts`, seul endroit du code qui la porte. Si le style et
// le code divergent, cette garde échoue.
//
// Le moteur applique la même règle de son côté (core/src/basemap.rs), parce
// qu'un style peut être fourni par l'utilisateur comme par l'application.

import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
export const STYLE_PATH = join(HERE, "..", "src", "basemap.style.json");
export const SOURCE_PATH = join(HERE, "..", "src", "basemap.source.ts");
export const CODE_DIR = join(HERE, "..", "src");
export const WINDOW_PAGE = join(HERE, "..", "index.html");

/** Les extensions de fichiers servis à la fenêtre, donc passés au crible. */
const SCREENED_EXTENSIONS = [".ts", ".tsx", ".js", ".mjs", ".css", ".html"];

const PREFIX = "pmtiles://";

/** `glyphs` et `sprite` : les deux portes de sortie classiques d'un style MapLibre. */
const KEYS_TO_WATCH = ["glyphs", "sprite"];

/**
 * La clé du fond de carte, relue dans le seul fichier qui la déclare.
 *
 * Une extraction par expression régulière plutôt qu'un import : ce script est du
 * JavaScript brut lancé par Node, il ne peut pas importer un module TypeScript.
 */
export function basemapKey(path = SOURCE_PATH) {
  const code = readFileSync(path, "utf8");
  const found = code.match(/export const BASEMAP_KEY = "([^"]+)"/);
  if (!found) {
    throw new Error(
      `clé du fond introuvable dans ${path} : la garde ne peut rien vérifier sans elle`,
    );
  }
  return found[1];
}

/** Les deux seules formes de `pmtiles://` tolérées : l'archive, et son gabarit de tuile. */
function allowedForms(key) {
  return [`${PREFIX}${key}`, `${PREFIX}${key}/{z}/{x}/{y}`];
}

/**
 * Pourquoi cette chaîne est une faute, ou `null` si elle est acceptable.
 *
 * Fonction pure : c'est elle que les cas négatifs mettent en échec exprès.
 */
export function stringFault(value, key) {
  const lower = value.toLowerCase();
  if (lower.startsWith(PREFIX)) {
    const rest = value.slice(PREFIX.length);
    if (rest.includes("://") || rest.startsWith("//")) {
      return `adresse distante imbriquée derrière ${PREFIX} : ${value}`;
    }
    if (!allowedForms(key).includes(value)) {
      return `clé inconnue derrière ${PREFIX} (attendu « ${key} ») : ${value} — Protocol en ferait un FetchSource, donc une requête réseau`;
    }
    return null;
  }
  if (lower.includes("://") || lower.startsWith("//")) {
    return `adresse réseau : ${value}`;
  }
  return null;
}

/** Les fautes d'un style MapLibre, parcouru valeur par valeur. */
export function analyzeStyle(style, key) {
  const faults = [];

  const walk = (value, trace) => {
    if (typeof value === "string") {
      const fault = stringFault(value, key);
      if (fault) faults.push(`${trace} : ${fault}`);
      return;
    }
    if (Array.isArray(value)) {
      value.forEach((v, i) => walk(v, `${trace}[${i}]`));
      return;
    }
    if (value && typeof value === "object") {
      for (const [k, v] of Object.entries(value)) walk(v, `${trace}.${k}`);
    }
  };
  walk(style, "style");

  for (const k of KEYS_TO_WATCH) {
    if (style && typeof style === "object" && k in style) {
      faults.push(`style.${k} est défini : vérifier qu'il est local (${style[k]})`);
    }
  }
  return faults;
}

/**
 * Les fautes d'un fichier de code.
 *
 * Le crible du style ne convient pas tel quel : `//` ouvre un commentaire en
 * TypeScript, donc seules deux formes sont cherchées, un schéma d'URL suivi d'au
 * moins un caractère, et une adresse relative au protocole ouvrant une chaîne ou
 * une parenthèse — cette dernière pour le `url(//…)` des feuilles de style.
 *
 * Le « au moins un caractère » laisse passer un schéma nu cité en commentaire,
 * comme `pmtiles://` dans une explication : il ne charge rien. Dans un style, en
 * revanche, un schéma nu reste une faute, et `analyzeStyle` le refuse.
 */
export function analyzeCode(text, trace, key) {
  const faults = [];
  for (const [raw] of text.matchAll(/\b[a-z][a-z0-9+.-]*:\/\/[^\s"'`),]+/gi)) {
    const fault = stringFault(raw, key);
    if (fault) faults.push(`${trace} : ${fault}`);
  }
  for (const [raw] of text.matchAll(/[("'`]\/\/[^\s"'`)]+/g)) {
    faults.push(`${trace} : adresse réseau relative au protocole : ${raw}`);
  }
  return faults;
}

/**
 * Les fichiers servis à la fenêtre, dans l'ordre, pour un rapport stable.
 *
 * Parcours **récursif** : un sous-dossier ajouté plus tard est passé au crible
 * sans que personne ait à y penser.
 */
export function codeFiles(dir = CODE_DIR, page = WINDOW_PAGE) {
  const walk = (d) =>
    readdirSync(d, { withFileTypes: true })
      .sort((a, b) => a.name.localeCompare(b.name))
      .flatMap((e) =>
        e.isDirectory()
          ? walk(join(d, e.name))
          : SCREENED_EXTENSIONS.some((x) => e.name.endsWith(x))
            ? [join(d, e.name)]
            : [],
      );
  return [page, ...walk(dir)];
}

// ------------------------------------------------------------- cas négatifs

// Un garde-fou qu'on n'a jamais vu refuser quelque chose ne garde rien. Ces cas
// vivent en mémoire : ils ne touchent aucun fichier du dépôt.
const rasterSource = (address) => ({
  version: 8,
  sources: { basemap: { type: "raster", url: address, tileSize: 256 } },
  layers: [],
});

const NEGATIVE_STYLE_CASES = [
  ["une source distante en clair", rasterSource("https://tuiles.exemple.fr/{z}/{x}/{y}.png")],
  ["une adresse relative au protocole", rasterSource("//tuiles.exemple.fr/{z}/{x}/{y}.png")],
  ["une adresse distante imbriquée derrière pmtiles://", rasterSource(`${PREFIX}https://exemple.fr/f.pmtiles`)],
  ["une clé inconnue derrière pmtiles://", rasterSource(`${PREFIX}une-autre-cle`)],
  ["un gabarit de tuile sur une autre clé", rasterSource(`${PREFIX}une-autre-cle/{z}/{x}/{y}`)],
  ["des glyphes", { version: 8, glyphs: "fonts/{fontstack}/{range}.pbf", sources: {}, layers: [] }],
  ["un sprite", { version: 8, sprite: "sprites/base", sources: {}, layers: [] }],
  ["une adresse cachée au fond d'une expression", {
    version: 8,
    sources: {},
    layers: [{ id: "x", type: "symbol", layout: { "icon-image": ["concat", "https://exemple.fr/", "p.png"] } }],
  }],
];

const NEGATIVE_CODE_CASES = [
  ["un fetch distant", 'await fetch("https://exemple.fr/f.pmtiles");'],
  ["une adresse relative au protocole", 'const u = "//exemple.fr/f.pmtiles";'],
  ["une autre clé pmtiles", `const u = "${PREFIX}une-autre-cle";`],
  ["une adresse distante imbriquée", `const u = "${PREFIX}https://exemple.fr/f.pmtiles";`],
  ["une feuille de style qui importe une police distante", "@import url(https://exemple.fr/police.css);"],
  ["une feuille de style relative au protocole", "@import url(//exemple.fr/police.css);"],
  ["une image de fond distante", "body { background: url(https://exemple.fr/fond.png); }"],
  ["une page qui charge un script distant", '<script src="https://exemple.fr/s.js"></script>'],
];

// ------------------------------------------------------------------ rapport

function main() {
  const key = basemapKey();
  const style = JSON.parse(readFileSync(STYLE_PATH, "utf8"));

  const faults = analyzeStyle(style, key);
  const files = codeFiles();
  for (const path of files) {
    faults.push(...analyzeCode(readFileSync(path, "utf8"), path, key));
  }

  // La clé doit apparaître dans le style, sinon la garde valide un style qui ne
  // charge rien, ou qui charge autre chose.
  const addresses = [];
  const collect = (v) => {
    if (typeof v === "string") {
      if (v.toLowerCase().startsWith(PREFIX)) addresses.push(v);
    } else if (Array.isArray(v)) v.forEach(collect);
    else if (v && typeof v === "object") Object.values(v).forEach(collect);
  };
  collect(style);
  if (!addresses.includes(`${PREFIX}${key}`)) {
    faults.push(
      `aucune source du style ne vaut ${PREFIX}${key} : la clé du code et celle du style ont divergé (vu : ${addresses.join(", ") || "aucune"})`,
    );
  }

  // Les cas négatifs : la garde doit refuser chacun d'eux.
  const gaps = [];
  for (const [name, testCase] of NEGATIVE_STYLE_CASES) {
    if (analyzeStyle(testCase, key).length === 0) gaps.push(`style — ${name}`);
  }
  for (const [name, text] of NEGATIVE_CODE_CASES) {
    if (analyzeCode(text, "cas", key).length === 0) gaps.push(`code — ${name}`);
  }
  // …et accepter la forme légitime, sinon elle refuserait tout, ce qui ne prouve
  // rien non plus.
  for (const form of allowedForms(key)) {
    if (stringFault(form, key) !== null) gaps.push(`forme légitime refusée — ${form}`);
  }

  if (gaps.length > 0) {
    console.error("Garde hors ligne : ÉCHEC, la garde ne sait pas refuser :");
    for (const g of gaps) console.error(`  - ${g}`);
    process.exit(1);
  }
  if (faults.length > 0) {
    console.error("Garde hors ligne : ÉCHEC, le réseau est désigné.");
    for (const f of faults) console.error(`  - ${f}`);
    process.exit(1);
  }

  console.log(
    `Garde hors ligne : OK, ${Object.keys(style.sources).length} source(s) de style et ${files.length} fichier(s) servi(s) à la fenêtre, aucune adresse réseau.`,
  );
  console.log(
    `  seule exception admise : ${PREFIX}${key}, et ${NEGATIVE_STYLE_CASES.length + NEGATIVE_CODE_CASES.length} cas négatifs sont refusés comme prévu.`,
  );
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) main();
