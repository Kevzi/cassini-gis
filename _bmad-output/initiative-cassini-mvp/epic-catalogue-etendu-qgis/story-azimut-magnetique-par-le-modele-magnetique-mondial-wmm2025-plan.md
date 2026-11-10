---
title: 'Azimut magnétique par le modèle magnétique mondial (WMM2025)'
type: 'feature'
ticket: '29'
created: '2026-11-10'
status: 'built'
baseline_revision: 'ccbc57919a92f3523dd21909c0e7527d49ba1b4e'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-complements-geometrie-et-mise-en-page/change-complements-geometrie-et-mise-en-page.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-catalogue-etendu-qgis/story-gisement-azimut-compte-depuis-le-nord-du-quadrillage-plan.md'
  - '{project-root}/docs/entrainement-du-modele.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problème :** randonneurs, orienteurs et lecteurs de boussole veulent un azimut magnétique ;
le noyau ne connaît aucune déclinaison.

**Approche :** `north: "magnetic"` et `date` sur `geo.distance_azimuth` et
`geo.destination_point` (après l'entrée 28) ; déclinaison du WMM2025 (NOAA/BGS, domaine
public) calculée dans le noyau d'après le rapport technique (doi 10.25923/prbc-s316, §1.2),
coefficients compilés dans le binaire ; azimut magnétique = azimut vrai − déclinaison (D4) ;
date absente fixée par le noyau, rendue et repassée au rejeu (D5, M4).

## Boundaries & Constraints

**Always :** salle blanche (écrit d'après le rapport, jamais d'après le code C de la NOAA, qui
n'est ni ouvert ni recopié) ; `libm` pour toute trigonométrie ; altitude 0 m sur l'ellipsoïde
(D4) ; avertissement, jamais refus, pour une date hors validité ou une zone peu fiable (D5) ;
`north` absent : résultat d'aujourd'hui ; journaux anciens rejoués à l'identique ; mention
17 U.S.C. 403 au dépôt ; famille `azimut_magnetique`, jeu à 8 600, graines inchangées.

**Never :** dépendance ajoutée ; lecture d'un fichier ou du réseau à l'exécution (D6) ;
altitude ou MNT ; WMMHR ; variation séculaire rendue ; phrase des recettes ou du jeu tenu à
l'écart dans le générateur ; réordonner `FAMILIES`.

## I/O & Edge-Case Matrix

| Scénario | Entrée | Sortie attendue | Erreur |
|---|---|---|---|
| Azimut magnétique | `north: "magnetic"`, `date` 2026-05-01 | `azimuth` = vrai − D(from), `back_azimuth` = contre-azimut vrai − D(to), `north`, `declination` = D(from) dans `unit`, `date` recopiée, sans `warning` | — |
| Date absente | `north: "magnetic"` sans `date` | `date` = date du jour UTC ; au rejeu, la date journalisée est repassée, même déclinaison des années plus tard | — |
| Date hors validité | `date` 2031-03-01 | résultat rendu, `warning` en français | — |
| Zone peu fiable | point où H < 6000 nT (près d'un pôle magnétique) | résultat rendu, `warning` (ombre si H < 2000 nT, prudence sinon) | — |
| Date mal formée | `2026-13-01`, `2026-02-30`, `10/05/2026` | — | `geo.invalid_date` |
| Date sans magnétique | `date` avec `north` absent, `true` ou `grid` | — | `geo.date_unused` |
| Zone avec magnétique | `utm_zone` et `north: "magnetic"` | — | `geo.zone_unused` |
| Point projeté | `geo.destination_point`, `north: "magnetic"` | azimut vrai = azimut d'entrée + D(from) ; `point`, `north`, `declination`, `date`, `warning` éventuel | — |

</frozen-after-approval>

## Code Map

- `/mnt/c/Users/kevin/Downloads/WMM2025COF/WMM.COF` (= `WMM2025.COF`, SHA-256 `dfa8597825af4e0b87ff4198a5b4fb661b3c49f4cd090cd0164e0259b075582f`, en-tête « 2025.0 WMM-2025 11/13/2024 ») et `WMM2025_TestValues.txt` (100 valeurs : année décimale, altitude km, lat, lon, D, I, H, X, Y, Z, F, dérivées) -- à copier tels quels dans `core/data/wmm2025/`, avec un `LISEZMOI.md` : source (ncei.noaa.gov/products/world-magnetic-model, téléchargé par le propriétaire le 2026-11-10), date, empreinte, rapport (doi 10.25923/prbc-s316), mention « données du gouvernement des États-Unis, non soumises au droit d'auteur (17 U.S.C. 403) », mise à jour par une nouvelle version du noyau (AD-17), WMM2030 attendu fin 2029. Ajouter un paragraphe au `NOTICE` racine.
- `core/src/wmm.rs` (nouveau, `pub mod` dans `lib.rs`) -- `include_str!` du COF, lecture une fois (`OnceLock`), `field(decimal_year, height_m, lat, lon) -> Field {x, y, z, h, f, d, i}`, `decimal_year(y, m, d)`, `parse_date`, `today_utc()` (jours depuis 1970 → date civile, sans dépendance), `warning(...)`. Équations du rapport §1.2 (ci-dessous).
- `core/src/registry.rs` -- `CommandSpec::replay_param` (l.79, 224) : ajouter `filled_param(name, field)` (M4) ; `core/src/journal/mod.rs` l.477-489 (retrait du paramètre de l'appelant : ne pas retirer un `filled_param`), l.555-566 (garde sous `replay`), l.1330-1345 (repassé au rejeu ; une ligne sans clé `replay` se rejoue telle quelle). Contrôler `check_contract` et le catalogue si la spec est comparée.
- `core/src/angle.rs` -- `North` gagne `Magnetic` (« magnetic »).
- `core/src/commands/geo_point.rs` -- `check_zone_used` (déjà refuse `utm_zone` hors `grid`) ; y ajouter `check_date_used` et la lecture de la date.
- `core/src/commands/geo_distance_azimuth.rs`, `geo_destination_point.rs` -- params `date` (chaîne facultative), sorties `declination`, `date`, `warning` (omises hors magnétique, comme `convergence`), descriptions FR/EN, exemple, `.tests`, `.filled_param("date", "date")`, tests de la matrice.
- `core/src/journal/tests.rs` -- rejeu : date absente → date journalisée repassée ; date du journal vieille d'un an (journal écrit à la main) → même résultat ; journal ancien sans `north`.
- catalogue §7 -- `north` à l'énumération complète, `date` en `params` ; `declination`, `date`, `warning` en `result` ; retirer `planned_by`, `planned_params`, `planned_result` ; texte : M1 à M4, précision mesurée.
- `training/src/families.rs` -- famille `azimut_magnetique` placée juste après `gisement` dans `FAMILIES` et sa fabrique juste après celle de `gisement` ; `date` seulement si écrite ; `lib.rs` `GEOMETRY` ; `docs/entrainement-du-modele.md` ligne de table.
- `qgis-plugin/cassini_qgis/summary.py`, `texts.py` -- « azimut magnétique », déclinaison, avertissement affiché ; test.

## Tasks & Acceptance

**Execution :**
- [x] `core/data/wmm2025/`, `NOTICE` -- coefficients, valeurs de test, LISEZMOI.
- [x] `core/src/wmm.rs` -- modèle et tests : table 3b du rapport (point −80°, 240°, 100 km, 2027,5 : φ′, r, X′, Y′, Z′, X, Y, Z, H, F, D, I à 1e-6 de leur unité) ; toutes les lignes de `WMM2025_TestValues.txt` (avec leur altitude) ; dates, année décimale, date du jour.
- [x] `registry.rs`, `journal/mod.rs` -- `filled_param`.
- [x] `angle.rs`, `geo_point.rs`, deux commandes -- paramètres, sorties, refus, tests de la matrice (deg et gon).
- [x] `journal/tests.rs` -- rejeu.
- [x] catalogue -- fiches complètes, `core/tests/catalogue.rs` vert.
- [x] `training/`, docs, extension.

**Acceptance Criteria :**
- Given chaque ligne de `WMM2025_TestValues.txt`, when le champ est calculé à sa date, son altitude et sa position, then Y et Z égalent les valeurs publiées à 1e-5 nT, X, H et F à 1e-3 nT, la déclinaison égale atan2(Y, X) des valeurs publiées à 1e-5° et la valeur publiée de D à 0,005° (arrondie au centième).
- Given la table 3d du rapport (−80°, 240°, 100 km, 2027,5), when le champ est calculé par `wmm::field`, then D = 67,9316432316° (« TrueN-to-MagN ») à 1e-9° près, et D − γ (convergence de l'entrée 28, zone 10) = 70,8861479117° (« GridN-to-MagN ») à 1e-9° près.

## Design Notes

Rapport §1.2 (pages 12 à 15) : géodésique (λ, φ, h) → sphérique géocentrique : Rc = A/√(1 −
e² sin² φ), p = (Rc + h) cos φ, z = (Rc(1 − e²) + h) sin φ, r = √(p² + z²), φ′ = asin(z/r) ; A =
6 378 137 m, 1/f = 298,257223563, rayon de référence a = 6 371 200 m. g(t) = g + (t − 2025,0) ġ.
Fonctions de Legendre semi-normalisées de Schmidt P(n, m)(sin φ′), N = 12 ; X′ = −Σ(a/r)^(n+2)
Σ(g cos mλ + h sin mλ) dP/dφ′ ; Y′ = (1/cos φ′) Σ(a/r)^(n+2) Σ m(g sin mλ − h cos mλ) P ;
Z′ = −Σ(n+1)(a/r)^(n+2) Σ(g cos mλ + h sin mλ) P. Rotation : X = X′ cos(φ′ − φ) − Z′ sin(φ′ − φ),
Y = Y′, Z = X′ sin(φ′ − φ) + Z′ cos(φ′ − φ) ; H = √(X² + Y²), F, D = atan2(Y, X), I = atan2(Z, H).
Récurrences, avec cos θ = sin φ′, sin θ = cos φ′ : P(1,1) = sin θ ; P(n,n) = √((2n−1)/(2n)) sin θ
P(n−1,n−1) ; P(n,m) = [(2n−1) cos θ P(n−1,m) − √((n−1)² − m²) P(n−2,m)] / √(n² − m²), dérivées
par la règle du produit (d/dφ′ = −d/dθ). Pour Y′, tenir Q = P/sin θ (m ≥ 1, mêmes récurrences, Q(1,1)
= 1) : ni division par cos φ′, ni singularité au pôle (prolongement par continuité, rapport §1.4).
Un prototype indépendant de ces formules redonne la table 3b au dernier chiffre ; sur les
valeurs de test, écarts max. Y 5e-7 nT, Z 2,2e-6 nT, X 7e-4 nT (écart propre au fichier publié,
la table 3b, elle, est retrouvée sur X au dernier chiffre), D 1,4e-6° contre atan2(Y, X) publiés.

## Verification

Travail dans le worktree `/home/kevin/cassini-worktrees/nord-choisi` (branche
`story/10-28-29-nord-choisi`) ; ne jamais changer de branche, ne rien pousser ni commiter.
Avant `cargo`, sous WSL :
`export LIBCLANG_PATH=/tmp/claude-1000/-mnt-c-Users-kevin-Downloads-Projet-code-CASSINI/38f6e1c7-3a62-4e6d-a1e2-bc44d994e1ac/scratchpad/clang/x/libclang-18.1.1.data/platlib/clang/native BINDGEN_EXTRA_CLANG_ARGS=-I/usr/lib/gcc/x86_64-linux-gnu/13/include`.
La famille `azimut_magnetique` écrit des tournures de randonnée et d'orientation (« azimut
magnétique », « à la boussole », « cap magnétique », « avec la déclinaison »), pour les deux
commandes ; aucune phrase recopiée de `ai/tests/`, `tests/epreuve-modele-local/` ni
`qgis-plugin/tests/recette/`.

**Commands :**
- `cargo fmt --check && cargo clippy -p cassini-core -p cassini-ai -p cassini -p cassini-training --all-targets -- -D warnings` -- sans avertissement.
- `cargo test -p cassini-core && cargo test -p cassini-training && cargo test -p cassini` -- verts (catalogue, recouvrement, recette hors ligne comprise).
- `uvx pytest qgis-plugin/tests/test_summary.py` -- vert.

## Implementation Notes

- Modèle : `core/src/wmm.rs` (`field`, `Date::parse`, `Date::decimal_year`, `Date::from_days`,
  `Date::today_utc`, `warning`) ; coefficients par `include_str!` et `OnceLock`. Table 3b
  retrouvée (φ′ et D, I à 1e-10 rad, r, X′, Y′, Z′, X, Y, Z, H, F à 1e-6), table 3d à 1e-9°
  (D et D − γ zone 10), les 100 valeurs de test aux tolérances de l'AC. `core/data/wmm2025/*`
  marqué `-text` dans `.gitattributes` (empreintes gardées sous Windows).
- `geo_point` : `check_date_used`, `magnetic_date`, `magnetic_field`. `utm_zone` avec
  `magnetic` : `geo.zone_unused` (règle existante de `check_zone_used`). Un `date` qui n'est pas
  une chaîne : `registry.invalid_params`.
- `CommandSpec::filled_param` (M4) : la valeur rendue est journalisée sous `replay` même quand
  l'appelant a écrit la date (elle vaut alors la sienne) ; au rejeu, elle remplace celle des
  paramètres ; une ligne sans `replay` se rejoue telle quelle. Non caché au modèle (`ai/src/plan.rs`
  ne masque que `replay_param` et `leave_out_param`).
- `destination_point` : l'avertissement de zone ne regarde que `from` (seul point où l'azimut
  est mesuré).
- Générateur : famille `azimut_magnetique`, poids 3, après `gisement` ; total 348, jeu à 8 600,
  graines inchangées. Le décalage du tirage faisait engendrer à `filtre_numerique`
  « Sélectionne les bâtiments de plus de 35 m de haut ! », masquée égale à RA04 (recette
  d'Annecy) : « Sélectionne » devient « Sélectionne-moi » devant une hauteur, sans tirage de
  plus (même précédent que `tampon_head`).
- Extension : « azimut magnétique », « (déclinaison … au …) », « Attention : » suivi de
  l'avertissement du noyau.

## Plan Change Log

## Review Triage Log

Passe 1 (lentilles `blind-hunter`, `edge-case-hunter`, `verification-gap`, `intent-alignment`,
2026-11-10) — high 0, medium 1, low 7, false 4, rejetés par décision ou hors du périmètre 6.
`intent-alignment` : lecture R1 (contrat des commandes) mise en œuvre en entier, R2 (phrase de
l'utilisateur dans QGIS) fournie par le générateur et le résumé, vérifiable seulement après le
ré-entraînement 5.49 (aucune installation ni recette dans ce ticket, consigne du délégant).

| Constat | Verdict | Route | Preuve / action |
|---|---|---|---|
| Tests qui comparent la date rendue à `today_utc()` lue à part (trois tests) : échec si minuit UTC passe entre les deux | low | patch | vrai sur trois tests (blind, verification-gap) ; date lue avant et après, l'une ou l'autre acceptée. |
| `summary._grid` : `north` liste ou objet → TypeError (non haché) là où `== 'grid'` ne levait rien | low | patch | vérifié dans le diff ; recherche seulement sur une chaîne, test ajouté. |
| Générateur : jours 1–28 seulement, jamais la forme JJ/MM/AAAA ni une date hors validité | medium | patch | une demande « le 10/05/2026 » est courante et le noyau la refuse : le modèle doit apprendre à la convertir ; tirages élargis. |
| Liste des mois recopiée dans un test | low | patch | constante du module réutilisée. |
| Retouche de `filtre_numerique` (« Sélectionne-moi ») hors sujet et non signalée | low | patch | méthode du garde-fou (« on retire la tournure fautive et on le signale ») ; signalée dans `docs/entrainement-du-modele.md`. |
| Fiches : précision à 0,005° de D publiée omise ; ligne `utm_zone` sans le refus hors `grid` | low | patch | fiches complétées. |
| Fichier `WMM2025_TestValues.txt` absent du diff relu | false | rejeté | exclu exprès du diff de relecture (données) ; présent et indexé dans l'arbre. |
| Triage vide alors que `lenses_ran` est rempli | false | rejeté | la relecture était en cours ; ce tableau la trace. |
| Catalogue qui écrit des décisions « à confirmer » | false | rejeté | pratique du changement de cap (détails tranchés en party, réversibles, confirmés plus tard) et consigne du délégant. |
| Plan versionné avec des chemins de session (`LIBCLANG_PATH`, worktree) | false | rejeté | même pratique dans les plans déjà au dépôt (5.47, 5.48, bogue du rejeu) ; sans effet sur le code. |
| Date absurde (ex. 0850) acceptée avec un simple avertissement | low | rejeté | décision M2 et D5 (jamais de refus) ; l'avertissement dit que la déclinaison est extrapolée. |
| Panneau anglais : avertissement du noyau en français | low | rejeté | comme tous les messages du noyau (erreurs comprises) ; traduire demanderait un code d'avertissement, hors du contrat. |
| Seule D(from) est rendue, `back_azimuth` corrigé de D(to) | low | rejeté | fiche du catalogue (« au point de départ ») et D2 ; corrigé au point de mesure. |
| `params.date` et `replay.date` différents dans un journal retouché à la main | low | rejeté | journal falsifié à la main, hors usage ; le rejeu repasse ce que le noyau a journalisé (AD-11). |
| Aucune démo réelle ni recette | low | rejeté | hors périmètre de ce ticket : ni installation ni entraînement (consigne) ; la famille est apprise au ré-entraînement 5.49, la recette vient en 5.50. |

## Décisions de party (2026-11-10)

Tranchées en party le 2026-11-10 (géodésien, architecte, géomètre-topographe), à confirmer par
le propriétaire :
- **M1** — Année décimale = année + (jour de l'année − 1) / nombre de jours de l'année (date à
  0 h UTC) ; la fiche le dit.
- **M2** — Toute date grégorienne valide AAAA-MM-JJ est acceptée, avec `warning` hors du
  2025-01-01 au 2029-12-31 ; une date mal formée ou inexistante : `geo.invalid_date`.
- **M3** — `warning` de zone d'ombre si H < 2000 nT, de prudence si H < 6000 nT (seuils du
  rapport, §1.8), à chaque point où un azimut est mesuré ; plusieurs avertissements en un texte.
- **M4** — Nouveau `CommandSpec::filled_param(name, field)` : la valeur de l'appelant est
  gardée ; la valeur rendue (`date`) est journalisée sous la clé `replay` et repassée au rejeu ;
  une ligne sans clé `replay` se rejoue telle quelle.
