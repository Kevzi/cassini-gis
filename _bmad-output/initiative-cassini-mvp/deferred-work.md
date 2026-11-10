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
  status: ticket, 2026-11-03, entrée 5.35 (`epic-agent-et-mcp/tickets.toml`)
  resolution: Confirmé par la recette QGIS de 5.9 (deux rejeux de suite) ; devenu le ticket 5.35 « Deux rejeux de suite dans la même session QGIS ».

- source_plan: `epic-facade-qgis/epic-facade-qgis-retrospective.md`
  summary: Le rejeu est aveugle à la forme d'une sortie QGIS : seuls `feature_count` et « le GeoPackage existe et n'est pas vide » sont contrôlés.
  evidence: F7 de la rétro de l'epic 9, vérifié par lecture. L'isochrone rend toujours `feature_count: 1` ; rejouée à pied ou en voiture, ou avec un polygone tout autre, elle serait « identique ». Écart assumé de l'architecture (écrit au commit `7829f94`), pas une faute du 9.11, mais l'isochrone est le verbe où cela compte le plus. Remède possible : une empreinte de géométrie ou une emprise (boîte englobante arrondie) dans le résultat typé, comparée au rejeu. Question de politique avant d'être technique (question ouverte 3 de la rétro) : le résultat typé suffit-il au MVP ?

- source_plan: `epic-facade-qgis/epic-facade-qgis-retrospective.md`
  summary: Le schéma de `Duration.value` de `analysis.isochrone` s'affiche `range(min = 0.0)`, inclusif, alors que la durée est strictement positive.
  evidence: F8 de la rétro de l'epic 9. Le noyau refuse 0 à la désérialisation (`positive`), mais le catalogue montrerait au modèle une borne inférieure atteignable. Écarté au triage du 9.11 (même motif que `analysis.buffer`). À reprendre si schemars gagne un minimum exclusif utilisable, ou en précisant « strictement positive » dans le schéma des deux commandes.

- source_plan: `epic-registre-projet-import/epic-registre-projet-import-retrospective.md`
  summary: Une montée de version du noyau casse sans le dire tout rejeu contenant `system.about` ; `versions` du journal n'est jamais relu au rejeu.
  evidence: F12 de la rétro de l'epic 2 (action R8, non reportée à la rétro de l'epic 9). `core_version` figure dans le résultat de `system.about`, qui est journalisé à chaque démarrage du client ; au rejeu après une montée de version, la comparaison diverge sur cette étape. Remède possible : ne pas comparer `core_version` (masque, comme `layer_id`), ou lire `versions` et le dire. Annotation 5.32 (2026-10-31) : les journaux ouverts par l'extension 5.32 n'ont plus de `system.about` d'accueil, l'extension lisant les versions dans la réponse de `session.hello`. Restent exposés : les anciens journaux, qui portent cette ligne dès l'étape 1 et divergeront à la première montée de version ; un `system.about` lancé à la main (mode expert, CLI, MCP), journalisé comme toute commande ; une extension antérieure à 5.32 couplée à un noyau neuf, qui lance encore ce `system.about` au démarrage.

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
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté, décision d'architecture : refuser un identifiant inconnu de la session relève de `judge` ou de `verify` (AD-23) ; à trancher par le propriétaire avant de coder.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-composer-un-plan-prompt-contraint-validation-sans-reparation-plan.md`
  summary: La redemande doit porter la liste des ecarts, pas seulement la premiere erreur de check_plan.
  evidence: check_plan s'arrete a la premiere erreur ; un plan a plusieurs fautes demande plus d'une redemande.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté à la prochaine passe sur le générateur et au ré-entraînement (lignée 5.10, 5.15, 5.17), hors de 5.8 : défaut du modèle, à corriger par des exemples d'entraînement, jamais par une règle de prompt.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-composer-un-plan-prompt-contraint-validation-sans-reparation-plan.md`
  summary: Verifier la taille du prompt (catalogue + inventaire) contre N_CTX 8192 et corriger le message ai.prompt_too_long.
  evidence: aucun plafond sur l'inventaire ; le message accuse la phrase alors que le catalogue peut etre en cause.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par 5.18 (76e0f7b) : budget d'inventaire contre N_CTX, plusieurs phrases enchaînées sans saturer le contexte.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-composer-un-plan-prompt-contraint-validation-sans-reparation-plan.md`
  summary: Eviter de recharger le GGUF a chaque tentative de composition.
  evidence: generate charge le modele et cree un contexte a chaque appel ; une redemande double le cout.
  status: close, 2026-10-18, entrée 12 (`epic-agent-et-mcp/story-modele-charge-une-fois-et-partie-fixe-du-prompt-gardee-en-ca-plan.md`)
  resolution: Le modèle est gardé dans le processus du noyau (clé : chemin, taille, date du fichier) et l'état du moteur après la partie fixe de l'invite et après le tour système est gardé en instantanés ; une redemande reprend l'instantané du tour système et ne relit que la sortie rejetée et les écarts. Seul le contexte est recréé à chaque appel.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-composer-un-plan-prompt-contraint-validation-sans-reparation-plan.md`
  summary: Tester le chemin de cassini plan sans --model (models.no_folder, ai.no_model).
  evidence: les tests CLI passent toujours --model absent.gguf.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Corrigé en 5.8 : `cassini plan` sans `--model` passe par `ModelComposer::choose(None)`, la règle du canal (`default_model` supprimé) ; tests `plan_without_a_model_follows_the_channel_rule` (`cli/src/main.rs`) et `plan_without_a_model_chooses_like_the_channel_and_hides_the_folder` (`cli/tests/end_to_end.rs`) : dossier absent (`models.no_folder`), vide (`ai.no_model`), premier modèle inutilisable, sans le chemin du dossier.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-composer-un-plan-prompt-contraint-validation-sans-reparation-plan.md`
  summary: Le critere des 80 % est tenu en comptant les refus attendus : 10/13 plans reels (77 %) ; echecs S16, S17, S19.
  evidence: mesure GPU 22/25 sur Qwen3.5-9B Q4_K_M, deux fois ; 12 phrases attendent un refus faute de commande dans le registre reel. Mesure CPU non confirmee.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par les mesures de 5.15 et 5.17 (tâches B) : `plan_reel` 25/25 avec le modèle ajusté (`docs/entrainement-du-modele.md`), au-dessus du critère des 80 %.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-executer-un-plan-valide-etape-par-etape-par-le-canal-plan.md`
  summary: Rendre la composition d'un plan (`plan.propose`) annulable ; aujourd'hui elle bloque la boucle du noyau jusqu'a sa fin.
  evidence: le fil lecteur continue, mais les requetes recues pendant l'inference attendent la fin ; `request.cancel` sur l'id de `plan.propose` repond `request.unknown`. Decision prise sans redemander (Design Notes du plan).
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté, décision d'architecture : rendre `plan.propose` annulable change le trait `Composer` ; à trancher par le propriétaire.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-executer-un-plan-valide-etape-par-etape-par-le-canal-plan.md`
  summary: `command.run` rend encore les chemins absolus d'une sortie QGIS (ex. `file` de `layout.export_pdf`) ; `plan.execute` les rend relatifs au dossier de travail.
  evidence: AD-9 interdit tout chemin absolu dans un message rendu ; seule la reponse de `plan.execute` est relativisee dans ce ticket.
  status: decision, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Décision 5.8 : les chemins absolus de `command.run` restent voulus, `runlog.result_folder` en a besoin pour « Ouvrir le dossier » ; l'extension les masque à l'affichage par `runlog.redact` (AD-9 tenu dans ce qui est montré).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-executer-un-plan-valide-etape-par-etape-par-le-canal-plan.md`
  summary: Verifier que le masquage des chemins de `ModelComposer` couvre un message de llama.cpp qui ecrit le chemin du modele sous une autre forme (canonique, separateurs, guillemets).
  evidence: maybe-false, medium si vrai (AD-9). Le masquage remplace les chaines exactes ; le test au faux `x.gguf` passe, mais un vrai GGUF corrompu sous Windows n'a pas ete essaye. A regler par cet essai.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Corrigé en 5.8 : `hide` (`cli/src/main.rs`) reconnaît chaque chemin masqué écrit avec `\` ou `/` et sous la forme longue `\\?\` ; test `hide_recognises_a_path_written_with_other_separators_or_the_long_prefix`. Les guillemets n'entrent pas dans le chemin et ne gênent pas le remplacement. L'essai sur un vrai GGUF corrompu sous Windows reste à faire à la main ; le masquage couvre désormais les écritures qu'il produirait.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-clarification-poser-une-question-au-lieu-de-deviner-plan.md`
  summary: Le modele ecrit parfois une question dans un refus (`reason` en forme de question) ; rien ne le detecte, l'utilisateur recoit `agent.refused` formule comme une question.
  evidence: Essais reels de la 5.5 (essai 3 : Q02, Q03, Q11 ; essai 2 : S21). Comportement du modele, a corriger par les exemples d'entrainement du 5.10 plutot que par une regle de reparation (AD-23).
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par le générateur (7addf14, 5.10 tâche 1, puis 4484464, 5.15 tâche A) : exemples d'entraînement de questions et de refus distincts ; plus de règle de réparation.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-modele-charge-une-fois-et-partie-fixe-du-prompt-gardee-en-ca-plan.md`
  summary: Precharger le modele et lire la partie fixe de l'invite au demarrage de cassini serve, pour que la premiere demande d'une session QGIS ne paie pas 173 s.
  evidence: mesure 12, S01 a 173,4 s sur 4 coeurs (chargement 4,1 s, lecture 163,7 s) ; le budget de 30 s ne compte que des la 2e demande, le premier appel reste visible a l'utilisateur.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté : le préchargement a été écarté par 5.19 (instantané du tour système à la première demande) ; à reprendre seulement sur décision du propriétaire.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-modele-charge-une-fois-et-partie-fixe-du-prompt-gardee-en-ca-plan.md`
  summary: Prouver avec le vrai modele que cassini serve garde le modele et l'instantane du tour systeme entre deux plan.propose avec l'inventaire envoye par QGIS.
  evidence: le cache n'est verifie que par appels directs a generate_measured (cache_reel) ; le chemin serve est raisonne, pas teste. A porter par la suite de bout en bout (entree 9).
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par 5.19 (c93836b) : recette enchaînée dans QGIS avec le vrai modèle, reprise de l'instantané mesurée sur le chemin `serve` (`CASSINI_TIMINGS`, jetons repris).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-panneau-saisie-en-langage-naturel-plan-a-valider-selecteur-d-plan.md`
  summary: Selecteur de modele dans les Reglages du panneau : methode du canal `agent.models` (liste sans dossier ni chemin, non journalisee, masquee par le compositeur de `cli`), `model` optionnel sur `plan.propose` (nom de fichier de la liste, absent = premier), erreur `models.unknown`, choix memorise dans `QSettings` `cassini/model`, rechargement par la cle de modele de l'entree 12.
  status: close — porte par l'entree 14 de l'epic 5 (2026-10-18)
  evidence: decoupe par le proprietaire (2026-10-18) : plan 5.6 a ~2 400 jetons ; le selecteur touche le noyau et le protocole, le parcours du plan non. `command.run model.list` ecarte : il journalise (rejeu qui diverge des qu'un fichier est ajoute) et rend le dossier absolu.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-panneau-saisie-en-langage-naturel-plan-a-valider-selecteur-d-plan.md`
  summary: Le modele refuse « Garde seulement les parcelles de la section AB. » en lisant « garde seulement » comme une suppression d'entites, alors que analysis.select_by_attribute cree une nouvelle couche ; a corriger par les exemples d'entrainement du 5.10 (tournures « garde seulement / ne garde que / conserve »).
  evidence: recette du proprietaire dans QGIS, 2026-10-18, projet-test-villeneuve, Qwen3.5-9B Q4_K_M sur CPU : `agent.refused` « Aucune commande du catalogue ne permet de modifier ou de supprimer des entites d'une couche existante (comme les parcelles de la section AB) ; toutes les commandes d'analyse creent de nouvelles couches sans alterer les sources. » ; la meme tournure sur « les zones du PLU de type U » a donne un plan correct. Aucune regle de reparation (AD-23).
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par le générateur (7addf14 et 4484464) : tournures « garde seulement / ne garde que / conserve » apprises vers `analysis.select_by_attribute`.
- source_plan: `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`
  summary: Bug latent du rejeu : `core/src/journal/summary.rs:24` prend la cle `truncated: true` de premier niveau pour le marqueur de resume ; un resultat de moins de 200 octets qui porte lui-meme `truncated: true` est juge identique a tout resultat garde ainsi marque. Placer les marqueurs sous une cle reservee (AD-11 resserree).
  evidence: revue adverse E1 (reviews/review-adverse-e1.md), verifie a la lecture du code le 2026-10-18 ; inatteignable aujourd'hui (un comptage tronque depasse 200 octets), atteignable par les resumes de l'epic 10.
  status: ticket, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Repris par l'entrée 10.1 (`epic-catalogue-etendu-qgis/tickets.toml`) : les marqueurs du journal passent sous une clé réservée.
- source_plan: `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`
  summary: `model.list` rend `folder` en chemin absolu, journalise par `cassini run` et visible par MCP ; ne rendre aucun chemin absolu (AD-9, AD-15 amendee).
  evidence: revues E1 (faits verifies, grille B2), 2026-10-18.
  status: ticket, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Repris par l'entrée 10.1 (`epic-catalogue-etendu-qgis/tickets.toml`) : `model.list` cesse de rendre son dossier en chemin absolu ; toujours vrai au 2026-10-31 (`core/src/commands/model_list.rs`, `folder`).
- source_plan: `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`
  summary: Renommer `analysis.zonal_statistics` en `analysis.count_points_in_polygon` et migrer son resultat au gabarit unique de resume (AD-27) : module Rust, table ALGORITHMS, test_analysis.py, sonde, README, prompt et jeu de l'epreuve ; les phrases S15, S23 et S24 visent en fait `raster.zonal_statistics`, non mecaniques.
  evidence: decision du proprietaire 2026-10-18 (contrat d'architecture, convention Familles de commandes) ; inventaire des occurrences dans reviews/review-faits-verifies-e1.md.
  status: ticket, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Repris par l'entrée 10.2 (`epic-catalogue-etendu-qgis/tickets.toml`), renommage et gabarit de résumé.
- source_plan: `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`
  summary: Mettre le code au niveau d'AD-24, AD-25, AD-27 et AD-28 avant la premiere primitive de l'epic 10 : validation par le noyau du resultat recu (`qgis.bad_result`) ; annulabilite declaree seulement si chaque algorithme porte CanCancel, avec test table/registre ; version de chaine par ligne et version de QGIS annoncees a `session.hello` et journalisees ; annonce apres le chargement des fournisseurs ; constantes de `analysis.isochrone` (vitesses, accrochage 100 m) passees en parametres du contrat de reseau.
  evidence: gate de revue E1 du 2026-10-18 (reviews/review-grille-e1.md H1-H4, M2, M4 ; review-adverse-e1.md) ; aujourd'hui toutes les commandes deleguees sont `cancellable(true)` et le noyau ne valide pas les resultats.
  status: ticket, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Repris par les entrées 10.1 (validation du résultat), 10.3 (annulabilité, version de chaîne), 10.4 (annonce après les fournisseurs, versions journalisées) et 10.6 (contrat de réseau) de `epic-catalogue-etendu-qgis/tickets.toml`.
- source_plan: none
  summary: 5.10, tache 2 — entrainement reproductible : une commande lance un LoRA ou QLoRA tenant dans 8 Go de VRAM sur un modele de base Apache-2.0 ou MIT de 1,5 a 4 milliards de parametres, sur le jeu engendre, et fusionne en GGUF.
  evidence: decoupe du ticket 5.10 par le proprietaire (2026-10-22, bmad-build) : quatre livrables fusionnables a part ; celui-ci attend le jeu de donnees (tache 1) et des dependances Python dont la licence est a valider.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Fait par `plan-5-10-t2-entrainement-reproductible.md` (built) : commande unique d'entraînement dans `training/`.
- source_plan: none
  summary: 5.10, tache 3 — branchement du modele ajuste dans `ai/` : prompt court de quelques centaines de jetons, chargement d'une base avec un adaptateur LoRA, selon le selecteur de 5.14.
  evidence: decoupe du ticket 5.10 par le proprietaire (2026-10-22, bmad-build) ; independant de l'entrainement cote code, utile seulement une fois un modele ajuste disponible.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Fait par `plan-5-10-t3-modele-ajuste-dans-ai.md` (built) : base et adaptateur chargés par `ai/`, prompt court.
- source_plan: none
  summary: 5.10, tache 4 — mesures et demo : score du modele ajuste sur le jeu tenu a l'ecart au moins egal au 9B, seuil strict de clarification (>= 9/12, <= 1 fausse question), < 30 s sur le 5800X bride a 4 coeurs, demo de bout en bout dans QGIS avec la phrase de demo exacte.
  evidence: decoupe du ticket 5.10 par le proprietaire (2026-10-22, bmad-build) ; ferme le ticket, attend les taches 1 a 3.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Fait par `plan-5-10-t4a-inventaire-au-format-appris.md` et `plan-5-10-t4b-mesures-et-demo.md` (built) ; les écarts restants (durée sur CPU, phrase E) sont des entrées à part, triées par 5.8.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-modele-de-composition-ajuste-et-plus-petit-fine-tuning-plan.md`
  summary: Avant l'entrainement (5.10 tache 2) : l'inventaire envoye par l'extension (`qgis-plugin/cassini_qgis/inventory.py` l.211) porte le nom et le type des champs mais pas leurs valeurs, alors que le jeu engendre les porte ; trancher : ajouter les valeurs distinctes a l'inventaire du produit, ou engendrer aussi des inventaires sans valeurs.
  evidence: constat de la tache 1 (2026-10-22) ; un modele entraine sur des valeurs qu'il ne verra pas en production apprendrait un appui absent (ex. typezone U).
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par 5.10 tâche 4a (6cdda0f) : l'inventaire du produit porte les valeurs des champs, au format appris par le modèle (`core/src/agent_inventory.rs`).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-modele-de-composition-ajuste-et-plus-petit-fine-tuning-plan.md`
  summary: Le generateur ne met jamais deux couches quasi synonymes cote a cote (« Routes » et « Reseau de voirie ») alors que le jeu tenu a l'ecart et les vrais projets le font ; ajouter une famille ou la phrase tranche, ou ou la sortie attendue est une question.
  evidence: relecture blind-hunter de la tache 1 ; garde volontaire dans `training/src/inventories.rs` (groupes exclusifs).
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté à la prochaine passe sur le générateur et au ré-entraînement (lignée 5.10, 5.15, 5.17), hors de 5.8 : défaut du modèle, à corriger par des exemples d'entraînement, jamais par une règle de prompt.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-modele-de-composition-ajuste-et-plus-petit-fine-tuning-plan.md`
  summary: `ai/tests/ecart_reel.rs` : sortir la decision par phrase dans une fonction testee sans modele (refus en Err(Refused), titre seul different, titre absent, question attendue mais plan rendu), et imprimer pour chaque echec une forme de l'ecart (commande, ordre, valeur) sans le texte de la phrase.
  evidence: relecture verification-gap et blind-hunter ; a fermer quand la tache 4 reutilise le notateur pour le modele ajuste.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos : `grade()` de `ai/tests/ecart_reel.rs` porte la décision par phrase et l'écart imprimé sans le texte, avec sa matrice de tests sans modèle (5.10 tâche 4).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-modele-de-composition-ajuste-et-plus-petit-fine-tuning-plan.md`
  summary: La commande `cassini-training` n'a aucun test d'integration (deux executions identiques, autre graine differente, code non nul sans --out).
  evidence: relecture verification-gap ; verifie a la main par cmp ; a ajouter avant que la tache 2 depende du fichier.
  status: close, 2026-10-22, 5.10 tache 2 (`plan-5-10-t2-entrainement-reproductible.md`)
  resolution: `training/tests/commande.rs` lance le binaire : meme graine, memes octets (formats structured et chat) ; autre graine, autre fichier ; --out absent, --format inconnu ou graine illisible, code non nul.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-modele-de-composition-ajuste-et-plus-petit-fine-tuning-plan.md`
  summary: CLAUDE.md §9 (structure du depot) ne cite pas le nouveau crate `/training` (chaine d'entrainement) ; a ajouter par le proprietaire.
  evidence: relecture quick ; un fichier de contexte d'agent ne se modifie pas dans un build.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté au propriétaire : `CLAUDE.md` ne se modifie pas dans un build ; ajouter `/training` à la structure du dépôt (§9).
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
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par le ré-entraînement de 5.15 tâche A (4484464) et B : le `SYSTEM` du prompt court a été réaligné sur `ai/src/plan.rs` au même passage.
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
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par le générateur (4484464, 5.15 tâche A) : valeurs d'un champ triées comme `core/src/agent_inventory.rs`.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t4a-inventaire-au-format-appris.md`
  summary: Formes d'inventaire jamais vues a l'entrainement : couche sans fichier (sorties temporaires des etapes precedentes, sans cle `source`), projet non enregistre (sans `project.name`), SCR non EPSG (sans `epsg`), table sans geometrie (`geometry: null` sur un vecteur), `unloaded_files` toujours vide dans QGIS (68 % des exemples en ont) ; mesurer en 4b, les apprendre au prochain entrainement.
  evidence: relectures edge-case-hunter, blind-hunter et quick de la tache 4a ; `forme_inventaire.rs` ne couvre que le cas nominal.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par le générateur (4484464, 5.15 tâche A) : formes d'inventaire du produit (couche sans source, projet non enregistré, SCR non EPSG, table sans géométrie) engendrées.
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
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par 5.15 tâche A (4484464, familles de refus ajoutées au générateur) puis 5.15 et 5.17 tâches B (ré-entraînement, `plan_reel` 25/25).
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t4b-mesures-et-demo.md`
  summary: La phrase de demo (A0) prend 41,4 s sur CPU 4 fils (fusionne ; 43,2 s base + adaptateur), au-dessus des 30 s : la lecture de l'invite est reglee (1 s par demande apres l'instantane), il reste l'ecriture des 248 jetons du plan a environ 6 jetons/s. Raccourcir le plan ecrit (identifiants de couche longs recopies a chaque etape, titre du PDF) au prochain entrainement, ou augmenter le debit d'ecriture.
  evidence: `duree_reel` de la tache 4b, binaire sans CUDA, masque 0x55, inventaire traduit du projet reel.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté à la prochaine passe sur le générateur et au ré-entraînement (lignée 5.10, 5.15, 5.17), hors de 5.8 : défaut du modèle, à corriger par des exemples d'entraînement, jamais par une règle de prompt. Durée : plan écrit plus court ou carte graphique (5.21).
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t4b-mesures-et-demo.md`
  summary: La phrase E du scenario (« a 10 minutes a pied de la gare de Villeneuve-le-Roi ») rend `agent.plan_invalid` deux fois avec le modele ajuste sur l'inventaire reel (aucune origine en coordonnees n'est donnee) ; elle devrait rendre une question. Famille a ajouter : isochrone sans coordonnees → question.
  evidence: `duree_reel` de la tache 4b, deux chemins.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par 5.16 (cf044a5, 4ff027d) : le garde-fou sur la provenance des coordonnées rend une question sur la phrase E au lieu d'un plan invalide.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-10-t4b-mesures-et-demo.md`
  summary: `ai/tests/duree_reel.rs` et `ecart_reel.rs` : sortir l'agregation des moyennes (amorce exclue, A0 compte) et le compte des redemandes (phrases passees par la redemande, sauvees) dans des fonctions testees sans modele.
  evidence: relecture verification-gap de la tache 4b ; une inversion du filtre ou un decalage des lignes fausserait la mesure consignee sans faire echouer aucun test.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Corrigé en 5.8 : `averages(...)` (`ai/tests/duree_reel.rs`) et `tally(...)` (`ai/tests/ecart_reel.rs`), testés sans modèle (`averages_exclude_the_warmup_and_count_a0`, `averages_of_the_warmup_alone_are_zero`, `tally_counts_each_kind_and_the_retries_that_saved_a_sentence`).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-selecteur-de-modele-dans-les-reglages-du-panneau-plan.md`
  summary: Reporter dans le contrat d'architecture (`bmad-architecture`) l'ecart assume a AD-15 : la liste des modeles du selecteur passe par la methode du canal `agent.models`, hors registre et hors journal (jamais `command.run model.list` cote extension), et `plan.propose` porte un `model` facultatif ; le noyau seul decouvre les modeles.
  evidence: decision du 2026-10-18, appliquee par 5.14 (`docs/canal-local.md`, « agent.models ») ; un fichier de contrat d'architecture ne se re-decide pas dans un build.
  status: close, 2026-10-23, mise a jour du contrat d'architecture (AD-23)
  resolution: l'ecart est deja ecrit dans AD-15 (« Ecart assume (2026-10-18) ») depuis la mise a jour E1 ; rien a changer au contrat.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-selecteur-de-modele-dans-les-reglages-du-panneau-plan.md`
  summary: Tester sans Qt le cablage du selecteur de modele de `panel.py` : sortir dans `models.py` la decision a la reception de MODELS (choix, oubli de `cassini/model`) et la regle « codes `models.unknown` / `models.unusable` qui redemandent la liste », chacune avec un test `unittest`.
  evidence: relecture verification-gap de 5.14 : aucun test ne charge `panel.py` (`FakePanel` dans `test_tracer.py`) ; remettre `propose(text)` dans `_send`, ou retirer `QSettings().remove(...)` d'`on_models`, ne fait echouer aucun test ; seule la recette manuelle du ticket 14 le verrait.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Corrigé en 5.8 : `models.asks_list_again(code)` et `models.received(listed, remembered)` (entrées, choix, oubli) dans `qgis-plugin/cassini_qgis/models.py`, appelés par `panel.py` ; tests `ReceivedTest` de `qgis-plugin/tests/test_models.py`, sans Qt.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-re-entrainement-du-modele-ajuste-sur-les-familles-manquantes-plan.md`
  summary: 5.15, tache B — re-entrainer par la commande unique (8 000 exemples, meme base, meme methode) sur le generateur de la tache A, fusionner en GGUF, re-mesurer la parite des decoupeurs, mesurer les deux declarations sur GPU (plan_reel >= 20/25 et <= 1 fausse question, ecart_reel >= 32/40, question_reel >= 9/12, phrase E en question), rejouer la demo QGIS avec la phrase exacte, consigner dans docs/entrainement-du-modele.md, puis recette du proprietaire ; fusion dans main seulement apres B.
  evidence: decoupe par le proprietaire (2026-10-23, bmad-build 5.15) : plan entier a ~2 900 jetons ; la tache A (code, testable sans GPU) passe d'abord, sur la meme branche, car le prompt court change et le modele livre n'y est pas entraine.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté hors de 5.8 : tâche B de 5.15 menée sous sa propre entrée ; ses suites d'entraînement vont à la prochaine passe sur le générateur et au ré-entraînement.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-re-entrainement-du-modele-ajuste-sur-les-familles-manquantes-plan.md`
  summary: 5.15 tache B — etendre le test de parite sur vrai modele (`ai/src/plan.rs`, `CASSINI_TEST_MODEL`) aux exemples de redemande : `chat_tokens(retry_turns(...))` egal au decoupage d'un tenant de `prompt_text(e)`.
  evidence: non verifie (medium si vrai) : le test sur vrai modele ne construit que `first_turns` ; la parite de redemande n'est prouvee qu'a l'octet (relecture verification-gap et edge-case de 5.15 t.A).
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté à la prochaine passe sur le générateur et au ré-entraînement (lignée 5.10, 5.15, 5.17), hors de 5.8 : défaut du modèle, à corriger par des exemples d'entraînement, jamais par une règle de prompt. Parité de redemande sur vrai modèle à faire au prochain ré-entraînement.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-re-entrainement-du-modele-ajuste-sur-les-familles-manquantes-plan.md`
  summary: 5.15 tache B — verifier dans le rapport d'entrainement que `ecartes_trop_longs` vaut 0 avec les prompts de redemande et les plans a 6 etapes, sinon relever `MAX_LENGTH` (2048) apres mesure.
  evidence: non verifie (medium si vrai) : le commentaire « moins de 1 400 jetons » date de 5.10 ; une redemande ajoute la sortie rejetee (jusqu'a 1 500 caracteres) et le tour des ecarts ; `data.drop_too_long` ecarterait les exemples les plus longs.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par la mesure de 5.17 tâche B : `ecartes_trop_longs` = 0, `jetons_max` 1659 sous `MAX_LENGTH` 2048 (`plan-5-17-tache-b-reentrainement-et-mesures.md`).
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-15-tache-b-reentrainement-et-mesures.md`
  summary: Nouvelle passe sur le generateur puis re-entrainement — isochrone dont l'origine est un lieu present comme couche de points (une gare de l'inventaire) sans le point choisi en coordonnees, qui doit rendre une question ; plans a plusieurs conditions sans etape perdue ; puis memes mesures que 5.15 et demo QGIS.
  evidence: 5.15 tache B (2026-10-23) : barre tenue sauf la phrase E du scenario, rendue en plan avec une origine inventee (EPSG:4326, 2,458 / 48,658) au lieu d'une question ; ecart_reel 32/40, dont trois plans a plusieurs conditions avec une etape de moins (formes dans docs/entrainement-du-modele.md, « Tache B de 5.15 »). Decision du proprietaire : consigner et s'arreter, la suite dans un autre ticket.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté à la prochaine passe sur le générateur et au ré-entraînement (lignée 5.10, 5.15, 5.17), hors de 5.8 : défaut du modèle, à corriger par des exemples d'entraînement, jamais par une règle de prompt.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-15-tache-b-reentrainement-et-mesures.md`
  summary: Une origine d'isochrone inventee par le modele passe `judge` et `check_plan` sans alerte : decider (architecture, pas un pansement de prompt) si le noyau doit exiger que des coordonnees viennent de la demande ou d'une etape, et non du modele seul.
  evidence: phrase E de 5.15 tache B : `analysis.isochrone` avec `origin` {epsg 4326, x 2.458, y 48.658}, absent de la demande, accepte apres une redemande ; le resultat serait faux sans aucun signal a l'utilisateur.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par 5.16 (921a7c7, cf044a5) : AD-23 étendu, le noyau exige que des coordonnées viennent de la demande ou d'une étape (garde-fou).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-nouvelle-passe-sur-le-generateur-isochrone-depuis-un-lieu-de-plan.md`
  summary: 5.17, tache B — re-entrainer par la commande unique (8 000 exemples, meme base, meme methode) sur le generateur de la tache A, fusionner en GGUF, parite des decoupeurs, essais reels sur GPU (plan_reel >= 20/25 et <= 1 fausse question, ecart_reel >= 32/40, question_reel >= 9/12, phrase E en question du modele sans le garde-fou), demo QGIS avec la phrase exacte, chiffres et note sur la mesure aveugle dans docs/entrainement-du-modele.md, puis recette du proprietaire ; fusion dans main seulement apres B.
  evidence: decoupe par le proprietaire (2026-10-23, bmad-build 5.17) : plan entier a ~1 800 jetons ; la tache A (generateur, testable sans GPU) passe d'abord, sur la meme branche.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté hors de 5.8 : tâche B de 5.17 menée sous sa propre entrée ; les familles restantes vont à la prochaine passe sur le générateur.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-nouvelle-passe-sur-le-generateur-isochrone-depuis-un-lieu-de-plan.md`
  summary: Isochrone demandee depuis plusieurs points donnes en coordonnees (« depuis les points A et B (Lambert-93) ») : aucun exemple de refus ; le modele pourrait faire un plan sur le premier point et oublier le second.
  evidence: non verifie (medium si vrai) : relecture blind-hunter de 5.17 t.A ; `isochrone_vers_cible` ne tire qu'un point, la decision a ne vise que des lieux nommes ; a juger sur les sorties du modele de la tache B.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté à la prochaine passe sur le générateur et au ré-entraînement (lignée 5.10, 5.15, 5.17), hors de 5.8 : défaut du modèle, à corriger par des exemples d'entraînement, jamais par une règle de prompt.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-17-tache-b-reentrainement-et-mesures.md`
  summary: Phrase E encore manquee par le modele de 5.17 dans le produit : `cassini plan` (processeur et CUDA) et le panneau QGIS rendent la question du garde-fou, seuls `duree_reel` et `llama-completion` une question du modele ; nouvelle passe a ouvrir en ticket (decision du proprietaire, 2026-10-24).
  evidence: mesures de 5.17 tache B (docs/entrainement-du-modele.md, « La phrase E ») ; le modele hesite entre question et point invente selon le calcul du prompt.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté à la prochaine passe sur le générateur et au ré-entraînement (lignée 5.10, 5.15, 5.17), hors de 5.8 : défaut du modèle, à corriger par des exemples d'entraînement, jamais par une règle de prompt.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-17-tache-b-reentrainement-et-mesures.md`
  summary: Isochrone depuis plusieurs points en coordonnees, constat observe en 5.17 tache B : le modele ecrit deux origines dans un meme objet, sortie rejetee, question du garde-fou qui ne dit pas pourquoi ; jamais un plan faux.
  evidence: `cassini plan` et `llama-completion` sur « … des points 655200, 6849800 et 656100, 6850400 (Lambert-93) » ; suite de l'entree differee de la tache A.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté à la prochaine passe sur le générateur et au ré-entraînement (lignée 5.10, 5.15, 5.17), hors de 5.8 : défaut du modèle, à corriger par des exemples d'entraînement, jamais par une règle de prompt.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-17-tache-b-reentrainement-et-mesures.md`
  summary: Plusieurs phrases enchainees dans une meme session saturent le contexte : les couches de resultat s'ajoutent a l'inventaire et, des la 4e phrase, `plan.propose` echoue en `ai.prompt_too_long` (N_CTX 8192) ; composition 68 s, 75 s puis 108 s avant l'echec.
  evidence: recette de 5.17 dans QGIS (2026-10-24), noyau et modele installes, projet de Villeneuve : phrases 1 a 3 justes, phrases 4 a 10 toutes en `ai.prompt_too_long` en 0,4 s ; les memes phrases passent une a une sur projet vierge. Prolonge l'entree existante sur N_CTX.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par 5.18 (76e0f7b) : budget d'inventaire, plus de `ai.prompt_too_long` dans une recette enchaînée.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-5-17-tache-b-reentrainement-et-mesures.md`
  summary: Reference a un resultat precedent (« Trouve les parcelles dans ces zones ... » apres « Garde les zones U du PLU. ») : le modele prend la couche source entiere au lieu de la couche de resultat ; famille d'entrainement a ajouter, jamais une regle de prompt.
  evidence: recette de 5.17 dans QGIS : intersection des parcelles avec `plu_zones` entiere (2 513) au lieu de la selection des zones U (2 429).
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté à la prochaine passe sur le générateur et au ré-entraînement (lignée 5.10, 5.15, 5.17), hors de 5.8 : défaut du modèle, à corriger par des exemples d'entraînement, jamais par une règle de prompt.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-plusieurs-phrases-enchainees-dans-une-session-sans-saturer-l-plan.md`
  summary: Les couches de resultat retirees de la vue du modele par le budget d'inventaire (5.18) ne sont signalees ni au modele ni a l'utilisateur ; une phrase qui en cite une (« la zone tampon de tout a l'heure ») finit en question ou en plan sans explication.
  evidence: relecture de 5.18 (blind-hunter) ; ni `plan.propose` ni le panneau ne portent de compte des couches retirees.
  status: ticket, 2026-10-31, entrée 5.33 (`epic-agent-et-mcp/tickets.toml`)
  resolution: Devenu le ticket 5.33 « Couches retirées par le budget d'inventaire signalées » ; après 5.9.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-plusieurs-phrases-enchainees-dans-une-session-sans-saturer-l-plan.md`
  summary: Non verifie (medium s'il est vrai) : la date d'un fichier de resultat GeoPackage pourrait changer quand QGIS l'ouvre ou le ferme (journal WAL), et fausser l'ordre « plus recents d'abord » du budget d'inventaire.
  evidence: relecture de 5.18 (edge-case-hunter). A trancher : comparer la date d'un `.gpkg` du dossier de travail avant et apres ouverture et fermeture dans QGIS.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté, essai à la main dans QGIS : comparer la date d'un `.gpkg` du dossier de travail avant et après ouverture et fermeture (WAL).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-plusieurs-phrases-enchainees-dans-une-session-sans-saturer-l-plan.md`
  summary: Justesse du modele sur un long inventaire de session : en 9e position d'une recette enchainee, la phrase de demo exacte rend une question (« Je ne vois pas de champ de type de zone dans la couche plu_zones ») alors qu'elle rend un plan juste en 1re position ; famille d'entrainement a ajouter (inventaires avec des couches de resultat d'une session, pres du budget de 14 000 caracteres), jamais une regle de prompt.
  evidence: seconde recette enchainee de 5.18 (2026-10-24), 19 couches dans le projet, `plu_zones` et `typezone` dans l'inventaire vu (couches de l'utilisateur jamais retirees) ; les inventaires d'entrainement ne portent que les couches citees et une a quatre de distraction (`training/src/inventories.rs`, `finish`).
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté à la prochaine passe sur le générateur et au ré-entraînement (lignée 5.10, 5.15, 5.17), hors de 5.8 : défaut du modèle, à corriger par des exemples d'entraînement, jamais par une règle de prompt.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-duree-par-phrase-sous-30-s-dans-une-session-qgis-sur-process-plan.md`
  summary: Avec les identifiants courts vus par le modele (5.19), H11 (isochrone depuis un point par la voirie) rend un refus au lieu du plan : `ecart_reel` 32/40 contre 33/40 ; re-entrainement de la famille isochrone depuis un point, avec identifiants courts, jamais une regle de prompt.
  evidence: mesure GPU de 5.19 (2026-10-29) ; identifiants complets 33/40, courts 32/40 ; decision du proprietaire : garder les courts et re-entrainer dans un ticket suivant.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté à la prochaine passe sur le générateur et au ré-entraînement (lignée 5.10, 5.15, 5.17), hors de 5.8 : défaut du modèle, à corriger par des exemples d'entraînement, jamais par une règle de prompt.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-duree-par-phrase-sous-30-s-dans-une-session-qgis-sur-process-plan.md`
  summary: La phrase qui suit un plan a plusieurs resultats lit une fois toutes ses couches de resultat (4 couches, 3 436 jetons, ~62 s sur CPU 4 fils), au-dessus des 30 s ; aucun reglage du budget n'y suffit : ne montrer au modele que la couche finale de chaque plan, ou alleger les champs des couches d'intersection.
  evidence: recette enchainee de 5.19 (2026-10-29), phrase 2 apres le plan de demo a 69,6 s sans aucun retrait ; decision du proprietaire : ticket suivant.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par 5.22 (8d88fb0) : intermédiaires cachés et résultats allégés dans la vue du modèle.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-duree-par-phrase-sous-30-s-dans-une-session-qgis-sur-process-plan.md`
  summary: La phrase de demo rejouee en cours de session (phrase 9 de la recette enchainee) prend 44,6 s sur CPU 4 fils, dont 27 s d'ecriture de son plan a cinq etapes (165 jetons) : au-dessus des 30 s meme avec les identifiants courts ; debit d'ecriture a augmenter (5.21, carte graphique) ou plan ecrit encore plus court.
  evidence: recette enchainee finale de 5.19 (2026-10-29) ; lecture 17 s apres reprise, ecriture 27 s.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté au propriétaire : phrase 9 à 38 s sur CPU seul ; GPU (5.21) tenu, CPU à décider (plan écrit plus court ou exigence revue).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-duree-par-phrase-sous-30-s-dans-une-session-qgis-sur-process-plan.md`
  summary: Une couche de resultat de selection garde tous les champs de sa couche source (zones U du PLU, ~1 100 jetons) : la phrase qui suit la lit une fois et depasse 30 s (36,6 s, phrase 3) ; meme levier que la phrase apres un plan a plusieurs resultats (champs alleges dans la vue du modele).
  evidence: recette enchainee finale de 5.19 (2026-10-29).
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par 5.22 (8d88fb0) : résultats sans valeurs dans la vue du modèle, même levier.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-phrase-qui-suit-un-plan-a-plusieurs-resultats-sous-30-s-sur-plan.md`
  summary: Juger la justesse du modèle sur une vue allégée (intermédiaires cachés, résultats sans valeurs), dans une session enchaînée, et pas seulement sur les inventaires fixes de plan_reel, question_reel et ecart_reel.
  evidence: Les trois essais GPU de 5.22 n'ont aucune couche de résultat, les leviers ne s'y appliquent pas ; la recette CPU montre la phrase 3 reliée au PLU entier au lieu de la sélection.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté à la prochaine passe sur le générateur et au ré-entraînement (lignée 5.10, 5.15, 5.17), hors de 5.8 : défaut du modèle, à corriger par des exemples d'entraînement, jamais par une règle de prompt. Essai en session enchaînée à joindre aux mesures du ré-entraînement.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-phrase-qui-suit-un-plan-a-plusieurs-resultats-sous-30-s-sur-plan.md`
  summary: Retrouver les couches intermédiaires après une relance du noyau ou un journal.replay (par exemple depuis le journal), pour qu'elles ne réapparaissent pas toute la session.
  evidence: State.intermediates est en mémoire et n'est alimenté que par plan.execute ; après relance ou rejeu, les intermédiaires sont vus et peuvent ramener le retrait par paquets de 5.19.
  status: ticket, 2026-10-31, entrée 5.34 (`epic-agent-et-mcp/tickets.toml`)
  resolution: Devenu le ticket 5.34 « Couches intermédiaires retrouvées après une relance du noyau ou un rejeu » ; après 5.9.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-phrase-qui-suit-un-plan-a-plusieurs-resultats-sous-30-s-sur-plan.md`
  summary: Famille d'entraînement (5.10) « couche citée introuvable → question sur la couche », pour une phrase qui cite une couche cachée.
  evidence: Recette 5.22, phrase 11 « Montre le tampon de 500 m. » : question hors de propos sur des coordonnées X/Y.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté à la prochaine passe sur le générateur et au ré-entraînement (lignée 5.10, 5.15, 5.17), hors de 5.8 : défaut du modèle, à corriger par des exemples d'entraînement, jamais par une règle de prompt.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-nombre-de-fils-par-defaut-coeurs-physiques-et-masque-d-affin-plan.md`
  summary: Rendre la vérification complète (cargo test, clippy, fmt, deny) possible sous Linux/WSL, en fournissant libclang à llama-cpp-sys.
  evidence: Relecture 5.20, constat 1 : l'espace de travail ne se construit pas sous WSL faute de libclang ; seuls des morceaux isolés y sont testés, les tickets 5.19 à 5.22 ne vérifient que sous PowerShell.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Corrigé en 5.8 : note de construction sous WSL dans `README.md` (Démarrage rapide) : paquets `build-essential cmake clang libclang-dev`, `LIBCLANG_PATH` et `BINDGEN_EXTRA_CLANG_ARGS` (recette trouvée en 5.25), et repli `-p cassini-core -p cassini-ai -p cassini` sans libdbus ni GTK.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-acceleration-par-carte-graphique-dans-le-noyau-livre-avec-re-plan.md`
  summary: Vérifier automatiquement la construction `gpu` (build.rs, load_backends, bloc gpu de `hors_ligne`) : job CI avec le SDK Vulkan, ou étape obligatoire de la liste de publication.
  evidence: Relecture 5.21, constat 5 : la CI ne compile jamais `--features gpu` ; une DLL réseau ajoutée au paquet ou un moteur processeur manquant ne ferait rien échouer.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté, à mettre sur la liste de publication (étape 7) : construction `--features gpu` vérifiée à chaque livraison, ou job CI `gpu` avec le SDK Vulkan.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-acceleration-par-carte-graphique-dans-le-noyau-livre-avec-re-plan.md`
  summary: Prouver sur un poste à carte intégrée seule que rien ne part sur cette carte (liste vide de périphériques bien transmise à llama.cpp).
  evidence: Relecture 5.21, constat 3 : le test `processor_parameters_offload_nothing` ne distingue pas liste vide et liste absente ; aucun poste à carte intégrée n'a été essayé.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté, matériel : essai sur un poste à carte intégrée seule.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-acceleration-par-carte-graphique-dans-le-noyau-livre-avec-re-plan.md`
  summary: Repli sur processeur quand la carte échoue au décodage ou à l'adaptateur (non vérifié ; medium si réel).
  evidence: Relecture 5.21, constat 6 : le repli ne couvre que le chargement et la création du contexte ; à trancher en lançant une longue session sur une carte de 4 Go.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté, matériel : longue session sur une carte de 4 Go pour trancher le repli au décodage.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-serveur-mcp-cassini-mcp-expose-le-registre-a-un-client-tiers-plan.md`
  summary: Vérifier que les clients MCP visés acceptent des noms d'outils avec un point (`system.about`), sinon les exposer sous une forme admise.
  evidence: non vérifié (maybe-false, medium si vrai) ; la spec MCP 2025-11-25 admet le point, certaines API de modèles n'admettent que [A-Za-z0-9_-] ; tranché par la recette hitl de 5.7.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par 5.30 (505795f) : noms d'outils sans point, acceptés par tous les clients.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-serveur-mcp-cassini-mcp-expose-le-registre-a-un-client-tiers-plan.md`
  summary: Tester la négociation des révisions MCP 2024-11-05 et 2026-07-28 (sans initialize) annoncées par l'architecture.
  evidence: le client de test ne négocie que 2025-11-25 ; rmcp épinglé =3.8.0, ne régresse qu'à une montée de version.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Corrigé en 5.8 : `an_old_revision_is_accepted_and_carries_the_instructions` (initialize en 2024-11-05, révision rendue telle quelle, outils listés) et `the_revision_without_initialize_discovers_the_instructions` (2026-07-28 : `server/discover` puis `tools/list` avec la révision dans `_meta`), dans `cli/tests/mcp.rs`. rmcp 3.8.0 gère la révision sans initialize : elle est couverte.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-serveur-mcp-cassini-mcp-expose-le-registre-a-un-client-tiers-plan.md`
  summary: Servir les appels MCP sans bloquer le runtime (ping et annulation pendant une commande longue) avant d'exposer les commandes de terrain.
  evidence: call_tool attend la réponse du fil d'exécution sur l'unique fil du runtime ; sans effet sur system.about et model.list.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par 5.25 (055b9e1) : chaque requête MCP est une tâche du runtime, `ping` et l'annulation restent servis pendant un appel.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-canal-d-ia-externe-cote-session-le-noyau-de-qgis-accepte-un-plan.md`
  summary: Exécuter les appels d'un relais sur le canal d'IA externe (AD-29) — command.run et request.cancel externes, état d'occupation unique dans les deux sens (session.busy), external.call et external.approve (120 s, external.not_approved), refus des chemins (external.path_refused), origine external au journal et rejeu, external.session_lost pendant un appel ou à la déconnexion du relais.
  evidence: 5.24 coupé en deux à la demande du propriétaire le 2026-10-30 (plan d'environ 2 700 jetons, CLAUDE.md §8 « une tâche courte par session ») ; 5.24 garde le transport, le cycle de vie, external.attach et le protocole 2. Décision du propriétaire déjà prise pour cette suite : un plan proposé non tranché rend la session occupée pour le relais seulement, l'extension garde le droit de le remplacer par plan.propose.
  status: ticket, 2026-10-30, entrée 5.28 (`epic-agent-et-mcp/tickets.toml`)
  resolution: Devenu le ticket 5.28 « Execution des appels du relais » ; 5.25 et 5.26 l'attendent.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-execution-des-appels-du-relais-la-session-qgis-execute-ce-qu-plan.md`
  summary: Validation des appels externes (AD-29, case « Me demander avant chaque appel ») — avec ask allumé, external.approve {call_id, command, params} vers l'extension avant d'exécuter, sans bloquer (AD-4), borné à 120 s et annulable ; refus, erreur ou délai rendent external.not_approved, rien n'est journalisé ; délai réglable par les tests.
  evidence: 5.28 coupé à la demande du propriétaire le 2026-10-30 (plan d'environ 2 600 jetons, cible 1 600) ; 5.28 garde l'exécution, l'occupation, external.call, les chemins, l'annulation, la perte de session et le journal. D'ici là, ask allumé refuse tout appel externe par external.not_approved. À créer en ticket avant 5.26, qui allume la case.
  status: ticket, 2026-10-30, entrée 5.29 (`epic-agent-et-mcp/tickets.toml`)
  resolution: Devenu le ticket 5.29 « Validation des appels externes » ; 5.26 l'attend.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-relais-cassini-mcp-pilote-la-session-qgis-quand-elle-l-autor-plan.md`
  summary: Tester `ping` et l'annulation pendant un appel autonome de `cassini mcp` en vol (moitié autonome de « runtime non bloquant », 5.25).
  evidence: Aucune commande `core` du binaire n'est assez lente pour tenir un appel en vol de façon déterministe ; un retour à une attente bloquante dans `call_autonomous` passerait inaperçu. À faire avec les commandes de terrain (étape 6).
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté à l'étape 6 : attend une commande de terrain assez lente pour tenir un appel en vol de façon déterministe.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-recette-claude-gpt-et-gemini-sur-la-phrase-de-demo-qgis-ouve-plan.md`
  summary: Finir la recette Gemini CLI sur la phrase de démo (sonde `probe_external.py`, nom `gemini`) : couche, PDF et rejeu identique.
  evidence: Le 2026-10-31, le relais est prouvé avec Gemini CLI 0.66 (catalogue reçu, `project.inventory` réussi, rejeu identique), mais l'API de Google a rendu des 503 « high demand », puis un 429 (quota gratuit de 20 requêtes par jour épuisé). Le propriétaire clôt 5.27 ainsi. À rejouer quand le quota revient, ou avec une clé facturée.
  status: report, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Reporté au propriétaire : recette Gemini CLI à reprendre quand le quota gratuit de l'API de Google le permet.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-recette-claude-gpt-et-gemini-sur-la-phrase-de-demo-qgis-ouve-plan.md`
  summary: Champ `instructions` du serveur MCP (`get_info` de `cassini mcp`) qui dit en quelques phrases que Cassini pilote le projet QGIS ouvert, pour que les clients le prennent d'eux-mêmes.
  evidence: Recette 5.27 : Codex CLI n'a pas employé Cassini avec la phrase seule (recherche sur le web) et ne l'a fait qu'en nommant QGIS et Cassini ; sa documentation dit qu'il lit `instructions`. Claude Code, lui, l'a pris de lui-même.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Corrigé en 5.8 : `get_info` de `cassini mcp` porte `instructions` (`INSTRUCTIONS`, `cli/src/mcp.rs`) : Cassini pilote le projet QGIS ouvert quand la session l'autorise, sinon les seules commandes du noyau. Lu par les tests MCP en 2024-11-05, 2025-11-25 et 2026-07-28. À revérifier avec Codex CLI à la prochaine recette des clients.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-recette-claude-gpt-et-gemini-sur-la-phrase-de-demo-qgis-ouve-plan.md`
  summary: Noms d'outils MCP sans point. `cassini mcp` expose les identifiants du registre tels quels (`analysis.buffer`) ; Antigravity CLI, successeur de Gemini CLI, les refuse tous (motif `^[a-zA-Z0-9_-]{1,64}$`). Exposer un nom sans point, le même pour tous les clients, et le ramener à l'identifiant à l'appel, dans les deux modes.
  evidence: Recette 5.27 du 2026-10-31 : `cli.log` d'agy 1.4.12, « tool name mcp_cassini_analysis.buffer violates ^[a-zA-Z0-9_-]{1,64}$ » pour les 11 outils, aucun appel possible. Seule voie restante pour la famille Gemini avec un compte Pro (Gemini CLI refuse les comptes personnels). Catalogue construit dans `cli/src/mcp.rs` (`catalogue`, `catalogue_from_session`) ; à trancher contre AD-29 (aucun code propre à un fournisseur : le renommage vaut pour tous) et contre la parité `run`/`serve`/`mcp`.
  status: close, 2026-10-31, entrée 5.8 (`epic-agent-et-mcp/story-refactor-sweep-plan.md`)
  resolution: Clos par 5.30 (505795f) : nom sans point exposé, ramené à l'identifiant à l'appel.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-noms-d-outils-mcp-sans-point-acceptes-par-tous-les-clients-plan.md`
  summary: Rejeu d'un `project.inventory` demandé après la création de couches. Le rejeu diverge, car l'inventaire liste des `layer_id` à suffixe aléatoire et des sources sous `replay/`, qui changent d'une exécution à l'autre.
  evidence: Recette 5.30 avec Antigravity CLI (2026-10-31, 18 h 45) : 17 appels réussis, puis `replay.diverged` à l'étape 13 (`project.inventory`, empreinte `6472c69290d01abf`). Les étapes 2 (inventaire initial) à 12 rejouent à l'identique. Risque high (rejeu) : relecture `thorough`.
  status: ticket, 2026-10-31, entrée 5.31 (`epic-agent-et-mcp/tickets.toml`)
  resolution: Devenu le ticket 5.31 « Rejeu d'un inventaire demandé après la création de couches » ; 5.8 l'attend.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-rejeu-d-un-inventaire-demande-apres-la-creation-de-couches-plan.md`
  summary: L'ouverture d'un projet dans QGIS ajoute au journal la poignée de main du client (`system.about`) ; rejouer un journal le modifie donc et allonge son nombre d'étapes.
  evidence: Recette 5.31 (2026-10-31) : rejeu des journaux Antigravity et Claude Code de 5.30 en 20 et 10 étapes au lieu de 19 et 9, journaux remis depuis une copie ; relecture `thorough` de 5.31, constat 9 (medium).
  status: ticket, 2026-10-31, entrée 5.32 (`epic-agent-et-mcp/tickets.toml`)
  resolution: Devenu le ticket 5.32 « Rejeu sans poignée de main » ; 5.9 l'attend.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-rejeu-sans-poignee-de-main-system-about-ne-s-ajoute-plus-au-plan.md`
  summary: Une seconde fenêtre QGIS en lecture seule montre le bandeau `refused` avec le bouton « Choisir le noyau », qui ne sert à rien, et sans « Relancer » une fois la première fenêtre fermée.
  evidence: `panel.py:126` (`"refused": ("refused", "choose")`) ; avant 5.32 la lecture seule tombait déjà sur ce bandeau ; depuis 5.32 le message du noyau y est affiché, mais l'action reste celle d'un refus de version (relecture `thorough` de 5.32, passe 3).

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-suite-de-bout-en-bout-phrase-plan-valide-execution-rejeu-san-plan.md`
  summary: Deux rejeux de suite dans la même session QGIS, sans rouvrir le projet : le second rend `replay.diverged` à l'étape 1, alors que le journal est bon.
  evidence: recette QGIS de 5.9 (2026-11-03, noyau installé de 5.32, sonde hors dépôt `donnees-test-cassini/mesure-5-9/probe_replay_5_9.py`) — GDAL « A file system object called '…\replay\1-analysis.select_by_attribute.gpkg' already exists » ; `core/src/journal/mod.rs:636` vide `replay/` par `let _ = fs::remove_dir_all(...)`, erreur ignorée, et QGIS tient encore ouverts les fichiers du premier rejeu. Avec le projet rouvert entre les deux rejeux : deux fois `identical: true`. Défaut antérieur à 5.9 ; ticket à créer (risque high, rejeu).
  status: ticket, 2026-11-03, entrée 5.35 (`epic-agent-et-mcp/tickets.toml`)
  resolution: Devenu le ticket 5.35 « Deux rejeux de suite dans la même session QGIS » ; après 5.9, sans en dépendre.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-deux-rejeux-de-suite-dans-la-meme-session-qgis-plan.md`
  summary: Les dossiers de repli `replay-<n>/` du rejeu ne sont jamais nettoyés une fois `replay/` redevenu libre.
  evidence: relecture thorough de 5.35 ; `fresh_replay_dir` ne vide que le dossier qu'il va utiliser ; accumulation bornée à un dossier par rejeu tenu par QGIS, GeoPackage et PDF périmés restent sur disque. Remède possible : balayage best-effort des `replay-<n>/` inutilisés après un rejeu réussi.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-deux-rejeux-de-suite-dans-la-meme-session-qgis-plan.md`
  summary: Le bilan du rejeu ne dit pas dans quel dossier (`replay/` ou `replay-<n>/`) les sorties ont été écrites.
  evidence: relecture thorough de 5.35 ; `ReplayReport` reste `{steps, identical}` ; `qgis-plugin/tests/probe_end_to_end.py:235-239` lit `replay/` en dur. Ajouter le dossier (relatif, POSIX) au bilan touche la surface publique de `journal.replay`.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-deux-rejeux-de-suite-dans-la-meme-session-qgis-plan.md`
  summary: La sortie `journal.io` de `fresh_replay_dir` (tous les dossiers en échec, `replay/` sans fichier) n'est couverte par aucun test.
  evidence: relecture thorough de 5.35 (verification-gap) ; seul le cas `replay.output_locked` est testé.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-second-rejeu-dans-la-meme-session-qgis-l-export-pdf-voit-les-plan.md`
  summary: Un export journalisé après un rejeu, dans la même session QGIS, dessine et compte les couches posées par ce rejeu ; un rejeu suivant les laisse hors de la carte (mémoire du noyau) et diverge à l'export.
  evidence: Relecture 5.36 (edge-case et blind hunter). Les exports hors rejeu ne reçoivent jamais `leave_out` ; le cas divergeait déjà avant 5.36, donc hors de l'intention de ce ticket.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-l-export-pdf-prend-une-echelle-demandee-plan.md`
  summary: Le refus du cadre vide d'un export à l'échelle juge les emprises des couches, pas leurs entités : un cadre dans un trou d'une couche éparse sort un PDF vide.
  evidence: `layout_export.py` `_touches` compare `carte.extent()` aux emprises ; une requête d'entités sur le cadre (limite 1) trancherait (relecture 5.38, constat 21).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-l-export-pdf-prend-une-echelle-demandee-plan.md`
  summary: `LayoutBackend.canvas_center` (centre de la vue QGIS réelle, reprojeté) n'est exécuté par aucun test ni sonde.
  evidence: la sonde tourne sans `iface` ; seule l'étape 4 de la recette du README le couvre ; une sonde avec fenêtre QGIS le vérifierait (relecture 5.38, constat 22).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-l-export-pdf-prend-une-echelle-demandee-plan.md`
  summary: Un journal qui contient une etape deleguee en erreur (ex. export PDF refuse) ne se rejoue pas : `Replay::run` reclame quand meme le fichier de sortie de l'etape et rend `replay.diverged`.
  evidence: recette 5.38 du 2026-11-03 (projet-sonde-5-38) : etape 2 `layout.export_pdf` en `layout.bad_params`, rejeu « la sortie out ... est absente ou vide » ; dans `core/src/journal/mod.rs`, la boucle `for name in &spec.output_params` ne saute pas les lignes portant `error`. Anterieur a 5.38.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-l-export-pdf-prend-une-echelle-demandee-plan.md`
  summary: Le schema de `layout.export_pdf` admet un titre vide, que le geste refuse ensuite (`layout.bad_params`) : le modele de base a ecrit `"title": ""` pour « Exporte la carte au 1:25 000 ».
  evidence: recette 5.38 (Qwen3.5-9B) ; `LayoutExportPdfParams.title` sans longueur minimale ni controle a la deserialisation, contrairement a `scale`. Anterieur a 5.38.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-raster-et-terrain/story-traceur-le-noyau-lit-et-ecrit-un-mnt-geotiff-dans-un-module-plan.md`
  summary: Le module `dem` hérite de la limite de décodage par défaut de `tiff` (environ 256 Mo) : un très grand MNT (1 m sur une commune) est refusé en `dem.unsupported` sans seuil annoncé.
  evidence: relecture 4.1, constat 8 ; fixer `Limits` et documenter la taille maximale avec la première commande de terrain (4.2).
  status: close, 2026-11-05, entrée 4.8 (`epic-raster-et-terrain/story-refactor-sweep-plan.md`)
  resolution: `Dem::open` fixe les `Limits` de `tiff` à `MAX_DECODED_BYTES` (1 Gio, documenté dans `core/src/dem.rs`) et refuse au-delà en `dem.unsupported` « MNT trop grand », seuil de 1 Gio annoncé dans le message (a378d094ff9e0c0c99e641477b71a34396b2630a, entrée 4.2).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-re-entrainement-groupe-point-en-utm-et-echelle-de-l-export-p-plan.md`
  summary: Le modèle de 5.39 refuse un export à l'échelle qui ne dit pas « PDF » (« Exporte la carte au 1:25 000. ») : ajouter au générateur des tournures d'export à l'échelle sans ce mot, apprises au prochain ré-entraînement groupé.
  evidence: essai `echelle_utm_reel` E01 et démo QGIS du 2026-11-04 (refus « seulement une page A4 paysage ») ; toutes les tournures de `export_pdf_scale` (`export_pdf_scale` dans `training/src/families.rs`) contiennent « PDF », et `refus_hors_registre` apprend des refus de format. Décision du propriétaire : 5.39 clos, ticket à part.
  status: ticket, 2026-11-04, entrée 5.40 (`epic-agent-et-mcp/tickets.toml`)

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-raster-et-terrain/story-les-sorties-raster-du-noyau-s-ouvrent-dans-le-projet-qgis-plan.md`
  summary: Le générateur d'exemples doit montrer des sources raster relatives (`dem/mnt.tif`, `../donnees/mnt.tif`) dans l'inventaire et leur recopie dans `dem`, pour le ré-entraînement groupé.
  evidence: Depuis 4.3 l'inventaire donne aux rasters un chemin relatif au projet, mais le modèle ajusté n'a vu que des noms de fichier seuls (fixtures de `core/src/agent_inventory.rs`) ; 4.3 interdit d'apprendre quoi que ce soit au modèle.
  status: close, 2026-11-05, entrée 4.8 (`epic-raster-et-terrain/story-refactor-sweep-plan.md`)
  resolution: `add_dem` du générateur tire une source `donnees/`, `dem/`, `../donnees/` ou le nom seul, avec un test des quatre formes (3dada665f617d58fd93ccba9f0b5e1dd75bcf84b, entrée 4.6) ; modèle ré-entraîné en 5.41.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-raster-et-terrain/story-terrain-viewshed-champ-de-vision-depuis-un-point-plan.md`
  summary: Tester terrain.viewshed sur un MNT à cellules rectangulaires (borne et fenêtre prises sur la plus petite cellule, axes x et y séparés).
  evidence: Tous les MNT des tests ont des cellules carrées ; remplacer `min(cell_size_x, cell_size_y)` par `cell_size_x` ne casserait aucun test (relecture thorough de 4.5, constat 12). À couvrir avec la comparaison de 4.7.
  status: close, 2026-11-05, entrée 4.8 (`epic-raster-et-terrain/story-refactor-sweep-plan.md`)
  resolution: Tests `rectangular_cells_bound_on_the_smaller_and_crop_each_axis_apart` (champ de vision : borne sur la plus petite cellule, fenêtre par axe) et `rectangular_cells_sample_at_the_smaller_cell` (ligne de vue : pas d'échantillonnage) ; tous deux échouent si `min(cell_size_x, cell_size_y)` devient `cell_size_x`.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-raster-et-terrain/story-le-modele-apprend-les-trois-commandes-de-terrain-plan.md`
  summary: Le garde-fou des coordonnées devrait accepter un système écrit une seule fois pour deux points (« entre 652 000 / 6 862 000 et 653 100 / 6 861 400, en Lambert-93 »).
  evidence: Décision du propriétaire du 2026-11-04 (4.6) : `guard_coordinates` (`core/src/point.rs`) retire un `epsg` du multiensemble des nombres de la demande pour chaque `Point`, donc une ligne de vue dont le système n'est écrit qu'une fois rend la question du garde-fou. Le générateur écrit donc le système à chaque point (familles `ligne_de_vue`, `question_terrain`) ; le noyau n'est pas touché par 4.6.
  status: ticket, 2026-11-05, entrée 4.10 (`epic-raster-et-terrain/tickets.toml`)

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-raster-et-terrain/story-le-modele-apprend-les-trois-commandes-de-terrain-plan.md`
  summary: Avant de mesurer 5.41, mettre à jour les attendus de terrain de `plan_reel` (`ai/tests/plan_reel.rs` attend encore un refus pour S20, S21, S22 et S24 ; `tests/epreuve-modele-local/sentences.json` et `reading_independent.json` écrivent `dem` en id de couche et `from`/`to`).
  evidence: Relecture de 4.6 : le générateur apprend désormais à planifier ces phrases (`dem` = `source` du raster, `observer`/`target`), donc un modèle ré-entraîné « échouerait » là où il a raison, et le `verify` de 5.41 exige aucun recul sur `plan_reel`. Attendu périmé depuis 4.2 (pente), 4.4 et 4.5, pas causé par 4.6. S21 écrit le système une seule fois pour deux points : il attend la question du garde-fou tant que l'entrée précédente n'est pas faite. S23 (moyenne de la pente par commune) reste un refus (statistiques sur un raster).
  status: close, 2026-11-05, entrée 4.8 (`epic-raster-et-terrain/story-refactor-sweep-plan.md`)
  resolution: `ai/tests/plan_reel.rs` attend un plan pour S20, S21 et S22 sur la `source` du MNT, S23 et S24 restant des refus (a30bf8559597eaf4778d6440199f0911fd305b34, entrée 5.41). `tests/epreuve-modele-local/sentences.json` et `reading_independent.json` ne sont pas réécrits : pièces de l'épreuve de l'étape 2, sur un catalogue factice (`catalog.json`) qui définit lui-même `dem` en id de couche et `from`/`to`, et dont les résultats archivés y renvoient.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-re-entrainement-groupe-commandes-de-terrain-et-export-a-l-ec-plan.md`
  summary: Au prochain ré-entraînement groupé, renforcer deux refus du générateur : terrain puis statistique par zone (`NO_RASTER_STATS`, surtout après un champ de vision) et point de terrain écrit dans un autre système que le MNT (`NO_TERRAIN_CRS`, surtout en WGS84).
  evidence: 5.41, entraînement n°2 : `plan_reel` 23/25, S24 (champ de vision puis moyenne par commune → plan du seul champ de vision) et S25 (point WGS84 sur MNT Lambert-93 → champ de vision en EPSG:4326, refusé ensuite par le noyau `terrain.crs_mismatch`). Le jeu n'en contient que quelques dizaines sur 8 000. Décision du propriétaire du 2026-11-05 : accepter le modèle à 23/25 et reporter ce renfort.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-re-entrainement-groupe-commandes-de-terrain-et-export-a-l-ec-plan.md`
  summary: Au prochain ré-entraînement groupé, élargir `foreign_point` (point UTM d'un autre fuseau que le MNT, forme « X / Y (WGS84) » de S25) et ajouter à `terrain_reel` des phrases de refus (point hors système, terrain puis statistique) et une ligne de vue au système écrit une fois.
  evidence: Relecture thorough de 5.41 : `foreign_point` ne tire que Lambert-93/WGS84 autour de Paris, jamais 32N sur 31N ; S25 échoue encore ; `terrain_reel` n'attend que des plans. Changer le générateur maintenant désaccorderait le modèle installé.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-raster-et-terrain/bug-le-rejeu-diverge-quand-un-raster-de-terrain-precede-un-export-plan.md`
  summary: Avant la publication (étape 7), empêcher qu'une extension d'après 4.9 tourne avec un noyau d'avant 4.9 (ou l'inverse) : les rasters du noyau ne s'ouvriraient plus, sans message ; monter la version du protocole ou vérifier au `session.hello` que le noyau dicte `layer.open_raster`.
  evidence: Relecture thorough de 4.9 : le panneau n'ouvre plus de raster de lui-même, le noyau ne dicte l'ouverture que s'il la connaît ; AD-17 livre l'extension et le noyau à part. Sans utilisateur à ce jour (noyau et extension installés ensemble), le risque naît à la publication.
  status: report, 2026-11-05, entrée 4.8 (`epic-raster-et-terrain/story-refactor-sweep-plan.md`)
  resolution: Liste de publication (étape 7) : version du protocole ou contrôle au `session.hello`. Aucun utilisateur n'a aujourd'hui un noyau et une extension de versions croisées.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-raster-et-terrain/bug-le-rejeu-diverge-quand-un-raster-de-terrain-precede-un-export-plan.md`
  summary: Un journal écrit avant 4.9 (raster de terrain sans ligne `layer.open_raster`) diverge toujours au rejeu en session neuve ; décider s'il faut le signaler avant de rejouer.
  evidence: Relecture thorough de 4.9 : le correctif ne vaut que pour les journaux nouveaux ; aucun journal d'utilisateur n'existe encore, d'où le report.
  status: report, 2026-11-05, entrée 4.8 (`epic-raster-et-terrain/story-refactor-sweep-plan.md`)
  resolution: Décision du propriétaire avant la publication (étape 7) : signaler ou non un journal sans `layer.open_raster` avant de le rejouer. Aucun journal d'utilisateur n'existe encore.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-raster-et-terrain/story-le-garde-fou-des-coordonnees-accepte-un-systeme-ecrit-une-se-plan.md`
  summary: Au prochain ré-entraînement groupé, apprendre au générateur (familles `ligne_de_vue`, `question_terrain`) des demandes dont le système n'est écrit qu'une fois pour deux points, que le garde-fou accepte depuis 4.10.
  evidence: Relecture thorough de 4.10 : le générateur écrit encore le système à chaque point ; le changer maintenant désaccorderait le modèle installé.
  status: report, 2026-11-05, entrée 4.8 (`epic-raster-et-terrain/story-refactor-sweep-plan.md`)
  resolution: Prochain ré-entraînement groupé (familles `ligne_de_vue`, `question_terrain` du générateur) ; le changer seul désaccorderait le modèle installé.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-raster-et-terrain/bug-la-pente-d-un-mnt-en-web-mercator-est-fausse-sans-erreur-et-la-pente-en-degres-depend-de-la-plateforme-plan.md`
  summary: Compléter la liste fermée des systèmes métriques du MNT (EPSG:9794 RGF93 v2b Lambert-93 et ses coniques, 3035 LAEA, UTM outre-mer 5490, 2972, 2975) et documenter cette liste pour l'utilisateur.
  evidence: Relecture de 4.11 — ces systèmes en vrais mètres sont désormais refusés par `is_known_metric` (core/src/dem.rs) ; le propriétaire a décidé qu'un système manquant s'ajoute par un ticket à part.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-raster-et-terrain/bug-la-pente-d-un-mnt-en-web-mercator-est-fausse-sans-erreur-et-la-pente-en-degres-depend-de-la-plateforme-plan.md`
  summary: Un journal de pente en degrés écrit avant 4.11 peut rendre replay.diverged au rejeu si libm::atan diffère de la libm du système sur ce MNT.
  evidence: Relecture de 4.11 — la pente en degrés passe de f64::atan à libm::atan ; la divergence, si elle survient, est signalée et non silencieuse.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-socle-et-carte/story-maintenir-la-recette-github-actions-a-jour-plan.md`
  summary: Corriger le commentaire d'en-tête de .github/workflows/recette.yml, qui dit n'utiliser que des actions officielles de GitHub alors que le job noyau utilise EmbarkStudios/cargo-deny-action.
  evidence: Relecture de 1.11 — commentaire ligne 6, antérieur au ticket.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-traceur-coord-convert-de-bout-en-bout-mgrs-compris-plan.md`
  summary: Avec Qwen3.5-9B (prompt long), toute phrase sur le projet de test projet-test-villeneuve.qgz échoue en ai.prompt_too_long : l'inventaire du projet et le catalogue ne tiennent pas dans le contexte.
  evidence: Démo de 11.2 — l'ancien noyau installé, sans coord.convert, échoue de même sur ce projet ; la démo n'a réussi que sur un projet vide.
  status: report, 2026-11-07, entrée 11.13 (`epic-geometrie-et-terrain-etendu/story-refactor-sweep-plan.md`)
  resolution: Recette QGIS de 11.16 jouée sur ce projet avec le modèle ajusté (16/16) ; le cas du prompt long de Qwen3.5-9B n'est pas remesuré. À reconstater à la répétition de démo (entrée 11.14).

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-reprojection-d-un-point-par-qgis-data-transform-point-plan.md`
  summary: Le geste data.transform_point ne vérifie pas QgsCoordinateTransform.isValid() ; une transformation invalide rendrait le point source étiqueté du code cible (medium, non vérifié).
  evidence: Relecture de 11.4 ; à établir par une sonde sous QGIS sur un projet dont le contexte de transformation nomme une grille absente.
  status: report, 2026-11-07, entrée 11.13 (`epic-geometrie-et-terrain-etendu/story-refactor-sweep-plan.md`)
  resolution: Demande une sonde sous QGIS sur un projet dont le contexte de transformation nomme une grille absente ; à jouer avant la publication (étape 7). Hors du ménage : le geste vit dans l'extension et rien ne prouve le défaut.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-un-seul-type-de-raster-rasterpath-partout-plan.md`
  summary: Apprendre au modèle local à citer un raster par `step:k` (sans champ) dans une chaîne pente → champ de vision.
  evidence: Le long prompt et l'entraînement restent inchangés au ticket 11.5 (décision du propriétaire) ; `check_plan` refuse désormais `step:k.file` sur un `RasterPath` ; à porter au ré-entraînement groupé (entrées 12 et 16).
  status: report, 2026-11-07, entrée 11.13 (`epic-geometrie-et-terrain-etendu/story-refactor-sweep-plan.md`)
  resolution: La forme `step:1` sans champ est apprise (`pente_seuil`, `terrain.slope` puis `raster.reclassify`, entrée 11.12 lot 2a, a9d0c064cc2081601c446d3697255330fe8de21b ; ré-entraînement de l'entrée 16, 93b2bfbb0405004da2d522b28b468db24b2fe4eb), mais aucune famille n'enchaîne une pente dans `terrain.viewshed` : la chaîne nommée reste au prochain ré-entraînement groupé.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-un-seul-type-de-raster-rasterpath-partout-plan.md`
  summary: Cacher à l'inventaire du modèle un raster intermédiaire consommé par `out:k.n` dans un plan.
  evidence: `intermediate_files` (`core/src/journal/mod.rs`) ne reconnaît que les sorties consommées par `layer_id` ; une pente lue par `terrain.viewshed` via `out:3.1` reste montrée au modèle (comportement antérieur inchangé).
  status: report, 2026-11-07, entrée 11.13 (`epic-geometrie-et-terrain-etendu/story-refactor-sweep-plan.md`)
  resolution: Change ce que voit le modèle dans l'inventaire : à traiter avec la traduction de `step:k` dans les listes (constat 143), dans un même ticket sur le journal.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-ecriture-geojson-du-noyau-et-ouverture-dictee-plan.md`
  summary: Jouer la sonde `probe_vector_output.py` (GeoJSON 2008 en EPSG:2154 ouvert par `layer.open_vector`) sous QGIS 4.x.
  evidence: le critère de l'entrée 11.8 nomme QGIS 3.44 et 4.x ; seul QGIS 3.44.15 est installé sur le poste (décision du propriétaire du 2026-11-06 : 3.44 seul, 4.x reporté).
  status: report, 2026-11-07, entrée 11.13 (`epic-geometrie-et-terrain-etendu/story-refactor-sweep-plan.md`)
  resolution: QGIS 4.x non installé sur le poste (décision du propriétaire du 2026-11-06) ; à jouer à son installation, avant la publication (étape 7).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-ecriture-geojson-du-noyau-et-ouverture-dictee-plan.md`
  summary: Rendre le `layer_id` de l'ouverture dictée `layer.open_vector` citable par `step:k.layer_id` dans un plan, pour enchaîner une sortie vecteur du noyau dans une analyse QGIS.
  evidence: `core/src/serve/plans.rs` (`execute`) ne garde dans `outputs[k]` que le résultat de la commande du noyau ; la couche ouverte par l'étape dictée n'y est pas. Parité avec le raster tenue, mais l'entrée 15 (`terrain.peaks`) en aura besoin. Toujours ouverte après l'entrée 11.15 : `terrain.peaks` a été livrée sans l'exiger (un plan ne cite pas encore la couche de points ouverte).
  status: report, 2026-11-07, entrée 11.13 (`epic-geometrie-et-terrain-etendu/story-refactor-sweep-plan.md`)
  resolution: Fonction nouvelle (sortie vecteur citable dans un plan), pas un ménage : ticket à créer quand un plan devra enchaîner une sortie vecteur du noyau dans une analyse QGIS.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-ecriture-geojson-du-noyau-et-ouverture-dictee-plan.md`
  summary: Le refus d'un système non concordant par `QgisBackend.add_vector` (`vector_output.py`) n'est vérifié que par la sonde manuelle `probe_vector_output.py`.
  evidence: les tests unitaires passent par `FakeBackend`, qui refait le refus lui-même ; rejouer la sonde à chaque changement de `vector_output.py`, ou l'automatiser si une recette sous QGIS voit le jour.
  status: report, 2026-11-07, entrée 11.13 (`epic-geometrie-et-terrain-etendu/story-refactor-sweep-plan.md`)
  resolution: Convention des sondes de l'extension : rejouer `probe_vector_output.py` à chaque changement de `QgisBackend.add_vector`. L'entrée 11.13 ne touche que le déroulé commun (`open_output.py`), pas ce fond.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-reclassement-et-superposition-ponderee-de-rasters-plan.md`
  summary: Traduire et vérifier `step:k` dans les éléments d'une liste de rasters (`criteria[i].raster`, `exclusions[i]` de `raster.weighted_overlay`), pour qu'un plan enchaîne pente, reclassement et superposition ; nouvelle entrée avant le ré-entraînement (12).
  evidence: `raster_markers` et la vérification du plan (`core/src/plan.rs`) ne lisent que les paramètres de premier niveau ; décision du propriétaire du 2026-11-06 de ne pas l'étendre dans l'entrée 11.9.
  status: report, 2026-11-07, entrée 11.13 (`epic-geometrie-et-terrain-etendu/story-refactor-sweep-plan.md`)
  resolution: Fonction nouvelle qui touche le journal et le rejeu : ticket à créer. En attendant, la vérification du plan refuse tôt une référence imbriquée (constat 145).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-reclassement-et-superposition-ponderee-de-rasters-plan.md`
  summary: La famille de refus « seuil sur la pente » du générateur (`refus_hors_registre`, cas 4, `NO_RASTER_THRESHOLD`, `training/src/families.rs`) apprend au modèle à refuser « les zones où la pente dépasse n % », que `terrain.slope` puis `raster.reclassify` savent désormais faire ; à remplacer par un plan au ré-entraînement (entrée 12).
  evidence: Le garde-fou `what_is_refused_out_of_registry_is_still_missing_from_the_registry` a échoué à l'arrivée de `raster.reclassify` (mot « reclass ») ; l'entrée 11.9 n'a ôté que ce mot du garde-fou, sans toucher au générateur ni aux exemples (frontière du plan).
  status: close, 2026-11-07, entrée 11.12 lot 2a (`epic-geometrie-et-terrain-etendu/story-familles-du-generateur-lot-2a-derives-du-mnt-plan.md`)
  resolution: Cas 4 et `NO_RASTER_THRESHOLD` retirés de `refus_hors_registre` ; la famille `pente_seuil` rend `terrain.slope` puis `raster.reclassify` sur `step:1`, et le garde-fou `selects_no_cell` échoue si un refus de sélection des cellules revient. Le modèle ne l'apprendra qu'au ré-entraînement groupé (entrée 16).
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-reclassement-et-superposition-ponderee-de-rasters-plan.md`
  summary: Jusqu'à l'entrée qui traduira `step:k` dans les éléments d'une liste, la vérification du plan (`core/src/plan.rs`) accepte un `step:k` écrit dans `criteria[i].raster` ou `exclusions[i]`, que le journal ouvre ensuite comme un fichier du projet (erreur « fichier introuvable » tardive) ; à refuser tôt par `plan.bad_reference`, ou à traiter dans cette même entrée.
  evidence: relecture `thorough` de 11.9 (blind-hunter, edge-case-hunter) ; `raster_markers` ne lit que les paramètres de premier niveau, tandis que `resolve_param` du journal descend désormais dans `liste.champ`.
  status: close, 2026-11-07, entrée 11.13 (`epic-geometrie-et-terrain-etendu/story-refactor-sweep-plan.md`)
  resolution: `check_plan` (`core/src/plan.rs`) refuse en `plan.bad_reference` toute référence `step:` écrite dans une liste ou un objet, à l'étape fautive ; test `a_reference_inside_a_list_is_refused_before_execution`.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/bug-le-test-d-extinction-du-canal-d-ia-externe-avec-des-relais-m-plan.md`
  summary: Le test a_relay_that_stops_reading_does_not_hold_the_switch_off ne prouve le garde-fou des lignes en file (5.43) que sous charge ; au repos, sa suppression passerait sous le seuil d'1 s.
  evidence: Mesure du 2026-11-06 sous Windows : sans correctif, 0,79 s au repos, et de 1,27 à 1,49 s sous charge (la mutation échoue 3 fois sur 3). Il faudrait un test qui compte les réponses traitées après l'extinction, ou une inondation plus forte sans faire exploser la mémoire.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/bug-le-test-de-fermeture-du-canal-d-ia-externe-echoue-de-temps-e-plan.md`
  summary: Sous Windows, si un relais prend la seule instance de tube en attente juste avant la fermeture, `wake` peut attendre jusqu'à ~5 s (WaitNamedPipeW) une instance qui ne viendra plus, et retarder l'extinction.
  evidence: maybe-false, gravité medium non vérifiée ; existait avant 5.42 (relecture rapide de 5.42). Preuve : mesurer sous Windows le temps de `session.external {enabled:false}` avec un relais qui se connecte juste avant.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-format-orientation-et-carroyage-de-la-mise-en-page-plan.md`
  summary: La famille de refus « papier » du générateur (`refus_hors_registre`, cas 1, `ONLY_A4`, `training/src/families.rs`) apprend au modèle à refuser « exporte en A3 », « en portrait », « en A0 », que `layout.export_pdf` sait désormais faire (`page_size`, `orientation`, `grid`) ; à remplacer par des plans au ré-entraînement (entrée 16), avec une famille pour le carroyage.
  evidence: Le garde-fou `what_is_refused_out_of_registry_is_still_missing_from_the_registry` a échoué à l'arrivée de `page_size` et `orientation` ; l'entrée 11.11 n'a ôté que la vérification des noms de paramètres de l'export, sans toucher au générateur ni aux exemples (frontière du plan : ni exemple ni ré-entraînement).
  status: close, 2026-11-07, entrée 11.12, lot 3 (`epic-geometrie-et-terrain-etendu/story-familles-du-generateur-lot-3-export-au-format-plan.md`)
  resolution: `ONLY_A4` retiré ; le cas papier de `refus_hors_registre` refuse désormais un format hors série (A5, lettre US) et l'impression ou le nombre d'exemplaires ; la famille `export_pdf_format` apprend `page_size`, `orientation` et `grid` (carroyage sans système → question).

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-format-orientation-et-carroyage-de-la-mise-en-page-plan.md`
  summary: Plus rien n'échoue si le remplacement de la famille de refus `ONLY_A4` est oublié : le garde-fou `what_is_refused_out_of_registry_is_still_missing_from_the_registry` n'a plus de volet « papier », et `docs/entrainement-du-modele.md:1119` décrit encore ce refus comme juste.
  evidence: Relecture rapide de 11.11 ; à l'entrée 16, remettre un garde-fou qui échoue tant qu'un exemple refuse un format ou une orientation désormais permis, et corriger la doc.
  status: close, 2026-11-07, entrée 11.12, lot 3 (`epic-geometrie-et-terrain-etendu/story-familles-du-generateur-lot-3-export-au-format-plan.md`)
  resolution: `refuses_a_known_page` rétabli dans `what_is_refused_out_of_registry_is_still_missing_from_the_registry` (échoue si un refus de `refus` ou `refus_hors_registre` cite A4…A0, portrait, paysage ou orientation) ; `paper_and_orientation_are_plans_never_refusals` au jeu par défaut ; `docs/entrainement-du-modele.md` corrigé.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-familles-du-generateur-pour-les-nouvelles-commandes-plan.md`
  summary: Familles du générateur pour les commandes raster et de terrain de l'epic 11 (`terrain.aspect`, `terrain.hillshade`, `terrain.viewshed_cumulative`, `terrain.peaks`, `raster.reclassify`, `raster.weighted_overlay`, `terrain.cost_distance`, `terrain.least_cost_path`), thèmes raster de l'inventaire (coût, critères, masque), validateur étendu aux paramètres raster et à `step:k`, et remplacement du refus `NO_RASTER_THRESHOLD` par pente puis reclassement.
  evidence: Plan de 11.12 à environ 2 800 jetons ; découpé en trois par le propriétaire le 2026-11-07, deuxième lot. À faire avant l'entrée 16 (ré-entraînement groupé).
  status: close, 2026-11-07, entrée 11.13 (`epic-geometrie-et-terrain-etendu/story-refactor-sweep-plan.md`)
  resolution: Fait en cinq lots de l'entrée 11.12 : lot 1 (acc9283880a7e55ae32408b7e46b61e4edecf604), 2a (a9d0c064cc2081601c446d3697255330fe8de21b), 2b-1 (aea48552199f486227328f31ec6192f72ae4c49a), 2b-2 (28725d2ea0de50dae286eb4ae6f17417034bcefd), 3 (01ed24f4c2e7c3841df6f02174d7fe707d1049d5) ; appris à l'entrée 16.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-familles-du-generateur-pour-les-nouvelles-commandes-plan.md`
  summary: Famille du générateur pour l'export au format (`layout.export_pdf` avec `page_size`, `orientation`, `grid`), remplacement du refus `ONLY_A4`, inversion du test `paper_and_orientation_refusals_stay_refusals` et correction de `docs/entrainement-du-modele.md` (refus A4 décrit comme juste).
  evidence: Plan de 11.12 à environ 2 800 jetons ; découpé en trois par le propriétaire le 2026-11-07, troisième lot. À faire avant l'entrée 16 (ré-entraînement groupé).
  status: close, 2026-11-07, entrée 11.12, lot 3 (`epic-geometrie-et-terrain-etendu/story-familles-du-generateur-lot-3-export-au-format-plan.md`)
  resolution: Famille `export_pdf_format` (poids 4), refus `ONLY_A4` remplacé, test inversé, doc corrigée ; 140 exemples au jeu par défaut, 4,6 % de redemandes.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-familles-du-generateur-pour-les-nouvelles-commandes-plan.md`
  summary: Avant de mesurer le modèle ré-entraîné (entrée 16), mettre à jour l'attendu de S25 dans `ai/tests/plan_reel.rs` (point WGS84 sur un MNT en Lambert-93) : refus aujourd'hui, plan `data.transform_point` puis `terrain.viewshed` sur `step:1.point` depuis le lot 1 de 11.12. Le renfort du refus `NO_TERRAIN_CRS` demandé après 5.41 est caduc ; `foreign_point` tire désormais aussi l'UTM 32N sur un MNT en 31N et inversement.
  evidence: Lot 1 de 11.12 : le générateur n'engendre plus ce refus et apprend la reprojection visible ; le plan du lot interdit de toucher `cassini-ai`. Un modèle ré-entraîné « échouerait » S25 là où il a raison.
  status: close, 2026-11-07, entrée 16 (`epic-geometrie-et-terrain-etendu/story-re-entrainement-groupe-et-recette-du-modele-plan.md`)
  resolution: S25 attend dans `ai/tests/plan_reel.rs` le plan `data.transform_point` (`point` 4326, `target_epsg` 2154) puis `terrain.viewshed` (`observer: step:1.point`, 10 m, 1 km) ; en-tête mis à jour ; tests sans modèle `the_terrain_sentences_expect_plans_except_the_raster_statistics` et `s25_reprojects_the_wgs84_point_into_the_dem_system_first`.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-familles-du-generateur-pour-les-nouvelles-commandes-plan.md`
  summary: `cargo clippy --workspace --all-targets -- -D warnings` ne se construit pas sous WSL (GTK pour `cassini-app`, script de `llama-cpp-sys-2`) : le critère « clippy sur tout l'espace » des tickets n'y est vérifiable que par crate.
  evidence: Relecture de 11.12 lot 1 ; clippy vert sur `cassini-training` et `cassini-core`. À confirmer par la recette GitHub après le push, ou en installant les bibliothèques système sous WSL.
  status: report, 2026-11-07, entrée 11.13 (`epic-geometrie-et-terrain-etendu/story-refactor-sweep-plan.md`)
  resolution: Cause d'environnement (GTK, `llama-cpp-sys-2` sous WSL) : clippy vérifié par crate sous WSL, l'espace entier par la recette GitHub.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-familles-du-generateur-lot-2a-derives-du-mnt-plan.md`
  summary: 11.12 lot 2b — thèmes raster de l'inventaire synthétique (coût, critères, masque), familles `raster.weighted_overlay`, `terrain.cost_distance`, `terrain.least_cost_path` et reclassement d'un raster autre que la pente, validateur étendu à `cost`, `criteria[i].raster` et `exclusions[i]`.
  evidence: Lot 2 de 11.12 découpé en 2a et 2b par le propriétaire le 2026-11-07 (taille du plan) ; 2a couvre les dérivés du MNT et le refus du seuil de pente. À faire avant l'entrée 16 (ré-entraînement groupé).
  status: scindée, 2026-11-07 : 2b-1 fait (`epic-geometrie-et-terrain-etendu/story-familles-du-generateur-lot-2b-rasters-de-cout-criteres-et-masque-plan.md`), 2b-2 dans l'entrée suivante
  resolution: 2b-1 : thème raster `cout` (« Coût de déplacement », « Friction », « Coût de traversée ») dans l'inventaire synthétique, cité par sa `source` protégée (`Builder::add_raster`) ; familles `cout_cumule` (`terrain.cost_distance`) et `chemin_moindre_cout` (`terrain.least_cost_path`, et la question quand aucun raster de coût n'est ouvert) ; le validateur contrôle `cost` comme `dem` et `raster`. Critères, masques, `raster.weighted_overlay` et reclassement d'un raster brut restent à l'entrée 2b-2.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-familles-du-generateur-lot-2b-rasters-de-cout-criteres-et-masque-plan.md`
  summary: 11.12 lot 2b-2 — thèmes raster de critères notés, rasters bruts (distance aux routes dans le groupe `voirie`, bruit) et masques dans l'inventaire synthétique ; familles `superposition_ponderee` (`raster.weighted_overlay`, 2 à 4 critères, poids écrits ou tous à 1, 0 à 2 masques écrits) et `reclassement_raster` (`raster.reclassify` sur un raster brut, valeurs implicites ou écrites) ; validateur étendu à `criteria[i].raster` et `exclusions[i]`, jamais `step:k` dans une liste.
  evidence: Plan du lot 2b à environ 2 500 jetons ; découpé par le propriétaire le 2026-11-07 : 2b-1 couvre le raster de coût, `terrain.cost_distance` et `terrain.least_cost_path`. À faire avant l'entrée 16 (ré-entraînement groupé).
  status: faite, 2026-11-07 (`epic-geometrie-et-terrain-etendu/story-familles-du-generateur-lot-2b-2-criteres-masques-et-superposition-plan.md`)
  resolution: huit thèmes raster civils dans l'inventaire synthétique, cités par leur `source` protégée (`Builder::add_raster`) : quatre critères notés (accessibilité, ensoleillement, proximité des services, calme), deux masques dans le groupe de leur vecteur (zones inondables dans `inondables`, espaces protégés dans `znieff`), deux rasters bruts (distance aux routes dans `voirie`, bruit) ; familles `superposition_ponderee` (`raster.weighted_overlay`, 2 à 4 critères dans l'ordre écrit, poids écrits ou tous à 1, 0 à 2 masques, question quand un critère nommé manque) et `reclassement_raster` (`raster.reclassify` directement sur le raster brut, seuil unique ou 2 à 3 classes contiguës, valeurs 1, 2, 3 ou écrites, question quand le raster manque) ; le validateur contrôle `criteria[i].raster` et `exclusions[i]` contre les `source` raster de l'inventaire, jamais `step:k`. Redemandes à 4,5 % à poids inchangé. Reste l'export au format (lot 3).

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-re-entrainement-groupe-et-recette-du-modele-plan.md`
  summary: Renforcer le générateur avant le prochain ré-entraînement groupé sur quatre constats du modèle 11.16 : un carroyage sans système doit rendre la question du système, jamais un refus (F03 de `epic11_reel` : refus « aucune commande n'exporte de carte ») ; « dans quelle direction sont tournées les pentes » est une exposition (A01, rendu en pente + reclassement) ; un PDF qui nomme ses couches reste un refus (S18 de `plan_reel` : export de toutes les couches visibles) ; un point nommé (« le château d'eau en X / Y (Lambert-93) ») n'appelle pas de question (S20). Ajouter aussi la reprojection d'un fuseau UTM à l'autre (deux ratés en validation).
  evidence: Mesures GPU de 11.16 (`~/cassini-entrainement/11-16-mesures/`) ; modèle accepté par le propriétaire le 2026-11-07 sans second entraînement, toutes les barres tenues.
  status: résolu côté générateur, 2026-11-09, entrée 11.23 (`epic-geometrie-et-terrain-etendu/story-re-entrainement-groupe-points-tels-qu-ecrits-et-commandes-de-plan.md`), partie A ; le modèle l'apprend à la partie B
  resolution: F03 : famille `carroyage_utm_projet` (aucun système → la question, jamais un refus ; « UTM » sans fuseau → `project.utm_epsg` ou la question). A01 : quatre tournures « orientation des pentes » dans `exposition`. S18 : cas `NO_LAYER_CHOICE` de `refus_hors_registre` (PDF qui nomme ses couches, format ou orientation compris). S20 : lieu nommé suivi de ses coordonnées dans `champ_de_vision`. Fuseau à fuseau : les deux ratés de validation étaient des points de terrain, que le noyau convertit désormais (11.22) ; `conversion_coordonnees` apprend en plus un point UTM vers le fuseau voisin (`coord.convert`, `utm_zone`).

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-re-entrainement-groupe-et-recette-du-modele-plan.md`
  summary: Mesurer sur processeur (`duree_reel`, 4 fils) le temps de la phrase de démo avec le modèle 11.16 et le registre de l'epic 11 (quatorze commandes de plus dans le prompt) ; seules des durées GPU sont consignées depuis l'epic 11.
  evidence: Relecture de 11.16 ; exigence produit « longue phrase en moins de 30 s sur CPU », démo déjà à 32-35 s sur CPU avant l'epic 11. La croissance du prompt vient des tickets 11.2 à 11.11, pas de ce ré-entraînement.
  status: report, 2026-11-07, entrée 11.13 (`epic-geometrie-et-terrain-etendu/story-refactor-sweep-plan.md`)
  resolution: Mesure à faire avant la démonstration (entrée 11.14), avec le modèle installé.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-re-entrainement-groupe-et-recette-du-modele-plan.md`
  summary: Ajouter à un essai réel des phrases qui donnent aux nouvelles commandes un point dans un autre système que le raster (WGS84 ou UTM pour `terrain.viewshed_cumulative`, `terrain.cost_distance`, `terrain.least_cost_path`) et une reprojection UTM 31N vers 32N, avant le prochain ré-entraînement groupé.
  evidence: Relecture de 11.16 : `epic11_reel` n'a que des points dans le système du raster ; la validation a manqué deux reprojections d'un fuseau à l'autre.
  status: résolu, 2026-11-09, entrée 11.23 (`epic-geometrie-et-terrain-etendu/story-re-entrainement-groupe-points-tels-qu-ecrits-et-commandes-de-plan.md`), partie A
  resolution: `ai/tests/epic11_reel.rs`, `FOREIGN_POINTS` (Y01 à Y04) : champ de vision cumulé depuis deux points WGS84, coût cumulé depuis un point UTM 31N, chemin de moindre coût d'un point WGS84 à un point Lambert-93 (attendus sur le plan complété), point UTM 31N vers le fuseau 32 ; mesurées et imprimées à part, hors de la barre de 27 / 28 ; test sans modèle : chaque plan tel qu'écrit passe le garde-fou et la complétion en fait l'attendu.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-second-jeu-de-test-reel-annecy-pret-pour-la-demonstration-plan.md`
  summary: Le rejeu d'un journal qui contient une étape QGIS échouée diverge à tort ; il faut sauter le contrôle du fichier de sortie quand l'étape a redonné la même erreur.
  evidence: Sonde enchaînée 11.18 sur Annecy (2026-11-07) : le plan 1 s'arrête sur `analysis.buffer` (`analysis.crs_not_metric`), puis le rejeu échoue en `replay.diverged` (« la sortie « out » de la commande analysis.buffer est absente ou vide »). `core/src/journal/mod.rs:999` accepte la même erreur, mais les lignes 1013-1023 exigent ensuite le fichier `out` même quand `outcome` est `Err`. Risque high (rejeu).

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-second-jeu-de-test-reel-annecy-pret-pour-la-demonstration-plan.md`
  summary: Le modèle ne compose pas `data.transform_point` avant `terrain.viewshed` et `terrain.line_of_sight` quand le point est en WGS84 et le MNT en Lambert-93 ; le noyau refuse alors en `terrain.crs_mismatch`.
  evidence: Sonde 11.18 sur Annecy, phrases 5 et 6. Cela rejoint le constat déjà reporté sur les points donnés dans un autre système que le raster ; à traiter par des exemples d'entraînement au prochain ré-entraînement groupé.
  status: résolu par 11.22 (complétion du plan par le noyau) et, côté générateur, entrée 11.23 (`epic-geometrie-et-terrain-etendu/story-re-entrainement-groupe-points-tels-qu-ecrits-et-commandes-de-plan.md`), partie A
  resolution: Le noyau ajoute la conversion au plan (11.22) ; le générateur recopie désormais le point tel qu'écrit (`terrain_autre_systeme`, `champ_de_vision_cumule`, `cout_cumule`, `chemin_moindre_cout`), une référence MGRS gardant `coord.convert` puis `data.transform_point`. À juger sur le modèle de la partie B.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-second-jeu-de-test-reel-annecy-pret-pour-la-demonstration-plan.md`
  summary: Une couche de points en EPSG:4326, le cas d'un CSV de gares en latitude et longitude, ne passe pas un tampon (`analysis.crs_not_metric`) tant que `data.reproject` reste seulement planifiée ; la phrase de démo transposée à Annecy échoue pour cette raison.
  evidence: Sonde 11.18, phrases 1 et 10, couche `gares` lue depuis `gares-74-4326.csv`. Le jeu de Villeneuve donne ses gares en 2154 et ne montrait pas ce cas.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-second-jeu-de-test-reel-annecy-pret-pour-la-demonstration-plan.md`
  summary: Sur le projet d'Annecy (10 couches, dont BD TOPO aux dizaines de champs), la composition prend 30 à 35 s par phrase sur GPU une fois les communes ramenées au 74 (37 à 60 s au premier passage), contre environ 14 s sur Villeneuve ; l'exigence est de moins de 30 s.
  evidence: Sonde 11.18, noyau `--features gpu` installé le 2026-11-07, modèle `cassini-composition-4b`. La longueur de l'inventaire est la cause probable ; elle reste à mesurer.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-second-jeu-de-test-reel-annecy-pret-pour-la-demonstration-plan.md`
  summary: Le carroyage UTM de 1 km en A3 est refusé (plus de 200 lignes par axe) sur le projet d'Annecy ; il faut vérifier quelle emprise la mise en page prend.
  evidence: Sonde 11.18, refus aux deux passages, y compris après que les communes ont été ramenées à un fichier du 74 seul (279 entités). L'emprise des couches est d'environ 125 × 100 km (MNT 25 m), soit environ 130 lignes à 1 km, marge de 5 % comprise : moins de 200.
  status: résolu, 2026-11-07
  resolution: Pas un bogue de Cassini. La couche `cours_d_eau` de la BD TOPO garde les rivières entières, jusqu'à Y = 6 249 877 : l'emprise réelle était de 196 × 352 km, et le refus était juste. Les cours d'eau du jeu de test sont découpés sur le 74 (`annecy/cours-d-eau-74.gpkg`), ce qui ramène l'emprise à 125 × 100 km. Même phrase, vrai modèle : PDF A3 carroyé en UTM 32N, rejeu identique.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-second-jeu-de-test-reel-annecy-pret-pour-la-demonstration-plan.md`
  summary: Dans l'export PDF, la légende déborde sur la flèche du nord dès que le projet a beaucoup de couches ou de catégories ; elle n'est ni bornée en hauteur ni répartie en colonnes.
  evidence: Export A3 du projet d'Annecy (12 couches, 9 catégories), 2026-11-07, rendu par pypdfium2. `qgis-plugin/cassini_qgis/layout_export.py` place la légende en (215, 22, 72, 100) et la flèche en (215, 130) sans tenir compte de la hauteur réelle de la légende.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-second-jeu-de-test-reel-annecy-pret-pour-la-demonstration-plan.md`
  summary: Le quadrillage de l'export PDF ne montre aucune coordonnée sur les bords quand QGIS a ses polices (plateforme Windows), alors que l'export demande des étiquettes à gauche et en bas.
  evidence: Même export d'Annecy, en UTM 32N à 10 km et à 1 km, avec QT_QPA_PLATFORM=windows : lignes tracées, aucune étiquette. Sous offscreen, des pavés apparaissaient dans la marge gauche (les polices y manquent). À vérifier aussi dans QGIS avec son interface.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-second-jeu-de-test-reel-annecy-pret-pour-la-demonstration-plan.md`
  summary: Le modèle choisit parfois un fuseau UTM sans rapport avec le lieu de la carte pour « un carroyage UTM » (EPSG:32621 au lieu de 32632 à Annecy, 1 essai sur 3) ; il faut des exemples d'entraînement où le fuseau se déduit de l'emprise du projet.
  evidence: Sonde 11.18, phrase « Exporte la carte en A3 paysage avec un carroyage UTM tous les 1 km » : 32632 à deux essais, 32621 au troisième, puis refus « plus de 200 lignes » parce que le cadre est à 5 000 km. À traiter avec le prochain ré-entraînement groupé.
  status: résolu côté code, 2026-11-08 (`plan-fuseau-utm-depuis-l-emprise.md`) ; le modèle l'apprend au ré-entraînement groupé 11.23
  resolution: L'extension ajoute à l'emprise de `project.inventory` sa version en degrés WGS84 reprojetée par QGIS (`extent.lonlat`) ; le noyau en tire le code EPSG du fuseau UTM du centre et le montre au modèle (`project.utm_epsg`, 32632 sur le projet d'Annecy, sonde QGIS 3.44 du 2026-11-08). Le générateur recopie ce code pour un carroyage « UTM » sans fuseau écrit, et garde la question quand l'inventaire n'en a pas ; un fuseau écrit l'emporte toujours. Apprentissage : 2026-11-09, entrée 11.23 (`epic-geometrie-et-terrain-etendu/story-re-entrainement-groupe-points-tels-qu-ecrits-et-commandes-de-plan.md`), partie A : famille `carroyage_utm_projet` (100 exemples à la graine 1, 65 recopies de `utm_epsg` et 11 questions au jeu par défaut, au lieu de 5 et 3), nom du projet cohérent avec son fuseau.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-second-jeu-de-test-reel-annecy-pret-pour-la-demonstration-plan.md`
  summary: À la réouverture d'un projet, le panneau n'affiche que l'historique de la session en cours, alors que le journal garde toutes les étapes passées (`journal.jsonl` complété, numérotation reprise) ; l'utilisateur ne retrouve pas ce qu'il a fait.
  evidence: Remarque du propriétaire (2026-11-07). `core/src/journal/mod.rs` rouvre le journal en ajout (`count_lines + 1`) ; `qgis-plugin/cassini_qgis/panel.py` ne montre que le « journal de la session ». Il faut un moyen pour le panneau de relire le journal du projet (message du canal ou commande), à décider au regard d'AD-12 et du catalogue figé.

- source_plan: `_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-second-jeu-de-test-reel-annecy-pret-pour-la-demonstration-plan.md`
  summary: Idée du propriétaire : une commande qui applique aux couches du projet une mise en forme conforme aux conventions des cartes topographiques (eau en bleu, routes selon leur importance, limites, PLU aux couleurs CNIG, relief), pour que l'IA puisse la demander.
  evidence: Le jeu d'Annecy a dû être mis en forme à la main (script `%TEMP%\\sonde-11-18\\style.py`) pour donner une carte lisible : les couches ouvertes brutes, MNT par-dessus, ne montraient rien. Commande nouvelle, donc changement du catalogue figé.

- source_plan: `_bmad-output/initiative-cassini-mvp/plan-inventaire-garde-entre-les-phrases.md`
  summary: Calculer l'inventaire du projet en tâche de fond (QgsTask, lecture par QgsVectorLayerFeatureSource) dès l'ouverture du projet ou l'ajout d'une couche, pour que la première phrase ne paie plus la première lecture des grosses couches.
  evidence: Annecy, 2026-11-08 : la 1re phrase prend 32,5 s, dont environ 24 s de lecture des valeurs de champs (250 000 routes, 600 000 bâtiments, 32 000 parcelles) ; les suivantes 4,3 s et 2,9 s, l'inventaire étant gardé. L'exigence est de moins de 30 s.

- source_plan: `_bmad-output/initiative-cassini-mvp/plan-points-d-un-autre-systeme-pour-le-terrain.md`
  summary: Le garde-fou AD-23 (`core/src/point.rs`, `guard_coordinates`) transforme en question un plan juste qui mêle un point reprojeté (`step:k.point` d'un `data.transform_point`) et un point écrit tel quel, par exemple un chemin de moindre coût du WGS84 vers le Lambert-93.
  evidence: Essai du générateur (2026-11-08) : « Quel est le chemin le moins coûteux pour aller du point de longitude 2,2038 et de latitude 48,6378 (WGS84) au point X = 660300, Y = 6865780 en Lambert-93… » → plan transform puis least_cost_path (start = step:1.point, end écrit) rendu en question « Il manque à la demande un point en coordonnées ». Quand les deux points sont reprojetés, le plan passe. À vérifier dans le produit, puis corriger dans le garde-fou.
- source_plan: `epic-geometrie-et-terrain-etendu/story-table-de-semiologie-et-reconnaissance-dans-le-noyau-plan.md`
  summary: Les teintes d'altitude ont des paliers fixes en mètres : un MNT de plaine (Villeneuve, 30 à 100 m) paraît presque uni. Il faudrait étirer la rampe aux altitudes minimale et maximale réelles du MNT, ce qui demande que `project.inventory` rende ces deux valeurs pour un raster (changement du catalogue).
  evidence: Relecture de 11.20 (blind-hunter) : avec des paliers à 0, 100 et 200 m, toute la plage de Villeneuve tombe dans un seul intervalle.

- source_plan: `epic-geometrie-et-terrain-etendu/story-table-de-semiologie-et-reconnaissance-dans-le-noyau-plan.md`
  summary: Le dégradé `red_blue` ne passe pas par la couleur claire du milieu de RdBu : `ramp` n'a que `from` et `to`.
  evidence: Relecture de 11.20 : l'interpolation linéaire de `#b2182b` à `#2166ac` donne des violets au lieu du blanc cassé `#f7f7f7`. Il faudrait un champ facultatif `mid`.

- source_plan: `epic-geometrie-et-terrain-etendu/story-table-de-semiologie-et-reconnaissance-dans-le-noyau-plan.md`
  summary: Les zones du PLU sont dessinées opaques, alors que le CNIG préconise 40 % de transparence : `categories` n'a pas d'`opacity`.
  evidence: Relecture de 11.20 (rapport de construction).
  resolution: Résolu par 11.21 (2026-11-08) : `categories` porte une `opacity` facultative, et la table pose 0,6 (40 % de transparence) sur les zones du PLU.

- source_plan: `epic-geometrie-et-terrain-etendu/story-table-de-semiologie-et-reconnaissance-dans-le-noyau-plan.md`
  summary: Le générateur refuse encore « change le style d'une couche » (`training/src/families.rs`, raison « Aucune commande du catalogue ne change le style d'une couche »), devenu faux avec `map.topographic_style` et `map.graduated_style`.
  evidence: Rapport de construction de 11.20. À corriger dans l'entrée du prochain ré-entraînement groupé, avec les exemples des deux commandes et l'essai `style_reel`.
  status: résolu, 2026-11-09, entrée 11.23 (`epic-geometrie-et-terrain-etendu/story-re-entrainement-groupe-points-tels-qu-ecrits-et-commandes-de-plan.md`), partie A
  resolution: Le cas 8 de `refus` rend `NO_CHOSEN_COLOR` (couleur choisie à la main : le catalogue sait les couleurs topographiques et le dégradé) ; familles `style_topographique` et `style_degrade` ; garde-fou `styles_no_layer` sur toutes les familles ; essai réel `ai/tests/style_reel.rs` (10 phrases, barre 9 / 10).

- source_plan: `epic-geometrie-et-terrain-etendu/story-table-de-semiologie-et-reconnaissance-dans-le-noyau-plan.md`
  summary: `map.graduated_style` ne sait pas inverser un dégradé (« du bleu au rouge »).
  evidence: Relecture de 11.20 (blind-hunter).

- source_plan: `epic-geometrie-et-terrain-etendu/story-gestes-layer-set-style-et-layer-arrange-dans-l-extension-et-plan.md`
  summary: Essayer les gestes de style et la légende d'altitude (légende Python, moment du filtre par le contenu de la carte, titre caché) sous QGIS 4.x ; seule la série 3.44 a été vérifiée.
  evidence: Relecture de 11.21 ; la sonde `probe_style.py` n'a tourné que sous QGIS 3.44.15.
- source_plan: `epic-geometrie-et-terrain-etendu/story-gestes-layer-set-style-et-layer-arrange-dans-l-extension-et-plan.md`
  summary: Les libellés de légende « Altitude (m) » et « Ombrage » sont écrits dans l'extension ; le noyau devrait les dicter dans le `StyleSpec` (AD-5).
  evidence: Relecture de 11.21 ; demande du propriétaire de corriger la légende du MNT, faite au plus court dans l'extension.
- source_plan: `epic-geometrie-et-terrain-etendu/story-gestes-layer-set-style-et-layer-arrange-dans-l-extension-et-plan.md`
  summary: La valeur de catégorie « » (vide) est refusée par l'extension et acceptée par le noyau ; le noyau devrait la refuser aussi.
  evidence: `fixtures/style-cases.json` porte ce cas avec `"core": null`.
- source_plan: `epic-geometrie-et-terrain-etendu/story-completion-du-plan-par-les-conversions-de-points-plan.md`
  summary: Les mesures du modèle et la composition par la CLI notent le plan brut du modèle, sans la complétion par les conversions de points ; les attendus des essais réels sur le plan complété (11.23) doivent l'appliquer.
  evidence: Relecture de 11.22 : seul `plan.propose` complète un plan ; le changement de cap du 2026-11-08 veut que `terrain_reel`, `epic11_reel` et S25 mesurent le plan complété.
  status: résolu pour les essais réels, 2026-11-09, entrée 11.23 (`epic-geometrie-et-terrain-etendu/story-re-entrainement-groupe-points-tels-qu-ecrits-et-commandes-de-plan.md`), partie A ; la composition par la CLI note toujours le plan brut
  resolution: `ai/tests/common/mod.rs` (`completed`, `complete_with`) applique `complete_conversions` au plan composé, le système d'un raster lu dans l'inventaire ; `plan_reel`, `terrain_reel`, `epic11_reel` et `style_reel` le mesurent. S25 garde son attendu complété ; un test sans modèle vérifie que le point écrit en WGS84 en est complété. `ecart_reel`, `question_reel`, `echelle_utm_reel` et `duree_reel` restent sur le plan brut : non changés ici (le jeu tenu à l'écart d'`ecart_reel` n'est pas lu par le générateur ni par ce ticket).
- source_plan: `epic-catalogue-etendu-qgis/story-traceur-reprojection-de-bout-en-bout-data-reproject-plan.md`
  summary: `data.reproject` (10.7) est construite avant les mises à niveau 10.1 à 10.6 (version de chaîne, annonce fidèle, résultat validé par le noyau, prompt filtré) ; elle passe par le chemin générique existant et devra être reprise par chacune de ces entrées.
  evidence: Dérogation décidée par le propriétaire le 2026-11-08 : construire `data.reproject` maintenant, avant 10.1-10.6 et avant les epics 6 et 7 ; l'ordre de l'epic 10 sera remis d'aplomb par `bmad-ticket`. Le ré-entraînement qui apprend les familles `reprojection_couche` et `reprojection_avant_analyse` est groupé dans 11.23 ; la phrase « Reprojette la couche des gares en WGS84 » est à essayer dans le panneau avec le modèle installé et à juger pour de bon après 11.23.
- source_plan: `plan-bug-etapes-du-plan-en-langage-courant.md`
  summary: Vérifier la pente maximale de 5 265,7 % rendue par `terrain.slope` sur `mnt-74-25m.tif` (Annecy) : artefact du MNT (bord, valeur de remplissage) ou du calcul de pente du noyau.
  evidence: Captures du panneau du 2026-11-08, « Fais la carte des pentes du MNT à 25 m en pourcentage » ; la phrase ne fait que recopier le maximum du résultat.
- source_plan: `epic-refonte-du-panneau/story-guidage-a-vous-repere-d-avancement-et-reformuler-plan.md`
  summary: Le texte indicatif de la saisie grisée (« Validez ou refusez d'abord le plan ci-dessus. », « Patientez : une étape est en cours. ») est coupé sur sa deuxième ligne à 144 dpi : la hauteur de la saisie ne suit pas.
  evidence: Captures du 2026-11-08 à 144 dpi (`%TEMP%\capture-panneau\planche-12-2-relecture.png`), états plan proposé et exécution.
- source_plan: `_bmad-output/initiative-cassini-mvp/plan-bug-fond-noir-des-rasters-de-resultat.md`
  summary: sonder le style `result_ramp` sous QGIS 4.x (transparence, rampe, légende continue) et rendre exposition, coût et superposition dans QGIS.
  evidence: la sonde du 2026-11-09 n'a tourné que sous QGIS 3.44 et seulement pour champ de vision, cumulé et pente ; les tests de l'extension passent par un faux moteur.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-catalogue-etendu-qgis/story-conversion-de-format-data-convert-plan.md`
  summary: un plan peut passer `step:k.file` (fichier écrit par une étape) au chemin de `layer.add` ; au rejeu, `layer.add` relit le fichier du dossier de travail d'origine, pas celui que le rejeu vient de récrire sous `replay/`.
  evidence: `resolve_references` met le chemin absolu du dossier de travail ; le contrôle d'entrée ne regarde que la taille et la date du fichier principal. Non atteint par le générateur aujourd'hui ; `check_plan` ne l'interdit pas.
- source_plan: `plan-memoire-entrainement.md`
  summary: Deux pistes d'accélération de l'entraînement reportées par le propriétaire le 2026-11-09 : (a) lot 2 avec accumulation 8 (même lot effectif de 16), à garder seulement s'il reste sous 8 Go sur l'échantillon « longs d'abord » et s'il est plus rapide ; (b) le coût fixe d'environ 10 min autour de l'entraînement (garder la base GGUF f16 et Q4_K_M convertie, ne pas reconstruire `llama-server` quand seul le chemin du venv change, `llama-completion` sur CUDA pour l'essai final, caches de compilation d'Unsloth et de torch hors de `/tmp`).
  evidence: Mesure partielle avant le report : lot 2 sur les exemples les plus longs, 28 s par pas sur les 10 premiers (contre 19 s en lot 1, compilation comprise), pic 5,89 Gio : plus lent. Dans `regroupe-11-23c`, `llama-server` a été reconstruit (6,5 min) parce que le chemin de nvcc avait changé, et la conversion puis la quantification de la base ont coûté 1 min 40. Une ébauche du code des deux pistes est gardée hors dépôt, non relue : `~/cassini-entrainement/pipeline-differe-memoire.py`.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-catalogue-etendu-qgis/story-retirer-une-couche-du-projet-layer-remove-plan.md`
  summary: décision du propriétaire — `layer.remove` sur une couche en mémoire (son contenu est perdu sans recours) ou en cours d'édition (modifications non enregistrées perdues) : refuser par un code nommé, prévenir dans le plan, ou laisser faire comme aujourd'hui.
  evidence: `QgsProject.removeMapLayer` détruit la couche ; la fiche du 2026-11-10 n'admet aucune erreur propre. Cassini n'ouvre ses couches qu'en lecture seule et écrit ses résultats en fichiers : le cas vient d'une couche créée ou éditée à la main.
  status: close, 2026-11-10, entrée 27 de l'epic 10 (`plan-10-27-refus-retrait-couche-non-enregistree.md`)
  resolution: Refus par `layer.unsaved` (changement de cap `change-refus-retrait-couche-non-enregistree`, approuvé le 2026-11-10) : la couche en mémoire ou en édition modifiée reste au projet, intacte ; sonde QGIS 37/37.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-catalogue-etendu-qgis/story-retirer-une-couche-du-projet-layer-remove-plan.md`
  summary: `check_plan` admet `step:k.layer_id` d'une étape `layer.remove` (sortie `none`) : l'enchaînement sur une couche retirée n'échoue qu'à l'exécution, par `layer.not_found`.
  evidence: le schéma de sortie de `layer.remove` porte `layer_id` ; une règle sur les sorties `none` toucherait aussi `map.graduated_style`, dont `step:k.layer_id` désigne une couche existante : décision d'architecture.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-catalogue-etendu-qgis/story-retirer-une-couche-du-projet-layer-remove-plan.md`
  summary: famille `retrait_couche` à compléter avant le prochain entraînement groupé, avec le propriétaire : retrait d'un raster (« retire le MNT du projet »), de plusieurs couches (une étape par couche), verbe nu sans « du projet » ni « de la carte ».
  evidence: la famille tire des couches vecteur seulement et toujours avec « du projet » ou « de la carte » ; le modèle n'a aucun exemple de ces formes.
  status: close, 2026-11-10, générateur après les quatre causes (`plan-generateur-apres-quatre-causes.md`)
  resolution: `retrait_couche` gagne le raster, plusieurs couches (une étape par couche) et le verbe seul → question ; poids 3 → 6, décision du propriétaire du 2026-11-10.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-recette-qgis-d-annecy-portee-a-20-phrases-barre-16-20-plan.md`
  summary: le rejeu d'un journal qui porte `project.inventory` (ou toute lecture du projet) diverge dès l'étape 1 quand il est lancé en fin de session : l'inventaire voit les couches de résultat ajoutées depuis, son empreinte change (`replay.diverged`).
  evidence: recette d'Annecy à 20 phrases du 2026-11-10, deux passages : `replay.diverged` à l'étape 1, `project.inventory` ; les recettes à 5 et 10 phrases n'avaient pas de lecture du projet. Décision au propriétaire : rejouer sur une copie neuve du projet (fermeture et réouverture), ne pas comparer le résultat des lectures du projet, ou ne pas journaliser ces lectures.
- source_plan: `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-recette-qgis-d-annecy-portee-a-20-phrases-barre-16-20-plan.md`
  summary: décision du propriétaire — RA13 (« coordonnées UTM d'un point WGS84 ») : accepter aussi `data.transform_point` vers 32632, qui rend le même point, à côté de `coord.convert` `to: ["utm"]`, ou garder un seul plan attendu.
  evidence: mesure de référence du 2026-11-10 (17/20) : le modèle installé rend `data.transform_point` vers 32632, compté raté par la règle stricte.
