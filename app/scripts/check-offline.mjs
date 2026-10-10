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

const ICI = dirname(fileURLToPath(import.meta.url));
export const CHEMIN_DU_STYLE = join(ICI, "..", "src", "basemap.style.json");
export const CHEMIN_DE_LA_SOURCE = join(ICI, "..", "src", "basemap.source.ts");
export const DOSSIER_DU_CODE = join(ICI, "..", "src");
export const PAGE_DE_LA_FENETRE = join(ICI, "..", "index.html");

/** Les extensions de fichiers servis à la fenêtre, donc passés au crible. */
const EXTENSIONS_CRIBLEES = [".ts", ".tsx", ".js", ".mjs", ".css", ".html"];

const PREFIXE = "pmtiles://";

/** `glyphs` et `sprite` : les deux portes de sortie classiques d'un style MapLibre. */
const CLES_A_SURVEILLER = ["glyphs", "sprite"];

/**
 * La clé du fond de carte, relue dans le seul fichier qui la déclare.
 *
 * Une extraction par expression régulière plutôt qu'un import : ce script est du
 * JavaScript brut lancé par Node, il ne peut pas importer un module TypeScript.
 */
export function cleDuFond(chemin = CHEMIN_DE_LA_SOURCE) {
  const code = readFileSync(chemin, "utf8");
  const trouve = code.match(/export const CLE_DU_FOND = "([^"]+)"/);
  if (!trouve) {
    throw new Error(
      `clé du fond introuvable dans ${chemin} : la garde ne peut rien vérifier sans elle`,
    );
  }
  return trouve[1];
}

/** Les deux seules formes de `pmtiles://` tolérées : l'archive, et son gabarit de tuile. */
function formesAutorisees(cle) {
  return [`${PREFIXE}${cle}`, `${PREFIXE}${cle}/{z}/{x}/{y}`];
}

/**
 * Pourquoi cette chaîne est une faute, ou `null` si elle est acceptable.
 *
 * Fonction pure : c'est elle que les cas négatifs mettent en échec exprès.
 */
export function fauteDeChaine(valeur, cle) {
  const bas = valeur.toLowerCase();
  if (bas.startsWith(PREFIXE)) {
    const reste = valeur.slice(PREFIXE.length);
    if (reste.includes("://") || reste.startsWith("//")) {
      return `adresse distante imbriquée derrière ${PREFIXE} : ${valeur}`;
    }
    if (!formesAutorisees(cle).includes(valeur)) {
      return `clé inconnue derrière ${PREFIXE} (attendu « ${cle} ») : ${valeur} — Protocol en ferait un FetchSource, donc une requête réseau`;
    }
    return null;
  }
  if (bas.includes("://") || bas.startsWith("//")) {
    return `adresse réseau : ${valeur}`;
  }
  return null;
}

/** Les fautes d'un style MapLibre, parcouru valeur par valeur. */
export function analyserStyle(style, cle) {
  const fautes = [];

  const parcourir = (valeur, trace) => {
    if (typeof valeur === "string") {
      const faute = fauteDeChaine(valeur, cle);
      if (faute) fautes.push(`${trace} : ${faute}`);
      return;
    }
    if (Array.isArray(valeur)) {
      valeur.forEach((v, i) => parcourir(v, `${trace}[${i}]`));
      return;
    }
    if (valeur && typeof valeur === "object") {
      for (const [k, v] of Object.entries(valeur)) parcourir(v, `${trace}.${k}`);
    }
  };
  parcourir(style, "style");

  for (const k of CLES_A_SURVEILLER) {
    if (style && typeof style === "object" && k in style) {
      fautes.push(`style.${k} est défini : vérifier qu'il est local (${style[k]})`);
    }
  }
  return fautes;
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
 * revanche, un schéma nu reste une faute, et `analyserStyle` le refuse.
 */
export function analyserCode(texte, trace, cle) {
  const fautes = [];
  for (const [brut] of texte.matchAll(/\b[a-z][a-z0-9+.-]*:\/\/[^\s"'`),]+/gi)) {
    const faute = fauteDeChaine(brut, cle);
    if (faute) fautes.push(`${trace} : ${faute}`);
  }
  for (const [brut] of texte.matchAll(/[("'`]\/\/[^\s"'`)]+/g)) {
    fautes.push(`${trace} : adresse réseau relative au protocole : ${brut}`);
  }
  return fautes;
}

/**
 * Les fichiers servis à la fenêtre, dans l'ordre, pour un rapport stable.
 *
 * Parcours **récursif** : un sous-dossier ajouté plus tard est passé au crible
 * sans que personne ait à y penser.
 */
export function fichiersDeCode(dossier = DOSSIER_DU_CODE, page = PAGE_DE_LA_FENETRE) {
  const parcourir = (d) =>
    readdirSync(d, { withFileTypes: true })
      .sort((a, b) => a.name.localeCompare(b.name))
      .flatMap((e) =>
        e.isDirectory()
          ? parcourir(join(d, e.name))
          : EXTENSIONS_CRIBLEES.some((x) => e.name.endsWith(x))
            ? [join(d, e.name)]
            : [],
      );
  return [page, ...parcourir(dossier)];
}

// ------------------------------------------------------------- cas négatifs

// Un garde-fou qu'on n'a jamais vu refuser quelque chose ne garde rien. Ces cas
// vivent en mémoire : ils ne touchent aucun fichier du dépôt.
const sourceRaster = (adresse) => ({
  version: 8,
  sources: { fond: { type: "raster", url: adresse, tileSize: 256 } },
  layers: [],
});

const CAS_NEGATIFS_STYLE = [
  ["une source distante en clair", sourceRaster("https://tuiles.exemple.fr/{z}/{x}/{y}.png")],
  ["une adresse relative au protocole", sourceRaster("//tuiles.exemple.fr/{z}/{x}/{y}.png")],
  ["une adresse distante imbriquée derrière pmtiles://", sourceRaster(`${PREFIXE}https://exemple.fr/f.pmtiles`)],
  ["une clé inconnue derrière pmtiles://", sourceRaster(`${PREFIXE}une-autre-cle`)],
  ["un gabarit de tuile sur une autre clé", sourceRaster(`${PREFIXE}une-autre-cle/{z}/{x}/{y}`)],
  ["des glyphes", { version: 8, glyphs: "fonts/{fontstack}/{range}.pbf", sources: {}, layers: [] }],
  ["un sprite", { version: 8, sprite: "sprites/base", sources: {}, layers: [] }],
  ["une adresse cachée au fond d'une expression", {
    version: 8,
    sources: {},
    layers: [{ id: "x", type: "symbol", layout: { "icon-image": ["concat", "https://exemple.fr/", "p.png"] } }],
  }],
];

const CAS_NEGATIFS_CODE = [
  ["un fetch distant", 'await fetch("https://exemple.fr/f.pmtiles");'],
  ["une adresse relative au protocole", 'const u = "//exemple.fr/f.pmtiles";'],
  ["une autre clé pmtiles", `const u = "${PREFIXE}une-autre-cle";`],
  ["une adresse distante imbriquée", `const u = "${PREFIXE}https://exemple.fr/f.pmtiles";`],
  ["une feuille de style qui importe une police distante", "@import url(https://exemple.fr/police.css);"],
  ["une feuille de style relative au protocole", "@import url(//exemple.fr/police.css);"],
  ["une image de fond distante", "body { background: url(https://exemple.fr/fond.png); }"],
  ["une page qui charge un script distant", '<script src="https://exemple.fr/s.js"></script>'],
];

// ------------------------------------------------------------------ rapport

function principal() {
  const cle = cleDuFond();
  const style = JSON.parse(readFileSync(CHEMIN_DU_STYLE, "utf8"));

  const fautes = analyserStyle(style, cle);
  const fichiers = fichiersDeCode();
  for (const chemin of fichiers) {
    fautes.push(...analyserCode(readFileSync(chemin, "utf8"), chemin, cle));
  }

  // La clé doit apparaître dans le style, sinon la garde valide un style qui ne
  // charge rien, ou qui charge autre chose.
  const adresses = [];
  const collecter = (v) => {
    if (typeof v === "string") {
      if (v.toLowerCase().startsWith(PREFIXE)) adresses.push(v);
    } else if (Array.isArray(v)) v.forEach(collecter);
    else if (v && typeof v === "object") Object.values(v).forEach(collecter);
  };
  collecter(style);
  if (!adresses.includes(`${PREFIXE}${cle}`)) {
    fautes.push(
      `aucune source du style ne vaut ${PREFIXE}${cle} : la clé du code et celle du style ont divergé (vu : ${adresses.join(", ") || "aucune"})`,
    );
  }

  // Les cas négatifs : la garde doit refuser chacun d'eux.
  const manques = [];
  for (const [nom, cas] of CAS_NEGATIFS_STYLE) {
    if (analyserStyle(cas, cle).length === 0) manques.push(`style — ${nom}`);
  }
  for (const [nom, texte] of CAS_NEGATIFS_CODE) {
    if (analyserCode(texte, "cas", cle).length === 0) manques.push(`code — ${nom}`);
  }
  // …et accepter la forme légitime, sinon elle refuserait tout, ce qui ne prouve
  // rien non plus.
  for (const forme of formesAutorisees(cle)) {
    if (fauteDeChaine(forme, cle) !== null) manques.push(`forme légitime refusée — ${forme}`);
  }

  if (manques.length > 0) {
    console.error("Garde hors ligne : ÉCHEC, la garde ne sait pas refuser :");
    for (const m of manques) console.error(`  - ${m}`);
    process.exit(1);
  }
  if (fautes.length > 0) {
    console.error("Garde hors ligne : ÉCHEC, le réseau est désigné.");
    for (const f of fautes) console.error(`  - ${f}`);
    process.exit(1);
  }

  console.log(
    `Garde hors ligne : OK, ${Object.keys(style.sources).length} source(s) de style et ${fichiers.length} fichier(s) servi(s) à la fenêtre, aucune adresse réseau.`,
  );
  console.log(
    `  seule exception admise : ${PREFIXE}${cle}, et ${CAS_NEGATIFS_STYLE.length + CAS_NEGATIFS_CODE.length} cas négatifs sont refusés comme prévu.`,
  );
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) principal();
