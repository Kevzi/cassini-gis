# Travail reporté

Chaque entrée vient d'un plan qui a rétréci son périmètre ou d'un constat de revue écarté.

- source_plan: `epic-socle-et-carte/story-generateur-de-fond-de-carte-pmtiles-de-recette-plan.md`
  summary: Dédupliquer les tuiles identiques dans l'archive PMTiles (run-length et décalages partagés).
  evidence: Constat de revue A2, verdict bas. Les 35 tuiles ne contiennent que 8 images distinctes, donc l'archive pourrait peser quelques Ko au lieu de 25,7. Le format le permet explicitement. Écarté : 25,7 Ko ne gêne personne et la déduplication ajoute de la complexité à un encodeur écrit à la main. À reprendre si un vrai fond de carte est un jour produit par ce générateur.
  status: decision, 2026-10-15, entrée 7 (`epic-socle-et-carte/story-passe-de-nettoyage-plan.md`)
  resolution: Report maintenu : 25,7 Ko ne gênent personne et la déduplication complexifie un encodeur écrit à la main ; à rouvrir si ce générateur produit un jour un vrai fond de carte.

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
  status: close, 2026-10-15, entrée 7 (`epic-socle-et-carte/story-passe-de-nettoyage-plan.md`)
  resolution: `app/scripts/verify-dist.mjs`, lancé par `npm run verify:dist` après `npm run build` et par la CI, vérifie que chaque adresse `/assets/...` de `dist/index.html`, et chaque `.js`, `.mjs` et `.css` référencé depuis elles, existe dans `dist/`, avec échec nommant le fichier. Ouvrir l'application construite demande un environnement graphique et reste reporté.

- source_plan: none
  summary: Décider s'il faut protéger les données d'un projet Cassini, et contre quoi — chiffrement au repos, ou intégrité vérifiable de l'historique.
  evidence: Demandé par le propriétaire le 2026-10-11, reporté par lui-même (« on verra »). Trois choses à trancher avant d'écrire une ligne. (1) Le modèle de menace : le vol du support se règle par le chiffrement du disque — BitLocker, ou un produit qualifié ANSSI en contexte étatique — et pas par Cassini ; la recopie par un utilisateur légitime ne se règle par aucun logiciel de bureau. (2) Chez un client défense, du chiffrement non certifié est un handicap plutôt qu'un argument : ils achètent de la qualification. (3) La cryptologie est elle-même encadrée : un logiciel publié en libre bénéficie en général des exemptions du domaine public, mais le module défense privé n'en bénéficierait pas, ce qui alourdirait ses démarches réglementaires. À vérifier auprès de l'ANSSI, pas à supposer. Piste jugée meilleure : l'intégrité plutôt que la confidentialité — signer l'historique des commandes pour prouver qu'un projet n'a pas été trafiqué, ce qui prolonge l'argument de reproductibilité (CAP-7) à faible coût.
  status: decision, 2026-10-15, entrée 7 (`epic-socle-et-carte/story-passe-de-nettoyage-plan.md`)
  resolution: Report maintenu, décidé par le propriétaire le 2026-10-11 (« on verra ») : la passe ne tranche pas à sa place.

- source_plan: `epic-socle-et-carte/story-passer-les-noms-du-code-en-anglais-plan.md`
  summary: Contrôler dans la recette que chaque id de `app/index.html` est bien celui visé par `app/src/main.ts` et `app/src/style.css`.
  evidence: Constat de revue de l'entrée 1.8, verdict bas, antérieur à cette entrée. Rien dans la recette ne relie les trois côtés : `tsc --noEmit` ne voit pas le HTML, et `check-offline.mjs` ne lit `index.html` que pour y chercher des adresses réseau. Un `id="map"` qui ne correspondrait plus à `container: "map"` laisserait toute la recette au vert et ne se verrait qu'à l'ouverture de la fenêtre, sur une page blanche. Le renommage de l'entrée 1.8 a traversé ce point aveugle sans dommage — concordance vérifiée à la main — mais sans le combler, le plan interdisant d'ajouter un test. Pour l'entrée 1.6 (recette automatique) ou 1.7 (passe de nettoyage), à rapprocher du contrôle de `dist/` déjà reporté par l'entrée 1.4.
  status: close, 2026-10-15, entrée 7 (`epic-socle-et-carte/story-passe-de-nettoyage-plan.md`)
  resolution: `app/scripts/verify-ids.mjs`, branché dans `npm run check`, relie les ids de `index.html` à `main.ts` et `style.css` dans les deux sens, avec échec nommant l'id orphelin ou mort et autocontrôle.

- source_plan: `epic-socle-et-carte/story-mettre-le-readme-au-plan-du-14-octobre-plan.md`
  summary: La recette imprimée dans le README conseille `cargo clippy --workspace --all-targets`, qui n'est pas exécutable sous WSL.
  evidence: Constat de revue de l'entrée 1.9, verdict bas, antérieur à cette entrée — la ligne vient de l'entrée 1.5. `libdbus-sys`, dépendance système de la coquille Tauri, ne se construit pas sous WSL, limite déjà consignée aux entrées 1.4, 1.5 et 1.9. Un visiteur sous Linux qui suit le README à la lettre obtient un échec de compilation et croira le dépôt cassé. Deux sorties possibles : installer `dbus-devel` en prérequis documenté, ou faire du `-p cassini-core` la commande imprimée et reléguer `--workspace` à une note. À trancher dans l'entrée 1.6 (recette automatique), qui devra de toute façon décider ce que la CI exécute.
  status: close, 2026-10-15, entrée 7 (`epic-socle-et-carte/story-passe-de-nettoyage-plan.md`)
  resolution: Le README garde la commande `--workspace` et ajoute la note WSL : `cargo clippy -p cassini-core -p cassini --all-targets --locked -- -D warnings`.

- source_plan: `epic-socle-et-carte/story-mettre-le-readme-au-plan-du-14-octobre-plan.md`
  summary: Le commentaire d'en-tête de `app/src-tauri/src/lib.rs` annonce le registre de commandes à l'étape 2, alors que la nouvelle séquence le place à l'étape 3.
  evidence: Constat de revue de l'entrée 1.9, verdict bas. La phrase est devenue fausse au commit de replanification du 2026-10-14, qui a renuméroté les étapes de CLAUDE.md §7 ; elle n'est pas causée par cette entrée, dont le diff ne touche pas ce fichier. À balayer dans l'entrée 1.7 (passe de nettoyage), avec une relecture des autres commentaires qui citent un numéro d'étape — un numéro d'étape dans un commentaire de code est un piège à obsolescence, et la passe pourrait décider de ne plus en citer du tout.
  status: close, 2026-10-15, entrée 7 (`epic-socle-et-carte/story-passe-de-nettoyage-plan.md`)
  resolution: L'en-tête de `lib.rs` ne cite plus aucune étape, rien à changer ; le commentaire de `app/src/main.ts` qui citait « Étape 1 » n'en cite plus.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-socle-et-carte/story-recette-automatique-sur-github-actions-plan.md`
  summary: Figer la version de Rust (`rust-toolchain.toml`) pour que la recette ne dépende pas de l'image du runner.
  evidence: le job `noyau` prend le Rust stable d'`ubuntu-latest` avec `clippy -D warnings` ; un nouveau clippy peut faire échouer un run sans changement de code.
  status: close, 2026-10-15, entrée 7 (`epic-socle-et-carte/story-passe-de-nettoyage-plan.md`)
  resolution: `rust-toolchain.toml` fige `1.93.1` avec `rustfmt` et `clippy` ; le noyau reste à `rust-version = "1.89"`.

- source_plan: `_bmad-output/initiative-cassini-mvp/plan-8-2-phrases-et-validateur.md`
  summary: Normaliser les unités (m et km, s et min) dans `canonical` de `plan_validator.py`, pour que 500 m et 0,5 km donnent des plans égaux.
  evidence: Constat de relecture, verdict bas. Sans cela le taux de 8.3 compte faux un plan géométriquement identique dont le modèle a choisi une autre unité. Écarté de 8.2 parce que le bloc figé du plan définit l'égalité sans normalisation. À reprendre au début de 8.3, avant la première mesure.
  status: close, 2026-10-15, entrée 8.3 (`epic-epreuve-du-modele-local/story-harnais-de-mesure-du-taux-de-plans-corrects-plan.md`)
  resolution: `canonical` ramène m et km en mètres, s et min en secondes ; 300 m et 0,3 km donnent le même plan, contrôlé par `check_sentences.py`.

- source_plan: none
  summary: Vectorisation d'images (plan satellite d'un village ou d'un réseau routier vers polygones et lignes exploitables pour des calculs), à faire bien après la démo.
  evidence: Demande du propriétaire le 2026-10-15. Deux niveaux : (1) raster déjà classé vers vecteur, commande déléguée à QGIS (GDAL Polygoniser, GRASS r.to.vect), peu de travail ; (2) détection des routes et bâtiments par un modèle de segmentation hors ligne, puis squelettisation et raccord en réseau topologiquement propre. Le niveau 2 est au hors-périmètre de CLAUDE.md §6 (« IA sur images satellite ») : il demande une épreuve préalable sur le modèle de l'étape 2 (un village, qualité des routes mesurée), et une licence MIT, Apache-2.0 ou BSD pour le modèle et ses poids (§2.2).
  status: decision, 2026-10-15, entrée 7 (`epic-socle-et-carte/story-passe-de-nettoyage-plan.md`)
  resolution: Report maintenu, demandé par le propriétaire le 2026-10-15 pour bien après la démo : la passe ne décide rien à sa place.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-registre-projet-import/story-traceur-la-crate-cassini-et-une-premiere-commande-par-cassin-plan.md`
  summary: `Registry::register` remplace en silence une commande de même id ; le contrat de 2.3 doit refuser un id en double.
  evidence: `core/src/registry.rs`, `register` : `BTreeMap::insert` écarte l'ancienne entrée sans erreur, contre CLAUDE.md §4 (chaque commande décrite une seule fois). À traiter dans 2.2 (enregistrement) ou 2.3 (contrôles du contrat).
  status: close, 2026-10-15, entrée 2.3 (`epic-registre-projet-import/story-contrat-complet-d-une-commande-controle-par-le-registre-lui-plan.md`)
  resolution: `register` garde la première entrée et note le doublon ; `check_contract` rend `contract.duplicate_id`.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-socle-et-carte/story-passe-de-nettoyage-plan.md`
  summary: Rendre l'archive PMTiles de recette indépendante de la version de zlib de Node, ou figer Node (`.nvmrc`, `engines`).
  evidence: Sous Node 24.13 (zlib 1.3.1), `verify-basemap.mjs` échoue « deux générations donnent le même fichier, octet pour octet » : l'archive commitée vient de Node 22 et le fichier régénéré a les mêmes 26360 octets mais d'autres bytes, la compression deflate changeant d'une version à l'autre. Constaté pendant 1.7, antérieur à elle, sans lien avec son diff ; la CI (Node 22) passe vraisemblablement. Ce qui trancherait : lancer la recette sous Node 22 ici, puis choisir entre figer Node ou comparer le contenu décompressé.
  status: close, 2026-10-15, entrée 7 (`epic-socle-et-carte/story-passe-de-nettoyage-plan.md`)
  resolution: `generate-basemap.mjs` n'utilise plus le deflate de Node : un encodeur Huffman fixe, déterministe, écrit l'IDAT des PNG et les deux gzip de l'archive ; `zlib` ne sert plus qu'aux CRC. L'archive est régénérée (50,3 Ko au lieu de 25,7) et `npm run check` passe sous Node 24.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-facade-qgis/story-garde-hors-ligne-et-recette-du-python-de-l-extension-plan.md`
  summary: Étendre la garde réseau de l'extension à `qgis.PyQt.QtNetwork`, `ftplib`, `smtplib`, `ssl`, `asyncio` et aux imports dynamiques.
  evidence: La garde 9.3 n'interdit que urllib, requests, socket, http (AD-14) ; `from qgis.PyQt import QtNetwork` ou `__import__("socket")` passeraient. Décision de périmètre à prendre avec le propriétaire.
  status: close, 2026-10-16, entrée 9.13 (`epic-facade-qgis/story-refactor-sweep-plan.md`)
  resolution: `check_extension.py` refuse aussi ftplib, smtplib, ssl, asyncio, telnetlib, xmlrpc, `qgis.PyQt.QtNetwork` et `QtWebSockets`, socketserver, selectors, webbrowser, QtWebKit et QtWebChannel, ainsi que les appels directs à `__import__` et `import_module` ; testé dans `test_check_extension.py`. La garde reste statique : un contournement par `getattr` ou un alias n'est pas lu, limite connue.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-facade-qgis/story-garde-hors-ligne-et-recette-du-python-de-l-extension-plan.md`
  summary: Faire couvrir `ai/Cargo.toml` par le test « noyau sans QGIS ».
  evidence: `/ai` est du noyau (CLAUDE.md §10) mais `test_noyau_sans_qgis` ne lit que `core/` et `cli/`.
  status: close, 2026-10-16, entrée 9.13 (`epic-facade-qgis/story-refactor-sweep-plan.md`)
  resolution: `test_noyau_sans_qgis` contrôle `ai/Cargo.toml` dès que le crate existe ; il n'existe pas encore (étape 5).

- source_plan: `epic-facade-qgis/story-panneau-lancer-une-commande-du-registre-voir-son-resultat-l-plan.md`
  summary: Le bouton local d'arret des gestes (9.1) est retire du panneau ; les gestes lances hors panneau ne sont plus arretables depuis lui.
  evidence: plugin._on_event ne traite plus ORDER_STARTED/ORDER_ENDED et cancel_current est inutilise ; a trancher avec l'epic agent (5).
  status: decision, 2026-10-16, entrée 9.13 (`epic-facade-qgis/story-refactor-sweep-plan.md`)
  resolution: Report maintenu : l'arrêt des gestes lancés hors panneau relève de l'agent (epic 5), qui seul lance des gestes hors panneau.
- source_plan: `epic-facade-qgis/story-panneau-lancer-une-commande-du-registre-voir-son-resultat-l-plan.md`
  summary: runlog.redact ne masque pas un chemin absolu inclus dans le message d'erreur du noyau (AD-9), non verifie.
  evidence: _render_result affiche le message brut ; a regler si le noyau emet un jour un chemin dans un message.
  status: close, 2026-10-16, entrée 9.13 (`epic-facade-qgis/story-refactor-sweep-plan.md`)
  resolution: `redact` masque tout chemin absolu inclus dans une chaîne (lettre de disque, UNC, `/chemin`) ; testé dans `RedactTest`.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-facade-qgis/story-ouvrir-un-geopackage-un-shapefile-et-un-geojson-par-qgis-plan.md`
  summary: Un Shapefile sans .prj (ou un SCR personnalise) donne un SCR vide dans le resultat de layer.add, alors qu'AD-19 veut un code EPSG explicite.
  evidence: QgisBackend.crs_of rend `authid()` sans controle ; a trancher : refuser avec une erreur, ou demander le SCR a l'ouverture (comme le CSV en 9.6).
  status: close, 2026-10-16, entrée 9.13 (`epic-facade-qgis/story-refactor-sweep-plan.md`)
  resolution: Constat périmé : `_resolve_crs` refuse déjà par `layer.unknown_crs`, ou accepte le paramètre `crs` ; couvert par `test_layer_add.py`.

- source_plan: `epic-facade-qgis/story-zone-tampon-par-le-cadre-de-traitement-de-qgis-plan.md`
  summary: Le rejeu d'un journal contenant `analysis.buffer` diverge (octets du GeoPackage, `layer_id` de session).
  evidence: `journal.rs` compare les octets des sorties et le résultat résumé ; à traiter avec l'entrée 9.14.
  status: close, 2026-10-16, entrée 9.14 (`epic-facade-qgis/story-suite-de-bout-en-bout-ouvrir-analyser-exporter-rejouer-plan.md`)
  resolution: `journal.replay` lie les `layer_id` anciens aux neufs, ignore `layer_id` à la comparaison et ne compare pas les octets des sorties QGIS (existence et non-vacuité) ; prouvé par `probe_end_to_end.py` et les tests de `journal.rs`.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-facade-qgis/story-intersection-et-selection-par-attribut-plan.md`
  summary: Tester `FieldCombo` (rebranchement sur `layer_id`, rechargement, choix conservé) dans un vrai panneau Qt.
  evidence: Seule la logique sans Qt est testée ; la règle 11 demande un test du composant, aujourd'hui couvert par la recette manuelle.
  status: decision, 2026-10-16, entrée 9.13 (`epic-facade-qgis/story-refactor-sweep-plan.md`)
  resolution: Report maintenu : un test de composant exige un QGIS avec écran, absent de la recette automatique ; la recette manuelle le couvre.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-facade-qgis/story-statistiques-par-zone-plan.md`
  summary: `analysis.zonal_statistics` peut lire un ancien champ `NUMPOINTS` si la couche de zones en a déjà un.
  evidence: non vérifié (medium si vrai) ; à trancher en lançant la commande sous QGIS sur la sortie d'un premier run réutilisée comme zones ; correctif = nom de champ unique ou refus `analysis.bad_params`.
  status: decision, 2026-10-16, entrée 9.13 (`epic-facade-qgis/story-refactor-sweep-plan.md`)
  resolution: Report maintenu : à vérifier sous QGIS avec `probe_zonal_statistics.py` ; sans preuve, le correctif reste hypothétique.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-facade-qgis/story-suite-de-bout-en-bout-ouvrir-analyser-exporter-rejouer-plan.md`
  summary: `Replay::run` ignore l'échec de `remove_dir_all(replay/)` ; un fichier périmé non vide passerait la règle « sortie QGIS existe et non vide ».
  evidence: medium non vérifié ; il faudrait un échec de suppression sous Windows (fichier tenu par QGIS) et une étape QGIS qui n'écrit rien ; remède probable : supprimer le fichier attendu avant chaque étape et refuser si la suppression échoue.

- source_plan: `epic-facade-qgis/epic-facade-qgis-retrospective.md`
  summary: Le rejeu est aveugle à la forme d'une sortie QGIS : seuls `feature_count` et « le GeoPackage existe et n'est pas vide » sont contrôlés.
  evidence: F7 de la rétro de l'epic 9, vérifié par lecture. L'isochrone rend toujours `feature_count: 1` ; rejouée à pied ou en voiture, ou avec un polygone tout autre, elle serait « identique ». Écart assumé de l'architecture (écrit au commit `7829f94`), pas une faute du 9.11, mais l'isochrone est le verbe où cela compte le plus. Remède possible : une empreinte de géométrie ou une emprise (boîte englobante arrondie) dans le résultat typé, comparée au rejeu. Question de politique avant d'être technique (question ouverte 3 de la rétro) : le résultat typé suffit-il au MVP ?

- source_plan: `epic-facade-qgis/epic-facade-qgis-retrospective.md`
  summary: Le schéma de `Duration.value` de `analysis.isochrone` s'affiche `range(min = 0.0)`, inclusif, alors que la durée est strictement positive.
  evidence: F8 de la rétro de l'epic 9. Le noyau refuse 0 à la désérialisation (`positive`), mais le catalogue montrerait au modèle une borne inférieure atteignable. Écarté au triage du 9.11 (même motif que `analysis.buffer`). À reprendre si schemars gagne un minimum exclusif utilisable, ou en précisant « strictement positive » dans le schéma des deux commandes.

- source_plan: `epic-registre-projet-import/epic-registre-projet-import-retrospective.md`
  summary: Une montée de version du noyau casse sans le dire tout rejeu contenant `system.about` ; `versions` du journal n'est jamais relu au rejeu.
  evidence: F12 de la rétro de l'epic 2 (action R8, non reportée à la rétro de l'epic 9). `core_version` figure dans le résultat de `system.about`, qui est journalisé à chaque démarrage du client ; au rejeu après une montée de version, la comparaison diverge sur cette étape. Remède possible : ne pas comparer `core_version` (masque, comme `layer_id`), ou lire `versions` et le dire.

- source_plan: `epic-registre-projet-import/epic-registre-projet-import-retrospective.md`
  summary: `p.qgz` et `p.qgs` côte à côte partagent le même dossier `p.cassini` (journal et verrou) ; un `p.cassini` en lien symbolique écrit hors du dossier de travail (AD-10).
  evidence: F13 de la rétro de l'epic 2. Le dossier est dérivé du nom du projet sans l'extension, et son existence n'est pas contrôlée comme un vrai dossier. Cas rares mais silencieux.

- source_plan: `epic-registre-projet-import/epic-registre-projet-import-retrospective.md`
  summary: Deux rejeux simultanés écrasent `replay/` ; la lecture d'une ligne à moitié écrite par une session active n'est pas protégée.
  evidence: F14 de la rétro de l'epic 2. Le verrou du journal ne couvre pas `replay/`, et un rejeu lit le journal pendant qu'une session peut encore y écrire.

- source_plan: `epic-registre-projet-import/epic-registre-projet-import-retrospective.md`
  summary: Windows : la comparaison de chemins du journal est sensible à la casse et ne gère pas le préfixe `\?\`.
  evidence: F18 de la rétro de l'epic 2. Un même fichier désigné avec une autre casse ou en chemin étendu peut être jugé hors du dossier du projet (ou l'inverse).

- source_plan: `epic-registre-projet-import/epic-registre-projet-import-retrospective.md`
  summary: Les messages d'erreur serde recopient la valeur fautive, donc possiblement un chemin absolu (AD-9).
  evidence: F19 de la rétro de l'epic 2. `registry.rs` renvoie le message de serde tel quel dans `registry.invalid_params`. À rapprocher du masquage des chemins côté extension (`redact`, 9.13), qui ne couvre pas le journal du noyau.

- source_plan: `epic-registre-projet-import/epic-registre-projet-import-retrospective.md`
  summary: `WriteOutputParams.out` figure au schéma des paramètres alors que la règle 6 interdit un nom de sortie dans les paramètres ; le catalogue le montrerait au modèle.
  evidence: F23 de la rétro de l'epic 2. Aujourd'hui sous `cfg(test)` (commande d'essai), donc sans effet pour l'utilisateur ; à corriger si la commande sort du cadre des tests.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-composer-un-plan-prompt-contraint-validation-sans-reparation-plan.md`
  summary: AD-23 : verifier que les identifiants de couche et les chemins d'un plan sont connus de la session (inventaire).
  evidence: la grammaire accepte tout JSON pour params et judge ne compare rien a l'inventaire ; un id invente peut etre rendu comme plan valide.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-composer-un-plan-prompt-contraint-validation-sans-reparation-plan.md`
  summary: La redemande doit porter la liste des ecarts, pas seulement la premiere erreur de check_plan.
  evidence: check_plan s'arrete a la premiere erreur ; un plan a plusieurs fautes demande plus d'une redemande.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-composer-un-plan-prompt-contraint-validation-sans-reparation-plan.md`
  summary: Verifier la taille du prompt (catalogue + inventaire) contre N_CTX 8192 et corriger le message ai.prompt_too_long.
  evidence: aucun plafond sur l'inventaire ; le message accuse la phrase alors que le catalogue peut etre en cause.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-composer-un-plan-prompt-contraint-validation-sans-reparation-plan.md`
  summary: Eviter de recharger le GGUF a chaque tentative de composition.
  evidence: generate charge le modele et cree un contexte a chaque appel ; une redemande double le cout.
  status: close, 2026-10-18, entrée 12 (`epic-agent-et-mcp/story-modele-charge-une-fois-et-partie-fixe-du-prompt-gardee-en-ca-plan.md`)
  resolution: Le modèle est gardé dans le processus du noyau (clé : chemin, taille, date du fichier) et l'état du moteur après la partie fixe de l'invite et après le tour système est gardé en instantanés ; une redemande reprend l'instantané du tour système et ne relit que la sortie rejetée et les écarts. Seul le contexte est recréé à chaque appel.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-composer-un-plan-prompt-contraint-validation-sans-reparation-plan.md`
  summary: Tester le chemin de cassini plan sans --model (models.no_folder, ai.no_model).
  evidence: les tests CLI passent toujours --model absent.gguf.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-composer-un-plan-prompt-contraint-validation-sans-reparation-plan.md`
  summary: Le critere des 80 % est tenu en comptant les refus attendus : 10/13 plans reels (77 %) ; echecs S16, S17, S19.
  evidence: mesure GPU 22/25 sur Qwen3.5-9B Q4_K_M, deux fois ; 12 phrases attendent un refus faute de commande dans le registre reel. Mesure CPU non confirmee.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-executer-un-plan-valide-etape-par-etape-par-le-canal-plan.md`
  summary: Rendre la composition d'un plan (`plan.propose`) annulable ; aujourd'hui elle bloque la boucle du noyau jusqu'a sa fin.
  evidence: le fil lecteur continue, mais les requetes recues pendant l'inference attendent la fin ; `request.cancel` sur l'id de `plan.propose` repond `request.unknown`. Decision prise sans redemander (Design Notes du plan).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-executer-un-plan-valide-etape-par-etape-par-le-canal-plan.md`
  summary: `command.run` rend encore les chemins absolus d'une sortie QGIS (ex. `file` de `layout.export_pdf`) ; `plan.execute` les rend relatifs au dossier de travail.
  evidence: AD-9 interdit tout chemin absolu dans un message rendu ; seule la reponse de `plan.execute` est relativisee dans ce ticket.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-executer-un-plan-valide-etape-par-etape-par-le-canal-plan.md`
  summary: Verifier que le masquage des chemins de `ModelComposer` couvre un message de llama.cpp qui ecrit le chemin du modele sous une autre forme (canonique, separateurs, guillemets).
  evidence: maybe-false, medium si vrai (AD-9). Le masquage remplace les chaines exactes ; le test au faux `x.gguf` passe, mais un vrai GGUF corrompu sous Windows n'a pas ete essaye. A regler par cet essai.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-clarification-poser-une-question-au-lieu-de-deviner-plan.md`
  summary: Le modele ecrit parfois une question dans un refus (`reason` en forme de question) ; rien ne le detecte, l'utilisateur recoit `agent.refused` formule comme une question.
  evidence: Essais reels de la 5.5 (essai 3 : Q02, Q03, Q11 ; essai 2 : S21). Comportement du modele, a corriger par les exemples d'entrainement du 5.10 plutot que par une regle de reparation (AD-23).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-modele-charge-une-fois-et-partie-fixe-du-prompt-gardee-en-ca-plan.md`
  summary: Precharger le modele et lire la partie fixe de l'invite au demarrage de cassini serve, pour que la premiere demande d'une session QGIS ne paie pas 173 s.
  evidence: mesure 12, S01 a 173,4 s sur 4 coeurs (chargement 4,1 s, lecture 163,7 s) ; le budget de 30 s ne compte que des la 2e demande, le premier appel reste visible a l'utilisateur.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-modele-charge-une-fois-et-partie-fixe-du-prompt-gardee-en-ca-plan.md`
  summary: Prouver avec le vrai modele que cassini serve garde le modele et l'instantane du tour systeme entre deux plan.propose avec l'inventaire envoye par QGIS.
  evidence: le cache n'est verifie que par appels directs a generate_measured (cache_reel) ; le chemin serve est raisonne, pas teste. A porter par la suite de bout en bout (entree 9).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-panneau-saisie-en-langage-naturel-plan-a-valider-selecteur-d-plan.md`
  summary: Selecteur de modele dans les Reglages du panneau : methode du canal `agent.models` (liste sans dossier ni chemin, non journalisee, masquee par le compositeur de `cli`), `model` optionnel sur `plan.propose` (nom de fichier de la liste, absent = premier), erreur `models.unknown`, choix memorise dans `QSettings` `cassini/model`, rechargement par la cle de modele de l'entree 12.
  status: close — porte par l'entree 14 de l'epic 5 (2026-10-18)
  evidence: decoupe par le proprietaire (2026-10-18) : plan 5.6 a ~2 400 jetons ; le selecteur touche le noyau et le protocole, le parcours du plan non. `command.run model.list` ecarte : il journalise (rejeu qui diverge des qu'un fichier est ajoute) et rend le dossier absolu.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-panneau-saisie-en-langage-naturel-plan-a-valider-selecteur-d-plan.md`
  summary: Le modele refuse « Garde seulement les parcelles de la section AB. » en lisant « garde seulement » comme une suppression d'entites, alors que analysis.select_by_attribute cree une nouvelle couche ; a corriger par les exemples d'entrainement du 5.10 (tournures « garde seulement / ne garde que / conserve »).
  evidence: recette du proprietaire dans QGIS, 2026-10-18, projet-test-villeneuve, Qwen3.5-9B Q4_K_M sur CPU : `agent.refused` « Aucune commande du catalogue ne permet de modifier ou de supprimer des entites d'une couche existante (comme les parcelles de la section AB) ; toutes les commandes d'analyse creent de nouvelles couches sans alterer les sources. » ; la meme tournure sur « les zones du PLU de type U » a donne un plan correct. Aucune regle de reparation (AD-23).
- source_plan: `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`
  summary: Bug latent du rejeu : `core/src/journal/summary.rs:24` prend la cle `truncated: true` de premier niveau pour le marqueur de resume ; un resultat de moins de 200 octets qui porte lui-meme `truncated: true` est juge identique a tout resultat garde ainsi marque. Placer les marqueurs sous une cle reservee (AD-11 resserree).
  evidence: revue adverse E1 (reviews/review-adverse-e1.md), verifie a la lecture du code le 2026-10-18 ; inatteignable aujourd'hui (un comptage tronque depasse 200 octets), atteignable par les resumes de l'epic 10.
- source_plan: `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`
  summary: `model.list` rend `folder` en chemin absolu, journalise par `cassini run` et visible par MCP ; ne rendre aucun chemin absolu (AD-9, AD-15 amendee).
  evidence: revues E1 (faits verifies, grille B2), 2026-10-18.
- source_plan: `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`
  summary: Renommer `analysis.zonal_statistics` en `analysis.count_points_in_polygon` et migrer son resultat au gabarit unique de resume (AD-27) : module Rust, table ALGORITHMS, test_analysis.py, sonde, README, prompt et jeu de l'epreuve ; les phrases S15, S23 et S24 visent en fait `raster.zonal_statistics`, non mecaniques.
  evidence: decision du proprietaire 2026-10-18 (contrat d'architecture, convention Familles de commandes) ; inventaire des occurrences dans reviews/review-faits-verifies-e1.md.
- source_plan: `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`
  summary: Mettre le code au niveau d'AD-24, AD-25, AD-27 et AD-28 avant la premiere primitive de l'epic 10 : validation par le noyau du resultat recu (`qgis.bad_result`) ; annulabilite declaree seulement si chaque algorithme porte CanCancel, avec test table/registre ; version de chaine par ligne et version de QGIS annoncees a `session.hello` et journalisees ; annonce apres le chargement des fournisseurs ; constantes de `analysis.isochrone` (vitesses, accrochage 100 m) passees en parametres du contrat de reseau.
  evidence: gate de revue E1 du 2026-10-18 (reviews/review-grille-e1.md H1-H4, M2, M4 ; review-adverse-e1.md) ; aujourd'hui toutes les commandes deleguees sont `cancellable(true)` et le noyau ne valide pas les resultats.
- source_plan: none
  summary: 5.10, tache 2 — entrainement reproductible : une commande lance un LoRA ou QLoRA tenant dans 8 Go de VRAM sur un modele de base Apache-2.0 ou MIT de 1,5 a 4 milliards de parametres, sur le jeu engendre, et fusionne en GGUF.
  evidence: decoupe du ticket 5.10 par le proprietaire (2026-10-22, bmad-build) : quatre livrables fusionnables a part ; celui-ci attend le jeu de donnees (tache 1) et des dependances Python dont la licence est a valider.
- source_plan: none
  summary: 5.10, tache 3 — branchement du modele ajuste dans `ai/` : prompt court de quelques centaines de jetons, chargement d'une base avec un adaptateur LoRA, selon le selecteur de 5.14.
  evidence: decoupe du ticket 5.10 par le proprietaire (2026-10-22, bmad-build) ; independant de l'entrainement cote code, utile seulement une fois un modele ajuste disponible.
- source_plan: none
  summary: 5.10, tache 4 — mesures et demo : score du modele ajuste sur le jeu tenu a l'ecart au moins egal au 9B, seuil strict de clarification (>= 9/12, <= 1 fausse question), < 30 s sur le 5800X bride a 4 coeurs, demo de bout en bout dans QGIS avec la phrase de demo exacte.
  evidence: decoupe du ticket 5.10 par le proprietaire (2026-10-22, bmad-build) ; ferme le ticket, attend les taches 1 a 3.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-modele-de-composition-ajuste-et-plus-petit-fine-tuning-plan.md`
  summary: Avant l'entrainement (5.10 tache 2) : l'inventaire envoye par l'extension (`qgis-plugin/cassini_qgis/inventory.py` l.211) porte le nom et le type des champs mais pas leurs valeurs, alors que le jeu engendre les porte ; trancher : ajouter les valeurs distinctes a l'inventaire du produit, ou engendrer aussi des inventaires sans valeurs.
  evidence: constat de la tache 1 (2026-10-22) ; un modele entraine sur des valeurs qu'il ne verra pas en production apprendrait un appui absent (ex. typezone U).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-modele-de-composition-ajuste-et-plus-petit-fine-tuning-plan.md`
  summary: Le generateur ne met jamais deux couches quasi synonymes cote a cote (« Routes » et « Reseau de voirie ») alors que le jeu tenu a l'ecart et les vrais projets le font ; ajouter une famille ou la phrase tranche, ou ou la sortie attendue est une question.
  evidence: relecture blind-hunter de la tache 1 ; garde volontaire dans `training/src/inventories.rs` (groupes exclusifs).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-modele-de-composition-ajuste-et-plus-petit-fine-tuning-plan.md`
  summary: `ai/tests/ecart_reel.rs` : sortir la decision par phrase dans une fonction testee sans modele (refus en Err(Refused), titre seul different, titre absent, question attendue mais plan rendu), et imprimer pour chaque echec une forme de l'ecart (commande, ordre, valeur) sans le texte de la phrase.
  evidence: relecture verification-gap et blind-hunter ; a fermer quand la tache 4 reutilise le notateur pour le modele ajuste.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-modele-de-composition-ajuste-et-plus-petit-fine-tuning-plan.md`
  summary: La commande `cassini-training` n'a aucun test d'integration (deux executions identiques, autre graine differente, code non nul sans --out).
  evidence: relecture verification-gap ; verifie a la main par cmp ; a ajouter avant que la tache 2 depende du fichier.
  status: close, 2026-10-22, 5.10 tache 2 (`plan-5-10-t2-entrainement-reproductible.md`)
  resolution: `training/tests/commande.rs` lance le binaire : meme graine, memes octets (formats structured et chat) ; autre graine, autre fichier ; --out absent, --format inconnu ou graine illisible, code non nul.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-modele-de-composition-ajuste-et-plus-petit-fine-tuning-plan.md`
  summary: CLAUDE.md §9 (structure du depot) ne cite pas le nouveau crate `/training` (chaine d'entrainement) ; a ajouter par le proprietaire.
  evidence: relecture quick ; un fichier de contexte d'agent ne se modifie pas dans un build.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t2-entrainement-reproductible.md`
  summary: L'extension joint a l'inventaire envoye au noyau quelques valeurs distinctes par champ texte (`qgis-plugin/cassini_qgis/inventory.py`), comme le jeu d'entrainement ; a faire avant la mesure du modele ajuste (5.10, tache 4).
  evidence: decision du proprietaire 2026-10-22 (bmad-build 5.10, tache 2) : « Ajouter les valeurs » plutot qu'entrainer sans valeurs.
  status: close, 2026-10-23, 5.10 tache 4a (`plan-5-10-t4a-inventaire-au-format-appris.md`)
  resolution: l'extension rapporte jusqu'a 11 valeurs par champ texte, le noyau (`core/src/agent_inventory.rs`) en garde 10 au plus, triees, et met l'inventaire dans la forme de l'entrainement avant la composition.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t2-entrainement-reproductible.md`
  summary: 5.10 tache 3 — parite au jeton entre l'entrainement et `ai/` : `ai` decoupe le prompt par segment (texte des tours en `special=false`), la chaine d'entrainement le decoupe d'un tenant ; et aucun test ne compare l'encadrement chatml de `training/src/prompt.rs` aux constantes de `ai/src/lib.rs` ni `system_text` a `inventory_text`.
  evidence: relectures verification-gap, edge-case-hunter et intent-alignment de la tache 2 ; un ecart d'un jeton ferait voir au modele ajuste un format qu'il n'a pas appris.
  status: close, 2026-10-22, 5.10 tache 3 (`plan-5-10-t3-modele-ajuste-dans-ai.md`)
  resolution: `ai/src/plan.rs` : `the_short_prompt_is_the_training_prompt_byte_for_byte_on_the_whole_generated_set` (texte des tours assemble par `chat_tokens_with` = `render_prompt`, sur les 5 000 exemples, encadrement chatml compris) et `the_short_prompt_tokens_are_those_of_the_training_prompt_read_in_one_piece` (vrai modele, base Q4_K_M : sur 300 prompts, les jetons lus par segments sont ceux du decoupage d'un tenant, avec `special=true`, du prompt d'entrainement par le decoupeur de llama.cpp du meme GGUF). L'egalite avec le decoupeur de Hugging Face, celui de l'entrainement, n'est pas verifiee par ce test.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t2-entrainement-reproductible.md`
  summary: Au prochain re-entrainement, rendre au `SYSTEM` du prompt court la precision « (en comptant a partir de 1) » pour `step:k.layer_id`, comme `ai/src/plan.rs`.
  evidence: relecture quick de la tache 2 ; la changer sans re-entrainer decalerait le prompt du modele deja entraine.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t2-entrainement-reproductible.md`
  summary: 5.10 tache 4 — mesurer les GGUF livres (base + adaptateur, et fusionne) par `ai/`, le score de validation de la tache 2 etant pris sur l'adaptateur en transformers.
  evidence: relecture intent-alignment de la tache 2 ; verifie a la main sur un exemple (plan identique), pas sur le jeu.
  status: close, 2026-10-23, 5.10 tache 4b (`plan-5-10-t4b-mesures-et-demo.md`)
  resolution: mesures par `ai/` sur GPU (`ecart_reel`, `question_reel`, `plan_reel`) des deux declarations : base + adaptateur 32/40, fusionne 33/40 sur le jeu tenu a l'ecart ; 11/12 questions ; 16/25 plans conformes, 0 fausse question (`docs/entrainement-du-modele.md`, « Tache 4 : mesures »).
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t3-modele-ajuste-dans-ai.md`
  summary: 5.10 tache 4 — comparer, sur le jeu engendre, les jetons du prompt court decoupe par le tokenizer Hugging Face de l'entrainement (`pipeline.py` `encode`) a ceux de llama.cpp sur le GGUF de base ; seule la parite `ai` / llama.cpp est verifiee.
  evidence: relectures intent-alignment et quick de la tache 3 ; un ecart entre les deux tokenizers ferait voir au modele un decoupage qu'il n'a pas appris, sans qu'aucun test ne le signale.
  status: close, 2026-10-23, 5.10 tache 4b (`plan-5-10-t4b-mesures-et-demo.md`)
  resolution: `training/entrainement/parite_decoupeurs.py` (lancee a la main, comparaison testee sans torch) : 300/300 prompts de validation identiques, 205 803 jetons, entre le decoupeur Hugging Face de la base epinglee et `llama-tokenize` du GGUF de base.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t3-modele-ajuste-dans-ai.md`
  summary: 5.10 tache 4 — mesurer ce que vaut la redemande (sortie rejetee puis ecarts) pour le modele ajuste, qui n'a jamais appris ce tour ; garder la redemande unique (AD-23), ou l'apprendre au prochain entrainement.
  evidence: relecture blind-hunter de la tache 3 ; `compose_plan_with` garde `retry_turns` pour le prompt court, hors de la distribution d'entrainement.
  status: close, 2026-10-23, 5.10 tache 4b (`plan-5-10-t4b-mesures-et-demo.md`)
  resolution: mesure : 1 (adaptateur) et 2 (fusionne) phrases sur 40 passent par la redemande, aucune n'est sauvee (le 9B en sauve 1). Redemande gardee (AD-23) ; l'apprendre est porte par l'entree de re-entrainement ci-dessous.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t3-modele-ajuste-dans-ai.md`
  summary: 5.14 — diagnostic des declarations de modele : une declaration illisible est ecartee sans trace (ses GGUF, adaptateur compris, reviennent nus dans la liste), et une declaration dont un GGUF manque reste listee avec une taille partielle et, premiere par son nom, ferait echouer chaque `plan.propose` ; le selecteur doit les signaler.
  evidence: relectures blind-hunter, edge-case-hunter et quick de la tache 3 ; comportement voulu par le plan (« ignoree », « reste listee »), sans retour a l'utilisateur.
  status: close, 2026-10-23, 5.14 (`epic-agent-et-mcp/story-selecteur-de-modele-dans-les-reglages-du-panneau-plan.md`)
  resolution: `core/src/commands/model_list.rs` `diagnose` (meme parcours que `model.list`, inchange) : une declaration illisible reste listee avec son `problem`, un GGUF nomme absent donne « fichier manquant : x.gguf » ; `agent.models` porte ce `problem`, le selecteur grise ces entrees avec leur raison, « absent = premier » prend le premier modele sans probleme, choisir un modele a probleme rend `models.unusable`.
- source_plan: none
  summary: 5.10, tache 4b — mesures et demo du modele ajuste : essais reels qui acceptent une declaration `*.modele.json`, notateur de `ecart_reel` teste sans modele, score sur le jeu tenu a l'ecart (base + adaptateur et fusionne) au moins egal a 32/40, seuil strict de clarification (>= 9/12, <= 1 fausse question sur 25), < 30 s sur le 5800X bride a 4 coeurs, parite des decoupeurs Hugging Face et llama.cpp, valeur de la redemande, demo de bout en bout dans QGIS avec la phrase de demo exacte.
  evidence: decoupe de la tache 4 par le proprietaire (2026-10-22, bmad-build) : l'inventaire du produit (`project.inventory`) ne porte ni champs ni geometrie, alors que le modele ajuste a appris sur des inventaires qui les portent ; la tache 4a l'aligne d'abord, la demo QGIS en depend.
  status: close, 2026-10-23, 5.10 tache 4b (`plan-5-10-t4b-mesures-et-demo.md`)
  resolution: 4b fermee comme « mesuree » (decision du proprietaire, 2026-10-23) : jeu tenu a l'ecart 32/40 et 33/40, 11/12 questions, 0 fausse question, parite des decoupeurs 300/300, demo QGIS reussie ; sous la barre : `plan_reel` 16/25 et A0 en 41,4 s sur CPU 4 fils. Re-entrainement propose ci-dessous ; 5.10 reste ouvert.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t4a-inventaire-au-format-appris.md`
  summary: 5.10 tache 4b — l'inventaire traduit du projet reel `projet-test-villeneuve.qgz` fait 2 439 jetons du GGUF de base, plus que la plus longue sequence entiere de l'entrainement (1 326 jetons) : mesurer l'effet sur le modele ajuste ; si besoin, apprendre des inventaires plus longs au prochain entrainement (champs nombreux, valeurs longues) plutot que tronquer l'inventaire.
  evidence: sonde de la tache 4a sous QGIS 3.44.15 ; la couche `plu_zones` seule fait 2 890 caracteres (24 champs, empreintes `gpu_doc_id`, libelles longs).
  status: close, 2026-10-23, 5.10 tache 4b (`plan-5-10-t4b-mesures-et-demo.md`)
  resolution: mesure par le noyau et l'extension actuels : 4 007 caracteres, 1 458 jetons ; l'ecart avec le chiffre de 4a (pris avant les correctifs de la relecture de 4a) n'est pas explique par un filtre de longueur, `core/src/agent_inventory.rs` n'en a pas ; l'invite entiere du prompt court 1 737 jetons. Le modele fusionne compose la phrase de demo juste sur cet inventaire, dans QGIS et dans `duree_reel`.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t4a-inventaire-au-format-appris.md`
  summary: Au prochain entrainement, le generateur ecrit les valeurs d'un champ triees, comme le produit (`core/src/agent_inventory.rs`), au lieu de l'ordre de definition du theme.
  evidence: relecture intent-alignment et blind-hunter de la tache 4a ; `training/src/inventories.rs` l.1349 trie des indices ; QGIS (`uniqueValues`) n'a pas d'ordre canonique, le produit trie.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t4a-inventaire-au-format-appris.md`
  summary: Formes d'inventaire jamais vues a l'entrainement : couche sans fichier (sorties temporaires des etapes precedentes, sans cle `source`), projet non enregistre (sans `project.name`), SCR non EPSG (sans `epsg`), table sans geometrie (`geometry: null` sur un vecteur), `unloaded_files` toujours vide dans QGIS (68 % des exemples en ont) ; mesurer en 4b, les apprendre au prochain entrainement.
  evidence: relectures edge-case-hunter, blind-hunter et quick de la tache 4a ; `forme_inventaire.rs` ne couvre que le cas nominal.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t4a-inventaire-au-format-appris.md`
  summary: 5.10 tache 4b — chronometrer `project.inventory` sur le vrai projet : un `uniqueValues` par champ texte de chaque couche vecteur, sur le fil de QGIS, a chaque `plan.propose`, non interruptible ; si c'est long, garder les valeurs en cache par session.
  evidence: relectures blind-hunter, edge-case-hunter et quick de la tache 4a ; gravite non verifiee (maybe-false) : la sonde a compte les jetons, pas la duree.
  status: close, 2026-10-23, 5.10 tache 4b (`plan-5-10-t4b-mesures-et-demo.md`)
  resolution: chronometre dans la demo de bout en bout : 0,05 s sur le vrai projet (6 couches). Pas de cache necessaire.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t4a-inventaire-au-format-appris.md`
  summary: 5.10 tache 4b — verifier par toute la chaine (`cassini serve`, canal, `plan.propose`, vrai modele) que le composeur recoit l'inventaire traduit du vrai projet ; la sonde de 4a a appele le geste puis `agent_inventory` directement.
  evidence: relecture quick de la tache 4a (critere 1 partiellement tenu).
  status: close, 2026-10-23, 5.10 tache 4b (`plan-5-10-t4b-mesures-et-demo.md`)
  resolution: demo de bout en bout : `cassini serve` lance par `client.py`, `plan.propose` demande `project.inventory` a l'extension ; l'inventaire brut recu, traduit, est identique octet pour octet a celui de la mesure CPU ; plan valide, execute, PDF ecrit.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t4b-mesures-et-demo.md`
  summary: Ticket de re-entrainement propose (5.10 reste ouvert) — familles a ajouter au generateur, jamais des regles de prompt : refus des demandes que le registre n'a pas encore (champ de vision, ligne de vue, pente, cadrage de la carte, PDF en A3, statistiques sur un raster), auxquelles le modele ajuste rend un plan de substitution ou un plan invalide (`plan_reel` 16/25, sous les 80 %) ; plans a plusieurs conditions qui perdent ou gagnent une etape (H08, H17 a H20, H22) ; « qui appartiennent a la commune » doit devenir une question (Q11) ; le tour de redemande (aucune phrase sauvee) ; un refus attendu (H40) rendu en plan.
  evidence: mesures GPU de la tache 4b (`docs/entrainement-du-modele.md`, « Tache 4 : mesures ») ; decision du proprietaire 2026-10-23 : sous la barre, chiffres consignes tels quels et re-entrainement propose.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t4b-mesures-et-demo.md`
  summary: La phrase de demo (A0) prend 41,4 s sur CPU 4 fils (fusionne ; 43,2 s base + adaptateur), au-dessus des 30 s : la lecture de l'invite est reglee (1 s par demande apres l'instantane), il reste l'ecriture des 248 jetons du plan a environ 6 jetons/s. Raccourcir le plan ecrit (identifiants de couche longs recopies a chaque etape, titre du PDF) au prochain entrainement, ou augmenter le debit d'ecriture.
  evidence: `duree_reel` de la tache 4b, binaire sans CUDA, masque 0x55, inventaire traduit du projet reel.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t4b-mesures-et-demo.md`
  summary: La phrase E du scenario (« a 10 minutes a pied de la gare de Villeneuve-le-Roi ») rend `agent.plan_invalid` deux fois avec le modele ajuste sur l'inventaire reel (aucune origine en coordonnees n'est donnee) ; elle devrait rendre une question. Famille a ajouter : isochrone sans coordonnees → question.
  evidence: `duree_reel` de la tache 4b, deux chemins.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t4b-mesures-et-demo.md`
  summary: `ai/tests/duree_reel.rs` et `ecart_reel.rs` : sortir l'agregation des moyennes (amorce exclue, A0 compte) et le compte des redemandes (phrases passees par la redemande, sauvees) dans des fonctions testees sans modele.
  evidence: relecture verification-gap de la tache 4b ; une inversion du filtre ou un decalage des lignes fausserait la mesure consignee sans faire echouer aucun test.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-selecteur-de-modele-dans-les-reglages-du-panneau-plan.md`
  summary: Reporter dans le contrat d'architecture (`bmad-architecture`) l'ecart assume a AD-15 : la liste des modeles du selecteur passe par la methode du canal `agent.models`, hors registre et hors journal (jamais `command.run model.list` cote extension), et `plan.propose` porte un `model` facultatif ; le noyau seul decouvre les modeles.
  evidence: decision du 2026-10-18, appliquee par 5.14 (`docs/canal-local.md`, « agent.models ») ; un fichier de contrat d'architecture ne se re-decide pas dans un build.
  status: close, 2026-10-23, mise a jour du contrat d'architecture (AD-23)
  resolution: l'ecart est deja ecrit dans AD-15 (« Ecart assume (2026-10-18) ») depuis la mise a jour E1 ; rien a changer au contrat.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-selecteur-de-modele-dans-les-reglages-du-panneau-plan.md`
  summary: Tester sans Qt le cablage du selecteur de modele de `panel.py` : sortir dans `models.py` la decision a la reception de MODELS (choix, oubli de `cassini/model`) et la regle « codes `models.unknown` / `models.unusable` qui redemandent la liste », chacune avec un test `unittest`.
  evidence: relecture verification-gap de 5.14 : aucun test ne charge `panel.py` (`FakePanel` dans `test_tracer.py`) ; remettre `propose(text)` dans `_send`, ou retirer `QSettings().remove(...)` d'`on_models`, ne fait echouer aucun test ; seule la recette manuelle du ticket 14 le verrait.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-re-entrainement-du-modele-ajuste-sur-les-familles-manquantes-plan.md`
  summary: 5.15, tache B — re-entrainer par la commande unique (8 000 exemples, meme base, meme methode) sur le generateur de la tache A, fusionner en GGUF, re-mesurer la parite des decoupeurs, mesurer les deux declarations sur GPU (plan_reel >= 20/25 et <= 1 fausse question, ecart_reel >= 32/40, question_reel >= 9/12, phrase E en question), rejouer la demo QGIS avec la phrase exacte, consigner dans docs/entrainement-du-modele.md, puis recette du proprietaire ; fusion dans main seulement apres B.
  evidence: decoupe par le proprietaire (2026-10-23, bmad-build 5.15) : plan entier a ~2 900 jetons ; la tache A (code, testable sans GPU) passe d'abord, sur la meme branche, car le prompt court change et le modele livre n'y est pas entraine.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-re-entrainement-du-modele-ajuste-sur-les-familles-manquantes-plan.md`
  summary: 5.15 tache B — etendre le test de parite sur vrai modele (`ai/src/plan.rs`, `CASSINI_TEST_MODEL`) aux exemples de redemande : `chat_tokens(retry_turns(...))` egal au decoupage d'un tenant de `prompt_text(e)`.
  evidence: non verifie (medium si vrai) : le test sur vrai modele ne construit que `first_turns` ; la parite de redemande n'est prouvee qu'a l'octet (relecture verification-gap et edge-case de 5.15 t.A).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-re-entrainement-du-modele-ajuste-sur-les-familles-manquantes-plan.md`
  summary: 5.15 tache B — verifier dans le rapport d'entrainement que `ecartes_trop_longs` vaut 0 avec les prompts de redemande et les plans a 6 etapes, sinon relever `MAX_LENGTH` (2048) apres mesure.
  evidence: non verifie (medium si vrai) : le commentaire « moins de 1 400 jetons » date de 5.10 ; une redemande ajoute la sortie rejetee (jusqu'a 1 500 caracteres) et le tour des ecarts ; `data.drop_too_long` ecarterait les exemples les plus longs.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-15-tache-b-reentrainement-et-mesures.md`
  summary: Nouvelle passe sur le generateur puis re-entrainement — isochrone dont l'origine est un lieu present comme couche de points (une gare de l'inventaire) sans le point choisi en coordonnees, qui doit rendre une question ; plans a plusieurs conditions sans etape perdue ; puis memes mesures que 5.15 et demo QGIS.
  evidence: 5.15 tache B (2026-10-23) : barre tenue sauf la phrase E du scenario, rendue en plan avec une origine inventee (EPSG:4326, 2,458 / 48,658) au lieu d'une question ; ecart_reel 32/40, dont trois plans a plusieurs conditions avec une etape de moins (formes dans docs/entrainement-du-modele.md, « Tache B de 5.15 »). Decision du proprietaire : consigner et s'arreter, la suite dans un autre ticket.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-15-tache-b-reentrainement-et-mesures.md`
  summary: Une origine d'isochrone inventee par le modele passe `judge` et `check_plan` sans alerte : decider (architecture, pas un pansement de prompt) si le noyau doit exiger que des coordonnees viennent de la demande ou d'une etape, et non du modele seul.
  evidence: phrase E de 5.15 tache B : `analysis.isochrone` avec `origin` {epsg 4326, x 2.458, y 48.658}, absent de la demande, accepte apres une redemande ; le resultat serait faux sans aucun signal a l'utilisateur.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-nouvelle-passe-sur-le-generateur-isochrone-depuis-un-lieu-de-plan.md`
  summary: 5.17, tache B — re-entrainer par la commande unique (8 000 exemples, meme base, meme methode) sur le generateur de la tache A, fusionner en GGUF, parite des decoupeurs, essais reels sur GPU (plan_reel >= 20/25 et <= 1 fausse question, ecart_reel >= 32/40, question_reel >= 9/12, phrase E en question du modele sans le garde-fou), demo QGIS avec la phrase exacte, chiffres et note sur la mesure aveugle dans docs/entrainement-du-modele.md, puis recette du proprietaire ; fusion dans main seulement apres B.
  evidence: decoupe par le proprietaire (2026-10-23, bmad-build 5.17) : plan entier a ~1 800 jetons ; la tache A (generateur, testable sans GPU) passe d'abord, sur la meme branche.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-nouvelle-passe-sur-le-generateur-isochrone-depuis-un-lieu-de-plan.md`
  summary: Isochrone demandee depuis plusieurs points donnes en coordonnees (« depuis les points A et B (Lambert-93) ») : aucun exemple de refus ; le modele pourrait faire un plan sur le premier point et oublier le second.
  evidence: non verifie (medium si vrai) : relecture blind-hunter de 5.17 t.A ; `isochrone_vers_cible` ne tire qu'un point, la decision a ne vise que des lieux nommes ; a juger sur les sorties du modele de la tache B.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-17-tache-b-reentrainement-et-mesures.md`
  summary: Phrase E encore manquee par le modele de 5.17 dans le produit : `cassini plan` (processeur et CUDA) et le panneau QGIS rendent la question du garde-fou, seuls `duree_reel` et `llama-completion` une question du modele ; nouvelle passe a ouvrir en ticket (decision du proprietaire, 2026-10-24).
  evidence: mesures de 5.17 tache B (docs/entrainement-du-modele.md, « La phrase E ») ; le modele hesite entre question et point invente selon le calcul du prompt.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-17-tache-b-reentrainement-et-mesures.md`
  summary: Isochrone depuis plusieurs points en coordonnees, constat observe en 5.17 tache B : le modele ecrit deux origines dans un meme objet, sortie rejetee, question du garde-fou qui ne dit pas pourquoi ; jamais un plan faux.
  evidence: `cassini plan` et `llama-completion` sur « … des points 655200, 6849800 et 656100, 6850400 (Lambert-93) » ; suite de l'entree differee de la tache A.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-17-tache-b-reentrainement-et-mesures.md`
  summary: Plusieurs phrases enchainees dans une meme session saturent le contexte : les couches de resultat s'ajoutent a l'inventaire et, des la 4e phrase, `plan.propose` echoue en `ai.prompt_too_long` (N_CTX 8192) ; composition 68 s, 75 s puis 108 s avant l'echec.
  evidence: recette de 5.17 dans QGIS (2026-10-24), noyau et modele installes, projet de Villeneuve : phrases 1 a 3 justes, phrases 4 a 10 toutes en `ai.prompt_too_long` en 0,4 s ; les memes phrases passent une a une sur projet vierge. Prolonge l'entree existante sur N_CTX.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-17-tache-b-reentrainement-et-mesures.md`
  summary: Reference a un resultat precedent (« Trouve les parcelles dans ces zones ... » apres « Garde les zones U du PLU. ») : le modele prend la couche source entiere au lieu de la couche de resultat ; famille d'entrainement a ajouter, jamais une regle de prompt.
  evidence: recette de 5.17 dans QGIS : intersection des parcelles avec `plu_zones` entiere (2 513) au lieu de la selection des zones U (2 429).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-plusieurs-phrases-enchainees-dans-une-session-sans-saturer-l-plan.md`
  summary: Les couches de resultat retirees de la vue du modele par le budget d'inventaire (5.18) ne sont signalees ni au modele ni a l'utilisateur ; une phrase qui en cite une (« la zone tampon de tout a l'heure ») finit en question ou en plan sans explication.
  evidence: relecture de 5.18 (blind-hunter) ; ni `plan.propose` ni le panneau ne portent de compte des couches retirees.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-plusieurs-phrases-enchainees-dans-une-session-sans-saturer-l-plan.md`
  summary: Non verifie (medium s'il est vrai) : la date d'un fichier de resultat GeoPackage pourrait changer quand QGIS l'ouvre ou le ferme (journal WAL), et fausser l'ordre « plus recents d'abord » du budget d'inventaire.
  evidence: relecture de 5.18 (edge-case-hunter). A trancher : comparer la date d'un `.gpkg` du dossier de travail avant et apres ouverture et fermeture dans QGIS.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-plusieurs-phrases-enchainees-dans-une-session-sans-saturer-l-plan.md`
  summary: Justesse du modele sur un long inventaire de session : en 9e position d'une recette enchainee, la phrase de demo exacte rend une question (« Je ne vois pas de champ de type de zone dans la couche plu_zones ») alors qu'elle rend un plan juste en 1re position ; famille d'entrainement a ajouter (inventaires avec des couches de resultat d'une session, pres du budget de 14 000 caracteres), jamais une regle de prompt.
  evidence: seconde recette enchainee de 5.18 (2026-10-24), 19 couches dans le projet, `plu_zones` et `typezone` dans l'inventaire vu (couches de l'utilisateur jamais retirees) ; les inventaires d'entrainement ne portent que les couches citees et une a quatre de distraction (`training/src/inventories.rs`, `finish`).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-duree-par-phrase-sous-30-s-dans-une-session-qgis-sur-process-plan.md`
  summary: Avec les identifiants courts vus par le modele (5.19), H11 (isochrone depuis un point par la voirie) rend un refus au lieu du plan : `ecart_reel` 32/40 contre 33/40 ; re-entrainement de la famille isochrone depuis un point, avec identifiants courts, jamais une regle de prompt.
  evidence: mesure GPU de 5.19 (2026-10-29) ; identifiants complets 33/40, courts 32/40 ; decision du proprietaire : garder les courts et re-entrainer dans un ticket suivant.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-duree-par-phrase-sous-30-s-dans-une-session-qgis-sur-process-plan.md`
  summary: La phrase qui suit un plan a plusieurs resultats lit une fois toutes ses couches de resultat (4 couches, 3 436 jetons, ~62 s sur CPU 4 fils), au-dessus des 30 s ; aucun reglage du budget n'y suffit : ne montrer au modele que la couche finale de chaque plan, ou alleger les champs des couches d'intersection.
  evidence: recette enchainee de 5.19 (2026-10-29), phrase 2 apres le plan de demo a 69,6 s sans aucun retrait ; decision du proprietaire : ticket suivant.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-duree-par-phrase-sous-30-s-dans-une-session-qgis-sur-process-plan.md`
  summary: La phrase de demo rejouee en cours de session (phrase 9 de la recette enchainee) prend 44,6 s sur CPU 4 fils, dont 27 s d'ecriture de son plan a cinq etapes (165 jetons) : au-dessus des 30 s meme avec les identifiants courts ; debit d'ecriture a augmenter (5.21, carte graphique) ou plan ecrit encore plus court.
  evidence: recette enchainee finale de 5.19 (2026-10-29) ; lecture 17 s apres reprise, ecriture 27 s.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-duree-par-phrase-sous-30-s-dans-une-session-qgis-sur-process-plan.md`
  summary: Une couche de resultat de selection garde tous les champs de sa couche source (zones U du PLU, ~1 100 jetons) : la phrase qui suit la lit une fois et depasse 30 s (36,6 s, phrase 3) ; meme levier que la phrase apres un plan a plusieurs resultats (champs alleges dans la vue du modele).
  evidence: recette enchainee finale de 5.19 (2026-10-29).

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-phrase-qui-suit-un-plan-a-plusieurs-resultats-sous-30-s-sur-plan.md`
  summary: Juger la justesse du modèle sur une vue allégée (intermédiaires cachés, résultats sans valeurs), dans une session enchaînée, et pas seulement sur les inventaires fixes de plan_reel, question_reel et ecart_reel.
  evidence: Les trois essais GPU de 5.22 n'ont aucune couche de résultat, les leviers ne s'y appliquent pas ; la recette CPU montre la phrase 3 reliée au PLU entier au lieu de la sélection.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-phrase-qui-suit-un-plan-a-plusieurs-resultats-sous-30-s-sur-plan.md`
  summary: Retrouver les couches intermédiaires après une relance du noyau ou un journal.replay (par exemple depuis le journal), pour qu'elles ne réapparaissent pas toute la session.
  evidence: State.intermediates est en mémoire et n'est alimenté que par plan.execute ; après relance ou rejeu, les intermédiaires sont vus et peuvent ramener le retrait par paquets de 5.19.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-phrase-qui-suit-un-plan-a-plusieurs-resultats-sous-30-s-sur-plan.md`
  summary: Famille d'entraînement (5.10) « couche citée introuvable → question sur la couche », pour une phrase qui cite une couche cachée.
  evidence: Recette 5.22, phrase 11 « Montre le tampon de 500 m. » : question hors de propos sur des coordonnées X/Y.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-nombre-de-fils-par-defaut-coeurs-physiques-et-masque-d-affin-plan.md`
  summary: Rendre la vérification complète (cargo test, clippy, fmt, deny) possible sous Linux/WSL, en fournissant libclang à llama-cpp-sys.
  evidence: Relecture 5.20, constat 1 : l'espace de travail ne se construit pas sous WSL faute de libclang ; seuls des morceaux isolés y sont testés, les tickets 5.19 à 5.22 ne vérifient que sous PowerShell.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-acceleration-par-carte-graphique-dans-le-noyau-livre-avec-re-plan.md`
  summary: Vérifier automatiquement la construction `gpu` (build.rs, load_backends, bloc gpu de `hors_ligne`) : job CI avec le SDK Vulkan, ou étape obligatoire de la liste de publication.
  evidence: Relecture 5.21, constat 5 : la CI ne compile jamais `--features gpu` ; une DLL réseau ajoutée au paquet ou un moteur processeur manquant ne ferait rien échouer.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-acceleration-par-carte-graphique-dans-le-noyau-livre-avec-re-plan.md`
  summary: Prouver sur un poste à carte intégrée seule que rien ne part sur cette carte (liste vide de périphériques bien transmise à llama.cpp).
  evidence: Relecture 5.21, constat 3 : le test `processor_parameters_offload_nothing` ne distingue pas liste vide et liste absente ; aucun poste à carte intégrée n'a été essayé.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-acceleration-par-carte-graphique-dans-le-noyau-livre-avec-re-plan.md`
  summary: Repli sur processeur quand la carte échoue au décodage ou à l'adaptateur (non vérifié ; medium si réel).
  evidence: Relecture 5.21, constat 6 : le repli ne couvre que le chargement et la création du contexte ; à trancher en lançant une longue session sur une carte de 4 Go.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-serveur-mcp-cassini-mcp-expose-le-registre-a-un-client-tiers-plan.md`
  summary: Vérifier que les clients MCP visés acceptent des noms d'outils avec un point (`system.about`), sinon les exposer sous une forme admise.
  evidence: non vérifié (maybe-false, medium si vrai) ; la spec MCP 2025-11-25 admet le point, certaines API de modèles n'admettent que [A-Za-z0-9_-] ; tranché par la recette hitl de 5.7.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-serveur-mcp-cassini-mcp-expose-le-registre-a-un-client-tiers-plan.md`
  summary: Tester la négociation des révisions MCP 2024-11-05 et 2026-07-28 (sans initialize) annoncées par l'architecture.
  evidence: le client de test ne négocie que 2025-11-25 ; rmcp épinglé =3.8.0, ne régresse qu'à une montée de version.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-serveur-mcp-cassini-mcp-expose-le-registre-a-un-client-tiers-plan.md`
  summary: Servir les appels MCP sans bloquer le runtime (ping et annulation pendant une commande longue) avant d'exposer les commandes de terrain.
  evidence: call_tool attend la réponse du fil d'exécution sur l'unique fil du runtime ; sans effet sur system.about et model.list.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-canal-d-ia-externe-cote-session-le-noyau-de-qgis-accepte-un-plan.md`
  summary: Exécuter les appels d'un relais sur le canal d'IA externe (AD-29) — command.run et request.cancel externes, état d'occupation unique dans les deux sens (session.busy), external.call et external.approve (120 s, external.not_approved), refus des chemins (external.path_refused), origine external au journal et rejeu, external.session_lost pendant un appel ou à la déconnexion du relais.
  evidence: 5.24 coupé en deux à la demande du propriétaire le 2026-10-30 (plan d'environ 2 700 jetons, CLAUDE.md §8 « une tâche courte par session ») ; 5.24 garde le transport, le cycle de vie, external.attach et le protocole 2. Décision du propriétaire déjà prise pour cette suite : un plan proposé non tranché rend la session occupée pour le relais seulement, l'extension garde le droit de le remplacer par plan.propose.
  status: ticket, 2026-10-30, entrée 5.28 (`epic-agent-et-mcp/tickets.toml`)
  resolution: Devenu le ticket 5.28 « Execution des appels du relais » ; 5.25 et 5.26 l'attendent.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-execution-des-appels-du-relais-la-session-qgis-execute-ce-qu-plan.md`
  summary: Validation des appels externes (AD-29, case « Me demander avant chaque appel ») — avec ask allumé, external.approve {call_id, command, params} vers l'extension avant d'exécuter, sans bloquer (AD-4), borné à 120 s et annulable ; refus, erreur ou délai rendent external.not_approved, rien n'est journalisé ; délai réglable par les tests.
  evidence: 5.28 coupé à la demande du propriétaire le 2026-10-30 (plan d'environ 2 600 jetons, cible 1 600) ; 5.28 garde l'exécution, l'occupation, external.call, les chemins, l'annulation, la perte de session et le journal. D'ici là, ask allumé refuse tout appel externe par external.not_approved. À créer en ticket avant 5.26, qui allume la case.
