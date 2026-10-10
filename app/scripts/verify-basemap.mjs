// Vérificateur du fond de carte PMTiles de recette.
//
// Le générateur écrit un format binaire à la main : « le fichier existe » ne
// prouve rien. Ici l'archive est relue par la bibliothèque `pmtiles` officielle,
// qui sert de juge indépendant, et chaque scénario de la matrice du plan de la
// story 1.2 est couvert.
//
// Exécuté par `npm run check`, donc une archive cassée fait échouer la recette.

import { readFileSync, existsSync } from "node:fs";
import zlib from "node:zlib";
import { PMTiles, Compression, TileType, zxyToTileId } from "pmtiles";
import {
  CHEMIN_SORTIE,
  CENTRE,
  EMPRISE,
  ZOOM_CENTRE,
  ZOOM_MIN,
  ZOOM_MAX,
  identifiantTuile,
  tuilesCouvrantes,
  construireArchive,
} from "./generate-basemap.mjs";

const SIGNATURE_PNG = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

/**
 * Valide un PNG en parcourant ses morceaux et en recalculant leur CRC, puis en
 * décompressant l'IDAT. Se contenter de la signature et d'`IEND` laisserait
 * passer un octet corrompu au milieu des données : c'est arrivé pendant
 * l'écriture de cette story.
 */
function pngValide(octets) {
  if (!octets.subarray(0, 8).equals(SIGNATURE_PNG)) return "signature absente";
  let i = 8;
  const vus = [];
  const idat = [];
  while (i + 12 <= octets.length) {
    const longueur = octets.readUInt32BE(i);
    const type = octets.subarray(i + 4, i + 8).toString("ascii");
    const fin = i + 8 + longueur;
    if (fin + 4 > octets.length) return `morceau ${type} tronqué`;
    const donnees = octets.subarray(i + 8, fin);
    const crcLu = octets.readUInt32BE(fin);
    const crcCalcule = zlib.crc32(Buffer.concat([octets.subarray(i + 4, i + 8), donnees]));
    if (crcLu !== crcCalcule) return `CRC du morceau ${type} invalide`;
    vus.push(type);
    if (type === "IDAT") idat.push(donnees);
    i = fin + 4;
    if (type === "IEND") break;
  }
  if (i !== octets.length) return "octets en trop après IEND";
  for (const attendu of ["IHDR", "IDAT", "IEND"]) {
    if (!vus.includes(attendu)) return `morceau ${attendu} manquant`;
  }
  try {
    zlib.inflateSync(Buffer.concat(idat));
  } catch {
    return "IDAT ne se décompresse pas";
  }
  return null;
}

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

/** Source PMTiles lisant un tampon en mémoire : aucun accès réseau, aucun fichier temporaire. */
class SourceTampon {
  constructor(tampon, cle) {
    this.tampon = tampon;
    this.cle = cle;
  }

  getKey() {
    return this.cle;
  }

  async getBytes(decalage, longueur) {
    const tranche = this.tampon.subarray(decalage, decalage + longueur);
    if (tranche.byteLength !== longueur) {
      // Sans ça une archive tronquée se lit à moitié et casse plus loin, de façon
      // inexplicable, au lieu d'échouer ici où la cause est visible.
      throw new Error(
        `lecture courte à ${decalage} : ${tranche.byteLength} octets au lieu de ${longueur}, archive tronquée`,
      );
    }
    return {
      data: tranche.buffer.slice(tranche.byteOffset, tranche.byteOffset + tranche.byteLength),
    };
  }
}

async function principal() {
  // --- Scénario : génération nominale -------------------------------------
  if (!existsSync(CHEMIN_SORTIE)) {
    console.error(`Vérification du fond de recette : ÉCHEC, fichier absent.\n  Lance d'abord : npm run fixtures`);
    process.exit(1);
  }
  const archive = readFileSync(CHEMIN_SORTIE);
  verifier("l'archive dépasse 127 octets", archive.length > 127, `${archive.length} octets`);
  verifier("la magie vaut PMTiles", archive.subarray(0, 7).toString("ascii") === "PMTiles");
  verifier("la version de spécification vaut 3", archive[7] === 3, `lu : ${archive[7]}`);
  verifier("l'archive pèse moins de 200 Ko", archive.length < 200 * 1024, `${(archive.length / 1024).toFixed(1)} Ko`);

  // --- Scénario : relecture par la bibliothèque ---------------------------
  // Aucun décompresseur fourni : on exerce volontairement le chemin par défaut
  // de la bibliothèque, celui que la carte empruntera à l'entrée 1.3.
  const pmtiles = new PMTiles(new SourceTampon(archive, CHEMIN_SORTIE));
  const entete = await pmtiles.getHeader();
  verifier("la bibliothèque lit l'en-tête", entete.specVersion === 3);
  verifier(
    "l'en-tête occupe exactement 127 octets",
    entete.rootDirectoryOffset === 127,
    `le répertoire racine commence à ${entete.rootDirectoryOffset}`,
  );
  verifier("le type de tuile est PNG", entete.tileType === TileType.Png, `lu : ${entete.tileType}`);
  verifier("la compression interne est gzip", entete.internalCompression === Compression.Gzip);
  verifier("les tuiles ne sont pas compressées", entete.tileCompression === Compression.None);
  verifier("l'archive est groupée par identifiant", entete.clustered === true);

  // --- Scénario : zoom hors bornes ---------------------------------------
  verifier("le zoom minimum annoncé est correct", entete.minZoom === ZOOM_MIN, `lu : ${entete.minZoom}`);
  verifier("le zoom maximum annoncé est correct", entete.maxZoom === ZOOM_MAX, `lu : ${entete.maxZoom}`);
  const sousBorne = await pmtiles.getZxy(ZOOM_MIN - 1, 0, 0);
  verifier("une tuile sous le zoom minimum est absente sans exception", sousBorne === undefined);
  const surBorne = await pmtiles.getZxy(ZOOM_MAX + 1, 0, 0);
  verifier("une tuile au-delà du zoom maximum est absente sans exception", surBorne === undefined);

  // --- Emprise annoncée ---------------------------------------------------
  const proche = (a, b) => Math.abs(a - b) < 1e-6;
  const style = JSON.parse(readFileSync(new URL("../src/basemap.style.json", import.meta.url), "utf8"));
  const anneau = style.sources?.repere?.data?.features?.[0]?.geometry?.coordinates?.[0] ?? [];
  const lons = anneau.map((c) => c[0]);
  const lats = anneau.map((c) => c[1]);
  verifier(
    "l'emprise du fond est bien celle du repère lu dans basemap.style.json",
    anneau.length > 0 &&
      proche(Math.min(...lons), entete.minLon) &&
      proche(Math.max(...lons), entete.maxLon) &&
      proche(Math.min(...lats), entete.minLat) &&
      proche(Math.max(...lats), entete.maxLat),
    anneau.length === 0 ? "repère introuvable dans le style" : `style : ${Math.min(...lons)},${Math.min(...lats)} → ${Math.max(...lons)},${Math.max(...lats)}`,
  );
  verifier(
    "l'emprise annoncée correspond aux constantes du générateur",
    proche(entete.minLon, EMPRISE.ouest) &&
      proche(entete.minLat, EMPRISE.sud) &&
      proche(entete.maxLon, EMPRISE.est) &&
      proche(entete.maxLat, EMPRISE.nord),
    `lu : ${entete.minLon},${entete.minLat} → ${entete.maxLon},${entete.maxLat}`,
  );

  // --- Centre annoncé : jamais contrôlé jusqu'ici, or une inversion
  // --- longitude/latitude à l'écriture passerait inaperçue ------------------
  verifier(
    "le centre annoncé est celui d'ouverture de la carte",
    proche(entete.centerLon, CENTRE.lon) && proche(entete.centerLat, CENTRE.lat),
    `lu : ${entete.centerLon}, ${entete.centerLat}`,
  );
  verifier("le zoom central annoncé est correct", entete.centerZoom === ZOOM_CENTRE, `lu : ${entete.centerZoom}`);

  // --- Métadonnées --------------------------------------------------------
  const metadonnees = await pmtiles.getMetadata();
  verifier("les métadonnées se relisent et annoncent le format png", metadonnees?.format === "png");

  // --- Scénario : identifiant de tuile connu -----------------------------
  verifier(
    "l'identifiant de référence de la spécification est retrouvé",
    identifiantTuile(12, 3423, 1763) === 19078479,
    `calculé : ${identifiantTuile(12, 3423, 1763)}`,
  );
  const tuiles = tuilesCouvrantes();
  const desaccords = tuiles.filter((t) => identifiantTuile(t.z, t.x, t.y) !== zxyToTileId(t.z, t.x, t.y));
  verifier(
    "notre calcul d'identifiant concorde avec celui de la bibliothèque sur les 35 tuiles",
    desaccords.length === 0,
    desaccords.length > 0 ? `${desaccords.length} désaccords, dont z${desaccords[0].z}/${desaccords[0].x}/${desaccords[0].y}` : "",
  );
  verifier(
    "le nombre de tuiles annoncé par l'en-tête correspond",
    entete.numAddressedTiles === tuiles.length,
    `en-tête : ${entete.numAddressedTiles}, attendu : ${tuiles.length}`,
  );

  // --- Chaque tuile annoncée est servie et est un PNG valide -------------
  let servies = 0;
  let malFormees = 0;
  let premierProbleme = null;
  let premiereManquante = null;
  for (const t of tuiles) {
    const reponse = await pmtiles.getZxy(t.z, t.x, t.y);
    if (!reponse) {
      if (!premiereManquante) premiereManquante = `z${t.z}/${t.x}/${t.y}`;
      continue;
    }
    servies += 1;
    const octets = Buffer.from(reponse.data);
    const probleme = pngValide(octets);
    if (probleme) {
      malFormees += 1;
      if (!premierProbleme) premierProbleme = `z${t.z}/${t.x}/${t.y} : ${probleme}`;
    }
  }
  verifier(
    "chaque tuile annoncée est servie",
    servies === tuiles.length,
    `${servies} sur ${tuiles.length}${premiereManquante ? `, première manquante ${premiereManquante}` : ""}`,
  );
  verifier(
    "chaque tuile servie est un PNG valide, CRC et IDAT compris",
    malFormees === 0,
    `${malFormees} mal formées${premierProbleme ? `, dont ${premierProbleme}` : ""}`,
  );

  // --- Scénario : hors de l'emprise --------------------------------------
  const horsEmprise = await pmtiles.getZxy(12, 0, 0);
  verifier("une tuile hors de l'emprise est absente sans exception", horsEmprise === undefined);

  // --- Scénario : déterminisme -------------------------------------------
  const refaite = construireArchive().archive;
  verifier(
    "deux générations donnent le même fichier, octet pour octet",
    refaite.equals(archive),
    `${refaite.length} contre ${archive.length} octets`,
  );

  // --- Le vérificateur sait-il échouer ? -------------------------------
  // Un garde-fou qu'on n'a jamais vu refuser quelque chose ne garde rien. On le
  // met donc en échec exprès, en mémoire, sans toucher au fichier du dépôt.
  const tuileSaine = Buffer.from((await pmtiles.getZxy(tuiles[0].z, tuiles[0].x, tuiles[0].y)).data);
  verifier("le validateur PNG accepte une tuile saine", pngValide(tuileSaine) === null);
  const corrompue = Buffer.from(tuileSaine);
  corrompue[corrompue.length - 20] ^= 0xff; // au milieu de l'IDAT, pas dans l'en-tête
  verifier(
    "le validateur PNG refuse un octet corrompu",
    pngValide(corrompue) !== null,
    "un octet inversé dans l'IDAT a été accepté : le garde-fou ne garde rien",
  );
  const tronquee = tuileSaine.subarray(0, tuileSaine.length - 10);
  verifier("le validateur PNG refuse une tuile tronquée", pngValide(tronquee) !== null);
  verifier("le validateur PNG refuse une signature absente", pngValide(Buffer.alloc(40)) !== null);

  // --- Rapport ------------------------------------------------------------
  const parZoom = {};
  for (const t of tuiles) parZoom[t.z] = (parZoom[t.z] ?? 0) + 1;
  console.log(
    echecs === 0
      ? `Vérification du fond de recette : OK, ${resultats.length} contrôles.`
      : `Vérification du fond de recette : ÉCHEC, ${echecs} contrôle(s) sur ${resultats.length}.`,
  );
  for (const l of resultats) if (echecs > 0 || l.startsWith("  ÉCHEC")) console.log(l);
  console.log(
    `  ${tuiles.length} tuiles, zooms ${ZOOM_MIN} à ${ZOOM_MAX} (${Object.entries(parZoom)
      .map(([z, n]) => `z${z}:${n}`)
      .join(", ")}), ${(archive.length / 1024).toFixed(1)} Ko`,
  );
  if (echecs > 0) process.exit(1);
}

principal().catch((e) => {
  console.error(`Vérification du fond de recette : ÉCHEC, exception inattendue.\n  ${e.message}`);
  process.exit(1);
});
