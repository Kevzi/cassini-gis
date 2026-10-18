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
