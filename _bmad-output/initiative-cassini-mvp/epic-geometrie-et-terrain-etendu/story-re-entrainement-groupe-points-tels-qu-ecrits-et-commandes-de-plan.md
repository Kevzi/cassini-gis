---
title: 'Ré-entraînement groupé : points tels qu''écrits et commandes de style'
type: 'feature'
ticket: '23'
created: '2026-11-08'
status: 'in-review'
baseline_revision: '8c69b8aabba1b452c7752aa76d2d81b8584982cd'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-conversion-des-points-ajoutee-au-plan/change-conversion-des-points-ajoutee-au-plan.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-re-entrainement-groupe-et-recette-du-modele-plan.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le modèle installé (terrain-11-18b) n'a jamais vu les commandes de style (`map.topographic_style`, `map.graduated_style`), `data.reproject`, ni le fuseau UTM lu dans l'inventaire ; il compose encore lui-même la conversion d'un point que le noyau ajoute désormais au plan (11.22) ; le générateur garde des refus devenus faux (style d'une couche) et des constats de 11.16 et de la sonde d'Annecy restent à apprendre.

**Approach:** partie A — mettre le générateur et les essais réels au niveau : points recopiés tels qu'écrits (le noyau complète le plan, décision 3 du changement de cap du 2026-11-08), familles des deux commandes de style et refus périmé retiré, essai réel `style_reel`, attendus des essais réels mesurés sur le plan complété, constats reportés de 11.16 traités ; partie B — ré-entraîner par la commande unique (même base, même méthode, GPU), valider, mesurer sur GPU contre les barres, sans installer.

## Boundaries & Constraints

**Always:** même base Qwen3.5-4B, même méthode, `DEFAULT_COUNT` et `DEFAULT_SEED` inchangés ; entraînement dans un worktree à part, jamais de changement de branche dans le dépôt principal pendant qu'il tourne ; `tests/epreuve-modele-local/` lu tel quel ; chaque exemple passe `check_plan` puis `guard_coordinates` ; essais réels en `--features cuda` ; barres de non-recul : `plan_reel` ≥ 23/25, `question_reel` ≥ 11/12, `ecart_reel` ≥ 32/40, `echelle_utm_reel` ≥ 7/8, `terrain_reel` ≥ 7/8, `epic11_reel` ≥ 27/28, et `style_reel` ≥ 9/10 sur des phrases nouvelles ; une phrase mal comprise devient un constat consigné, jamais une règle de prompt ; vocabulaire civil (§2.9).

**Never:** installer le modèle (décision du propriétaire au réveil) ; pousser sur GitHub ; changer le noyau, le prompt ou l'extension ; un troisième entraînement si les deux premiers ratent (règle de 11.16 : au plus un second essai, après correction du générateur, puis arrêt et compte rendu).

**Decisions (Claude, nuit du 2026-11-08, autorisation du propriétaire, à relire au réveil):** plan approuvé par Claude ; le fuseau UTM (plan `plan-fuseau-utm-depuis-l-emprise.md`) et `data.reproject` (10.7) sont déjà dans le générateur et sont appris ici ; installation laissée au propriétaire.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Point d'un autre système | « champ de vision depuis 45,90 N, 6,17 E (WGS84) sur le MNT à 25 m » (MNT 2154) | plan du modèle : `terrain.viewshed` avec le point en 4326 tel qu'écrit ; plan complété par le noyau : `data.transform_point` puis le champ de vision | un `data.transform_point` écrit par le modèle reste admis (le noyau n'en ajoute pas un second) |
| Liste de points d'un autre système | champ de vision cumulé, deux points WGS84 | points recopiés tels quels, plus de question | — |
| Style topographique | « mets les couches aux couleurs topographiques » | `map.topographic_style` | — |
| Dégradé | « colore les communes selon la population » | `map.graduated_style` sur le champ nommé | champ absent : question |
| Refus périmé | « mets les gares en rouge » | plus de refus « aucune commande ne change le style » | — |
| Carroyage UTM sans fuseau | inventaire avec `utm_epsg` | grille sur ce fuseau | sans `utm_epsg` : question |
| Reprojection d'une couche | gares en 4326, tampon de 500 m | `data.reproject` puis le tampon | — |

</frozen-after-approval>

## Code Map

- `training/src/families.rs` -- familles : `terrain_autre_systeme` et variantes « point d'un autre système » de 11.18 (attendu sans `data.transform_point`, point recopié), questions sur les listes de points d'un autre système (à retirer), refus « Aucune commande du catalogue ne change le style d'une couche » (cas 8 de `refus`, ~l. 2738), garde-fous du type `what_is_refused_out_of_registry_is_still_missing_from_the_registry` ; ajouter les familles `map.topographic_style` et `map.graduated_style` ; constats de 11.16 reportés (`deferred-work.md`, entrée « Renforcer le générateur… quatre constats » : carroyage sans système → question jamais refus, « dans quelle direction sont tournées les pentes » → exposition, PDF qui nomme ses couches → refus, …).
- Fuseau UTM (plan `plan-fuseau-utm-depuis-l-emprise.md`) : à la graine par défaut, seulement 6 plans recopient `utm_epsg` et 3 questions le demandent sur 8 000 exemples — trop peu pour l'apprendre ; donner au carroyage UTM sans fuseau écrit sa propre famille (au moins ~60 exemples, recopie et question), où le fuseau écrit diffère de `utm_epsg` dans les deux sens (ex. « en UTM 31N » avec `utm_epsg = 32632`), et où le nom du projet tiré est cohérent avec son fuseau (relecture du fuseau UTM, 2026-11-08), en évitant de reproduire une phrase des essais réels (`training/tests/recouvrement.rs` échoue sinon : retirer la tournure fautive plutôt que changer la graine).
- `training/src/inventories.rs` -- inventaires (styles : couches vecteur avec champs numériques pour le dégradé).
- `core/src/commands/map_topographic_style.rs`, `map_graduated_style.rs` (ou noms voisins) -- paramètres exacts à recopier dans les attendus.
- `core/src/plan_completion.rs` ou équivalent de 11.22 -- complétion du plan par le noyau ; les essais réels la passent avant de comparer (`deferred-work.md` l. ~895).
- `ai/tests/plan_reel.rs` (S25), `terrain_reel.rs`, `epic11_reel.rs` -- attendus sur le plan complété ; `ai/tests/common/` -- outillage commun ; nouvel `ai/tests/style_reel.rs` (10 phrases nouvelles, jamais vues par le générateur).
- `training/entrainement/pipeline.py` -- commande unique : `--sortie ~/cassini-entrainement/<nom>` ; validation à la fin (~50 min) ; durée ~4 h pour 500 pas.
- `docs/entrainement-du-modele.md` -- familles, refus retirés, part des familles.
- Mémoire de méthode : 11.16 et terrain-11-18b (plans voisins dans ce dossier) pour la suite des essais GPU et leur journal.

## Tasks & Acceptance

**Execution:**
- [x] Partie A — `training/src/families.rs`, `inventories.rs`, tests du générateur : points tels qu'écrits, styles, refus retiré, constats de 11.16.
- [x] Partie A — `ai/tests/` : attendus sur le plan complété, nouvel essai `style_reel`.
- [x] Partie A — `docs/entrainement-du-modele.md`, `deferred-work.md` (résolutions).
- [ ] Partie A — `cargo test -p cassini-training`, `cargo test -p cassini-ai` sans modèle, fmt, clippy (Windows) ; relecture quick ; commit sur la branche.
- [ ] Partie B — worktree à part sur le commit de la partie A, `pipeline.py --sortie ~/cassini-entrainement/regroupe-11-23`, journal consigné.
- [ ] Partie B — validation, puis essais GPU contre les barres ; résultats dans les Implementation Notes ; modèle laissé dans `C:\Users\kevin\cassini-modeles\ajuste-11-23\`, non installé.

**Acceptance Criteria:**
- Given le générateur à jour, when on lance ses tests, then chaque exemple passe `check_plan` et `guard_coordinates`, et aucun refus ne dit qu'aucune commande ne change le style.
- Given le modèle entraîné, when on lance les essais GPU, then chaque barre est tenue, ou l'écart est consigné et le modèle n'est pas proposé à l'installation.

## Implementation Notes

- 2026-11-09, partie A (sans entraînement ni commit) :
  - Générateur : points de terrain d'un autre système recopiés tels qu'écrits (`foreign_terrain_point` remplace `into_dem_system` ; une référence MGRS garde `coord.convert` puis `data.transform_point`) dans `terrain_autre_systeme`, `champ_de_vision_cumule`, `cout_cumule`, `chemin_moindre_cout` ; `list_system_question` retirée ; ligne de vue et chemin peuvent mêler un point étranger et un point du système du raster.
  - Familles neuves : `style_topographique` (3), `style_degrade` (4), `carroyage_utm_projet` (3, 100 exemples, 65 recopies de `utm_epsg` et 11 questions au jeu par défaut) ; nom du projet cohérent avec son fuseau (`inventories::project_names`).
  - Refus du style remplacé par `NO_CHOSEN_COLOR` ; garde-fou `styles_no_layer`. Constats de 11.16 : A01 (`exposition`), S18 (`NO_LAYER_CHOICE`), S20 (`champ_de_vision`), F03 (`carroyage_utm_projet`), point UTM vers le fuseau voisin (`conversion_coordonnees`).
  - `redemande` 44 → 52 (380 sur 8 000, 4,75 %). Tournure « Où la pente du terrain dépasse-t-elle » retirée (recouvrait Z01).
  - Essais réels : `common::completed` / `complete_with` ; `plan_reel`, `terrain_reel`, `epic11_reel` mesurent le plan complété ; `epic11_reel` gagne `FOREIGN_POINTS` (Y01 à Y04, mesurés hors barre) ; nouvel essai `style_reel` (10 phrases, barre 9 / 10) ; recouvrement étendu (105 phrases des essais réels).
  - Vérifié (Windows) : `cargo test -p cassini-training --release` (101 + 3 + 1 + 5 verts), `cargo test -p cassini-ai` sans modèle (vert), `cargo clippy -p cassini-ai -p cassini-training --all-targets -- -D warnings` propre, `cargo fmt --check` propre.
  - Restent de la partie A : relecture `quick` et commit sur la branche.

- Partie B (nuit du 8 au 9 novembre 2026) : entraînement lancé à 00 h 23 depuis le worktree `~/cassini-worktrees/11-23` (commit `7599eff`), terminé à 06 h 29 (367 min) ; sortie `~/cassini-entrainement/regroupe-11-23/`, modèle copié dans `C:\Users\kevin\cassini-modeles\ajuste-11-23\`, **non installé**. Validation : 297/300 (terrain-11-18b : 292/300).
- Essais GPU (`--features cuda`, journal `~/cassini-entrainement/11-23-mesures/essais-gpu.log`) : `plan_reel` 22/25 (barre 23), `question_reel` 12/12, `ecart_reel` 32/40, `echelle_utm_reel` 8/8, `terrain_reel` 8/8, `epic11_reel` 28/28 mais F03 en échec (barre « carroyage sans système → question »), points d'un autre système 4/4 (hors barre), `style_reel` 9/10 (ST08 raté).
- Lecture des échecs :
  - S05 « Mets les bâtiments en Lambert-93 » : le modèle répond `data.reproject` ; la couche des bâtiments est en 4326 dans l'inventaire de l'épreuve, la réponse est juste et l'attendu (refus) est périmé depuis 10.7. Corriger l'attendu ferait 23/25.
  - S07 « zone tampon de 300 m autour des gares » : le modèle ajoute un `data.reproject` vers 2154 alors que les gares sont déjà en 2154 — erreur réelle, la famille `reprojection_avant_analyse` est trop généralisée.
  - F03 « carroyage UTM au kilomètre » sans `utm_epsg` dans l'inventaire : le modèle invente EPSG:32620 au lieu de poser la question — erreur réelle (65 recopies contre 11 questions dans le jeu).
  - S21 (ligne de vue en Lambert-93) et ST08 (dégradé à intervalles égaux en bleus) : écarts de paramètres, à lire de près.
- Barres non tenues (`plan_reel` 22 < 23 tant que l'attendu de S05 n'est pas corrigé ; F03) : le modèle n'est pas proposé à l'installation. Le second essai (au plus un) n'est pas lancé : il prend la carte graphique environ 6 h et attend la décision du propriétaire (règle de 11.16). Pistes : attendu de S05 ; exemples où une couche déjà métrique n'est jamais reprojetée ; plus de questions quand l'inventaire n'a pas de `utm_epsg`.

## Plan Change Log

## Review Triage Log

Partie A, boucle 0 (relecture quick) — low 4 ; corrigés 3, rejeté 1. Choix faits seul par Claude (nuit autorisée).

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| `status`/`resolution` en double dans deux entrées de deferred-work, clé `note` isolée | low | patch | anciennes lignes retirées, `note` fondue dans `resolution` |
| Espace perdue dans `NO_RASTER_STATS` | low | patch | rétablie |
| Garde-fou du jeu par défaut trop étroit (« pas dans le système ») | low | patch | condition ajoutée |
| Cas S20 : commune du fuseau 31 avec des coordonnées en 32N | low | rejeté | aucun effet sur le plan attendu ; réalisme à revoir au prochain passage du générateur |

## Verification

**Commands:** Rust sous Windows seulement (WSL n'a pas libclang) : `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test -p cassini-training'`.
- `cargo test -p cassini-training` et `cargo test -p cassini-ai` -- expected: verts sans modèle
- `CASSINI_TEST_MODEL=<déclaration> cargo test -p cassini-ai --release --features cuda -- --ignored <essai>` -- expected: les barres
- `cargo fmt --check`, `cargo clippy -p cassini-ai -p cassini-training --all-targets -- -D warnings` -- expected: propres
