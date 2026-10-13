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
  OUTPUT_PATH,
  CENTER,
  EXTENT,
  ZOOM_CENTER,
  ZOOM_MIN,
  ZOOM_MAX,
  tileId,
  coveringTiles,
  buildArchive,
} from "./generate-basemap.mjs";

const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

/**
 * Valide un PNG en parcourant ses morceaux et en recalculant leur CRC, puis en
 * décompressant l'IDAT. Se contenter de la signature et d'`IEND` laisserait
 * passer un octet corrompu au milieu des données : c'est arrivé pendant
 * l'écriture de cette story.
 */
function validatePng(bytes) {
  if (!bytes.subarray(0, 8).equals(PNG_SIGNATURE)) return "signature absente";
  let i = 8;
  const seen = [];
  const idat = [];
  while (i + 12 <= bytes.length) {
    const length = bytes.readUInt32BE(i);
    const type = bytes.subarray(i + 4, i + 8).toString("ascii");
    const end = i + 8 + length;
    if (end + 4 > bytes.length) return `morceau ${type} tronqué`;
    const data = bytes.subarray(i + 8, end);
    const crcRead = bytes.readUInt32BE(end);
    const crcComputed = zlib.crc32(Buffer.concat([bytes.subarray(i + 4, i + 8), data]));
    if (crcRead !== crcComputed) return `CRC du morceau ${type} invalide`;
    seen.push(type);
    if (type === "IDAT") idat.push(data);
    i = end + 4;
    if (type === "IEND") break;
  }
  if (i !== bytes.length) return "octets en trop après IEND";
  for (const expected of ["IHDR", "IDAT", "IEND"]) {
    if (!seen.includes(expected)) return `morceau ${expected} manquant`;
  }
  try {
    zlib.inflateSync(Buffer.concat(idat));
  } catch {
    return "IDAT ne se décompresse pas";
  }
  return null;
}

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

/** Source PMTiles lisant un tampon en mémoire : aucun accès réseau, aucun fichier temporaire. */
class BufferSource {
  constructor(buffer, key) {
    this.buffer = buffer;
    this.key = key;
  }

  getKey() {
    return this.key;
  }

  async getBytes(offset, length) {
    const slice = this.buffer.subarray(offset, offset + length);
    if (slice.byteLength !== length) {
      // Sans ça une archive tronquée se lit à moitié et casse plus loin, de façon
      // inexplicable, au lieu d'échouer ici où la cause est visible.
      throw new Error(
        `lecture courte à ${offset} : ${slice.byteLength} octets au lieu de ${length}, archive tronquée`,
      );
    }
    return {
      data: slice.buffer.slice(slice.byteOffset, slice.byteOffset + slice.byteLength),
    };
  }
}

async function main() {
  // --- Scénario : génération nominale -------------------------------------
  if (!existsSync(OUTPUT_PATH)) {
    console.error(`Vérification du fond de recette : ÉCHEC, fichier absent.\n  Lance d'abord : npm run fixtures`);
    process.exit(1);
  }
  const archive = readFileSync(OUTPUT_PATH);
  check("l'archive dépasse 127 octets", archive.length > 127, `${archive.length} octets`);
  check("la magie vaut PMTiles", archive.subarray(0, 7).toString("ascii") === "PMTiles");
  check("la version de spécification vaut 3", archive[7] === 3, `lu : ${archive[7]}`);
  check("l'archive pèse moins de 200 Ko", archive.length < 200 * 1024, `${(archive.length / 1024).toFixed(1)} Ko`);

  // --- Scénario : relecture par la bibliothèque ---------------------------
  // Aucun décompresseur fourni : on exerce volontairement le chemin par défaut
  // de la bibliothèque, celui que la carte empruntera à l'entrée 1.3.
  const pmtiles = new PMTiles(new BufferSource(archive, OUTPUT_PATH));
  const header = await pmtiles.getHeader();
  check("la bibliothèque lit l'en-tête", header.specVersion === 3);
  check(
    "l'en-tête occupe exactement 127 octets",
    header.rootDirectoryOffset === 127,
    `le répertoire racine commence à ${header.rootDirectoryOffset}`,
  );
  check("le type de tuile est PNG", header.tileType === TileType.Png, `lu : ${header.tileType}`);
  check("la compression interne est gzip", header.internalCompression === Compression.Gzip);
  check("les tuiles ne sont pas compressées", header.tileCompression === Compression.None);
  check("l'archive est groupée par identifiant", header.clustered === true);

  // --- Scénario : zoom hors bornes ---------------------------------------
  check("le zoom minimum annoncé est correct", header.minZoom === ZOOM_MIN, `lu : ${header.minZoom}`);
  check("le zoom maximum annoncé est correct", header.maxZoom === ZOOM_MAX, `lu : ${header.maxZoom}`);
  const belowBound = await pmtiles.getZxy(ZOOM_MIN - 1, 0, 0);
  check("une tuile sous le zoom minimum est absente sans exception", belowBound === undefined);
  const aboveBound = await pmtiles.getZxy(ZOOM_MAX + 1, 0, 0);
  check("une tuile au-delà du zoom maximum est absente sans exception", aboveBound === undefined);

  // --- Emprise annoncée ---------------------------------------------------
  const near = (a, b) => Math.abs(a - b) < 1e-6;
  const style = JSON.parse(readFileSync(new URL("../src/basemap.style.json", import.meta.url), "utf8"));
  const ring = style.sources?.marker?.data?.features?.[0]?.geometry?.coordinates?.[0] ?? [];
  const lons = ring.map((c) => c[0]);
  const lats = ring.map((c) => c[1]);
  check(
    "l'emprise du fond est bien celle du repère lu dans basemap.style.json",
    ring.length > 0 &&
      near(Math.min(...lons), header.minLon) &&
      near(Math.max(...lons), header.maxLon) &&
      near(Math.min(...lats), header.minLat) &&
      near(Math.max(...lats), header.maxLat),
    ring.length === 0 ? "repère introuvable dans le style" : `style : ${Math.min(...lons)},${Math.min(...lats)} → ${Math.max(...lons)},${Math.max(...lats)}`,
  );
  check(
    "l'emprise annoncée correspond aux constantes du générateur",
    near(header.minLon, EXTENT.west) &&
      near(header.minLat, EXTENT.south) &&
      near(header.maxLon, EXTENT.east) &&
      near(header.maxLat, EXTENT.north),
    `lu : ${header.minLon},${header.minLat} → ${header.maxLon},${header.maxLat}`,
  );

  // --- Centre annoncé : jamais contrôlé jusqu'ici, or une inversion
  // --- longitude/latitude à l'écriture passerait inaperçue ------------------
  check(
    "le centre annoncé est celui d'ouverture de la carte",
    near(header.centerLon, CENTER.lon) && near(header.centerLat, CENTER.lat),
    `lu : ${header.centerLon}, ${header.centerLat}`,
  );
  check("le zoom central annoncé est correct", header.centerZoom === ZOOM_CENTER, `lu : ${header.centerZoom}`);

  // --- Métadonnées --------------------------------------------------------
  const metadata = await pmtiles.getMetadata();
  check("les métadonnées se relisent et annoncent le format png", metadata?.format === "png");

  // --- Scénario : identifiant de tuile connu -----------------------------
  check(
    "l'identifiant de référence de la spécification est retrouvé",
    tileId(12, 3423, 1763) === 19078479,
    `calculé : ${tileId(12, 3423, 1763)}`,
  );
  const tiles = coveringTiles();
  const mismatches = tiles.filter((t) => tileId(t.z, t.x, t.y) !== zxyToTileId(t.z, t.x, t.y));
  check(
    "notre calcul d'identifiant concorde avec celui de la bibliothèque sur les 35 tuiles",
    mismatches.length === 0,
    mismatches.length > 0 ? `${mismatches.length} désaccords, dont z${mismatches[0].z}/${mismatches[0].x}/${mismatches[0].y}` : "",
  );
  check(
    "le nombre de tuiles annoncé par l'en-tête correspond",
    header.numAddressedTiles === tiles.length,
    `en-tête : ${header.numAddressedTiles}, attendu : ${tiles.length}`,
  );

  // --- Chaque tuile annoncée est servie et est un PNG valide -------------
  let served = 0;
  let malformed = 0;
  let firstProblem = null;
  let firstMissing = null;
  for (const t of tiles) {
    const response = await pmtiles.getZxy(t.z, t.x, t.y);
    if (!response) {
      if (!firstMissing) firstMissing = `z${t.z}/${t.x}/${t.y}`;
      continue;
    }
    served += 1;
    const bytes = Buffer.from(response.data);
    const problem = validatePng(bytes);
    if (problem) {
      malformed += 1;
      if (!firstProblem) firstProblem = `z${t.z}/${t.x}/${t.y} : ${problem}`;
    }
  }
  check(
    "chaque tuile annoncée est servie",
    served === tiles.length,
    `${served} sur ${tiles.length}${firstMissing ? `, première manquante ${firstMissing}` : ""}`,
  );
  check(
    "chaque tuile servie est un PNG valide, CRC et IDAT compris",
    malformed === 0,
    `${malformed} mal formées${firstProblem ? `, dont ${firstProblem}` : ""}`,
  );

  // --- Scénario : hors de l'emprise --------------------------------------
  const outsideExtent = await pmtiles.getZxy(12, 0, 0);
  check("une tuile hors de l'emprise est absente sans exception", outsideExtent === undefined);

  // --- Scénario : déterminisme -------------------------------------------
  const rebuilt = buildArchive().archive;
  check(
    "deux générations donnent le même fichier, octet pour octet",
    rebuilt.equals(archive),
    `${rebuilt.length} contre ${archive.length} octets`,
  );

  // --- Le vérificateur sait-il échouer ? -------------------------------
  // Un garde-fou qu'on n'a jamais vu refuser quelque chose ne garde rien. On le
  // met donc en échec exprès, en mémoire, sans toucher au fichier du dépôt.
  const healthyTile = Buffer.from((await pmtiles.getZxy(tiles[0].z, tiles[0].x, tiles[0].y)).data);
  check("le validateur PNG accepte une tuile saine", validatePng(healthyTile) === null);
  const corrupted = Buffer.from(healthyTile);
  corrupted[corrupted.length - 20] ^= 0xff; // au milieu de l'IDAT, pas dans l'en-tête
  check(
    "le validateur PNG refuse un octet corrompu",
    validatePng(corrupted) !== null,
    "un octet inversé dans l'IDAT a été accepté : le garde-fou ne garde rien",
  );
  const truncated = healthyTile.subarray(0, healthyTile.length - 10);
  check("le validateur PNG refuse une tuile tronquée", validatePng(truncated) !== null);
  check("le validateur PNG refuse une signature absente", validatePng(Buffer.alloc(40)) !== null);

  // --- Rapport ------------------------------------------------------------
  const byZoom = {};
  for (const t of tiles) byZoom[t.z] = (byZoom[t.z] ?? 0) + 1;
  console.log(
    failures === 0
      ? `Vérification du fond de recette : OK, ${results.length} contrôles.`
      : `Vérification du fond de recette : ÉCHEC, ${failures} contrôle(s) sur ${results.length}.`,
  );
  for (const l of results) if (failures > 0 || l.startsWith("  ÉCHEC")) console.log(l);
  console.log(
    `  ${tiles.length} tuiles, zooms ${ZOOM_MIN} à ${ZOOM_MAX} (${Object.entries(byZoom)
      .map(([z, n]) => `z${z}:${n}`)
      .join(", ")}), ${(archive.length / 1024).toFixed(1)} Ko`,
  );
  if (failures > 0) process.exit(1);
}

main().catch((e) => {
  console.error(`Vérification du fond de recette : ÉCHEC, exception inattendue.\n  ${e.message}`);
  process.exit(1);
});
