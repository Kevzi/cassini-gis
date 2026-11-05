---
title: 'Comparaison à QGIS : justesse et vitesse des commandes de terrain, provenance du code de visibilité'
type: 'chore'
ticket: '7'
created: '2026-11-05'
status: 'built'
baseline_revision: 'd45d5a5cfd944fc06844e5da2794c284dd959583'
route: 'full'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Les trois commandes de terrain du noyau (pente, ligne de vue, champ de vision) sont testées sur des MNT synthétiques, mais rien ne montre encore qu'elles donnent le même résultat que l'outil de référence des utilisateurs, QGIS (et GRASS pour la visibilité), sur un vrai MNT ; la provenance du code de visibilité n'a pas été relue.

**Approach:** Sur le MNT réel de Villeneuve-le-Roi en Lambert-93 (hors dépôt), lancer chaque commande du noyau et son équivalent QGIS avec les mêmes paramètres, mesurer l'écart cellule par cellule (ou verdict par verdict) et le temps des deux outils, puis écrire une note de comparaison qui explique chaque écart et relit la provenance des trois commandes.

## Boundaries & Constraints

**Always:**
- Mêmes paramètres des deux côtés, écrits explicitement : observateur 1,6 m, cible 0 m, courbure et réfraction actives avec le même coefficient (0,13, passé aussi à GRASS par `refraction_coeff`), rayon 10 km ; observateurs et cibles au centre d'une cellule.
- Pente : `native:slope` (facteur Z 1), en degrés ; champ de vision : `grass:r.viewshed -c -r -b` ; ligne de vue : `r.viewshed` lu à la cellule de la cible, sur 3 observateurs × 10 cibles = 30 couples.
- Écarts mesurés : pente — cellules dont le masque sans-donnée diffère, écart moyen et maximal en degrés ; champ de vision — proportion de cellules qui diffèrent parmi celles que les deux outils définissent ; ligne de vue — nombre de verdicts qui diffèrent. Chaque écart est nul ou expliqué par une différence d'algorithme nommée.
- Temps : durée murale de chaque appel des deux outils sur la même machine Windows, plus le temps d'un appel à vide de chacun (démarrage), à titre d'information, sans seuil.
- L'outil de comparaison n'utilise que le noyau (`Dem::open`, `Registry::standard().call`), sans nouvelle dépendance ; les sorties de QGIS passent par `gdal_translate` (côté QGIS) en GeoTIFF non compressé avant lecture.
- Aucune donnée du MNT ni aucune sortie n'entre au dépôt : seuls les chiffres de la note.
- Décision (propriétaire, 2026-11-05) : Claude lance lui-même le script de comparaison (noyau, QGIS, GRASS) depuis WSL ; le propriétaire valide la note. Plan gardé entier malgré sa longueur.

**Never:**
- Corriger une commande du noyau dans ce ticket : un écart inexpliqué ou une erreur du noyau devient un ticket de bogue, et la note le dit.
- Toucher au comportement des commandes ; seuls les commentaires de provenance peuvent changer.
- Ajouter une commande au registre pour la comparaison.
- Lier GDAL, GEOS ou GRASS au noyau.

## I/O & Edge-Case Matrix

| Scénario | Entrée / état | Attendu | Traitement |
|---|---|---|---|
| Pente | MNT réel, degrés | Masques comparés, écarts moyen et max donnés | Bords : `native:slope` calcule les bords, le noyau les laisse sans donnée — écart attendu, compté à part |
| Champ de vision | 3 observateurs, rayon 10 km | Proportion de cellules différentes par observateur | Cellules définies par un seul des deux outils comptées à part, pas comme désaccord |
| Ligne de vue | 30 couples | Verdicts concordants ou écart expliqué | Couple discordant listé avec sa distance et sa position |
| Sortie QGIS illisible par `Dem::open` | Compression ou prédicteur non pris en charge | Normalisée par `gdal_translate -co COMPRESS=NONE -ot Float32` | Erreur claire si la grille diffère de celle du MNT |

</frozen-after-approval>

## Code Map

- `core/src/registry.rs` l.394 -- `Registry::standard().call(id, json)` : appel direct de `terrain.line_of_sight` depuis l'outil de comparaison.
- `core/src/dem.rs` -- `Dem::open`, `value(col,row)`, champs `origin_*`, `cell_size_*`, `width`, `height` : lire MNT et sorties. Lecture seule.
- `core/src/commands/terrain_slope.rs`, `terrain_line_of_sight.rs`, `terrain_viewshed.rs` -- en-têtes `//!` de provenance à relire. Pente : Horn (1981) sans référence complète ; ligne de vue : méthode décrite, source du coefficient 0,13 et de la formule de courbure non citée ; champ de vision : Franklin et Ray (1994), variante décrite. Commits `a378d09`, `c246372`, `4e4a12f`, `61728bb`.
- `cli/src/main.rs` -- `cassini run <id> '<json>' --project <fichier>` : appel chronométré du noyau ; la réponse donne `file`.
- Outils QGIS 3.44.15 (Windows, joignables depuis WSL par `powershell.exe`) : `C:\Program Files\QGIS 3.44.15\bin\qgis_process-qgis-ltr.bat`, `gdal_translate.exe`. `grass:r.viewshed` : `coordinates`, `observer_elevation` (1,75 par défaut), `target_elevation`, `max_distance`, `refraction_coeff` (0,14286 par défaut), `-c`, `-r`, `-b`.
- MNT : `../donnees-test-cassini/dem/mnt-villeneuve-le-roi-2154.tif`, EPSG:2154, 783 × 712, cellule 23,656 m, Int16, sans-donnée −32768.
- Noyau Windows : `C:\ct\release\cassini.exe`, à reconstruire depuis la révision de départ (mémoire « Mettre à jour QGIS et le noyau »).
- Plan 4.2, Implementation Notes : comparaison antérieure à `gdaldem slope -alg Horn`, écart max 3,8e-6°, masque identique ; à rappeler dans la note.

## Tasks & Acceptance

**Execution:**
- [x] `core/examples/comparer_terrain.rs` -- outil en trois modes : `points <mnt>` écrit les 3 observateurs et 30 cibles (centres de cellules, déterministes, à l'intérieur de l'emprise, distances 1 à 10 km) en JSON ; `comparer <dossier>` lit les sorties des deux outils et le MNT, calcule les lignes de vue du noyau par le registre, lit `r.viewshed` aux cibles, et imprime les tableaux d'écarts en Markdown ; tests unitaires des fonctions de comparaison (grilles différentes refusées, comptes) -- l'instrument de mesure, sans QGIS
- [x] `tests/comparaison-terrain/comparer.ps1` -- lance, en chronométrant, le noyau (pente, 3 champs de vision) puis QGIS (`native:slope`, 3 `r.viewshed`), plus un appel à vide de chacun ; normalise les sorties QGIS ; écrit `temps.json` dans un dossier de travail hors dépôt passé en paramètre ; fichier en UTF-8 avec BOM -- le protocole rejouable
- [x] `tests/comparaison-terrain/LISEZMOI.md` -- comment lancer les deux étapes -- reproductibilité
- [x] `core/src/commands/terrain_slope.rs`, `terrain_line_of_sight.rs`, `terrain_viewshed.rs` -- compléter la provenance (références complètes, source de la formule de courbure et du coefficient de réfraction, aucune ligne de code reprise) -- relecture de provenance
- [x] `docs/comparaison-terrain-qgis.md` -- la note : protocole, paramètres, tableaux d'écarts et de temps, explication de chaque écart, provenance des trois commandes avec ses sources -- le livrable

**Acceptance Criteria:**
- Given la note, when on la lit, then elle donne pour la pente, la ligne de vue et le champ de vision l'écart mesuré et son explication, et pour chaque commande le temps du noyau et celui de QGIS sur la même machine.
- Given la note, when on lit sa section provenance, then chaque source publique ou fichier porté est cité, et elle dit qu'aucun code de GRASS, Whitebox ou GDAL n'a été repris.
- Given le dépôt, when tournent `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` et `cargo deny check`, then tout est vert sans nouvelle dépendance.

## Implementation Notes

- Noyau Windows reconstruit (`--features gpu`) depuis la révision de départ avant la mesure ; protocole passé deux fois dans deux dossiers vierges hors dépôt, écarts identiques, temps proches (sauf le premier `r.viewshed` à froid).
- Résultats : pente, intérieur identique à 3,8e-6° près, 346 bords et 2 662 voisines de trou calculés par QGIS seul ; ligne de vue, 30/30 verdicts concordants ; champ de vision, 0,27 % / 1,28 % / 0,00 % de désaccords, 99,3 à 100 % sur une lisière. Aucun écart inexpliqué, aucun ticket de bogue.
- `core/Cargo.toml` : section `[[example]] test = true` pour que `cargo test --workspace` passe les tests de l'instrument ; aucune dépendance ajoutée.
- `gdal_translate` reçoit aussi `-a_srs EPSG:2154` : les sorties de GRASS n'ont pas forcément de clé EPSG lisible par `Dem::open` ; la grille reste contrôlée contre celle du MNT.
- `cargo test --workspace` et `cargo clippy` passent sous Windows (cible `C:\ct`) ; sous WSL, le workspace ne compile pas (GTK de la coquille Tauri et libclang absents), sans lien avec ce ticket. `cargo fmt --check` et `cargo deny check` verts.
- Référence à faire confirmer par le propriétaire : l'attribution du coefficient 0,13 à Gauss (triangulation du Hanovre) est la tradition géodésique courante, citée de mémoire.

## Plan Change Log

## Review Triage Log

**Passe 1 (quick, 2026-11-05)** — high 0, medium 2, low 2, false 3, maybe-false 0 ; 1 low rejeté.

| # | Constat | Verdict | Route | Preuve / action |
|---|---|---|---|---|
| 1 | Temps QGIS de la ligne de vue absent (« — ») alors que le critère 1 le demande pour chaque commande | medium | patch | Vrai : la note ne donne aucun temps QGIS ; ajouter le coût équivalent (un `r.viewshed` par observateur, 3 appels, lus pour les 30 couples). |
| 2 | Lisières gonflées : GRASS `-b` écrit 0 hors du rayon et sur les trous, lu comme « masqué » par `qgis_at` et `judge_pair` | medium | patch | Vrai : le noyau y est sans donnée, une cellule visible au bord du disque passe pour une lisière ; ne lire GRASS que là où le noyau définit la cellule (rayon, MNT valide), puis refaire `comparer` et corriger les chiffres de la note. |
| 3 | Durée « 30 appels du registre » englobe 60 ouvertures de GeoTIFF | low | patch | Vrai : `started` couvre la boucle entière ; ne chronométrer que l'appel du registre. |
| 4 | « 3 à 90 fois plus vite » ne couvre pas 2,9× (O2) ni 124× (O3) | low | patch | Vrai, recalculé depuis le tableau ; corriger la fourchette. |
| 5 | Attribution de k = 0,13 à Gauss écrite comme un fait | false | — | C'est l'attribution classique des manuels de géodésie (coefficient de Gauss, mesure du Hanovre) ; la confirmation reste signalée au propriétaire à la présentation. |
| 6 | « Géodésie classique » et « méthode générale » ne sont pas des sources | false | — | Aucune source précise n'a servi ni aucun fichier n'a été porté ; la note le dit honnêtement, ce que demande la vérification. |
| 7 | `comparer.ps1` casse sous PowerShell 7.3+ (guillemets doublés) | low | rejeté | Le LISEZMOI impose `powershell.exe` (5.1, celui de Windows) ; le correctif ajouterait une garde pour un usage non documenté. |
| 8 | Construction de l'exemple sous WSL non attestée | false | — | `cargo test -p cassini-core --example comparer_terrain` passé sous WSL le 2026-11-05 (7 tests), clippy et fmt verts. |

## Design Notes

**Écarts attendus, à confirmer par la mesure.**
- Pente : `native:slope` remplace les voisines hors grille par la cellule centrale et calcule donc les bords ; le noyau, comme `gdaldem`, les laisse sans donnée. Intérieur : même formule de Horn, écart d'arrondi f32 attendu.
- Visibilité : `r.viewshed` (balayage de Haverkort et al.) interpole l'altitude sur les bords de cellules traversés et juge le centre de la cellule cible ; le noyau échantillonne à pas constant en bilinéaire. Désaccords attendus sur les lisières de zones visibles, pas au cœur. GRASS prend la courbure sur l'ellipsoïde du lieu, le noyau sur une sphère de 6 371 km : écart de chute négligeable sous 10 km, à chiffrer dans la note.

**Pourquoi un exemple Cargo et pas un script Python de QGIS.** Il réutilise la porte raster du noyau, tourne sans QGIS et n'ajoute aucune dépendance ; seul le lancement des outils de QGIS reste dans le script PowerShell.

## Verification

**Commands:**
- `cargo test --workspace && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo deny check` -- expected: tout vert
- `cargo run --release -p cassini-core --example comparer_terrain -- points ../donnees-test-cassini/dem/mnt-villeneuve-le-roi-2154.tif` -- expected: 3 observateurs et 30 cibles en JSON
- `powershell.exe -File tests/comparaison-terrain/comparer.ps1 -Travail <dossier hors dépôt>` puis `cargo run --release -p cassini-core --example comparer_terrain -- comparer <dossier>` -- expected: tableaux d'écarts et de temps

**Manual checks (if no CLI):**
- Chaque écart non nul de la note a une explication ; aucun fichier du MNT ou des sorties n'est suivi par git.
