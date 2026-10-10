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
import { CHEMIN_SORTIE, EMPRISE, ZOOM_MIN, ZOOM_MAX, ZOOM_CENTRE, tuilesCouvrantes } from "./generate-basemap.mjs";
import { CHEMIN_DU_STYLE, cleDuFond } from "./check-offline.mjs";

const PREFIXE = "pmtiles://";
const SIGNATURE_PNG = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

const DOSSIER_TAURI = fileURLToPath(new URL("../src-tauri/", import.meta.url));
const CHEMIN_CONF_TAURI = new URL("../src-tauri/tauri.conf.json", import.meta.url);
const CHEMIN_LIB_RS = new URL("../src-tauri/src/lib.rs", import.meta.url);
const CHEMIN_SOURCE_TS = new URL("../src/basemap.source.ts", import.meta.url);

/** Le fichier de l'archive, nommé par le générateur et par personne d'autre. */
const CHEMIN_DE_LARCHIVE = resolve(CHEMIN_SORTIE);

// Aucune requête réseau n'est permise. Plutôt que de l'espérer, on casse `fetch`
// et on compte les tentatives : un chemin de chargement qui glisserait vers le
// réseau échoue ici, bruyamment.
let tentativesReseau = 0;
globalThis.fetch = (ressource) => {
  tentativesReseau += 1;
  return Promise.reject(new Error(`requête réseau tentée vers ${String(ressource)}`));
};

/** Le compte des requêtes réseau au moment où le chemin nominal est fini. */
let reseauSurLeCheminNominal = null;

let echecs = 0;
const resultats = [];

function verifier(nom, condition, detail = "") {
  if (condition) {
    resultats.push(`  ok   ${nom}`);
  } else {
    echecs += 1;
    resultats.push(`  ÉCHEC ${nom}${detail ? ` — ${detail}` : ""}`);
  }
}

/**
 * Source PMTiles lisant le fichier du dépôt par plages.
 *
 * C'est une **doublure** de la commande Rust, pas elle : elle sert à faire
 * tourner le vrai `Protocol` sans fenêtre Tauri. Elle applique la même règle de
 * bornes — une plage qui dépasse la fin est refusée, jamais tronquée — mais son
 * message est le sien, et la tenue de cette règle par le moteur est contrôlée
 * là où elle est écrite, par les tests de `BasemapSource::lire_plage`.
 */
class SourceFichier {
  constructor(chemin, cle) {
    this.chemin = chemin;
    this.cle = cle;
    this.taille = statSync(chemin).size;
    this.plagesLues = 0;
  }

  getKey() {
    return this.cle;
  }

  async getBytes(decalage, longueur) {
    this.plagesLues += 1;
    if (decalage + longueur > this.taille) {
      throw new Error(
        `doublure : plage refusée, ${longueur} octets demandés à ${decalage} dans un fichier de ${this.taille}`,
      );
    }
    const tampon = Buffer.alloc(longueur);
    const fd = openSync(this.chemin, "r");
    try {
      const lus = longueur === 0 ? 0 : readSync(fd, tampon, 0, longueur, decalage);
      if (lus !== longueur) {
        throw new Error(`lecture courte à ${decalage} : ${lus} octets au lieu de ${longueur}`);
      }
    } finally {
      closeSync(fd);
    }
    return { data: tampon.buffer.slice(tampon.byteOffset, tampon.byteOffset + tampon.byteLength) };
  }
}

/** La valeur de `const NOM: &str = "…"` dans un fichier Rust. */
function constanteRust(code, nom) {
  const trouve = code.match(new RegExp(`const ${nom}: &str = "([^"]+)"`));
  return trouve ? trouve[1] : null;
}

async function principal() {
  if (!existsSync(CHEMIN_SORTIE)) {
    console.error(
      `Vérification du protocole : ÉCHEC, archive absente.\n  Lance d'abord : npm run fixtures`,
    );
    process.exit(1);
  }

  const cle = cleDuFond();
  const style = JSON.parse(readFileSync(CHEMIN_DU_STYLE, "utf8"));
  const codeSourceTs = readFileSync(CHEMIN_SOURCE_TS, "utf8");
  const conf = JSON.parse(readFileSync(CHEMIN_CONF_TAURI, "utf8"));
  const libRs = readFileSync(CHEMIN_LIB_RS, "utf8");

  // --- Le style et le code portent la même clé -----------------------------
  const sourcesPmtiles = Object.entries(style.sources ?? {}).filter(
    ([, s]) => typeof s?.url === "string" && s.url.startsWith(PREFIXE),
  );
  verifier(
    "le style déclare exactement une source pmtiles",
    sourcesPmtiles.length === 1,
    `${sourcesPmtiles.length} trouvée(s)`,
  );
  if (sourcesPmtiles.length !== 1) {
    rapporter();
    process.exit(1);
  }
  const [nomDeLaSource, sourceDuStyle] = sourcesPmtiles[0];
  const adresse = sourceDuStyle.url;
  verifier(
    "la clé du style est celle du code, octet pour octet",
    adresse === `${PREFIXE}${cle}`,
    `style : ${adresse}, code : ${PREFIXE}${cle}`,
  );
  verifier("la source est de type raster", sourceDuStyle.type === "raster");

  // --- Tuiles de 256 px ----------------------------------------------------
  // MapLibre prend 512 par défaut pour une source raster, alors que le
  // générateur écrit des tuiles de 256 : sans `tileSize` explicite, l'échelle
  // de la carte est fausse d'un facteur deux, silencieusement.
  verifier(
    "le style déclare des tuiles de 256 px",
    sourceDuStyle.tileSize === 256,
    `lu : ${sourceDuStyle.tileSize}`,
  );
  verifier(
    "les bornes de zoom du style sont celles du générateur",
    sourceDuStyle.minzoom === ZOOM_MIN && sourceDuStyle.maxzoom === ZOOM_MAX,
    `style : ${sourceDuStyle.minzoom}–${sourceDuStyle.maxzoom}, générateur : ${ZOOM_MIN}–${ZOOM_MAX}`,
  );

  // --- La couche du fond passe sous le repère -------------------------------
  const idsDesCouches = (style.layers ?? []).map((c) => c.id);
  const couche = (style.layers ?? []).find((c) => c.source === nomDeLaSource);
  verifier("une couche consomme la source du fond", couche !== undefined);
  verifier(
    "la couche du fond est dessinée sous le repère",
    couche !== undefined &&
      idsDesCouches.indexOf(couche.id) < idsDesCouches.indexOf("repere-surface"),
    `ordre : ${idsDesCouches.join(" < ")}`,
  );

  // --- Le protocole, piloté pour de vrai -----------------------------------
  const source = new SourceFichier(CHEMIN_DE_LARCHIVE, cle);
  // `getHeaderAndRoot` de la bibliothèque demande toujours les 16 384 premiers
  // octets, sans regarder la taille du fichier. Comme le moteur refuse une plage
  // qui dépasse la fin plutôt que de la tronquer, une archive plus courte que ça
  // ne se chargerait pas du tout. Le contrôle est ici pour que ça se voie tout
  // de suite si le générateur rétrécit un jour son fond.
  verifier(
    "l'archive atteint les 16 384 octets du premier appel de la bibliothèque",
    source.taille >= 16384,
    `${source.taille} octets : la bibliothèque demanderait une plage hors fichier`,
  );
  const protocole = new Protocol();
  protocole.add(new PMTiles(source));
  verifier(
    "l'archive est enregistrée sous la clé du style, donc Protocol n'a pas à la chercher",
    protocole.get(adresse.slice(PREFIXE.length)) !== undefined,
  );

  // --- Scénario : le TileJSON vient de l'en-tête, pas des métadonnées ------
  // Constat A10 : avec `metadata: false`, `tilev4` construit bounds, minzoom et
  // maxzoom depuis l'en-tête de l'archive. `getTileJson`, seul lecteur des
  // métadonnées JSON, n'est appelé que si `metadata` est vrai — et lui seul
  // ajoute un champ `tilejson`, dont l'absence le prouve ici.
  const tileJson = (await protocole.tilev4({ url: adresse, type: "json" }, new AbortController())).data;
  verifier(
    "le TileJSON annonce le gabarit de tuile du protocole",
    tileJson.tiles?.[0] === `${adresse}/{z}/{x}/{y}`,
    `lu : ${tileJson.tiles?.[0]}`,
  );
  verifier(
    "le TileJSON tient ses bornes de zoom de l'en-tête",
    tileJson.minzoom === ZOOM_MIN && tileJson.maxzoom === ZOOM_MAX,
    `lu : ${tileJson.minzoom}–${tileJson.maxzoom}`,
  );
  const proche = (a, b) => Math.abs(a - b) < 1e-6;
  verifier(
    "le TileJSON tient son emprise de l'en-tête, pas du monde entier",
    Array.isArray(tileJson.bounds) &&
      proche(tileJson.bounds[0], EMPRISE.ouest) &&
      proche(tileJson.bounds[1], EMPRISE.sud) &&
      proche(tileJson.bounds[2], EMPRISE.est) &&
      proche(tileJson.bounds[3], EMPRISE.nord),
    `lu : ${JSON.stringify(tileJson.bounds)}`,
  );
  verifier(
    "les métadonnées JSON ne sont pas lues (metadata: false)",
    tileJson.tilejson === undefined,
    `un champ tilejson est apparu : ${tileJson.tilejson}`,
  );

  // --- Scénario : affichage nominal, une tuile connue ----------------------
  const tuiles = tuilesCouvrantes();
  const tuileCentrale = tuiles.find((t) => t.z === ZOOM_CENTRE) ?? tuiles[0];
  const reponse = await protocole.tilev4(
    { url: `${adresse}/${tuileCentrale.z}/${tuileCentrale.x}/${tuileCentrale.y}` },
    new AbortController(),
  );
  const octetsDeLaTuile = reponse.data ? Buffer.from(reponse.data) : Buffer.alloc(0);
  verifier(
    "une tuile connue est servie, et c'est un PNG",
    octetsDeLaTuile.subarray(0, 8).equals(SIGNATURE_PNG),
    `${octetsDeLaTuile.length} octets, en-tête ${octetsDeLaTuile.subarray(0, 8).toString("hex")}`,
  );

  // --- Scénario : tuile absente -------------------------------------------
  const horsEmprise = await protocole.tilev4(
    { url: `${adresse}/${ZOOM_CENTRE}/0/0` },
    new AbortController(),
  );
  verifier(
    "une tuile hors de l'emprise revient vide, sans exception",
    horsEmprise.data === null,
    `lu : ${horsEmprise.data === null ? "null" : typeof horsEmprise.data}`,
  );
  const horsZoom = await protocole.tilev4(
    { url: `${adresse}/${ZOOM_MAX + 1}/0/0` },
    new AbortController(),
  );
  verifier("une tuile au-delà du zoom maximum revient vide", horsZoom.data === null);

  // --- Scénario : plage hors fichier --------------------------------------
  // Ce contrôle porte sur la doublure de ce script, pas sur le moteur : il
  // établit que le scénario est bien joué sur une source qui refuse, pas qui
  // tronque. Le refus du moteur, lui, est couvert par les tests unitaires de
  // `BasemapSource::lire_plage`, et nommer les trois nombres est leur travail.
  let plageRefusee = false;
  try {
    await source.getBytes(source.taille - 1, 2);
  } catch {
    plageRefusee = true;
  }
  verifier(
    "la doublure de ce script refuse une plage hors fichier (le moteur a ses propres tests)",
    plageRefusee,
  );
  verifier(
    "l'archive a bien été lue par plages",
    source.plagesLues > 1,
    `${source.plagesLues} plage(s)`,
  );

  // --- Aucune requête réseau sur le chemin nominal -------------------------
  reseauSurLeCheminNominal = tentativesReseau;
  verifier(
    "aucune requête réseau n'a été tentée",
    tentativesReseau === 0,
    `${tentativesReseau} tentative(s)`,
  );

  // --- Le mode de défaillance à craindre, mis à l'épreuve -----------------
  // Une clé que `Protocol` ne connaît pas le fait construire un `FetchSource`.
  // Sur un poste connecté, ça ne se verrait pas. Ici, `fetch` est cassé, donc
  // la tentative est comptée : c'est la preuve que la garde de check-offline
  // doit connaître la clé, et pas seulement le schéma.
  const protocoleNu = new Protocol();
  let partiSurLeReseau = false;
  try {
    await protocoleNu.tilev4(
      { url: `${PREFIXE}une-cle-non-enregistree`, type: "json" },
      new AbortController(),
    );
  } catch (e) {
    partiSurLeReseau = /requête réseau tentée/.test(e.message);
  }
  verifier(
    "une clé non enregistrée part bien sur le réseau (donc la garde a un vrai travail)",
    partiSurLeReseau && tentativesReseau > 0,
    partiSurLeReseau ? "" : "aucune requête réseau : le mode de défaillance a changé, revoir check-offline.mjs",
  );

  // --- Un seul endroit porte le chemin, un seul porte la clé ---------------
  const ressources = conf.bundle?.resources;
  verifier(
    "l'installeur embarque l'archive en ressource",
    ressources !== undefined && !Array.isArray(ressources) && typeof ressources === "object",
    `lu : ${JSON.stringify(ressources)}`,
  );
  const entreesRessources = Object.entries(ressources ?? {});
  verifier(
    "la ressource embarquée est exactement l'archive produite par le générateur",
    entreesRessources.length === 1 &&
      resolve(DOSSIER_TAURI, entreesRessources[0][0]) === CHEMIN_DE_LARCHIVE,
    `ressource : ${entreesRessources.map(([s]) => s).join(", ")}, générateur : ${CHEMIN_DE_LARCHIVE}`,
  );
  const cheminDansLeMoteur = constanteRust(libRs, "CHEMIN_DE_LARCHIVE");
  verifier(
    "le moteur et l'installeur portent le même chemin de ressource",
    cheminDansLeMoteur !== null && entreesRessources[0]?.[1] === cheminDansLeMoteur,
    `moteur : ${cheminDansLeMoteur}, installeur : ${entreesRessources[0]?.[1]}`,
  );
  const commande = codeSourceTs.match(/const COMMANDE = "([^"]+)"/)?.[1] ?? null;
  verifier(
    "la commande appelée par l'interface est enregistrée par le moteur",
    commande !== null &&
      new RegExp(`fn ${commande}\\s*\\(`).test(libRs) &&
      new RegExp(`generate_handler!\\[[^\\]]*\\b${commande}\\b`, "s").test(libRs),
    `commande : ${commande}`,
  );

  rapporter();
  if (echecs > 0) process.exit(1);
}

function rapporter() {
  console.log(
    echecs === 0
      ? `Vérification du protocole : OK, ${resultats.length} contrôles.`
      : `Vérification du protocole : ÉCHEC, ${echecs} contrôle(s) sur ${resultats.length}.`,
  );
  for (const l of resultats) if (echecs > 0 || l.startsWith("  ÉCHEC")) console.log(l);
  console.log(
    `  chargement par plages locales, ${reseauSurLeCheminNominal ?? "?"} requête(s) réseau sur le chemin nominal.`,
  );
}

principal().catch((e) => {
  console.error(`Vérification du protocole : ÉCHEC, exception inattendue.\n  ${e.stack ?? e.message}`);
  process.exit(1);
});
