import {
  addProtocol,
  Map as MapLibreMap,
  NavigationControl,
  ScaleControl,
  setWorkerUrl,
} from "maplibre-gl";
import type { ErrorEvent, MapOptions } from "maplibre-gl";
import "maplibre-gl/dist/maplibre-gl.css";
import { PMTiles, Protocol } from "pmtiles";
import "./style.css";
import { BasemapSource } from "./basemap.source";
import workerUrl from "maplibre-gl/dist/maplibre-gl-worker.mjs?url";
import rawStyle from "./basemap.style.json";

// Étape 1 : la fenêtre, la carte, et un style qui ne sort jamais sur le réseau.
// Le fond de carte est l'archive PMTiles du dépôt, lue par le moteur Rust et
// servie à la fenêtre par l'IPC Tauri. Aucune requête distante, à aucun moment.
//
// Le style est importé en JSON, donc TypeScript en déduit `version: number`
// alors que MapLibre attend le littéral `8`. On passe par le type public
// `MapOptions["style"]` plutôt que d'importer le paquet de spécification, qui
// n'est qu'une dépendance transitive.
const style = rawStyle as unknown as NonNullable<MapOptions["style"]>;

const statusBar = document.querySelector<HTMLDivElement>("#status");

function display(message: string): void {
  if (statusBar) statusBar.textContent = message;
}

// Un échec du fond de carte doit rester lisible. MapLibre marque sa source
// chargée **avant** d'émettre l'erreur, et son `loaded()` répond vrai dès qu'une
// source a échoué : `load` se déclenche donc juste après l'erreur, à chaque
// fois. Sans ce verrou, « Carte prête » recouvrirait le nom du fichier manquant.
//
// Le verrou ne retient que les messages automatiques : un déplacement de la
// carte est une action de l'utilisateur, et a le droit de reprendre la ligne.
let basemapFailed = false;

// MapLibre fabrique l'adresse de son worker à l'exécution, à partir de
// `import.meta.url`, par concaténation. Vite n'émet un fichier que s'il voit une
// adresse écrite en clair : il n'a donc jamais recopié le worker dans la
// construction, et l'application se chargeait sur une adresse morte — « Worker
// failed to load », constaté sur Windows à l'entrée 1.4.
//
// L'import en `?url` règle les deux moitiés du problème : Vite émet le fichier
// parce que le chemin est une constante, et nous rendons l'adresse produite à
// MapLibre. Tout reste local, donc le CSP suffit tel quel (`worker-src 'self'`).
setWorkerUrl(workerUrl);

// Le protocole doit être enregistré **avant** la création de la carte : MapLibre
// résout les sources du style dès la construction, et un protocole inconnu
// ferait échouer la source au lieu de la charger.
//
// `new Protocol()` sans option : `metadata: true` déclencherait une lecture
// supplémentaire des métadonnées JSON, inutile ici puisque l'emprise et les
// bornes de zoom sont déjà dans l'en-tête de l'archive.
//
// L'instance est enregistrée explicitement par `add`, ce qui est la seule chose
// qui empêche `Protocol` de fabriquer un `FetchSource` sur la clé du style.
const protocol = new Protocol();
protocol.add(
  new PMTiles(
    new BasemapSource((message) => {
      basemapFailed = true;
      display(`Fond de carte indisponible : ${message}`);
    }),
  ),
);
addProtocol("pmtiles", protocol.tile);

const map = new MapLibreMap({
  container: "map",
  style,
  center: [2.415, 48.735],
  zoom: 12,
  attributionControl: false,
});

map.addControl(new NavigationControl({ showCompass: true }), "top-right");
map.addControl(new ScaleControl({ unit: "metric" }), "bottom-right");

map.on("load", () => {
  if (!basemapFailed) display("Carte prête — hors ligne");
});

// L'échec du fond de carte a déjà été nommé par sa source, et plus précisément :
// cet événement ne fait que relayer la même erreur. Il reste en place pour tout
// le reste, par exemple un style refusé.
map.on("error", (e: ErrorEvent) => {
  if (!basemapFailed) display(`Erreur de carte : ${e.error.message}`);
});

map.on("moveend", () => {
  const c = map.getCenter();
  display(`${c.lat.toFixed(5)}, ${c.lng.toFixed(5)} — zoom ${map.getZoom().toFixed(1)}`);
});
