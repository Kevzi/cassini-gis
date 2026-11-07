---
title: 'Refactor sweep'
type: 'refactor'
ticket: '13'
created: '2026-11-07'
status: 'built'
baseline_revision: '9e34edb06ff22b5f04d6b2339ddf59653b63dc14'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** l'epic 11 laisse des doublons dans les modules raster, coût et GeoJSON (sept constantes `*_NODATA` à -9999, dix copies des aides de test `fixture` et `Scratch`, deux gestes Python d'ouverture presque identiques, `raster_output.py` et `vector_output.py`, renvoyés explicitement à cette entrée), deux gardes promises « au premier appelant » non posées (ligne GeoJSON de moins de deux sommets) et un `step:k` écrit dans un élément de liste qui passe la vérification du plan puis échoue tard en « fichier introuvable » (`deferred-work.md`, constat de 11.9). Une quinzaine de constats de l'epic sont encore sans `status:`.

**Approach:** factoriser sans changer de comportement visible (mêmes valeurs, mêmes messages, mêmes octets écrits), poser les deux gardes avec leurs tests, puis écrire un `status:` et une `resolution:` sur chaque constat ouvert de l'epic 11.

## Boundaries & Constraints

**Always:** sorties raster et GeoJSON identiques à l'octet (les tests d'octets figés restent verts sans retouche) ; codes et messages d'erreur existants inchangés ; nouvelles erreurs typées (`plan.bad_reference`, `geojson.unwritable`) avec message en français ; `cargo fmt`, `clippy -D warnings`, recette de l'extension et garde `check_extension.py` verts ; aucune dépendance ajoutée.

**Never:** traduire `step:k` dans une liste (constat 143, futur ticket) ; toucher au générateur, aux exemples, au modèle ou à `cassini-ai` ; changer le catalogue des commandes ; toucher `geodesy.rs`/`geodesic.rs`/`point.rs` hors doublon avéré (aucun trouvé) ; modifier les tickets (`tickets.toml`).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Plan, liste | `raster.weighted_overlay` avec `criteria[0].raster = "step:1"` ou `exclusions[0] = "step:1"` | refusé à la vérification, étape nommée | `plan.bad_reference`, message : les références `step:k` ne valent qu'au premier niveau d'un paramètre |
| Plan, premier niveau | `terrain.slope` `{"raster": "step:1"}` après une pente | inchangé, accepté | — |
| Plan, texte imbriqué ordinaire | liste dont un texte ne commence pas par `step:` | inchangé | — |
| GeoJSON, ligne courte | `LineString` de 0 ou 1 sommet | rien n'est écrit | `geojson.unwritable`, nomme le rang de l'entité |
| GeoJSON, même cellule | chemin de moindre coût départ = arrivée (2 sommets confondus) | écrit comme aujourd'hui | — |

</frozen-after-approval>

## Code Map

- `core/src/dem.rs` -- y poser `pub const NODATA: f32 = -9999.0;` (module sans dépendance au reste du crate, AD-20) ; `same_grid_as` le prend déjà en argument.
- `core/src/commands/{raster_reclassify,raster_weighted_overlay,terrain_aspect,terrain_cost_distance,terrain_hillshade,terrain_slope,terrain_viewshed}.rs` -- `*_NODATA` (l. 24, 26, 27, 42, 26, 25, 41) remplacées par `dem::NODATA` ; `terrain_viewshed_cumulative.rs` en lit une (8 usages). Aucun usage hors `core/src/commands/`.
- `core/src/commands/mod.rs` l. 31 -- module `#[cfg(test)]` existant (`build_support`) ; y ajouter `#[cfg(test)] pub(crate) mod test_support;` (`fixture(name)` = `CARGO_MANIFEST_DIR/../fixtures/name` ; `Scratch` = dossier temporaire effacé au `Drop`, `new(name)`, `join(name)`, nom unique par `process::id()` plus un compteur atomique pour que deux modules utilisant le même nom, « trou », « refus », ne se marchent pas dessus).
- Dix modules de test qui recopient `fixture`/`Scratch` (`raster_reclassify.rs:229`, `raster_weighted_overlay.rs:292`, `terrain_aspect.rs:164`, `terrain_cost_distance.rs:238`, `terrain_hillshade.rs:216`, `terrain_least_cost_path.rs:205`, `terrain_peaks.rs:392`, `terrain_slope.rs:171`, `terrain_viewshed.rs:328`, `terrain_viewshed_cumulative.rs:288`) -- supprimer la copie, importer ; les appels `Scratch::new("…")` restent tels quels. `terrain_line_of_sight.rs:512` n'a que `fixture` : l'importer. `model_list.rs`, `dem.rs`, `geojson.rs` ont des aides différentes : ne pas toucher.
- `core/src/plan.rs` -- `check_plan` (l. 203) ne lit que le premier niveau via `parse_reference` (l. 96) ; ajouter, pour chaque paramètre tableau ou objet, une descente récursive qui rend `plan.bad_reference` dès qu'un texte imbriqué est une référence (`parse_reference` rend `Some`). Modèle de test : l. 568-596.
- `core/src/geojson.rs` -- `check_finite` (l. 99) : ajouter le refus d'une `LineString` de moins de 2 sommets (message « rien n'est écrit », comme l'existant) ; test calqué sur `a_non_finite_number_writes_nothing`. Appelants : `terrain_peaks.rs:371` (points), `terrain_least_cost_path.rs:180` (toujours ≥ 2 sommets, test l. 294). Propriétés en double : clés `&'static str` écrites par le code, garde non posée (consigné).
- `qgis-plugin/cassini_qgis/raster_output.py`, `vector_output.py` -- `_singleshot`, `__init__`, `start`, `cancel`, `_run`, `_finish` identiques ; seuls `_open`, le message d'échec inattendu et `QgisBackend` diffèrent. Extraire une base commune dans un nouveau `qgis-plugin/cassini_qgis/open_output.py` (même import à double forme que `gestures`), sous-classes avec `_open` et un attribut de message. Tests : `qgis-plugin/tests/test_raster_output.py`, `test_vector_output.py`, `test_tracer.py` ; `delegated.py` importe les deux gestes (noms de classes inchangés).
- `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- entrées de `epic-geometrie-et-terrain-etendu/` sans `status:` (index 136-143, 145, 150, 153, 156-158), forme de 4.8 : `status: <close|report>, 2026-11-07, entrée 11.13 (\`epic-geometrie-et-terrain-etendu/story-refactor-sweep-plan.md\`)` puis `resolution:`.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/dem.rs`, commandes raster -- constante `NODATA` unique -- un seul -9999 à changer.
- [x] `core/src/commands/test_support.rs` + dix modules de test -- aides partagées -- environ 250 lignes de copies en moins.
- [x] `core/src/plan.rs` -- refus tôt de `step:k` imbriqué, tests (critère, masque, objet imbriqué, premier niveau inchangé) -- constat 145.
- [x] `core/src/geojson.rs` -- garde de la ligne courte et son test -- garde promise par 11.8.
- [x] `qgis-plugin/cassini_qgis/open_output.py` + deux gestes -- base commune -- doublon renvoyé à 11.13 ; vérifier que la liste des fichiers de l'extension (garde, paquet) admet le nouveau module.
- [x] `deferred-work.md` -- trier les constats ouverts de l'epic : close pour 145 (ici), 150 (lots de 11.12 faits), 138 s'il est appris en 11.12/11.16 (à vérifier dans les familles) ; report sinon, avec le ticket ou l'étape visés.

**Acceptance Criteria:**
- Given l'arbre après le ticket, when `grep -rn '= -9999.0;' core/src --include=*.rs` hors tests, then une seule définition, dans `dem.rs`.
- Given les tests d'octets figés existants (`two_runs_give_identical_bytes`, `octets-figes`), when la recette tourne, then ils passent sans retouche.
- Given `deferred-work.md`, when on liste les entrées de `epic-geometrie-et-terrain-etendu/`, then aucune n'est sans `status:`.

## Implementation Notes

Implémenté directement, sans sous-agent : le plugin context-mode modifie l'appel `Agent`, que le mode auto ne peut alors pas vérifier.

- `dem::NODATA` remplace les sept constantes `*_NODATA` (aucun usage hors de `core/src/commands/`, ni dans `ai`, `cli` ni le dépôt privé) ; elles sont supprimées, pas gardées en alias.
- `core/src/commands/test_support.rs` porte `#![cfg(test)]`, comme `test_echo.rs` : `build.rs` le déclare en `pub mod`, mais il n'est compilé que pour les tests. `Scratch` garde son champ public (un test du masque lit `dir.0`) et prend la méthode `write` de la superposition. Les noms de dossier deviennent `cassini-<pid>-<n>-<nom>` (compteur atomique).
- `plan.rs` : `nested_reference` descend dans les listes et les objets ; le refus vient avant la lecture de premier niveau, avec le nom du paramètre.
- Écart à la borne « Never : toucher au générateur » : `training/src/lib.rs`, test `listed_rasters_of_an_overlay_are_sources_of_rasters_of_the_inventory`, attendait « raster absent » pour `step:1` dans une liste ; le validateur rend maintenant `plan.bad_reference` (refus plus tôt, même verdict). Seule l'assertion de ce cas change ; ni famille ni exemple ne bouge.
- `open_output.py` : `OpenOutputGesture` (déroulé, `UNEXPECTED`) ; les deux gestes ne gardent que `QgisBackend`, `__init__` et `_open`. Noms des classes et des messages inchangés.
- `deferred-work.md` : 14 constats triés (close : 145, 150 ; report : les autres, 138 compris, avec le ticket ou l'étape).

**Vérifié** : `cargo fmt --all --check` ; clippy `-D warnings` sur `cassini-core` et `cassini-training` (WSL) ; `cargo test -p cassini-core` (549), `-p cassini-training` (86) sous WSL, `-p cassini-ai -p cassini` sous Windows (17 lots, aucun échec) ; extension 436 tests, garde propre ; `training/entrainement/tests` OK ; `cargo deny check licenses` : ok ; recette hors ligne (`hors_ligne`, release, Windows) : ok.

## Plan Change Log

## Review Triage Log

Passe 1 (quick, 2026-11-07) : high 0, medium 0, low 5, false 1, maybe-false 0.

| # | Constat | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 1 | `cargo fmt --all --check` échouerait : deux lignes vides de suite après la suppression des `*_NODATA` (six fichiers) | false (échec) / low (lignes vides) | patch | `cargo fmt --all --check` sort 0 ; les lignes vides doubles existaient bien : supprimées. |
| 2 | Borne « Never : toucher au générateur » franchie (`training/src/lib.rs`) | low | rejet | Seule l'assertion d'un test du validateur change, conséquence obligée du refus tôt exigé par l'intention (`check_plan` passe avant le contrôle des rasters) ; ni famille ni exemple ne bouge. Signalé au propriétaire à la présentation. |
| 3 | Constat 138 clos sur une autre chaîne (pente → reclassement, pas pente → champ de vision) | low | patch | Vérifié : aucune famille n'enchaîne une pente dans `terrain.viewshed`. Passé en `report`, résolution reformulée. |
| 4 | Test du refus imbriqué sans paramètre objet ni imbrication profonde | low | patch | Ajoutés : `observer.x = "step:1.x"` et `extra: [["step:1"]]` dans un critère. |
| 5 | Message `UNEXPECTED` de chaque geste non testé | low | patch | Les tests `test_an_exploding_backend_still_answers_the_order` vérifient maintenant le message exact. |
| 6 | `check_finite` contrôle aussi la longueur des lignes | low | patch | Renommée `check_feature`. |

## Design Notes

Refus tôt plutôt que traduction : le constat 143 (traduire `step:k` dans `criteria[i].raster`) reste un ticket à part, car il touche le journal et le rejeu. Le refus en attendant transforme une erreur tardive et trompeuse en erreur typée, sans rien permettre de nouveau ; le générateur n'écrit jamais `step:k` dans une liste (11.12 lot 2b-2), donc le modèle n'est pas affecté.

## Verification

**Commands:**
- `cargo test -p cassini-core` et `cargo test --workspace` sous Windows (recette de 5.8, `CARGO_TARGET_DIR=C:\ct`) -- expected: tout passe
- `cargo clippy -p cassini-core --all-targets -- -D warnings` -- expected: aucun avertissement (espace entier par la recette GitHub)
- `cargo fmt --all --check` -- expected: rien à reformater
- `cd qgis-plugin/tests && python3 -I -m unittest discover -s .` et `python3 -I qgis-plugin/tests/check_extension.py` -- expected: tout vert, garde propre
- `cargo deny check licenses` (ou l'audit de licences de la recette) et la recette hors ligne -- expected: verts

**Manual checks:**
- `deferred-work.md` : aucune entrée de l'epic 11 sans `status:`.
