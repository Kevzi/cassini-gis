---
title: 'La table fermée ne contient que les commandes déléguées'
type: 'chore'
ticket: '17'
created: '2026-11-06'
status: done
baseline_revision: 'd6373fd9ed9138334190cfecc5f4939dde27cc23'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** rien ne vérifie que les commandes que l'extension sait exécuter (et annonce à `session.hello`) sont exactement les commandes `runs_in: qgis` du registre (AD-24, amendement du 2026-11-05). Une ligne qui survivrait pour une commande calculée par le noyau (par exemple `raster.reclassify`), ou une commande déléguée sans geste, passerait la recette sans bruit, alors qu'AD-6 interdit « les deux ».

**Approach:** sortir de `plugin.py` (qui importe QGIS) la construction du registre des gestes dans une fonction sans QGIS, que `plugin.py` appelle ; une recette Python compare ses identifiants à ceux des fiches « faite » `runs_in: qgis` du catalogue figé, lui-même confronté au registre par `core/tests/catalogue.rs`. L'égalité se tient donc par transitivité, sans faire dépendre le Python du binaire Rust ni l'inverse.

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/plugin.py` l. 189-198 -- les huit `registry.register(...)` + `analysis.register_all` ; `SlowProbe` (l. 30) reste ici (QTimer au chargement).
- `qgis-plugin/cassini_qgis/delegated.py` (nouveau) -- `build_registry(probe)` : rend le `GestureRegistry` complet ; `test.qgis_probe` reçoit la fabrique passée. Imports doubles paquet/à plat, comme les modules voisins.
- `qgis-plugin/tests/test_delegated.py` (nouveau) -- lit les blocs ` ```json fiche ` de `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` ; attendu = ids `status: faite`, `runs_in: qgis` ; trouvé = `build_registry(object).ids()` ; les ids `test.` sont exclus des deux côtés (même règle que `TEST_PREFIX` de `catalogue.rs`). Une fonction pure `compare(expected, found) -> list[str]` nomme chaque écart en français ; tests : état actuel vide, ligne pour `raster.reclassify` (fiche `runs_in: core`) nommée, délégué sans ligne nommé.
- État actuel : 11 fiches déléguées faites = 11 gestes hors `test.qgis_probe`.

## Implementation Notes

Route oneshot : ~100 lignes, deux fichiers neufs et un déplacement dans `plugin.py`, sans changement de comportement. « Table fermée » s'entend ici comme l'ensemble des gestes de l'extension (ce qu'elle annonce), pas seulement `analysis.ALGORITHMS`, qui n'en est qu'une partie : c'est bien l'annonce qui violerait AD-6. Le catalogue sert de pont parce que le job Python de la CI tourne sans Rust.

- Fichiers : `delegated.py` (nouveau, `build_registry`), `plugin.py` (appel, imports réduits), `tests/test_delegated.py` (nouveau, 8 tests), `tests/test_tracer.py` (l. 824 : le geste raster se cherche via `delegated`), commentaire de `probe_external.py`.
- `compare` prend les fiches plutôt que l'ensemble attendu : il lui faut le `runs_in` de chaque id pour nommer la cause (« calculée par le noyau », « pas encore faite », « absente du catalogue »).
- Le cas d'AD-24, `raster.reclassify` (fiche planifiée `runs_in: core`), est testé tel quel.
- Vérifié : `python -I -m unittest discover -s qgis-plugin/tests` 417 OK ; garde de l'extension propre ; `cargo test -p cassini-core --test catalogue` 27 OK.

## Review Triage Log

Relecture quick — high 0, medium 0, low 2, false 1 ; 2 patchs.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Le lecteur Python (regex) ne découpe pas les blocs comme `parse_catalogue` (lignes comparées après `trim`) : une fiche à bornes indentées serait lue par Rust et pas par Python, et un délégué sans geste passerait | low | patch | lecture ligne à ligne, bornes sans espaces ; test `test_fences_are_read_like_the_rust_reader` ; 52 fiches lues des deux côtés |
| `compare(fiches, table_ids)` au lieu de `compare(expected, found)` du Code Map | false | rejeté | la cause de chaque écart demande le `runs_in` des fiches ; le comportement attendu est tenu |
| Le commentaire de `probe_external.py` renvoie à `plugin.py`, et les sondes listent leurs propres gestes | low | patch (commentaire) | les sondes manuelles enregistrent volontairement un sous-ensemble, avant ce ticket déjà ; seul le renvoi était faux |

## Verification

**Commands:**
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert, dont `test_delegated`
- `python -I qgis-plugin/tests/check_extension.py` -- expected: code 0
- `cargo test -p cassini-core --test catalogue` -- expected: vert (pont catalogue ↔ registre)
