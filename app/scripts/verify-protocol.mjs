// Vérificateur du branchement du fond de carte dans la carte.
//
// `verify-basemap.mjs` prouve que l'archive est conforme à la spécification.
// Ici on prouve que le **chemin de chargement** fonctionne : le vrai `Protocol`
// de la bibliothèque pmtiles, piloté sur une source qui lit le fichier du dépôt
// par plages, exactement comme la commande Rust le fait pour la fenêtre.
//
// Ce que ce script ne peut pas faire : ouvrir une fenêtre. L'affichage réel des
// tuiles et le relevé de trafic réseau nul demandent Tauri, et c'est l'entrée
// 1.4. D'ici là, ce script est la preuve du branchement.
//
// Il contrôle aussi que les trois endroits qui doivent s'accorder s'accordent :
// la clé portée par `app/src/basemap.source.ts` et celle du style, le chemin
// porté par `app/src-tauri/src/lib.rs` et la ressource de l'installeur, et le
// nom de la commande appelée par l'interface et celui enregistré par le moteur.
//
// Enfin il met le mode de défaillance à l'épreuve : une clé non enregistrée
// part-elle vraiment sur le réseau ? Oui — et c'est pour ça que la garde de
// `check-offline.mjs` connaît la clé attendue et pas seulement le schéma.

import { existsSync, closeSync, openSync, readFileSync, readSync, statSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { Protocol, PMTiles } from "pmtiles";
import { OUTPUT_PATH, EXTENT, ZOOM_MIN, ZOOM_MAX, ZOOM_CENTER, coveringTiles } from "./generate-basemap.mjs";
import { STYLE_PATH, basemapKey } from "./check-offline.mjs";

const PREFIX = "pmtiles://";
const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

const TAURI_DIR = fileURLToPath(new URL("../src-tauri/", import.meta.url));
const TAURI_CONF_PATH = new URL("../src-tauri/tauri.conf.json", import.meta.url);
const LIB_RS_PATH = new URL("../src-tauri/src/lib.rs", import.meta.url);
const SOURCE_TS_PATH = new URL("../src/basemap.source.ts", import.meta.url);

/** Le fichier de l'archive, nommé par le générateur et par personne d'autre. */
const ARCHIVE_PATH = resolve(OUTPUT_PATH);

// Aucune requête réseau n'est permise. Plutôt que de l'espérer, on casse `fetch`
// et on compte les tentatives : un chemin de chargement qui glisserait vers le
// réseau échoue ici, bruyamment.
let networkAttempts = 0;
globalThis.fetch = (resource) => {
  networkAttempts += 1;
  return Promise.reject(new Error(`requête réseau tentée vers ${String(resource)}`));
};

/** Le compte des requêtes réseau au moment où le chemin nominal est fini. */
let networkOnNominalPath = null;

let failures = 0;
const results = [];

function check(name, condition, detail = "") {
  if (condition) {
    results.push(`  ok   ${name}`);
  } else {
    failures += 1;
    results.push(`  ÉCHEC ${name}${detail ? ` — ${detail}` : ""}`);
  }
}

/**
 * Source PMTiles lisant le fichier du dépôt par plages.
 *
 * C'est une **doublure** de la commande Rust, pas elle : elle sert à faire
 * tourner le vrai `Protocol` sans fenêtre Tauri. Elle applique la même règle de
 * bornes — une plage qui dépasse la fin est refusée, jamais tronquée — mais son
 * message est le sien, et la tenue de cette règle par le moteur est contrôlée
 * là où elle est écrite, par les tests de `BasemapSource::read_range`.
 */
class FileSource {
  constructor(path, key) {
    this.path = path;
    this.key = key;
    this.size = statSync(path).size;
    this.rangesRead = 0;
  }

  getKey() {
    return this.key;
  }

  async getBytes(offset, length) {
    this.rangesRead += 1;
    if (offset + length > this.size) {
      throw new Error(
        `doublure : plage refusée, ${length} octets demandés à ${offset} dans un fichier de ${this.size}`,
      );
    }
    const buffer = Buffer.alloc(length);
    const fd = openSync(this.path, "r");
    try {
      const bytesRead = length === 0 ? 0 : readSync(fd, buffer, 0, length, offset);
      if (bytesRead !== length) {
        throw new Error(`lecture courte à ${offset} : ${bytesRead} octets au lieu de ${length}`);
      }
    } finally {
      closeSync(fd);
    }
    return { data: buffer.buffer.slice(buffer.byteOffset, buffer.byteOffset + buffer.byteLength) };
  }
}

/** La valeur de `const NAME: &str = "…"` dans un fichier Rust. */
function rustConstant(code, name) {
  const found = code.match(new RegExp(`const ${name}: &str = "([^"]+)"`));
  return found ? found[1] : null;
}

async function main() {
  if (!existsSync(OUTPUT_PATH)) {
    console.error(
      `Vérification du protocole : ÉCHEC, archive absente.\n  Lance d'abord : npm run fixtures`,
    );
    process.exit(1);
  }

  const key = basemapKey();
  const style = JSON.parse(readFileSync(STYLE_PATH, "utf8"));
  const tsSourceCode = readFileSync(SOURCE_TS_PATH, "utf8");
  const conf = JSON.parse(readFileSync(TAURI_CONF_PATH, "utf8"));
  const libRs = readFileSync(LIB_RS_PATH, "utf8");

  // --- Le style et le code portent la même clé -----------------------------
  const pmtilesSources = Object.entries(style.sources ?? {}).filter(
    ([, s]) => typeof s?.url === "string" && s.url.startsWith(PREFIX),
  );
  check(
    "le style déclare exactement une source pmtiles",
    pmtilesSources.length === 1,
    `${pmtilesSources.length} trouvée(s)`,
  );
  if (pmtilesSources.length !== 1) {
    report();
    process.exit(1);
  }
  const [sourceName, styleSource] = pmtilesSources[0];
  const address = styleSource.url;
  check(
    "la clé du style est celle du code, octet pour octet",
    address === `${PREFIX}${key}`,
    `style : ${address}, code : ${PREFIX}${key}`,
  );
  check("la source est de type raster", styleSource.type === "raster");

  // --- Tuiles de 256 px ----------------------------------------------------
  // MapLibre prend 512 par défaut pour une source raster, alors que le
  // générateur écrit des tuiles de 256 : sans `tileSize` explicite, l'échelle
  // de la carte est fausse d'un facteur deux, silencieusement.
  check(
    "le style déclare des tuiles de 256 px",
    styleSource.tileSize === 256,
    `lu : ${styleSource.tileSize}`,
  );
  check(
    "les bornes de zoom du style sont celles du générateur",
    styleSource.minzoom === ZOOM_MIN && styleSource.maxzoom === ZOOM_MAX,
    `style : ${styleSource.minzoom}–${styleSource.maxzoom}, générateur : ${ZOOM_MIN}–${ZOOM_MAX}`,
  );

  // --- La couche du fond passe sous le repère -------------------------------
  const layerIds = (style.layers ?? []).map((c) => c.id);
  const layer = (style.layers ?? []).find((c) => c.source === sourceName);
  check("une couche consomme la source du fond", layer !== undefined);
  check(
    "la couche du fond est dessinée sous le repère",
    layer !== undefined && layerIds.indexOf(layer.id) < layerIds.indexOf("marker-fill"),
    `ordre : ${layerIds.join(" < ")}`,
  );

  // --- Le protocole, piloté pour de vrai -----------------------------------
  const source = new FileSource(ARCHIVE_PATH, key);
  // `getHeaderAndRoot` de la bibliothèque demande toujours les 16 384 premiers
  // octets, sans regarder la taille du fichier. Comme le moteur refuse une plage
  // qui dépasse la fin plutôt que de la tronquer, une archive plus courte que ça
  // ne se chargerait pas du tout. Le contrôle est ici pour que ça se voie tout
  // de suite si le générateur rétrécit un jour son fond.
  check(
    "l'archive atteint les 16 384 octets du premier appel de la bibliothèque",
    source.size >= 16384,
    `${source.size} octets : la bibliothèque demanderait une plage hors fichier`,
  );
  const protocol = new Protocol();
  protocol.add(new PMTiles(source));
  check(
    "l'archive est enregistrée sous la clé du style, donc Protocol n'a pas à la chercher",
    protocol.get(address.slice(PREFIX.length)) !== undefined,
  );

  // --- Scénario : le TileJSON vient de l'en-tête, pas des métadonnées ------
  // Constat A10 : avec `metadata: false`, `tilev4` construit bounds, minzoom et
  // maxzoom depuis l'en-tête de l'archive. `getTileJson`, seul lecteur des
  // métadonnées JSON, n'est appelé que si `metadata` est vrai — et lui seul
  // ajoute un champ `tilejson`, dont l'absence le prouve ici.
  const tileJson = (await protocol.tilev4({ url: address, type: "json" }, new AbortController())).data;
  check(
    "le TileJSON annonce le gabarit de tuile du protocole",
    tileJson.tiles?.[0] === `${address}/{z}/{x}/{y}`,
    `lu : ${tileJson.tiles?.[0]}`,
  );
  check(
    "le TileJSON tient ses bornes de zoom de l'en-tête",
    tileJson.minzoom === ZOOM_MIN && tileJson.maxzoom === ZOOM_MAX,
    `lu : ${tileJson.minzoom}–${tileJson.maxzoom}`,
  );
  const near = (a, b) => Math.abs(a - b) < 1e-6;
  check(
    "le TileJSON tient son emprise de l'en-tête, pas du monde entier",
    Array.isArray(tileJson.bounds) &&
      near(tileJson.bounds[0], EXTENT.west) &&
      near(tileJson.bounds[1], EXTENT.south) &&
      near(tileJson.bounds[2], EXTENT.east) &&
      near(tileJson.bounds[3], EXTENT.north),
    `lu : ${JSON.stringify(tileJson.bounds)}`,
  );
  check(
    "les métadonnées JSON ne sont pas lues (metadata: false)",
    tileJson.tilejson === undefined,
    `un champ tilejson est apparu : ${tileJson.tilejson}`,
  );

  // --- Scénario : affichage nominal, une tuile connue ----------------------
  const tiles = coveringTiles();
  const centerTile = tiles.find((t) => t.z === ZOOM_CENTER) ?? tiles[0];
  const response = await protocol.tilev4(
    { url: `${address}/${centerTile.z}/${centerTile.x}/${centerTile.y}` },
    new AbortController(),
  );
  const tileBytes = response.data ? Buffer.from(response.data) : Buffer.alloc(0);
  check(
    "une tuile connue est servie, et c'est un PNG",
    tileBytes.subarray(0, 8).equals(PNG_SIGNATURE),
    `${tileBytes.length} octets, en-tête ${tileBytes.subarray(0, 8).toString("hex")}`,
  );

  // --- Scénario : tuile absente -------------------------------------------
  const outsideExtent = await protocol.tilev4(
    { url: `${address}/${ZOOM_CENTER}/0/0` },
    new AbortController(),
  );
  check(
    "une tuile hors de l'emprise revient vide, sans exception",
    outsideExtent.data === null,
    `lu : ${outsideExtent.data === null ? "null" : typeof outsideExtent.data}`,
  );
  const outsideZoom = await protocol.tilev4(
    { url: `${address}/${ZOOM_MAX + 1}/0/0` },
    new AbortController(),
  );
  check("une tuile au-delà du zoom maximum revient vide", outsideZoom.data === null);

  // --- Scénario : plage hors fichier --------------------------------------
  // Ce contrôle porte sur la doublure de ce script, pas sur le moteur : il
  // établit que le scénario est bien joué sur une source qui refuse, pas qui
  // tronque. Le refus du moteur, lui, est couvert par les tests unitaires de
  // `BasemapSource::read_range`, et nommer les trois nombres est leur travail.
  let rangeRejected = false;
  try {
    await source.getBytes(source.size - 1, 2);
  } catch {
    rangeRejected = true;
  }
  check(
    "la doublure de ce script refuse une plage hors fichier (le moteur a ses propres tests)",
    rangeRejected,
  );
  check(
    "l'archive a bien été lue par plages",
    source.rangesRead > 1,
    `${source.rangesRead} plage(s)`,
  );

  // --- Aucune requête réseau sur le chemin nominal -------------------------
  networkOnNominalPath = networkAttempts;
  check(
    "aucune requête réseau n'a été tentée",
    networkAttempts === 0,
    `${networkAttempts} tentative(s)`,
  );

  // --- Le mode de défaillance à craindre, mis à l'épreuve -----------------
  // Une clé que `Protocol` ne connaît pas le fait construire un `FetchSource`.
  // Sur un poste connecté, ça ne se verrait pas. Ici, `fetch` est cassé, donc
  // la tentative est comptée : c'est la preuve que la garde de check-offline
  // doit connaître la clé, et pas seulement le schéma.
  const bareProtocol = new Protocol();
  let wentToNetwork = false;
  try {
    await bareProtocol.tilev4(
      { url: `${PREFIX}une-cle-non-enregistree`, type: "json" },
      new AbortController(),
    );
  } catch (e) {
    wentToNetwork = /requête réseau tentée/.test(e.message);
  }
  check(
    "une clé non enregistrée part bien sur le réseau (donc la garde a un vrai travail)",
    wentToNetwork && networkAttempts > 0,
    wentToNetwork ? "" : "aucune requête réseau : le mode de défaillance a changé, revoir check-offline.mjs",
  );

  // --- Un seul endroit porte le chemin, un seul porte la clé ---------------
  const resources = conf.bundle?.resources;
  check(
    "l'installeur embarque l'archive en ressource",
    resources !== undefined && !Array.isArray(resources) && typeof resources === "object",
    `lu : ${JSON.stringify(resources)}`,
  );
  const resourceEntries = Object.entries(resources ?? {});
  check(
    "la ressource embarquée est exactement l'archive produite par le générateur",
    resourceEntries.length === 1 &&
      resolve(TAURI_DIR, resourceEntries[0][0]) === ARCHIVE_PATH,
    `ressource : ${resourceEntries.map(([s]) => s).join(", ")}, générateur : ${ARCHIVE_PATH}`,
  );
  const pathInEngine = rustConstant(libRs, "ARCHIVE_PATH");
  check(
    "le moteur et l'installeur portent le même chemin de ressource",
    pathInEngine !== null && resourceEntries[0]?.[1] === pathInEngine,
    `moteur : ${pathInEngine}, installeur : ${resourceEntries[0]?.[1]}`,
  );
  const command = tsSourceCode.match(/const COMMAND = "([^"]+)"/)?.[1] ?? null;
  check(
    "la commande appelée par l'interface est enregistrée par le moteur",
    command !== null &&
      new RegExp(`fn ${command}\\s*\\(`).test(libRs) &&
      new RegExp(`generate_handler!\\[[^\\]]*\\b${command}\\b`, "s").test(libRs),
    `commande : ${command}`,
  );

  report();
  if (failures > 0) process.exit(1);
}

function report() {
  console.log(
    failures === 0
      ? `Vérification du protocole : OK, ${results.length} contrôles.`
      : `Vérification du protocole : ÉCHEC, ${failures} contrôle(s) sur ${results.length}.`,
  );
  for (const l of results) if (failures > 0 || l.startsWith("  ÉCHEC")) console.log(l);
  console.log(
    `  chargement par plages locales, ${networkOnNominalPath ?? "?"} requête(s) réseau sur le chemin nominal.`,
  );
}

main().catch((e) => {
  console.error(`Vérification du protocole : ÉCHEC, exception inattendue.\n  ${e.stack ?? e.message}`);
  process.exit(1);
});
