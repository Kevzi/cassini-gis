# Travail reporté

Chaque entrée vient d'un plan qui a rétréci son périmètre ou d'un constat de revue écarté.

- source_plan: `epic-socle-et-carte/story-generateur-de-fond-de-carte-pmtiles-de-recette-plan.md`
  summary: Dédupliquer les tuiles identiques dans l'archive PMTiles (run-length et décalages partagés).
  evidence: Constat de revue A2, verdict bas. Les 35 tuiles ne contiennent que 8 images distinctes, donc l'archive pourrait peser quelques Ko au lieu de 25,7. Le format le permet explicitement. Écarté : 25,7 Ko ne gêne personne et la déduplication ajoute de la complexité à un encodeur écrit à la main. À reprendre si un vrai fond de carte est un jour produit par ce générateur.

- source_plan: `epic-socle-et-carte/story-generateur-de-fond-de-carte-pmtiles-de-recette-plan.md`
  summary: Ajouter bounds, center, minzoom et maxzoom aux métadonnées JSON de l'archive.
  evidence: Constat de revue A10, verdict indécidable en l'état. Les consommateurs qui construisent un TileJSON depuis les métadonnées plutôt que depuis l'en-tête pourraient retomber sur l'emprise du monde entier. Ce qui trancherait : lire le code du protocole pmtiles de MapLibre. L'entrée 1.3 le tranchera de fait, puisqu'elle branche la carte sur ce chemin.
  status: close, 2026-10-10, entrée 1.3 (`story-brancher-le-fond-pmtiles-dans-la-carte-plan.md`)
  resolution: Rien à ajouter au générateur. Avec `metadata: false`, `Protocol.tilev4` construit `tiles`, `bounds`, `minzoom` et `maxzoom` depuis l'en-tête de l'archive ; `getTileJson`, seul lecteur des métadonnées JSON, n'est appelé que si `metadata` est vrai, et c'est aussi le seul à poser un champ `tilejson`. `app/scripts/verify-protocol.mjs` contrôle les trois valeurs issues de l'en-tête et l'absence de ce champ, donc le constat est désormais couvert par la recette plutôt que reporté.

- source_plan: `epic-socle-et-carte/story-brancher-le-fond-pmtiles-dans-la-carte-plan.md`
  summary: Réconcilier CLAUDE.md §9 (« code et noms de variables en anglais ») avec un dépôt dont tout le code est nommé en français.
  evidence: Constat de revue Q8, verdict moyen mais non causé par cette entrée. La ligne de base est déjà entièrement française (`depuis_chemin_local`, `designe_le_reseau`, `tuilePng`, `version_du_moteur`…) ; l'entrée 1.3 étend l'écart d'environ 400 lignes sans le créer. Deux issues possibles, toutes deux hors du périmètre d'une entrée de branchement de carte : renommer le code en anglais, ou modifier §9. Décision du propriétaire. Le coût du renommage croît à chaque epic, donc trancher tôt est moins cher que tard.
  status: close, 2026-10-13, entrée 1.8 (`story-passer-les-noms-du-code-en-anglais-plan.md`)
  resolution: Le propriétaire a tranché pour le renommage, pas pour la modification de §9. Les identifiants français du code sont passés en anglais à comportement identique : variantes et champs de `BasemapError`, méthodes de `BasemapSource`, nom de la commande et clés d'arguments de l'IPC, ids HTML et variables CSS, ids de source et de couches du style, et identifiants des quatre scripts de recette. La prose française reste intacte — commentaires, messages affichés et libellés de recette — donc §9 est appliqué sans être modifié et l'écart est refermé au lieu de croître d'epic en epic.

- source_plan: `epic-socle-et-carte/story-compiler-et-lancer-l-application-sous-windows-plan.md`
  summary: Contrôler que tout fichier désigné par le paquet construit existe réellement dans `dist/`, et faire ouvrir l'application construite par la recette.
  evidence: Le défaut du worker MapLibre datait de l'étape 1 et a survécu à deux revues indépendantes et à toute la recette, parce que `npm run build` construit sans jamais ouvrir le résultat : `dist/` ne contenait que trois fichiers alors que le paquet en désignait un quatrième. Un contrôle qui relit le paquet produit et vérifie l'existence de chaque adresse d'asset qu'il nomme aurait attrapé celui-là, hors ligne et sans fenêtre. Pour l'entrée 1.6 (recette automatique), qui construit déjà. Une vérification plus complète — ouvrir l'application construite et lire son état — demande un environnement graphique et reste à arbitrer.

- source_plan: none
  summary: Décider s'il faut protéger les données d'un projet Cassini, et contre quoi — chiffrement au repos, ou intégrité vérifiable de l'historique.
  evidence: Demandé par le propriétaire le 2026-10-11, reporté par lui-même (« on verra »). Trois choses à trancher avant d'écrire une ligne. (1) Le modèle de menace : le vol du support se règle par le chiffrement du disque — BitLocker, ou un produit qualifié ANSSI en contexte étatique — et pas par Cassini ; la recopie par un utilisateur légitime ne se règle par aucun logiciel de bureau. (2) Chez un client défense, du chiffrement non certifié est un handicap plutôt qu'un argument : ils achètent de la qualification. (3) La cryptologie est elle-même encadrée : un logiciel publié en libre bénéficie en général des exemptions du domaine public, mais le module défense privé n'en bénéficierait pas, ce qui alourdirait un dossier d'export déjà en cours ([retiré]). À vérifier auprès de l'ANSSI, pas à supposer. Piste jugée meilleure : l'intégrité plutôt que la confidentialité — signer l'historique des commandes pour prouver qu'un projet n'a pas été trafiqué, ce qui prolonge l'argument de reproductibilité (CAP-7) à faible coût.

- source_plan: `epic-socle-et-carte/story-passer-les-noms-du-code-en-anglais-plan.md`
  summary: Contrôler dans la recette que chaque id de `app/index.html` est bien celui visé par `app/src/main.ts` et `app/src/style.css`.
  evidence: Constat de revue de l'entrée 1.8, verdict bas, antérieur à cette entrée. Rien dans la recette ne relie les trois côtés : `tsc --noEmit` ne voit pas le HTML, et `check-offline.mjs` ne lit `index.html` que pour y chercher des adresses réseau. Un `id="map"` qui ne correspondrait plus à `container: "map"` laisserait toute la recette au vert et ne se verrait qu'à l'ouverture de la fenêtre, sur une page blanche. Le renommage de l'entrée 1.8 a traversé ce point aveugle sans dommage — concordance vérifiée à la main — mais sans le combler, le plan interdisant d'ajouter un test. Pour l'entrée 1.6 (recette automatique) ou 1.7 (passe de nettoyage), à rapprocher du contrôle de `dist/` déjà reporté par l'entrée 1.4.

- source_plan: `epic-socle-et-carte/story-mettre-le-readme-au-plan-du-14-octobre-plan.md`
  summary: La recette imprimée dans le README conseille `cargo clippy --workspace --all-targets`, qui n'est pas exécutable sous WSL.
  evidence: Constat de revue de l'entrée 1.9, verdict bas, antérieur à cette entrée — la ligne vient de l'entrée 1.5. `libdbus-sys`, dépendance système de la coquille Tauri, ne se construit pas sous WSL, limite déjà consignée aux entrées 1.4, 1.5 et 1.9. Un visiteur sous Linux qui suit le README à la lettre obtient un échec de compilation et croira le dépôt cassé. Deux sorties possibles : installer `dbus-devel` en prérequis documenté, ou faire du `-p cassini-core` la commande imprimée et reléguer `--workspace` à une note. À trancher dans l'entrée 1.6 (recette automatique), qui devra de toute façon décider ce que la CI exécute.

- source_plan: `epic-socle-et-carte/story-mettre-le-readme-au-plan-du-14-octobre-plan.md`
  summary: Le commentaire d'en-tête de `app/src-tauri/src/lib.rs` annonce le registre de commandes à l'étape 2, alors que la nouvelle séquence le place à l'étape 3.
  evidence: Constat de revue de l'entrée 1.9, verdict bas. La phrase est devenue fausse au commit de replanification du 2026-10-14, qui a renuméroté les étapes de CLAUDE.md §7 ; elle n'est pas causée par cette entrée, dont le diff ne touche pas ce fichier. À balayer dans l'entrée 1.7 (passe de nettoyage), avec une relecture des autres commentaires qui citent un numéro d'étape — un numéro d'étape dans un commentaire de code est un piège à obsolescence, et la passe pourrait décider de ne plus en citer du tout.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-socle-et-carte/story-recette-automatique-sur-github-actions-plan.md`
  summary: Figer la version de Rust (`rust-toolchain.toml`) pour que la recette ne dépende pas de l'image du runner.
  evidence: le job `noyau` prend le Rust stable d'`ubuntu-latest` avec `clippy -D warnings` ; un nouveau clippy peut faire échouer un run sans changement de code.

- source_plan: `_bmad-output/initiative-cassini-mvp/plan-8-2-phrases-et-validateur.md`
  summary: Normaliser les unités (m et km, s et min) dans `canonical` de `plan_validator.py`, pour que 500 m et 0,5 km donnent des plans égaux.
  evidence: Constat de relecture, verdict bas. Sans cela le taux de 8.3 compte faux un plan géométriquement identique dont le modèle a choisi une autre unité. Écarté de 8.2 parce que le bloc figé du plan définit l'égalité sans normalisation. À reprendre au début de 8.3, avant la première mesure.
  status: close, 2026-10-15, entrée 8.3 (`epic-epreuve-du-modele-local/story-harnais-de-mesure-du-taux-de-plans-corrects-plan.md`)
  resolution: `canonical` ramène m et km en mètres, s et min en secondes ; 300 m et 0,3 km donnent le même plan, contrôlé par `check_sentences.py`.

- source_plan: none
  summary: Vectorisation d'images (plan satellite d'un village ou d'un réseau routier vers polygones et lignes exploitables pour des calculs), à faire bien après la démo.
  evidence: Demande du propriétaire le 2026-10-15. Deux niveaux : (1) raster déjà classé vers vecteur, commande déléguée à QGIS (GDAL Polygoniser, GRASS r.to.vect), peu de travail ; (2) détection des routes et bâtiments par un modèle de segmentation hors ligne, puis squelettisation et raccord en réseau topologiquement propre. Le niveau 2 est au hors-périmètre de CLAUDE.md §6 (« IA sur images satellite ») : il demande une épreuve préalable sur le modèle de l'étape 2 (un village, qualité des routes mesurée), et une licence MIT, Apache-2.0 ou BSD pour le modèle et ses poids (§2.2).
