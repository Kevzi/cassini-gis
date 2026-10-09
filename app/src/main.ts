import { Map as CarteMapLibre, NavigationControl, ScaleControl } from "maplibre-gl";
import type { ErrorEvent, MapOptions } from "maplibre-gl";
import "maplibre-gl/dist/maplibre-gl.css";
import "./style.css";
import styleBrut from "./basemap.style.json";

// Étape 1 : la fenêtre, la carte, et un style qui ne sort jamais sur le réseau.
// Le fond de carte PMTiles arrive à la tâche 3 ; ce style provisoire ne sert
// qu'à prouver que la carte vit et se navigue sans aucune requête distante.
//
// Le style est importé en JSON, donc TypeScript en déduit `version: number`
// alors que MapLibre attend le littéral `8`. On passe par le type public
// `MapOptions["style"]` plutôt que d'importer le paquet de spécification, qui
// n'est qu'une dépendance transitive.
const style = styleBrut as unknown as NonNullable<MapOptions["style"]>;

const etat = document.querySelector<HTMLDivElement>("#etat");

function afficher(message: string): void {
  if (etat) etat.textContent = message;
}

const carte = new CarteMapLibre({
  container: "carte",
  style,
  center: [2.415, 48.735],
  zoom: 12,
  attributionControl: false,
});

carte.addControl(new NavigationControl({ showCompass: true }), "top-right");
carte.addControl(new ScaleControl({ unit: "metric" }), "bottom-right");

carte.on("load", () => {
  afficher("Carte prête — hors ligne");
});

carte.on("error", (e: ErrorEvent) => {
  afficher(`Erreur de carte : ${e.error.message}`);
});

carte.on("moveend", () => {
  const c = carte.getCenter();
  afficher(`${c.lat.toFixed(5)}, ${c.lng.toFixed(5)} — zoom ${carte.getZoom().toFixed(1)}`);
});
