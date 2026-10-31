---
title: 'Refactor sweep'
type: 'refactor'
ticket: '8'
created: '2026-10-31'
status: 'done'
baseline_revision: 'ab8e3f18f36b15d1a919830daf36b684e381224b'
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

**Problem:** L'epic 5 laisse 57 constats ouverts dans `deferred-work.md` (sans `status:`), dont une partie est déjà réglée par des tickets plus récents. Il laisse aussi un doublon réel : `cassini plan` sans `--model` choisit son modèle par `default_model`, une autre règle que le canal (`ModelComposer::choose`), et son message `ai.no_model` montre le dossier absolu des modèles (AD-9).

**Approach:** Corriger les six petits écarts testables sans QGIS, sans vrai modèle et sans ré-entraînement. Écrire ensuite un `status:` et une `resolution:` sur chacun des 57 constats ouverts : clos (avec sa preuve), corrigé ici, ou reporté (avec sa raison et sa destination).

Décisions du propriétaire (2026-10-31) : le constat 111 (`system.about` journalisé) est reporté à un ticket dédié, la règle du journal ne change pas en 5.8 ; le plan est gardé entier (~2 300 jetons), correctifs et tri ensemble.

## Boundaries & Constraints

**Always:** comportement inchangé hors des six correctifs ; aucun chemin absolu dans un message (AD-9) ; `core` ne dépend pas de `ai` ; chaque correctif vient avec son test ; les invariants AD-1 à AD-29 ne sont pas re-décidés.

**Never:** aucune règle de prompt pour un défaut du modèle, aucun ré-entraînement ni générateur (5.10, 5.15 et 5.17 restent hors de 5.8) ; aucune nouvelle fonction : 87 (signaler les couches retirées par le budget) et 95 (retrouver les intermédiaires après un rejeu) sont reportés comme tickets à créer ; aucune retouche de `CLAUDE.md` (56) ; ne pas rendre relatifs les chemins de `command.run` (38 : `runlog.result_folder` a besoin du chemin absolu pour « Ouvrir le dossier »).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| plan sans modèle, dossier absent | `cassini plan "…" --inventory i.json`, `CASSINI_MODELS` vers un dossier absent | code 1, erreur typée sur err | message sans le chemin du dossier |
| plan sans modèle, dossier vide | idem, dossier vide | code 1, `ai.no_model` | message sans le chemin |
| plan sans modèle, premier inutilisable | dossier où le premier modèle a un problème, le second est sain | le second est choisi, comme dans le canal | — |
| chemin écrit autrement | message de llama.cpp citant le modèle avec `\` au lieu de `/`, ou préfixé `\\?\` | remplacé par le seul nom du fichier | — |
| révision MCP ancienne | `initialize` en `2024-11-05` | réponse d'initialisation acceptée, outils listés | — |

</frozen-after-approval>

## Code Map

- `cli/src/main.rs:541` `default_model` -- à supprimer ; `plan()` (l.563) prend `ModelComposer::from_env().choose(None)` quand `--model` manque, et masque ses erreurs par les `hidden` rendus.
- `cli/src/main.rs:416-428` `scrub` / `hide` -- remplacement exact seulement ; étendre `hide` aux écritures `/` ↔ `\` et au préfixe `\\?\` de chaque chemin masqué.
- `core/src/serve/calls.rs:392-460` `scrub`/`scrub_text` -- masquage du relais, à ne pas toucher ; inspiration pour les variantes.
- `ai/tests/duree_reel.rs:216-235`, `ai/tests/ecart_reel.rs:391-430` -- calculs en ligne à sortir dans des fonctions testées sans modèle.
- `qgis-plugin/cassini_qgis/models.py`, `panel.py:591`, `panel.py:645-660` -- règle « redemander la liste » et oubli de `cassini/model` à sortir dans `models.py`.
- `cli/src/mcp.rs:413` `get_info` -- ajouter `instructions` ; `cli/tests/mcp.rs:720` -- client de test, seule révision `2025-11-25` négociée.
- `README.md` (section construction, vers l.340) -- note WSL, recette trouvée en 5.25 (plan l.95 : `LIBCLANG_PATH`, `BINDGEN_EXTRA_CLANG_ARGS`).
- `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- les 57 entrées ouvertes ; forme existante : `status: close|decision|ticket|report, <date>, entrée 5.8 (...)` puis `resolution: …`.

## Tasks & Acceptance

**Execution:**
- [x] `cli/src/main.rs` -- supprimer `default_model` ; `plan()` sans `--model` passe par `ModelComposer::choose(None)` ; étendre `hide` -- un seul choix de modèle, AD-9 (35, 39)
- [x] `cli/tests/end_to_end.rs` (ou tests de `main.rs`) -- les trois cas « plan sans modèle » de la matrice, et un test de `hide` sur `\`, `/` et `\\?\` -- preuve
- [x] `ai/tests/duree_reel.rs`, `ai/tests/ecart_reel.rs` -- sortir `averages(...)` et `tally(...)`, avec tests sans modèle (73)
- [x] `qgis-plugin/cassini_qgis/models.py`, `panel.py`, `qgis-plugin/tests/test_models.py` -- sortir `asks_list_again(code)` et la décision à la réception de la liste (choix, oubli) ; tests sans Qt (75)
- [x] `cli/src/mcp.rs`, `cli/tests/mcp.rs` -- `instructions` dans `get_info` (2–3 phrases : Cassini pilote le projet QGIS ouvert, sinon commandes du noyau seules) ; test d'`initialize` en `2024-11-05` qui lit `instructions` (102, 108)
- [x] `README.md` -- note de construction sous WSL (variables, paquets) (97)
- [x] `deferred-work.md` -- un `status:` et une `resolution:` sur chaque entrée ouverte de l'epic 5, d'après les Design Notes

**Acceptance Criteria:**
- Given le dépôt, when `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --check` tournent, then ils passent.
- Given les tests de l'extension, when `python -m unittest discover -s qgis-plugin/tests` et `python -I qgis-plugin/tests/check_extension.py` tournent, then ils passent.
- Given `deferred-work.md`, when on compte les entrées de l'epic 5 sans `status:`, then il n'en reste aucune.

## Implementation Notes

- `plan_model(model, composer)` (`cli/src/main.rs`) : `--model` donné, le fichier tel quel ; sinon `ModelComposer::choose(None)`. Les chemins à masquer viennent de ce choix (dossier compris), l'erreur de composition est masquée par eux.
- `hide` essaie, pour chaque chemin, la forme longue `\\?\` en `\` d'abord, puis le chemin tel quel, en `\`, en `/`.
- rmcp 3.8.0 gère la révision 2026-07-28 sans `initialize` (`server/discover`, révision dans `_meta`) : elle est testée, pas seulement notée.
- `docs/entrainement-du-modele.md` : la phrase sur le modèle pris sans `--model` suit la règle unique.
- Vérification sous Windows (PowerShell depuis WSL, `LIBCLANG_PATH` d'outils-cassini, `CARGO_TARGET_DIR=C:\ct`) : `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` verts ; tests Python et garde de l'extension sous WSL, verts. La note WSL du README n'a pas été rejouée sur un WSL neuf (libclang n'y est pas installé).

## Plan Change Log

## Review Triage Log

Passe 1 (`quick`) — high 0, medium 2, low 2, false 0, maybe-false 0.

- medium, patch : `deferred-work.md` rangs 45 à 51 restés sans `status:` (entrées d'architecture du 2026-10-18 et découpe de 5.10 sans `source_plan`), hors du filtre du tri. 45 à 48 → `ticket` vers 10.1, 10.2, 10.3, 10.4 et 10.6 (déjà décrits dans `epic-catalogue-etendu-qgis/tickets.toml`, `model.list` compris) ; 49 à 51 → `close` par les plans 5-10-t2, t3, t4a et t4b (built).
- medium, patch (regroupe le constat sur la résolution du rang 35) : `assert_hides` de `cli/tests/end_to_end.rs` ne voit pas un chemin Windows échappé en JSON (`\\` doublé) ; vérifié, `fail()` sérialise le rapport. Ajout de la forme échappée.
- low, patch : troisième cas « premier inutilisable » sans preuve que `b.gguf` est choisi ; ajout de l'assertion sur son nom.
- low, patch : forme longue UNC `\\?\UNC\serveur\…` non masquée par `hide` ; vérifié, aucune des quatre variantes ne la contient. Variante ajoutée, avec un cas de test.

## Design Notes

Numéro = rang de l'entrée parmi les `- source_plan` de `deferred-work.md`.

- **Corrigé ici :** 35, 39, 73, 75, 97, 102 (révision 2024-11-05 ; 2026-07-28 sans `initialize` notée comme non couverte si le SDK ne la gère pas), 108.
- **Clos (preuve à citer) :** 33 (5.18 budget), 36, 70 (5.15/5.17 `plan_reel` 25/25), 38 (décision : chemin absolu voulu par l'extension, masqué à l'affichage par `runlog.redact`), 40, 44, 59, 66, 67 (générateur 4484464/7addf14), 42 (5.19), 52, 54 (`grade()`), 72, 80 (5.16 garde-fou), 78, 85 (5.18), 91, 93 (5.22), 101, 109 (5.30, 505795f), 103 (5.25).
- **Reporté, passe d'entraînement :** 32, 53, 71, 76, 77, 79, 81, 82, 83, 84, 86, 89, 90, 94, 96.
- **Reporté, propriétaire ou architecture :** 31 (refus d'un id inconnu : `judge` ou `verify`), 37 (composition annulable, change `Composer`), 41 (préchargement écarté par 5.19), 56 (`CLAUDE.md`), 92 (phrase 9 à 38 s sur CPU), 107 (quota Gemini).
- **Reporté, tickets à créer :** 87, 95, 111 (poignée de main `system.about` journalisée ; décision du propriétaire du 2026-10-31, hors du ménage).
- **Reporté, matériel ou QGIS à la main :** 88 (WAL GeoPackage), 98 (job CI `gpu`, à mettre sur la liste de publication), 99, 100 (carte graphique), 106 (attend une commande de terrain lente, étape 6).

## Verification

**Commands:**
- `cargo test --workspace` -- expected: tout passe (sous WSL, variables de la note README)
- `cargo clippy --workspace --all-targets -- -D warnings` -- expected: aucun avertissement
- `cargo fmt --check` -- expected: rien à reformater
- `python -m unittest discover -s qgis-plugin/tests` -- expected: tout passe
- `python -I qgis-plugin/tests/check_extension.py` -- expected: « Garde de l'extension : propre. »
