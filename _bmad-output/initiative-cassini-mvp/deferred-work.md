# Travail reporté

Chaque entrée vient d'un plan qui a rétréci son périmètre ou d'un constat de revue écarté.

- source_plan: `epic-socle-et-carte/story-generateur-de-fond-de-carte-pmtiles-de-recette-plan.md`
  summary: Dédupliquer les tuiles identiques dans l'archive PMTiles (run-length et décalages partagés).
  evidence: Constat de revue A2, verdict bas. Les 35 tuiles ne contiennent que 8 images distinctes, donc l'archive pourrait peser quelques Ko au lieu de 25,7. Le format le permet explicitement. Écarté : 25,7 Ko ne gêne personne et la déduplication ajoute de la complexité à un encodeur écrit à la main. À reprendre si un vrai fond de carte est un jour produit par ce générateur.

- source_plan: `epic-socle-et-carte/story-generateur-de-fond-de-carte-pmtiles-de-recette-plan.md`
  summary: Ajouter bounds, center, minzoom et maxzoom aux métadonnées JSON de l'archive.
  evidence: Constat de revue A10, verdict indécidable en l'état. Les consommateurs qui construisent un TileJSON depuis les métadonnées plutôt que depuis l'en-tête pourraient retomber sur l'emprise du monde entier. Ce qui trancherait : lire le code du protocole pmtiles de MapLibre. L'entrée 1.3 le tranchera de fait, puisqu'elle branche la carte sur ce chemin.
  status: close, 2026-10-10, entrée 1.3 (`story-brancher-le-fond-pmtiles-dans-la-carte-plan.md`)
  resolution: Rien à ajouter au générateur. Avec `metadata: false`, `Protocol.tilev4` construit `tiles`, `bounds`, `minzoom` et `maxzoom` depuis l'en-tête de l'archive ; `getTileJson`, seul lecteur des métadonnées JSON, n'est appelé que si `metadata` est vrai, et c'est aussi le seul à poser un champ `tilejson`. `app/scripts/verify-protocole.mjs` contrôle les trois valeurs issues de l'en-tête et l'absence de ce champ, donc le constat est désormais couvert par la recette plutôt que reporté.

- source_plan: `epic-socle-et-carte/story-brancher-le-fond-pmtiles-dans-la-carte-plan.md`
  summary: Réconcilier CLAUDE.md §9 (« code et noms de variables en anglais ») avec un dépôt dont tout le code est nommé en français.
  evidence: Constat de revue Q8, verdict moyen mais non causé par cette entrée. La ligne de base est déjà entièrement française (`depuis_chemin_local`, `designe_le_reseau`, `tuilePng`, `version_du_moteur`…) ; l'entrée 1.3 étend l'écart d'environ 400 lignes sans le créer. Deux issues possibles, toutes deux hors du périmètre d'une entrée de branchement de carte : renommer le code en anglais, ou modifier §9. Décision du propriétaire. Le coût du renommage croît à chaque epic, donc trancher tôt est moins cher que tard.

- source_plan: `epic-socle-et-carte/story-compiler-et-lancer-l-application-sous-windows-plan.md`
  summary: Contrôler que tout fichier désigné par le paquet construit existe réellement dans `dist/`, et faire ouvrir l'application construite par la recette.
  evidence: Le défaut du worker MapLibre datait de l'étape 1 et a survécu à deux revues indépendantes et à toute la recette, parce que `npm run build` construit sans jamais ouvrir le résultat : `dist/` ne contenait que trois fichiers alors que le paquet en désignait un quatrième. Un contrôle qui relit le paquet produit et vérifie l'existence de chaque adresse d'asset qu'il nomme aurait attrapé celui-là, hors ligne et sans fenêtre. Pour l'entrée 1.6 (recette automatique), qui construit déjà. Une vérification plus complète — ouvrir l'application construite et lire son état — demande un environnement graphique et reste à arbitrer.

- source_plan: none
  summary: Décider s'il faut protéger les données d'un projet Cassini, et contre quoi — chiffrement au repos, ou intégrité vérifiable de l'historique.
  evidence: Demandé par le propriétaire le 2026-10-11, reporté par lui-même (« on verra »). Trois choses à trancher avant d'écrire une ligne. (1) Le modèle de menace : le vol du support se règle par le chiffrement du disque — BitLocker, ou un produit qualifié ANSSI en contexte étatique — et pas par Cassini ; la recopie par un utilisateur légitime ne se règle par aucun logiciel de bureau. (2) Chez un client défense, du chiffrement non certifié est un handicap plutôt qu'un argument : ils achètent de la qualification. (3) La cryptologie est elle-même encadrée : un logiciel publié en libre bénéficie en général des exemptions du domaine public, mais le module défense privé n'en bénéficierait pas, ce qui alourdirait un dossier d'export déjà en cours ([retiré]). À vérifier auprès de l'ANSSI, pas à supposer. Piste jugée meilleure : l'intégrité plutôt que la confidentialité — signer l'historique des commandes pour prouver qu'un projet n'a pas été trafiqué, ce qui prolonge l'argument de reproductibilité (CAP-7) à faible coût.
