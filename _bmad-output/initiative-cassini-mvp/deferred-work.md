# Travail reporté

Chaque entrée vient d'un plan qui a rétréci son périmètre ou d'un constat de revue écarté.

- source_plan: `epic-socle-et-carte/story-generateur-de-fond-de-carte-pmtiles-de-recette-plan.md`
  summary: Dédupliquer les tuiles identiques dans l'archive PMTiles (run-length et décalages partagés).
  evidence: Constat de revue A2, verdict bas. Les 35 tuiles ne contiennent que 8 images distinctes, donc l'archive pourrait peser quelques Ko au lieu de 25,7. Le format le permet explicitement. Écarté : 25,7 Ko ne gêne personne et la déduplication ajoute de la complexité à un encodeur écrit à la main. À reprendre si un vrai fond de carte est un jour produit par ce générateur.

- source_plan: `epic-socle-et-carte/story-generateur-de-fond-de-carte-pmtiles-de-recette-plan.md`
  summary: Ajouter bounds, center, minzoom et maxzoom aux métadonnées JSON de l'archive.
  evidence: Constat de revue A10, verdict indécidable en l'état. Les consommateurs qui construisent un TileJSON depuis les métadonnées plutôt que depuis l'en-tête pourraient retomber sur l'emprise du monde entier. Ce qui trancherait : lire le code du protocole pmtiles de MapLibre. L'entrée 1.3 le tranchera de fait, puisqu'elle branche la carte sur ce chemin.
