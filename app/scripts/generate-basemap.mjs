// Générateur du fond de carte PMTiles de recette.
//
// Cassini ne télécharge aucune donnée (CLAUDE.md §2.4), donc le fond de carte
// qui sert à la recette est **fabriqué** ici : une archive PMTiles v3 à tuiles
// PNG couvrant la commune de Villeneuve-le-Roi, aux zooms 10 à 14.
//
// Aucune dépendance tierce : Node seul, avec son `zlib` intégré. Aucune requête
// réseau. Le fichier produit est déterministe, octet pour octet.
//
// Spécification suivie : https://github.com/protomaps/PMTiles/blob/main/spec/v3/spec.md
// Attention : le pseudocode de son annexe A.1 omet le nombre d'entrées en tête
// du répertoire, alors que le §4.2 l'exige et que le décodage du §4.3 le lit en
// premier. Voir `encoderRepertoire`.

import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import zlib from "node:zlib";

const ICI = dirname(fileURLToPath(import.meta.url));
export const CHEMIN_SORTIE = join(ICI, "..", "..", "fixtures", "basemap-villeneuve-le-roi.pmtiles");

/** Emprise du repère déjà présent dans `app/src/basemap.style.json`. */
export const EMPRISE = { ouest: 2.37, sud: 48.71, est: 2.46, nord: 48.76 };
/** Centre d'ouverture de la carte, aligné sur `app/src/main.ts`. */
export const CENTRE = { lon: 2.415, lat: 48.735 };
export const ZOOM_MIN = 10;
export const ZOOM_MAX = 14;
export const ZOOM_CENTRE = 12;

const COTE_TUILE = 256;
const COTE_CELLULE = 64;

// ---------------------------------------------------------------- varint

/** Écrit un entier non signé en varint little-endian base 128. */
function ecrireVarint(octets, valeur) {
  let v = BigInt(valeur);
  if (v < 0n) throw new Error(`varint négatif interdit : ${valeur}`);
  while (v >= 128n) {
    octets.push(Number((v & 127n) | 128n));
    v >>= 7n;
  }
  octets.push(Number(v));
}

// ------------------------------------------------- identifiant de tuile

function pivoter(n, x, y, rx, ry) {
  if (ry === 0) {
    if (rx === 1) {
      x = n - 1 - x;
      y = n - 1 - y;
    }
    return [y, x];
  }
  return [x, y];
}

/** Position de (x, y) sur la courbe de Hilbert d'ordre `n` = 2^z. */
export function indiceHilbert(n, x, y) {
  let d = 0;
  for (let s = n / 2; s > 0; s = Math.floor(s / 2)) {
    const rx = (x & s) > 0 ? 1 : 0;
    const ry = (y & s) > 0 ? 1 : 0;
    d += s * s * ((3 * rx) ^ ry);
    [x, y] = pivoter(n, x, y, rx, ry);
  }
  return d;
}

/**
 * Identifiant PMTiles d'une tuile : le cumul des tuiles des zooms inférieurs,
 * plus la position sur la courbe de Hilbert du zoom courant.
 *
 * Valeur de référence de la spécification : (12, 3423, 1763) vaut 19078479.
 *
 * Limité au zoom 26 : au-delà, `4 ** z` et l'accumulation de Hilbert sortent
 * des entiers exactement représentables en JavaScript et l'identifiant devient
 * faux silencieusement. Le MVP s'arrête au zoom 14.
 */
export function identifiantTuile(z, x, y) {
  if (!Number.isInteger(z) || z < 0 || z > 26) throw new Error(`zoom hors bornes : ${z}`);
  const n = 2 ** z;
  if (!Number.isInteger(x) || x < 0 || x >= n) throw new Error(`x hors bornes au zoom ${z} : ${x}`);
  if (!Number.isInteger(y) || y < 0 || y >= n) throw new Error(`y hors bornes au zoom ${z} : ${y}`);
  return (4 ** z - 1) / 3 + indiceHilbert(n, x, y);
}

// --------------------------------------------------- emprise vers tuiles

const lonVersX = (lon, z) => Math.floor(((lon + 180) / 360) * 2 ** z);

const latVersY = (lat, z) => {
  const r = (lat * Math.PI) / 180;
  return Math.floor(((1 - Math.log(Math.tan(r) + 1 / Math.cos(r)) / Math.PI) / 2) * 2 ** z);
};

/** Les tuiles couvrant l'emprise, du zoom minimum au maximum, triées par identifiant. */
export function tuilesCouvrantes() {
  const liste = [];
  for (let z = ZOOM_MIN; z <= ZOOM_MAX; z += 1) {
    const n = 2 ** z;
    const x0 = Math.max(0, lonVersX(EMPRISE.ouest, z));
    const x1 = Math.min(n - 1, lonVersX(EMPRISE.est, z));
    // Le nord donne le plus petit y : l'axe des tuiles descend.
    const y0 = Math.max(0, latVersY(EMPRISE.nord, z));
    const y1 = Math.min(n - 1, latVersY(EMPRISE.sud, z));
    for (let x = x0; x <= x1; x += 1) {
      for (let y = y0; y <= y1; y += 1) {
        liste.push({ z, x, y, id: identifiantTuile(z, x, y) });
      }
    }
  }
  liste.sort((a, b) => a.id - b.id);
  return liste;
}

// -------------------------------------------------------------------- PNG

function morceauPng(type, donnees) {
  const prefixe = Buffer.alloc(8);
  prefixe.writeUInt32BE(donnees.length, 0);
  prefixe.write(type, 4, "ascii");
  const crc = Buffer.alloc(4);
  // Le CRC couvre le type et les données, pas la longueur.
  crc.writeUInt32BE(zlib.crc32(Buffer.concat([prefixe.subarray(4), donnees])), 0);
  return Buffer.concat([prefixe, donnees, crc]);
}

/**
 * Une tuile : damier en niveaux de gris avec bordure claire. La phase du damier
 * change selon (x + y), pour que deux tuiles voisines s'inversent et que le
 * déplacement se voie. Le niveau de gris du fond, lui, dépend du **zoom** : sans
 * ça, une tuile du zoom 10 et une du zoom 14 sont identiques octet pour octet et
 * zoomer ne change rien à l'écran.
 *
 * Gris + alpha (type de couleur PNG 4) plutôt que RVBA : deux fois moins
 * d'octets pour un rendu identique ici, ce qui tient le fixture sous 200 Ko.
 */
export function tuilePng(z, x, y) {
  const phase = (x + y) % 2;
  // Du plus sombre au plus clair quand on zoome : 5 paliers pour 5 niveaux.
  const palier = Math.min(4, Math.max(0, z - ZOOM_MIN));
  const sombre = 24 + palier * 7;
  const clair = sombre + 16;
  const lignes = [];
  for (let py = 0; py < COTE_TUILE; py += 1) {
    const ligne = Buffer.alloc(1 + COTE_TUILE * 2); // octet de filtre 0, puis (gris, alpha)
    let o = 1;
    for (let px = 0; px < COTE_TUILE; px += 1) {
      const damier = (Math.floor(px / COTE_CELLULE) + Math.floor(py / COTE_CELLULE) + phase) % 2 === 0;
      const bord = px < 2 || py < 2 || px > COTE_TUILE - 3 || py > COTE_TUILE - 3;
      ligne[o] = bord ? 190 : damier ? sombre : clair;
      ligne[o + 1] = 255;
      o += 2;
    }
    lignes.push(ligne);
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(COTE_TUILE, 0);
  ihdr.writeUInt32BE(COTE_TUILE, 4);
  ihdr[8] = 8; // profondeur de bit
  ihdr[9] = 4; // type de couleur : gris + alpha
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    morceauPng("IHDR", ihdr),
    morceauPng("IDAT", zlib.deflateSync(Buffer.concat(lignes), { level: 9 })),
    morceauPng("IEND", Buffer.alloc(0)),
  ]);
}

// ------------------------------------------------------------- répertoire

/**
 * Encode un répertoire : nombre d'entrées, identifiants delta-encodés,
 * longueurs de série, longueurs, puis décalages — chacun en varint.
 *
 * Le nombre d'entrées est exigé par le §4.2 et lu en premier par le §4.3, mais
 * absent du pseudocode de l'annexe A.1. L'oublier produit une archive illisible.
 */
export function encoderRepertoire(entrees) {
  if (entrees.length === 0) throw new Error("un répertoire doit contenir au moins une entrée");
  const octets = [];
  ecrireVarint(octets, entrees.length);
  let dernierId = 0;
  for (const e of entrees) {
    ecrireVarint(octets, e.id - dernierId);
    dernierId = e.id;
  }
  for (const e of entrees) ecrireVarint(octets, e.serie);
  for (const e of entrees) ecrireVarint(octets, e.longueur);
  let octetSuivant = 0;
  for (const [i, e] of entrees.entries()) {
    // Un décalage contigu avec la tuile précédente s'écrit 0 ; sinon décalage + 1.
    if (i > 0 && e.decalage === octetSuivant) ecrireVarint(octets, 0);
    else ecrireVarint(octets, e.decalage + 1);
    octetSuivant = e.decalage + e.longueur;
  }
  return Buffer.from(octets);
}

// ---------------------------------------------------------------- en-tête

function ecrirePosition(tampon, decalage, lon, lat) {
  tampon.writeInt32LE(Math.round(lon * 1e7), decalage);
  tampon.writeInt32LE(Math.round(lat * 1e7), decalage + 4);
}

/** L'en-tête de 127 octets, disposition du §3.1 de la spécification. */
export function encoderEntete(c) {
  const h = Buffer.alloc(127);
  h.write("PMTiles", 0, "ascii");
  h[7] = 3;
  const u64 = [
    c.decalageRepertoireRacine, c.longueurRepertoireRacine,
    c.decalageMetadonnees, c.longueurMetadonnees,
    c.decalageRepertoiresFeuilles, c.longueurRepertoiresFeuilles,
    c.decalageTuiles, c.longueurTuiles,
    c.nbTuilesAdressees, c.nbEntreesTuiles, c.nbContenusTuiles,
  ];
  u64.forEach((v, i) => h.writeBigUInt64LE(BigInt(v), 8 + i * 8));
  h[96] = 1; // groupé par identifiant
  h[97] = 2; // compression interne : gzip
  h[98] = 1; // compression des tuiles : aucune, le PNG est déjà compressé
  h[99] = 2; // type de tuile : PNG
  h[100] = ZOOM_MIN;
  h[101] = ZOOM_MAX;
  ecrirePosition(h, 102, EMPRISE.ouest, EMPRISE.sud);
  ecrirePosition(h, 110, EMPRISE.est, EMPRISE.nord);
  h[118] = ZOOM_CENTRE;
  ecrirePosition(h, 119, CENTRE.lon, CENTRE.lat);
  return h;
}

// -------------------------------------------------------------- assemblage

export function construireArchive() {
  const tuiles = tuilesCouvrantes();
  const blobs = [];
  const entrees = [];
  let decalage = 0;
  for (const t of tuiles) {
    const blob = tuilePng(t.z, t.x, t.y);
    blobs.push(blob);
    entrees.push({ id: t.id, serie: 1, longueur: blob.length, decalage });
    decalage += blob.length;
  }
  const donneesTuiles = Buffer.concat(blobs);

  const metadonnees = zlib.gzipSync(
    Buffer.from(
      JSON.stringify({
        name: "Cassini — fond de recette, Villeneuve-le-Roi",
        description:
          "Fond de carte fabrique localement pour la recette. Aucune donnee reelle, aucun telechargement.",
        attribution: "Cassini, jeu de test genere",
        type: "baselayer",
        format: "png",
        generated_by: "app/scripts/generate-basemap.mjs",
      }),
    ),
    { level: 9 },
  );
  const repertoire = zlib.gzipSync(encoderRepertoire(entrees), { level: 9 });

  const decalageRepertoireRacine = 127;
  const decalageMetadonnees = decalageRepertoireRacine + repertoire.length;
  const decalageTuiles = decalageMetadonnees + metadonnees.length;

  if (decalageMetadonnees > 16384) {
    throw new Error(
      `le répertoire racine dépasse les 16 384 premiers octets (${decalageMetadonnees}) : il faudrait des répertoires feuilles`,
    );
  }

  const entete = encoderEntete({
    decalageRepertoireRacine,
    longueurRepertoireRacine: repertoire.length,
    decalageMetadonnees,
    longueurMetadonnees: metadonnees.length,
    decalageRepertoiresFeuilles: decalageTuiles,
    longueurRepertoiresFeuilles: 0,
    decalageTuiles,
    longueurTuiles: donneesTuiles.length,
    nbTuilesAdressees: entrees.length,
    nbEntreesTuiles: entrees.length,
    nbContenusTuiles: entrees.length,
  });

  return { archive: Buffer.concat([entete, repertoire, metadonnees, donneesTuiles]), tuiles };
}

function principal() {
  const { archive, tuiles } = construireArchive();
  mkdirSync(dirname(CHEMIN_SORTIE), { recursive: true });
  writeFileSync(CHEMIN_SORTIE, archive);
  const parZoom = {};
  for (const t of tuiles) parZoom[t.z] = (parZoom[t.z] ?? 0) + 1;
  console.log(`Fond de recette écrit : ${CHEMIN_SORTIE}`);
  console.log(
    `  ${tuiles.length} tuiles, zooms ${ZOOM_MIN} à ${ZOOM_MAX} (${Object.entries(parZoom)
      .map(([z, n]) => `z${z}:${n}`)
      .join(", ")})`,
  );
  console.log(`  ${(archive.length / 1024).toFixed(1)} Ko, aucune requête réseau`);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) principal();
