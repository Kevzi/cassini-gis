# Revue par grille (good-spine checklist) — delta E1 du 2026-10-18

Objet : le delta du contrat `architecture-noyau-et-facade-qgis.md` daté du 2026-10-18
(précision d'AD-6, écart d'AD-15, AD-22 liée à CAP-14, AD-24, AD-25, AD-26, conventions
« Familles de commandes » et « Réglages », ligne CAP-14 de la carte, deux entrées Deferred).
Jugé contre le reste du contrat, la spec (CAP-14 et contraintes), l'epic 10 (E1–E4), et le
code réel : `qgis-plugin/cassini_qgis/analysis.py`, `core/src/commands/`, `core/src/serve.rs`,
`ai/src/plan.rs`, `docs/canal-local.md`, ticket 14 de `epic-agent-et-mcp/tickets.toml`.

## Verdict

Le delta va dans la bonne direction : il ratifie fidèlement le motif existant (fiche Rust
typée + ligne de la table fermée `ALGORITHMS`, résumé borné avec `truncated`), tranche le
nommage et ratifie l'écart d'AD-15 conformément au ticket 14. Il n'est pas encore un substrat
suffisant pour découper E2–E4 : la forme de la table n'accueille ni entrée raster, ni sortie
autre qu'une couche vecteur, ni format de sortie variable ; les chaînes contredisent AD-10 sur
les intermédiaires et sont muettes sur AD-12 ; AD-25 promet une chose que le code du prompt ne
fait pas ; AD-26 contredit la spec et `layer.add`, et ne dit pas comment ses fichiers d'entrée
sont désignés. Aucun constat critique, sept constats hauts.

## Constats

### Haut

**H1 — La table fermée n'a pas la forme des primitives d'E2 ; AD-24 ne fixe pas ses points d'extension.**
Constat : `ProcessingGesture._begin` exige une couche d'entrée vecteur (`is_vector`), une
éventuelle seconde couche vecteur (`_overlay`), et `QgisBackend.load_output` ouvre la sortie
en `QgsVectorLayer(..., "ogr")`. Côté noyau, `CommandSpec.output_ext` est une chaîne unique
par commande. Or E2 contient : `raster.zonal_statistics` (seconde entrée **raster**, refusée
par `_overlay`) ; l'atlas PDF (entrée = une mise en page, sortie = un PDF, pas une couche ; la
mise en page est aujourd'hui un geste à part, `layout_export.py`, hors table) ; la conversion
de format (extension de sortie choisie par paramètre, incompatible avec un `output_ext`
unique ; un Shapefile laisse des fichiers annexes que `_remove` ne nettoie pas) ;
l'analyse du plus proche voisin (`qgis:nearestneighbouranalysis` rend des nombres et un HTML,
pas de couche). Chaque ticket d'E2 inventera donc sa clé de table ou son geste à part : c'est
exactement la divergence qu'AD-24 dit prévenir (« deux façons d'ajouter une commande
déléguée »).
Correctif : ajouter à AD-24 une phrase qui ferme le vocabulaire de la ligne : « une ligne
déclare le **genre** de chaque entrée (`vector`, `raster`, `layout`) et le **genre** de sa
sortie (`layer`, `table`, `file`) ; un genre nouveau s'ajoute dans `ProcessingGesture`, une
fois, par le premier ticket qui en a besoin, jamais par un geste à part ». Et trancher le cas
du format variable : soit une commande par format de sortie (`data.convert_to_geopackage`,
`data.convert_to_shapefile`…), soit `output_ext` devient une énumération fermée portée par un
paramètre typé, le noyau gardant le nom (AD-10) et la liste des fichiers annexes à nettoyer.

**H2 — Les intermédiaires « temporaires » d'une chaîne contredisent AD-10 sans le dire.**
Constat : AD-10 : « toute sortie — couche créée, PDF, **fichier intermédiaire** — est nommée
par le noyau dans un dossier de travail […] rien n'est écrit en dehors de ce dossier ». AD-24 :
« les intermédiaires étant temporaires ». Dans le code, l'étape 1 écrit `TEMPORARY_OUTPUT` :
couche en mémoire pour un algorithme `native:`, mais **fichiers dans le dossier temporaire du
système** pour un algorithme GRASS — hors du dossier de travail, non nommés par le noyau.
Correctif : écrire l'écart explicitement dans AD-24 : « Écart à AD-10 : les intermédiaires
d'une chaîne sont des sorties temporaires du cadre de traitement (`TEMPORARY_OUTPUT`), jamais
nommées par le noyau ni journalisées, libérées par QGIS ; elles ne contiennent que des données
dérivées et ne sont jamais rendues au noyau. » Ou, à défaut, exiger que le noyau nomme chaque
intermédiaire (`output_params` multiples). Une des deux, pas le silence.

**H3 — Annulabilité (AD-12) des chaînes et des commandes GRASS : muette.**
Constat : toutes les commandes déléguées sont déclarées `.cancellable(true)` côté Rust, alors
que la chaîne vit côté extension. Un algorithme sans `Qgis.ProcessingAlgorithmFlag.CanCancel`
(fréquent hors `native:`, GRASS compris) n'interrompt pas son travail ; le noyau attend
`CANCEL_GRACE` (5 s, `docs/canal-local.md`) puis supprime les sorties — sous Windows, le cas
exact qu'AD-10 interdit (fichier tenu ouvert). AD-12 exige la déclaration « non annulable au
registre », mais rien ne relie la déclaration Rust au contenu de la chaîne Python.
Correctif : dans AD-24, « une commande n'est déclarée `cancellable` que si **chaque**
algorithme de sa chaîne porte le drapeau `CanCancel` ; l'annulation prend effet au plus tard
entre deux étapes ; un test de l'extension vérifie, ligne par ligne, la cohérence entre la
table et le drapeau `cancellable` du catalogue reçu à `session.hello` ».

**H4 — AD-25 promet « l'agent ne la planifie pas », mais le modèle voit tout le registre.**
Constat : `ai/src/plan.rs::fixed_system_text` construit le catalogue montré au modèle avec
`registry.specs()` — toutes les commandes, disponibles ou non — et c'est voulu : cette partie
fixe est gardée en cache parce qu'elle « ne dépend que du registre » (5.12). Un plan qui nomme
une commande non annoncée est ensuite refusé (`serve.rs`, test
`a_plan_naming_an_unannounced_command_is_refused_naming_the_step`). Avec des commandes
dépendant de GRASS, l'utilisateur sans GRASS attendra une composition entière pour obtenir un
refus, au lieu d'un plan ou d'une question justifiée (seuil de 70 % de CAP-14).
Correctif : trancher dans AD-25 l'un des deux : (a) « le catalogue montré au modèle ne
contient que les commandes disponibles de la session ; la partie fixe du prompt est mise en
cache par couple (registre, ensemble annoncé) » ; ou (b) reformuler en « un plan qui nomme une
commande indisponible est refusé avant d'être montré, en la nommant ». (a) est cohérent avec
AD-7 ; il doit être décidé avec le Deferred « Taille du prompt » (voir M1).

**H5 — AD-26 contredit la spec CAP-14 et la frontière de l'epic, sans conflit de source déclaré.**
Constat : spec, CAP-14, *success* : « chaque primitive retenue est une commande du registre
**exécutée par le cadre de traitement de QGIS** ». Epic 10, *Boundaries* : « Pas de nouveau
calcul dans le noyau ». Spec, contraintes : « la lecture des formats [est] déléguée à QGIS ».
AD-26 met le géocodage dans le noyau, qui lit lui-même deux CSV. La décision est défendable
(AD-5 : l'appariement est un raisonnement), mais le contrat lie CAP-14 en la contredisant.
Correctif : ajouter à AD-26 une ligne « Conflit de source : la spec (CAP-14, *success*) et
l'epic 10 (*Boundaries*) voulaient tout par QGIS ; le géocodage fait exception parce que
[raison] ; à reporter dans la spec par `bmad-spec` et dans l'epic avant l'inception d'E4 ».

**H6 — AD-26 ne fixe pas comment le fichier d'adresses et le fichier BAN sont désignés.**
Constat : le noyau doit lire des fichiers, donc recevoir des chemins (AD-9) ; mais AD-23
n'autorise dans un plan que « des chemins déjà connus de la session », et `project.inventory`
ne rend que le **nom** du fichier source, sans chemin. Un fichier BAN de plusieurs centaines
de mégaoctets ne sera pas ouvert comme couche ; il sera souvent hors de l'arborescence du
projet (journalisé non portable, AD-9). Un ticket prendra un `layer_id` (que le noyau ne peut
pas lire), un autre un chemin tapé par l'utilisateur (que le modèle recopierait).
Correctif : dans AD-26, « les entrées de `geocode.*` sont des chemins relatifs au projet
(AD-9), connus de la session soit par l'inventaire, qui rend alors le chemin relatif de la
source d'une couche CSV ouverte, soit par un choix de fichier fait par l'utilisateur dans le
panneau et transmis au noyau ; le modèle ne compose jamais un chemin ». Préciser aussi les
paramètres de lecture (séparateur, encodage, noms de colonnes) comme paramètres typés.

**H7 — « Un CSV à coordonnées qui déclare son code EPSG » contredit `layer.add`.**
Constat : `core/src/commands/layer_add.rs` : `crs` est « **obligatoire pour un CSV**, qui n'en
porte jamais » ; colonnes par défaut `X` et `Y`. Un CSV ne peut pas « déclarer » d'EPSG. De
plus AD-26 dit « `layer.add` l'ouvre ensuite » sans dire qui enchaîne : le modèle dans le plan,
ou le noyau d'office (AD-5 interdit seulement à l'extension d'enchaîner).
Correctif : « le résultat typé de `geocode.*` porte `out`, `crs` (ex. `EPSG:4326` si l'on
retient les colonnes lon/lat de la BAN, à vérifier sur la documentation officielle du format),
`x_field`, `y_field`, et des dénombrements (appariées, non appariées, liste bornée des non
appariées avec `truncated`) ; l'ouverture est une étape `layer.add` du plan qui cite ces champs
par référence `step:k.champ` ». Choisir une seule colonne de coordonnées : les x/y Lambert de la
BAN changent de système outre-mer, lon/lat non.

### Moyen

**M1 — Le Deferred « Taille du prompt » laisse diverger les tickets d'E2.**
Constat : registre actuel hors tests : 11 commandes ; E2–E4 en ajoutent une douzaine. Le seuil
de 20 sera franchi **au milieu d'E2**, pas après. Et rien ne dit si une commande ajoutée
apporte un exemple à la partie fixe du prompt (`examples()` dans `ai/src/plan.rs`), une phrase
au jeu de 5.10, ou seulement sa description : chaque ticket fera autrement, et le prompt
grossira selon l'humeur du ticket.
Correctif : sortir de Deferred la règle de contenu par commande : « une commande ajoutée
n'apporte **aucun** exemple à la partie fixe ; elle apporte sa description bilingue et au
moins deux phrases au jeu d'épreuve et de ré-entraînement de 5.10 ». Garder en Deferred le
seul choix tous/sous-ensemble, mais le dater : décidé à l'inception, avant le premier ticket
d'E2, et lié à H4.

**M2 — « Tout seuil est un paramètre » est invérifiable tel quel, et le code existant le contredit.**
Constat : `analysis.py` : `_ORIGIN_SNAP_METERS = 100.0` ; isochrone à vitesses fixes 5 et
50 km/h, `TOLERANCE: 0`, et la valeur de 100 m est recopiée dans la description Rust de
`analysis.isochrone`. AD-24 dit « tout seuil », sans borne de portée (la spec dit « d'une
commande de CAP-14 »).
Correctif : définir le terme : « un seuil est une valeur qui change la réponse à la question
de l'utilisateur (distance, pente, surface, tolérance d'accrochage, score d'appariement) ; un
réglage technique de l'algorithme (stratégie, sens par défaut) reste une constante de la
traduction ». Dire que la règle vaut pour CAP-14 et que l'isochrone de CAP-3 garde ses
constantes par écart assumé (ou ouvrir un ticket). La tolérance d'accrochage d'E3 et le seuil
de score d'E4 sont nommés explicitement comme paramètres.

**M3 — La liste blanche n'est pas vérifiable : aucune liste, aucun test.**
Constat : « aucune ligne ne nomme un algorithme qui accède au réseau » sans dire lesquels.
Le test `requestAboutToBeCreated` d'AD-14 ne voit pas un fournisseur tiers qui utilise ses
propres bibliothèques Python ni un exécutable externe.
Correctif : « la recette parcourt chaque algorithme de chaque chaîne de la table et refuse :
une liste nommée et versionnée d'identifiants interdits (`native:filedownloader`,
`native:batchnominatimgeocoder`, les requêtes HTTP du cadre de traitement — identifiants à
vérifier dans la documentation de QGIS 3.44 et 4.x) et tout fournisseur hors `native`,
`qgis`, `gdal`, `grass` sauf ajout motivé au contrat ; chaque ligne passe une fois sous le test
de zéro requête d'AD-14, sur fixture, dans QGIS 3.44 et 4.x ».

**M4 — AD-25 : moment et profondeur de la vérification.**
Constat : (1) les fournisseurs Python (GRASS en particulier) sont chargés par des extensions
de QGIS ; si `session.hello` part avant leur chargement, la commande est déclarée indisponible
pour toute la session, et AD-25 interdit de revérifier. (2) Un algorithme peut exister mais
avoir renommé un paramètre entre 3.44 et 4.x (AD-18) : il est annoncé, puis échoue.
Correctif : « l'annonce part après `iface.initializationCompleted` ; un ajout ou retrait de
fournisseur (`QgsProcessingRegistry.providerAdded/providerRemoved`) rouvre la session par un
nouveau `session.hello` hors plan en cours ; l'existence vérifiée couvre l'algorithme **et**
chaque nom de paramètre que la traduction remplit (`parameterDefinition`) ». Ajouter :
« à disponibilité égale, une chaîne préfère `native:`/`qgis:` à un fournisseur optionnel »,
pour qu'E3 ne choisisse pas GRASS par défaut.

**M5 — Rejeu (AD-11) du géocodage : déterminisme non exigé.**
Constat : `docs/canal-local.md` : « les sorties du noyau restent comparées octet par octet ».
Le CSV de `geocode.*` le sera donc ; AD-26 ne demande ni ordre stable, ni format de nombre
fixe, ni départage des ex aequo.
Correctif : « la sortie de `geocode.*` est déterministe : lignes dans l'ordre du fichier
d'adresses, coordonnées écrites avec un nombre fixe de décimales, ex aequo départagés par une
clé stable (identifiant BAN), aucune date ni aucun chemin dans le fichier ».

**M6 — Annulation et blocage du noyau pour `geocode.*`.**
Constat : `docs/canal-local.md` : « Aucune commande du noyau (`core`) n'est annulable », et la
boucle principale exécute une étape à la fois. Apparier un fichier BAN départemental (des
centaines de milliers de lignes) bloque le noyau, contre l'esprit d'AD-4, et AD-26 ne dit pas
s'il est annulable.
Correctif : trancher dans AD-26 : soit « `geocode.*` est la première commande `core`
annulable : drapeau d'annulation consulté toutes les N lignes, CSV partiel supprimé, ligne de
journal `cancelled` », soit « non annulable au registre (AD-12), avec une borne de taille
d'entrée refusée par une erreur nommée ».

**M7 — `raster.*` peut dupliquer `terrain.*`.**
Constat : la convention réserve `terrain.*` au noyau, mais rien n'empêche un ticket d'ajouter
`raster.slope` ou un champ de vision par `gdal:`/GRASS : deux implémentations d'une même
fonction, ce que CLAUDE.md §4 interdit.
Correctif : « aucune commande `raster.*` ne recouvre une commande `terrain.*` (pente, champ de
vision, ligne de vue) ; le recouvrement se signale avant d'agir ».

### Bas

**B1 — La précision d'AD-6 est inexacte et incomplète.** `plan.execute` et `journal.replay`
agissent sur le projet (par des commandes), et le rejeu n'écrit pas le journal ;
`command.run` et la notification `plan.step` manquent à l'énumération. Correctif : « n'agissent
sur le projet et les données **que par des commandes du registre** » ; renvoyer à
`docs/canal-local.md` comme liste fermée des méthodes, `command.run` compris.

**B2 — `model.list` « reste au registre pour la ligne de commande ».** AD-13 interdit une
commande propre à un point d'entrée ; or `model.list` est offerte au modèle et à MCP, et rend
`folder`, un chemin absolu (ce que le ticket 14 reproche à `command.run model.list`), contre
AD-9. Correctif : retirer `folder` du résultat (ou le rendre relatif) et écrire « commande
ordinaire du registre, utile surtout à la ligne de commande ».

**B3 — Diagramme de séquence périmé.** Il montre `agent.plan(demande)` ; la méthode est
`plan.propose` et `agent.plan` a été retirée des exemples de nommage. Correctif : renommer.

**B4 — Carte CAP-14 incomplète.** Il manque AD-5 (chaîne sans décision), AD-10 (sorties),
AD-11 (rejeu), AD-12 (annulation), AD-14 (recette hors ligne, exigée par CAP-14 *success*),
AD-16 (fournisseurs GPL), AD-7. Correctif : ajouter ces références à la ligne.

**B5 — « Ne s'exécutent que dans le processus de QGIS ».** GRASS tourne en exécutable lancé
par QGIS, pas dans son processus. Correctif : « lancés par QGIS, jamais liés ni lancés par le
noyau ».

**B6 — Version de la chaîne absente du journal.** AD-11 journalise les versions du noyau, du
protocole et du registre, pas celle de l'extension ni de QGIS, alors que la chaîne d'une
commande vit dans l'extension. Correctif : `session.hello` porte la version de l'extension et
celle de QGIS, recopiées dans chaque ligne du journal, pour qu'un `replay.diverged` soit
diagnosticable.

**B7 — Vérification intermédiaire d'une chaîne.** « Ni branchement ni choix » n'autorise pas
littéralement le contrôle `empty_error` existant. Correctif : « un contrôle qui arrête la
chaîne par une erreur nommée (AD-21) est permis ; un choix entre deux suites ne l'est pas ».

**B8 — Hors contrat, à corriger ailleurs.** L'epic 10 déclare `covers: [CAP-3, CAP-6, CAP-9]`
au lieu de CAP-14 ; le ticket de renommage d'`analysis.zonal_statistics` n'existe que dans
`.memlog.md`.

## Points vérifiés sans constat

- AD-15 (écart) correspond au ticket 14 et au code : `CASSINI_MODELS`, tri par nom de
  fichier (« le premier » est donc déterministe), aucun chemin dans `agent.models`.
- Convention « Réglages » : ratifie `model_list.rs` (variable d'environnement, aucun fichier
  de configuration).
- Résumé borné : ratifie `analysis_zonal_statistics.rs` (`length(max = 100)`, `truncated`) ;
  compatible avec AD-8 et AD-21 puisque la couche garde tout.
- AD-22, AD-23, AD-1, AD-16, AD-19, AD-20 : aucune nouvelle AD ne les affaiblit ; AD-26
  garde le noyau testable sans QGIS (AD-1) et sans reprojection (AD-19).
