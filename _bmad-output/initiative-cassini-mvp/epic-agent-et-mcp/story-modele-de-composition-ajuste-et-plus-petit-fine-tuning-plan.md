---
title: 'Modele ajuste, tache 1 : jeu d exemples engendre et jeu tenu a l ecart'
type: 'feature'
ticket: '10'
created: '2026-10-22'
status: 'built'
baseline_revision: '26b9383c8920eaf31cd88409b60c39ee1809a001'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['quick', 'blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Pour ajuster un petit modèle, il manque des exemples « phrase française + inventaire → plan » engendrés depuis le registre réel, et un jeu de phrases inédites qui mesure honnêtement ce que le modèle a appris. Faute de barre chiffrée, on ne saura pas si le modèle ajusté vaut Qwen3.5-9B.

**Approach:** Tâche 1 sur 4 du ticket 5.10, découpé par le propriétaire le 2026-10-22 (tâches 2 à 4 dans `deferred-work.md`). Un générateur Rust déterministe (graine fixe) croise des familles de tournures métier (urbanisme, environnement, réseaux, SIG général) avec des inventaires synthétiques. Il construit le plan attendu, puis garde l'exemple seulement s'il passe `Registry::check_plan` (questions et refus compris), et écrit des enregistrements structurés JSONL `{inventory, request, expected}`, sans texte de prompt. À côté : un jeu tenu à l'écart (la phrase de démo exacte de CLAUDE.md comprise) et le score de Qwen3.5-9B avec le prompt complet sur ce jeu.

## Boundaries & Constraints

**Always:** Enregistrements structurés seulement : la mise en forme en prompt appartient aux tâches 2 et 3. Un exemple dont le plan échoue à `check_plan` n'est jamais écrit. Générateur reproductible : même graine, même fichier, à l'octet près. Aucune nouvelle dépendance : le hasard vient d'un petit générateur pseudo-aléatoire écrit à la main. Nouveau crate `training/` (`cassini-training`, `publish = false`, Apache-2.0), membre du workspace, qui dépend de `cassini-core` et pas de `cassini-ai`. Vocabulaire civil seulement (CLAUDE.md §2.9). Le jeu tenu à l'écart est écrit sans que l'auteur du générateur l'ait lu (décision du 2026-10-17). Un test vérifie qu'aucune de ses phrases n'apparaît dans le jeu engendré (comparaison sur le texte normalisé : minuscules, sans accents ni ponctuation). Une phrase mal comprise devient une famille d'exemples, jamais une règle de prompt. Décisions du propriétaire (2026-10-22) : les familles civiles de demandes types sont rédigées par Claude dans le générateur, sans fichier externe (la seule liste trouvée hors dépôt est celle de la défense, exclue) ; le jeu tenu à l'écart est écrit par un sous-agent séparé ; le plan reste entier malgré ses ~2 000 jetons.

**Never:** Toucher le prompt, la grammaire, `judge` ou `compose_plan` de `ai/src/plan.rs`. Toucher `tests/epreuve-modele-local/` (gelé). Ajouter une commande au registre. Entraîner, fusionner en GGUF ou écrire du Python (tâche 2). Versionner un fichier tiré de `../donnees-test-cassini/`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Génération | graine, nombre visé | JSONL : chaque ligne `{inventory, request, expected}`, `expected` = plan, question ou refus | — |
| Plan invalide engendré | une tournure donne un plan que `check_plan` refuse | exemple écarté et compté, jamais écrit | le résumé imprime les écartés par famille |
| Même graine deux fois | — | fichiers identiques | — |
| Recouvrement | une phrase tenue à l'écart engendrée telle quelle | le test de recouvrement échoue en la citant | — |
| Score de référence | modèle absent | essai sauté, message sur `stderr` | — |

</frozen-after-approval>

## Code Map

- `core/src/registry.rs` -- `Registry::standard()`, `specs()`, `spec(id)` ; `check_plan` dans `core/src/plan.rs` (format `Plan::{Calls,Refusal,Question}`, chaînes `step:k.layer_id`) : lecture seule.
- Commandes réelles : `layer.add` (`path`), `layer.describe`, `project.inventory`, `analysis.{buffer,intersect,select_by_attribute,isochrone,zonal_statistics}`, `layout.export_pdf` (`title`), `model.list`, `system.about`. Opérateurs de sélection : `eq ne lt lte gt gte contains begins_with` ; pas de `in`, ni d'union.
- `ai/src/plan.rs` -- `examples()` (l.127) et `SYSTEM` (l.117) : forme des plans et des inventaires à reprendre, et règles de chaînage (le tampon sur la couche de référence, les intersects successifs, la couche cherchée en tête). À ne pas modifier.
- `ai/tests/plan_reel.rs` -- modèle de l'essai de score (`compose_plan` + `model_generator`, comparaison exacte des `calls`, `#[ignore]`, `CASSINI_TEST_MODEL`).
- `ai/tests/question_reel.rs` -- les 12 phrases ambiguës (vues, donc pas inédites).
- `tests/epreuve-modele-local/inventory.json` -- forme d'un inventaire (`layer_id`, `name`, `geometry`, `source`, `fields[].values`). Les vraies valeurs du PLU de Villeneuve (`typezone` : U, N, Nh) sont dans l'inventaire hors dépôt, à imiter et jamais copier.
- `deferred-work.md` l.196 et l.208 -- tournures à couvrir : question écrite dans un refus, « garde seulement / ne garde que / conserve ».
- `Cargo.toml` -- ajouter `training` aux `members`.

## Tasks & Acceptance

**Execution:**
- [x] `ai/tests/tenu_a_l_ecart/` (`inventory.json`, `sentences.json`) -- écrit **en premier par un sous-agent séparé** ; 40 phrases inédites sur un inventaire de projet d'urbanisme : 26 plans (la phrase de démo exacte comprise), 8 questions, 6 refus -- la barre honnête
- [x] `training/Cargo.toml`, `Cargo.toml` -- nouveau crate -- foyer de la chaîne d'entraînement
- [x] `training/src/{lib.rs,rng.rs,inventories.rs,families.rs}` -- générateur : inventaires synthétiques variés (noms, identifiants, champs, valeurs) ; familles de tournures avec emplacements ; plan attendu construit puis filtré par `check_plan` ; tests unitaires (déterminisme, filtre, chaque famille rend au moins un exemple valide) -- le cœur
- [x] `training/src/main.rs` -- `cargo run -p cassini-training -- --seed N --count M --out <fichier.jsonl>`, résumé par famille sur `stderr` -- « une commande pour régénérer »
- [x] `training/tests/recouvrement.rs` -- génère avec la graine par défaut et vérifie qu'aucune phrase tenue à l'écart n'y figure -- garde-fou
- [x] `ai/tests/ecart_reel.rs` -- score sur le jeu tenu à l'écart (`#[ignore]`) : plans comparés exactement, sauf le `title` du PDF (sa présence suffit) ; question et refus comptés par sorte ; ligne `SCORE` -- la barre chiffrée
- [x] `docs/entrainement-du-modele.md` -- commande de génération, familles, règle du jeu tenu à l'écart, score de référence du 9B -- consigné

**Acceptance Criteria:**
- Given la graine par défaut, when on génère 5 000 exemples, then le fichier compte au moins 5 000 lignes, toutes relues sans erreur par `check_plan`, et deux générations sont identiques à l'octet.
- Given le jeu engendré, when on le parcourt, then chaque famille de la décision du 2026-10-22 y figure (vocabulaire d'urbanisme comme « constructible », filtres « garde seulement », distances, chaînes à plusieurs conditions, PDF, questions sur couche ou champ absents, refus).
- Given Qwen3.5-9B Q4_K_M sur GPU, when `ecart_reel` tourne, then le score de référence est noté dans `docs/entrainement-du-modele.md` et dans ce plan.

## Plan Change Log

## Review Triage Log

Passe 1 (quick puis thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment ; la relecture approfondie a été ajoutée par CLAUDE.md §8, risque high). 0 high, 9 medium, 9 low, 4 false ; aucun intent_gap ni bad_plan.

| # | Verdict | Route | Evidence |
|---|---------|-------|----------|
| 1 | medium | patch | `inventories.rs` l.259 : `UA`, `UB` à côté du `U` forcé ; « constructible » → `typezone eq U` apprend à oublier des zones urbaines. Valeurs retirées (pas CNIG). |
| 2 | medium | patch | `question_champ_absent`, branche constructible : `libelle` (« Zone urbaine ») gardé, question attendue contradictoire. `libelle` interdit. |
| 3 | medium | patch | `ouverture` l.1426-1433 : `replacen` produit « des captages des eau potable ». Article de tête seul remplacé. |
| 4 | low | patch | `filtre_different` l.667 « toutes les bâtiments », `refus` l.1700 « de Annecy ». Accord par `fem`, élision. |
| 5 | low | patch | `validate` vérifie `contains('?')`, la doc dit « finit par » ; gabarit l.1517 finit par un point. `ends_with`, gabarit reformulé. |
| 6 | low | patch | `DEFAULT_SEED` « jeu versionné » contredit la doc. Commentaire corrigé. |
| 7 | medium | patch | Clé `family` dans le JSONL, hors du format figé `{inventory, request, expected}`. Retirée de la ligne. |
| 8 | medium | patch | AC 1 énoncé à la graine par défaut et 5 000 exemples, testé à 1 500 et 400. Tests portés à l'échelle. |
| 9 | medium | patch | `recouvrement.rs` ne voit que l'identique ; `draft_constructible` + `pdf_clause` peut rendre une quasi-copie de H01 (autre distance, autre commune). Seconde comparaison, chiffres et communes masqués. |
| 10 | medium | patch | `recette.yml` l.54 ne lance pas `cassini-training` : gardes de recouvrement et de validité jamais jouées en CI. `-p cassini-training` ajouté. |
| 11 | low | patch | `the_held_out_set_is_well_formed` ne vérifie pas les couches et chemins contre l'inventaire tenu à l'écart. Vérification ajoutée. |
| 12 | medium | patch | `ecart_reel` : l'assertion `check_plan` en cours de boucle perd la ligne SCORE ; `500.0` ≠ `500` en égalité de `Value`. Plan invalide compté en échec, nombres normalisés ; score de référence re-mesuré. |
| 13 | low | patch | `exemples.jsonl` non ignoré par git ; `docs/README.md` sans ligne vide ; score de référence sans commit du prompt. Corrigés. |
| 14 | medium | defer | Inventaire du produit sans valeurs de champs (`inventory.py` l.211) contre jeu engendré avec valeurs : à trancher avant la tâche 2. |
| 15 | medium | defer | Aucune paire de couches quasi synonymes dans le jeu engendré, présentes dans le jeu tenu à l'écart. Nouvelle famille : tâche 2. |
| 16 | low | defer | Décision de notation de `ecart_reel` testée seulement sous `#[ignore]`, sans diagnostic par échec. Tâche 4. |
| 17 | low | defer | La commande `cassini-training` n'a pas de test d'intégration ; `cmp` à la main. |
| 18 | low | defer | CLAUDE.md §9 ne cite pas `/training` : fichier de contexte d'agent, au propriétaire. |
| 19 | low | rejeté | `MAX_STEPS` recopié de `cassini-ai` : `training` ne dépend pas de `ai` (exigé), un test croisé ajouterait une dépendance ; plafond stable. |
| 20 | low | rejeté | `--count` énorme ou nul : outil de développeur, échec bruyant acceptable. |
| 21 | low | rejeté | Doublons comptés sur la ligne entière : une même demande sur deux inventaires est un exemple distinct voulu. |
| 22 | low | rejeté | `validate` ne vérifie pas `field`/`value` contre l'inventaire : toutes les familles les tirent de la couche engendrée (`require`) ; garde non démontrée. |
| 23 | false | rejeté | `Ok(Plan::Refusal)` compté faux : `compose_plan` (`ai/src/plan.rs` l.382) rend toujours `Err(Refused)`. |
| 24 | false | rejeté | Entrées `deferred-work` en `source_plan: none` : format imposé par l'étape 1 (découpe avant le plan). |
| 25 | false | rejeté | `review: quick` pour un risque high : la relecture approfondie a été lancée (`lenses_ran`). |
| 26 | false | rejeté | `ticket: '10'` ambigu : le gabarit impose l'id de `find`. Format de `expected` décrit dans le plan : correctif = éditer le plan, rejeté. |

## Design Notes

« Constructible » se traduit par `typezone` égal à `U` : c'est la zone urbaine du standard CNIG. `AUc` est aussi constructible, mais le registre n'a ni `in` ni union, donc la tournure ne peut pas le couvrir. La limite est écrite dans le document.

Sous-agent du jeu tenu à l'écart : il reçoit le catalogue (`cassini run` ou les fiches), les règles de chaînage et la phrase de démo. Il ne reçoit ni les familles du générateur ni ses tournures. L'implémenteur du générateur ne lit pas `tenu_a_l_ecart/` : seuls le test de recouvrement et l'essai de score le lisent. Ordre imposé : générateur, familles et leurs tests d'abord, commités dans l'arbre de travail ; ensuite seulement `recouvrement.rs` et `ecart_reel.rs`, écrits d'après le format décrit sous Verification, sans afficher le contenu des phrases. Aucune famille n'est retouchée après coup pour coller au jeu tenu à l'écart ; si le recouvrement échoue, on retire la tournure fautive du générateur et on le signale.

## Verification

Environnement : sous WSL, `cassini-ai` ne compile pas (libclang manque à llama.cpp) ; `core` et `training` s'y testent. Tout ce qui touche `ai` (et `cargo test --workspace`) se lance sous Windows depuis WSL : `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test --workspace'` (les lignes « NativeCommandError » ne sont que le stderr de cargo). Modèle : `C:\Users\kevin\cassini-modeles\modeles\Qwen3.5-9B-Q4_K_M.gguf`. Le jeu tenu à l'écart est déjà écrit (`ai/tests/tenu_a_l_ecart/`, fichiers non ouverts sous la clé `unloaded_files`) ; son `sentences.json` porte `{id, text, expected, family}`, `expected` = plan `calls`, `{"kind":"question"}` ou `{"kind":"refusal"}`.

**Commands:**
- `cargo test --workspace`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo deny check` -- expected: succès
- `cargo run -p cassini-training --release -- --seed 1 --count 5000 --out <scratch>/a.jsonl` deux fois, puis `cmp` -- expected: identiques
- PowerShell, `--features cuda` : `CASSINI_TEST_MODEL=<Qwen3.5-9B Q4_K_M> cargo test -p cassini-ai --release --features cuda -- --ignored --nocapture ecart_reel` -- expected: une ligne `SCORE`

## Score de référence

Mesuré le 2026-10-22 : Qwen3.5-9B Q4_K_M sur GPU, prompt complet, jeu tenu à l'écart :
`SCORE 32/40 : plans 19/26, questions 7/8, refus 6/6` (344 s). Les 7 plans manqués ont rendu
un plan ; la question manquée a rendu un refus. Recouvrement : aucune phrase tenue à
l'écart dans les 5 000 exemples de la graine 1.

Contre-mesure après les correctifs de la relecture (notation : nombres normalisés, plan invalide compté faux) : identique, `SCORE 32/40 : plans 19/26, questions 7/8, refus 6/6` (346 s), aucun plan invalide rendu. La phrase de démo H01 est parmi les plans manqués (plan rendu, mais pas le plan attendu). Suite complète sous Windows verte (fmt, clippy `-D warnings`, `cargo test --workspace`, `cargo deny`) ; recouvrement exact et quasi-copie (chiffres et communes masqués) : aucun.
