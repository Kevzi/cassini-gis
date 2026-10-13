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
// premier. Voir `encodeDirectory`.

import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import zlib from "node:zlib";

const HERE = dirname(fileURLToPath(import.meta.url));
export const OUTPUT_PATH = join(HERE, "..", "..", "fixtures", "basemap-villeneuve-le-roi.pmtiles");

/** Emprise du repère déjà présent dans `app/src/basemap.style.json`. */
export const EXTENT = { west: 2.37, south: 48.71, east: 2.46, north: 48.76 };
/** Centre d'ouverture de la carte, aligné sur `app/src/main.ts`. */
export const CENTER = { lon: 2.415, lat: 48.735 };
export const ZOOM_MIN = 10;
export const ZOOM_MAX = 14;
export const ZOOM_CENTER = 12;

const TILE_SIDE = 256;
const CELL_SIDE = 64;

// ---------------------------------------------------------------- varint

/** Écrit un entier non signé en varint little-endian base 128. */
function writeVarint(bytes, value) {
  let v = BigInt(value);
  if (v < 0n) throw new Error(`varint négatif interdit : ${value}`);
  while (v >= 128n) {
    bytes.push(Number((v & 127n) | 128n));
    v >>= 7n;
  }
  bytes.push(Number(v));
}

// ------------------------------------------------- identifiant de tuile

function rotate(n, x, y, rx, ry) {
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
export function hilbertIndex(n, x, y) {
  let d = 0;
  for (let s = n / 2; s > 0; s = Math.floor(s / 2)) {
    const rx = (x & s) > 0 ? 1 : 0;
    const ry = (y & s) > 0 ? 1 : 0;
    d += s * s * ((3 * rx) ^ ry);
    [x, y] = rotate(n, x, y, rx, ry);
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
export function tileId(z, x, y) {
  if (!Number.isInteger(z) || z < 0 || z > 26) throw new Error(`zoom hors bornes : ${z}`);
  const n = 2 ** z;
  if (!Number.isInteger(x) || x < 0 || x >= n) throw new Error(`x hors bornes au zoom ${z} : ${x}`);
  if (!Number.isInteger(y) || y < 0 || y >= n) throw new Error(`y hors bornes au zoom ${z} : ${y}`);
  return (4 ** z - 1) / 3 + hilbertIndex(n, x, y);
}

// --------------------------------------------------- emprise vers tuiles

const lonToX = (lon, z) => Math.floor(((lon + 180) / 360) * 2 ** z);

const latToY = (lat, z) => {
  const r = (lat * Math.PI) / 180;
  return Math.floor(((1 - Math.log(Math.tan(r) + 1 / Math.cos(r)) / Math.PI) / 2) * 2 ** z);
};

/** Les tuiles couvrant l'emprise, du zoom minimum au maximum, triées par identifiant. */
export function coveringTiles() {
  const list = [];
  for (let z = ZOOM_MIN; z <= ZOOM_MAX; z += 1) {
    const n = 2 ** z;
    const x0 = Math.max(0, lonToX(EXTENT.west, z));
    const x1 = Math.min(n - 1, lonToX(EXTENT.east, z));
    // Le nord donne le plus petit y : l'axe des tuiles descend.
    const y0 = Math.max(0, latToY(EXTENT.north, z));
    const y1 = Math.min(n - 1, latToY(EXTENT.south, z));
    for (let x = x0; x <= x1; x += 1) {
      for (let y = y0; y <= y1; y += 1) {
        list.push({ z, x, y, id: tileId(z, x, y) });
      }
    }
  }
  list.sort((a, b) => a.id - b.id);
  return list;
}

// -------------------------------------------------------------------- PNG

function pngChunk(type, data) {
  const prefix = Buffer.alloc(8);
  prefix.writeUInt32BE(data.length, 0);
  prefix.write(type, 4, "ascii");
  const crc = Buffer.alloc(4);
  // Le CRC couvre le type et les données, pas la longueur.
  crc.writeUInt32BE(zlib.crc32(Buffer.concat([prefix.subarray(4), data])), 0);
  return Buffer.concat([prefix, data, crc]);
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
export function pngTile(z, x, y) {
  const phase = (x + y) % 2;
  // Du plus sombre au plus clair quand on zoome : 5 paliers pour 5 niveaux.
  const step = Math.min(4, Math.max(0, z - ZOOM_MIN));
  const dark = 24 + step * 7;
  const light = dark + 16;
  const lines = [];
  for (let py = 0; py < TILE_SIDE; py += 1) {
    const line = Buffer.alloc(1 + TILE_SIDE * 2); // octet de filtre 0, puis (gris, alpha)
    let o = 1;
    for (let px = 0; px < TILE_SIDE; px += 1) {
      const checker = (Math.floor(px / CELL_SIDE) + Math.floor(py / CELL_SIDE) + phase) % 2 === 0;
      const edge = px < 2 || py < 2 || px > TILE_SIDE - 3 || py > TILE_SIDE - 3;
      line[o] = edge ? 190 : checker ? dark : light;
      line[o + 1] = 255;
      o += 2;
    }
    lines.push(line);
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(TILE_SIDE, 0);
  ihdr.writeUInt32BE(TILE_SIDE, 4);
  ihdr[8] = 8; // profondeur de bit
  ihdr[9] = 4; // type de couleur : gris + alpha
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    pngChunk("IHDR", ihdr),
    pngChunk("IDAT", zlib.deflateSync(Buffer.concat(lines), { level: 9 })),
    pngChunk("IEND", Buffer.alloc(0)),
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
export function encodeDirectory(entries) {
  if (entries.length === 0) throw new Error("un répertoire doit contenir au moins une entrée");
  const bytes = [];
  writeVarint(bytes, entries.length);
  let lastId = 0;
  for (const e of entries) {
    writeVarint(bytes, e.id - lastId);
    lastId = e.id;
  }
  for (const e of entries) writeVarint(bytes, e.runLength);
  for (const e of entries) writeVarint(bytes, e.length);
  let nextByte = 0;
  for (const [i, e] of entries.entries()) {
    // Un décalage contigu avec la tuile précédente s'écrit 0 ; sinon décalage + 1.
    if (i > 0 && e.offset === nextByte) writeVarint(bytes, 0);
    else writeVarint(bytes, e.offset + 1);
    nextByte = e.offset + e.length;
  }
  return Buffer.from(bytes);
}

// ---------------------------------------------------------------- en-tête

function writePosition(buffer, offset, lon, lat) {
  buffer.writeInt32LE(Math.round(lon * 1e7), offset);
  buffer.writeInt32LE(Math.round(lat * 1e7), offset + 4);
}

/** L'en-tête de 127 octets, disposition du §3.1 de la spécification. */
export function encodeHeader(c) {
  const h = Buffer.alloc(127);
  h.write("PMTiles", 0, "ascii");
  h[7] = 3;
  const u64 = [
    c.rootDirectoryOffset, c.rootDirectoryLength,
    c.metadataOffset, c.metadataLength,
    c.leafDirectoriesOffset, c.leafDirectoriesLength,
    c.tileDataOffset, c.tileDataLength,
    c.addressedTilesCount, c.tileEntriesCount, c.tileContentsCount,
  ];
  u64.forEach((v, i) => h.writeBigUInt64LE(BigInt(v), 8 + i * 8));
  h[96] = 1; // groupé par identifiant
  h[97] = 2; // compression interne : gzip
  h[98] = 1; // compression des tuiles : aucune, le PNG est déjà compressé
  h[99] = 2; // type de tuile : PNG
  h[100] = ZOOM_MIN;
  h[101] = ZOOM_MAX;
  writePosition(h, 102, EXTENT.west, EXTENT.south);
  writePosition(h, 110, EXTENT.east, EXTENT.north);
  h[118] = ZOOM_CENTER;
  writePosition(h, 119, CENTER.lon, CENTER.lat);
  return h;
}

// -------------------------------------------------------------- assemblage

export function buildArchive() {
  const tiles = coveringTiles();
  const blobs = [];
  const entries = [];
  let offset = 0;
  for (const t of tiles) {
    const blob = pngTile(t.z, t.x, t.y);
    blobs.push(blob);
    entries.push({ id: t.id, runLength: 1, length: blob.length, offset });
    offset += blob.length;
  }
  const tileData = Buffer.concat(blobs);

  const metadata = zlib.gzipSync(
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
  const directory = zlib.gzipSync(encodeDirectory(entries), { level: 9 });

  const rootDirectoryOffset = 127;
  const metadataOffset = rootDirectoryOffset + directory.length;
  const tileDataOffset = metadataOffset + metadata.length;

  if (metadataOffset > 16384) {
    throw new Error(
      `le répertoire racine dépasse les 16 384 premiers octets (${metadataOffset}) : il faudrait des répertoires feuilles`,
    );
  }

  const header = encodeHeader({
    rootDirectoryOffset,
    rootDirectoryLength: directory.length,
    metadataOffset,
    metadataLength: metadata.length,
    leafDirectoriesOffset: tileDataOffset,
    leafDirectoriesLength: 0,
    tileDataOffset,
    tileDataLength: tileData.length,
    addressedTilesCount: entries.length,
    tileEntriesCount: entries.length,
    tileContentsCount: entries.length,
  });

  return { archive: Buffer.concat([header, directory, metadata, tileData]), tiles };
}

function main() {
  const { archive, tiles } = buildArchive();
  mkdirSync(dirname(OUTPUT_PATH), { recursive: true });
  writeFileSync(OUTPUT_PATH, archive);
  const byZoom = {};
  for (const t of tiles) byZoom[t.z] = (byZoom[t.z] ?? 0) + 1;
  console.log(`Fond de recette écrit : ${OUTPUT_PATH}`);
  console.log(
    `  ${tiles.length} tuiles, zooms ${ZOOM_MIN} à ${ZOOM_MAX} (${Object.entries(byZoom)
      .map(([z, n]) => `z${z}:${n}`)
      .join(", ")})`,
  );
  console.log(`  ${(archive.length / 1024).toFixed(1)} Ko, aucune requête réseau`);
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) main();
