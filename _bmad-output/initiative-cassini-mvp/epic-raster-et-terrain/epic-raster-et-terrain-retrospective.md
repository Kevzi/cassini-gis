---
epic: epic-raster-et-terrain
date: 2026-11-05
verdict: accepted-with-open-items
criteria: declared
headless: false
---

# Rétrospective — epic 4 : Analyse de terrain dans le noyau

## Epic summary

Epic `epic-raster-et-terrain` (id 4, risque `high`, couvre CAP-11). Dix entrées, toutes à
`state = done` d'après `tickets.py status` ; `pending_tickets` vide ; aucune entrée restée à
`built`. Ordre de construction : 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.9, 4.7, 4.10, 4.8.

| Réf | Titre | Risque (plan) | Relecture | Fusion |
|---|---|---|---|---|
| 4.1 | Traceur : lecture et écriture d'un MNT GeoTIFF | high | thorough | `39ee814` |
| 4.2 | `terrain.slope` | medium | quick | `9462344` |
| 4.3 | Les sorties raster s'ouvrent dans le projet QGIS | high | thorough | `d227433` |
| 4.4 | `terrain.line_of_sight` | medium | quick (2 boucles) | `cc6bd00` |
| 4.5 | `terrain.viewshed` | high | thorough (2 boucles) | `eeb5848` |
| 4.6 | Le modèle apprend les trois commandes | medium | quick | `2137597` |
| 4.9 | Bogue : le rejeu diverge quand un raster précède un export | high | thorough | `1d15656` |
| 4.7 | Comparaison à QGIS, provenance | low | quick | `98b7764` |
| 4.10 | Garde-fou : système écrit une seule fois | high | thorough | `6f81188` |
| 4.8 | Ménage de fin d'epic | low | quick | `5698fea` |

**Plages de diff.** Les `baseline_revision` des plans s'entrelacent avec le travail de l'epic 5
(5.39, 5.40, 5.41 fusionnés entre 4.1 et 4.9 sur `main`) : la règle « d'une baseline à la
suivante » aurait attribué à l'epic 4 des commits de l'epic 5. Chaque ticket a donc été mesuré
sur sa propre branche, `M^1..M^2` de son commit de fusion (`git_evidence.py`, une plage par
ticket) — écart à la méthode, consigné ici. Volumes (commits hors fusion, fichiers, lignes) :
4.1 1/9/+1230 ; 4.2 1/7/+781−13 ; 4.3 1/19/+962−38 ; 4.4 2/7/+963−7 ; 4.5 2/8/+1238−66 ;
4.6 4/21/+1916−135 ; 4.9 1/22/+1144−380 ; 4.7 1/10/+1464−2 ; 4.10 2/4/+270−50 ; 4.8 1/7/+179−21.

**Inventaire des preuves.**
- Disponibles : fichier d'epic (avec Done when), `tickets.toml`, dix plans avec journal de tri
  et vérification, `deferred-work.md`, `docs/comparaison-terrain-qgis.md`, l'historique git.
- Absents : aucun fichier d'histoire (aucune entrée n'a été tirée en story) — normal ici ;
  **journaux de session non rassemblés** : l'analyse des leçons de processus s'appuie sur les
  seuls plans (Plan Change Log, journaux de tri), pas sur le déroulé des sessions.
- Rétro précédente : l'epic précédent dans l'ordre (`epic-agent-et-mcp`, epic 5) **n'a pas de
  fichier de rétrospective**. Voir la section dédiée.

## Findings

Sources : relecture de code à trois lentilles (adversarial, edge-case, verification-gap) sur le
diff `21eaf27..HEAD` restreint à `dem.rs`, `point.rs`, `terrain_*.rs`, `layer_open_raster.rs`,
`serve.rs`, `journal/mod.rs` et `raster_output.py` (4 327 lignes) ; vues agrégées calculées sur
place. Chaque constat de relecture a été revérifié dans le code avant d'être retenu ; ceux qui
ne tenaient pas sont listés à la fin.

### Constats retenus

**F1 — La pente en degrés dépend de la bibliothèque mathématique de la plateforme** (trois
lentilles d'accord). `core/src/commands/terrain_slope.rs:107` : `p.atan().to_degrees()`. Le
ticket 4.4 a retiré `hypot` pour cette raison exacte (`terrain_line_of_sight.rs:273-275`, relecture
4.4 constat 6) ; la pente, construite avant (4.2), n'a pas reçu le même traitement, et aucun test
ne fige des octets de sortie de pente : `a_tilted_plane_has_a_uniform_slope_in_degrees`
(`terrain_slope.rs:250`) calcule son attendu avec le même `atan`. Conséquence possible : un
journal écrit sous Windows et rejoué sous Linux rend `replay.diverged` sur la pente (rare, à
cause de l'arrondi en f32, mais non exclu). *Disposition : fix now.* *Leçon : divergence de
convention entre deux tickets — la règle « que des opérations IEEE exactes dans ce qui se
rejoue » n'était écrite nulle part, seulement dans un commentaire de 4.4.*

**F2 — Un MNT en Web Mercator (EPSG:3857) est accepté comme « métrique »** (adversarial,
vérifié). `core/src/dem.rs:454-456` : quand la clé d'unité linéaire vaut le mètre, tout code
projeté passe ; la liste fermée `is_known_metric` (`dem.rs:475`) ne sert que si la clé manque.
Or les « mètres » de 3857 sont dilatés d'environ 1/cos(lat), ≈ 1,5 en France : pente sous-estimée,
distances, courbure et borne de rayon faussées, sans erreur. La décision du 2026-11-03 (« tout
MNT en système projeté métrique ») ne tranchait pas ce cas. *Disposition : fix now (refuser 3857
et ses alias, avec message de reprojection) — à confirmer par le propriétaire.*

**F3 — Le champ de vision donne « visible » avec certitude derrière un trou du MNT** (adversarial,
vérifié). `terrain_line_of_sight.rs:345-347` : un échantillon sans donnée « ne masque rien » —
choix écrit au plan de 4.4 (ligne 39 du plan), et la ligne de vue rend `nodata_samples`. Le champ
de vision hérite de la règle mais ne rend aucun indicateur par cellule (relecture 4.5, constats
14 et 27, rejetés à l'époque faute d'appelant). Pour ce domaine, un faux « visible » est l'erreur
coûteuse. *Disposition : defer, décision du propriétaire (garder, ou marquer les cellules dont
la ligne a traversé un trou).*

**F4 — Les unités verticales du MNT ne sont jamais lues** (adversarial, vérifié : aucune
occurrence de `VerticalUnitsGeoKey`/4099 dans `dem.rs`). Un MNT aux altitudes en pieds ou en
décimètres entiers est accepté ; la pente est fausse d'un facteur 3,28 ou 10. *Disposition :
defer (lire la clé et refuser autre chose que le mètre ou l'absence).*

**F5 — Le GeoTIFF écrit par le noyau n'est vérifié dans QGIS que par une sonde manuelle**
(verification-gap). `dem.rs:257-320` écrit les GeoKeys à la main ; le seul test aller-retour
(`writing_twice_gives_identical_bytes_read_back_unchanged`) relit avec le même décodeur ;
`qgis-plugin/tests/test_raster_output.py` utilise un `FakeBackend`, et `probe_raster_output.py`
ne tourne pas en CI. Atténué par la comparaison 4.7, qui a relu les sorties de QGIS sur la grille
du noyau, et par les démos réelles. *Disposition : defer, rejoint la dette connue de preuve
dynamique QGIS (`recette.yml`).*

**F6 — Une ouverture dictée qui échoue fait échouer l'étape de terrain déjà réussie et
journalisée** (edge-case et adversarial). `core/src/journal/mod.rs:276-283` (`call_logged`) :
`call_step(OPEN_RASTER)?` après une étape réussie. Le comportement est voulu (relecture 4.9,
constat 10, AD-21 : « les deux lignes sont journalisées »), mais la ligne d'erreur
`layer.open_raster` rend ensuite le journal non rejouable à l'identique — même famille que
l'entrée de `deferred-work.md` « un journal qui contient une étape déléguée en erreur ne se
rejoue pas » (relecture 5.38). *Disposition : defer, à traiter avec cette entrée existante.*

**F7 — Branche de `intermediate_files` testée sur un journal impossible** (verification-gap).
`journal/mod.rs:550-557` et `journal/tests.rs:1856` : le test fait consommer l'id du raster
ouvert par une étape de plan suivante, ce qu'aucun plan réel ne peut produire (l'ouverture
dictée n'est pas une étape référençable, `serve/plans.rs:325-332`). *Disposition : defer (ménage :
retirer la branche ou écrire le test du vrai flux).*

**F8 — Message trompeur quand un plan contient `layer.open_raster`** (verification-gap).
`serve/plans.rs:110-117` répond « l'extension ne l'a pas annoncée » alors que la commande est
dictée par le noyau ; `command.run` rend bien `registry.dictated`. Inatteignable par le modèle
(grammaire), atteignable par un plan MCP. *Disposition : defer, low.*

**F9 — Garde-fou des coordonnées, détails de détection des systèmes** (edge-case et
adversarial, vérifiés en partie). `point.rs:280-306` : `epsg2154` dans un nom de fichier
(`mnt-epsg2154.tif`) compte comme système écrit (le tiret n'est pas alphanumérique) ; « en
4326 », « IGNF:LAMB93 » ne sont pas reconnus. Effets bornés : le noyau refuse ensuite un point
hors MNT (`terrain.outside_dem`). *Disposition : defer, low.*

**F10 — Valeurs infinies dans le MNT** (edge-case). `dem.rs:229` masque NaN et la valeur
sans donnée, pas ±inf. Rare dans un MNT réel. *Disposition : accept, noté.*

### Vues agrégées

- **Delta d'architecture.** Aucune dépendance nouvelle hors `tiff` 0.11.3 (MIT) ;
  `cargo deny check licenses bans` vert ; ni GDAL, ni GEOS, ni PROJ dans l'arbre de
  `cassini-core`. Une seule porte raster (`core/src/dem.rs`, AD-20). Le noyau ne dépend pas de
  QGIS : l'ouverture des rasters passe par une commande déléguée dictée (`layer.open_raster`, 4.9).
- **Carte de duplication.** Pas de duplication trouvée : `terrain.viewshed` réutilise le noyau
  `Line`/`sight` et `check_crs`/`check_extent` de la ligne de vue (`terrain_viewshed.rs:32`),
  et deux tests le vérifient cellule par cellule.
- **Croissance de taille.** `dem.rs` 1 103 lignes (22 tests), `point.rs` 1 299 (21 tests),
  `terrain_viewshed.rs` 892, `terrain_line_of_sight.rs` 865 : tailles raisonnables, tests compris.
  `serve.rs` (3 472 lignes) n'a pris que +203 nettes ; son code tient en 993 lignes, le reste est
  le module de tests. `training/src/families.rs` (4 209 lignes, +560 nettes dans l'epic) est le
  seul fichier à surveiller.
- **Divergence de patron.** F1 : `hypot` retiré en 4.4 pour le déterminisme, `atan` gardé en 4.2.
- **Réconciliation spec / réalisé.**
  - CLAUDE.md §3 dit « analyse de terrain reprise ou réimplémentée depuis `whitebox_next_gen` » ;
    le réalisé n'en reprend rien : les trois commandes sont écrites d'après les publications
    (Horn 1981, Franklin et Ray 1994), le GeoTIFF lu par `tiff` (`wbgeotiff` refusé par
    `deny.toml`, plan de 4.1). Le fichier d'epic garde la note « Unknown : maturité de
    `wbgeotiff` », tranchée en 4.1. *Réconciliation proposée.*
  - Le fichier d'epic annonce « huit entrées » (inception) ; il y en a dix (4.9 et 4.10 ajoutées
    en cours). Écart accepté, rien à corriger.

### Leçons de processus (tirées des plans)

- **Les analyses classées « medium » ont eu la relecture rapide.** 4.2 et 4.4 sont des analyses,
  que CLAUDE.md §8 range en risque `high` ; elles ont été planifiées `medium`. La relecture rapide
  de 4.4 a tout de même trouvé un défaut de déterminisme (hypot) et un écart d'intention (mesures
  nues sur le canal) ; celle de 4.2 n'a pas vu `atan` (F1). La relecture 4.4 l'avait signalé
  (constat 4).
- **Les défauts entre tickets sont sortis en recette réelle, pas en relecture.** 4.9 (le panneau
  ouvrait les rasters lui-même, hors journal, donc l'export rejoué divergeait) vient de 4.3 et
  n'a été vu qu'à la recette enchaînée de 5.41 ; 4.10 vient d'une décision de 4.6. Les deux ont
  été traités dans l'epic, avant la fermeture.
- **Une borne non mesurée a coûté une boucle.** 4.5 a figé « 1 000 cellules, quelques secondes »
  avant de mesurer (≈ 21 s) ; la relecture l'a vu, et le plan a été repris (Plan Change Log de 4.5).
- **Les reports sont tenus.** Toutes les entrées de `deferred-work.md` issues de l'epic portent un
  `status:` (vérifié par 4.8).

### Constats écartés après vérification

- Taille nulle du MNT (`width - 1` qui déborde) : `tiff` 0.11.3 refuse déjà une image de largeur
  ou hauteur nulle (`decoder/image.rs:151`).
- `cassini replay` hors QGIS bute sur `layer.open_raster` : voulu (AD-11), déjà tranché en
  relecture 4.9 (constat 11) ; un journal QGIS contenant des étapes déléguées ne se rejouait déjà
  pas hors QGIS.
- Couches accumulées et dossiers verrouillés après ~20 rejeux sous Windows : préexistant aux
  couches vectorielles (relecture 4.9, constat 9 ; 5.35, 5.36).
- Système emprunté sans contrôle de plausibilité des x/y : décision du propriétaire en 4.10
  (relecture 4.10) ; un point hors MNT est refusé par le noyau.
- Octets du GeoTIFF dépendants de la version du compresseur : `Cargo.lock` fige
  `miniz_oxide` ; mêmes octets vérifiés sous Linux et Windows (relecture 4.1, constat 11). Risque
  réel seulement à une montée de version — noté, sans action.
- Cache GDAL périmé sous Linux après réécriture du même fichier de rejeu : affirmation non
  vérifiable dans les preuves disponibles.
- Durée du champ de vision non bornée : mesurée en 4.5 (test `#[ignore]` de mesure, plan de 4.5) ;
  la borne de 500 cellules en découle.

## Behavior verification

Exercé le 2026-11-05, sur cette révision (`e225385`) :

- `cargo test -p cassini-core` (WSL) : 362 + 32 + 8 tests verts, 1 ignoré (mesure). Le paquet
  `cassini` (CLI) ne compile pas sous WSL (libclang absent pour `llama-cpp-sys-2`) : non relancé
  ici.
- `cargo deny check licenses bans` : vert.
- **Les trois commandes sans QGIS, depuis la CLI** (`cassini.exe` installé, construit au 4.9 le
  2026-11-05 ; 4.10 et 4.8 n'ont pas touché aux calculs), sur `fixtures/mnt-colline-2154.tif`, dans
  un projet temporaire, deux fois chacune :
  - pente : min 3,53°, max 23,29°, 196 cellules sans donnée ; ligne de vue (2 m / 2 m sur 440 m) :
    masquée, obstacle à 10 m ; champ de vision (rayon 1 km) : 274 visibles, 2 226 masquées ;
  - mêmes empreintes MD5 aux deux passages pour la pente et le champ de vision ;
  - `cassini.exe replay` du journal : `{"steps":6,"identical":true}`.
- Non exercé ici : rejeu croisé Windows ↔ Linux (F1), ouverture dans QGIS (démontrée par les
  sondes réelles de 4.3 et 4.9, consignées dans leurs plans).

## Previous-retro follow-through

L'epic précédent dans l'ordre (`epic-agent-et-mcp`) **n'a pas de fichier de rétrospective** :
rien à suivre de ce côté, faute de fichier (et non faute d'actions).

Les deux règles de processus tirées de la rétro de l'epic 9 et inscrites à CLAUDE.md §8 ont été
contrôlées :

- **A1 (ticket de fermeture construit en dernier)** — tenue : `after = [1, 2, 3, 4, 5, 6, 7, 9,
  10]` pour 4.8 dans `tickets.toml`, et 4.8 fusionné après tous (`5698fea`).
- **A6 (relecture à plusieurs lentilles pour le rejeu, le canal, les analyses)** — tenue en
  partie : thorough pour 4.1, 4.3, 4.5, 4.9, 4.10 ; quick pour 4.2 et 4.4, deux analyses classées
  `medium` (voir Leçons de processus et F1).

## Action items

Tous proposés, **rien n'est appliqué** ; le propriétaire décide.

| # | Action | Constats | Propriétaire | Nature |
|---|---|---|---|---|
| R1 | Rendre la pente en degrés indépendante de la plateforme : `atan` portable (fonction en opérations IEEE exactes, ou crate `libm` MIT/Apache-2.0 après accord sur la dépendance) ; ajouter un test qui fige les octets (ou l'empreinte) de la pente de la colline, vérifié par la CI Windows et Linux. | F1 | Dev (`bmad-build`, bogue court) | Remédiation |
| R2 | Refuser les MNT en pseudo-mètres (EPSG:3857, 3395, 900913…) avec un message qui invite à reprojeter dans QGIS ; à confirmer d'abord par le propriétaire. | F2 | Propriétaire puis Dev | Remédiation |
| R3 | Décider du champ de vision derrière les trous : garder « visible », ou marquer les cellules dont la ligne a traversé du sans-donnée (valeur 2, ou compte dans le résultat). | F3 | Propriétaire | Décision |
| R4 | Reporter dans `deferred-work.md` : unités verticales (F4), preuve QGIS automatisée du GeoTIFF écrit (F5), ouverture dictée en erreur à joindre à l'entrée existante sur les étapes déléguées en erreur (F6), branche `intermediate_files` (F7), message du plan (F8), détection des systèmes (F9). | F4–F9 | Dev | Suivi |
| R5 | Écrire la règle de déterminisme comme convention du dépôt : dans tout calcul dont la sortie se rejoue, seulement des opérations IEEE correctement arrondies (+, −, ×, ÷, `sqrt`), jamais `hypot`, `atan`, `sin`… de la libm. | F1, divergence de patron | Propriétaire, avec `bmad-architecture` (table des conventions) | Leçon de processus |
| R6 | Classer `high` toute entrée qui ajoute une commande d'analyse (comme CLAUDE.md §8 le dit déjà), au moment du plan ; ne pas laisser un plan ramener une analyse à `medium`. | Leçon A6 | Propriétaire | Leçon de processus |
| R7 | Réconcilier CLAUDE.md §3 (« reprise ou réimplémentée depuis `whitebox_next_gen` ») avec le réalisé (écrit d'après les publications, GeoTIFF par `tiff`) et clore la note « Unknown : maturité de `wbgeotiff` » du fichier d'epic. | Réconciliation | Propriétaire | Réconciliation de spec |
| R8 | Écrire une rétrospective de l'epic 5 (agent et MCP), absente : c'est la rétro que la suivante devrait suivre. | Previous-retro | Propriétaire | Suivi |

## Acceptance verdict

**accepted-with-open-items** — critères déclarés (Done when du fichier d'epic) :

1. **Tenu.** `terrain.slope`, `terrain.line_of_sight`, `terrain.viewshed` sont des commandes du
   registre (`core/src/commands/terrain_*.rs`), avec 9, 13 et 14 tests ; elles répondent sur un
   MNT ouvert dans le projet (sondes réelles de 4.3, 4.4, 4.9).
2. **Tenu.** `cargo deny` vert, aucune dépendance GPL/LGPL ; provenance écrite dans
   `docs/comparaison-terrain-qgis.md` (section « Provenance du code », commits `a378d09`,
   `c246372`, `4e4a12f`, `61728bb`) et en tête de chaque module.
3. **Tenu sur une même plateforme** (exercé ci-dessus : mêmes octets, rejeu `identical`).
   Réserve : F1, rejeu croisé Windows ↔ Linux de la pente non garanti.
4. **Tenu.** Les trois commandes tournent depuis `cassini.exe run`, sans QGIS (exercé ci-dessus).
5. **Tenu.** `docs/comparaison-terrain-qgis.md` : pente à 4·10⁻⁶ ° de QGIS ; 30 lignes de vue au
   verdict de GRASS ; champs de vision concordants à 98,7-100 %, écarts expliqués.

Toutes les entrées sont `done`. Restent ouverts, sans bloquer : F1 et F2 (à corriger), F3
(décision), F4 à F9 (reports).

## Décisions du propriétaire (2026-11-05)

Prises à la lecture de cette rétrospective ; elles tranchent les questions ouvertes.

- **F2 / R2** : refuser les MNT en pseudo-mètres (EPSG:3857 et alias) par une erreur typée qui
  invite à reprojeter dans QGIS, comme pour un MNT en degrés. Pas d'avertissement seul.
- **F3 / R3** : ne pas accepter en silence un « visible » derrière un trou ; le champ de vision
  marque ces cellules d'une valeur à part, « incertain », à côté de visible et masqué.
- **F1 / R1** : `atan` portable par la crate `libm` (MIT/Apache-2.0, sans dépendance) ; l'accord
  sur la dépendance est redemandé au moment de l'ajouter (CLAUDE.md §8).
- **R8** : rétrospective de l'epic 5, dans une session à part.
- **Ordre** : commit de cette fiche ; un ticket court pour R1 et R2 ; un ticket pour R3 ; puis la
  rétrospective de l'epic 5.

## Open questions

- Les journaux de session n'ont pas été lus : le pourquoi des choix de risque `medium` pour 4.2 et
  4.4 n'est connu que par la note de la relecture 4.4.
