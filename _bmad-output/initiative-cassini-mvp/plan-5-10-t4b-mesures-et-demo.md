---
title: 'Modele ajuste, tache 4b : mesures et demo'
type: 'feature'
ticket: '10'
created: '2026-10-23'
status: 'built'
baseline_revision: '540f59c763eb6ffcadf4ce943484dee5244be130'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['quick', 'blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: ['{project-root}/docs/entrainement-du-modele.md', '{project-root}/docs/mesure-duree-par-phrase.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le modèle ajusté de la tâche 2, branché par la tâche 3 et nourri dans QGIS par l'inventaire de la tâche 4a, n'a encore jamais été mesuré contre la barre du ticket 5.10 : 32/40 sur le jeu tenu à l'écart (score de Qwen3.5-9B), seuil strict de clarification, moins de 30 s par longue phrase sur le 5800X bridé à 4 cœurs, et la démo de bout en bout dans QGIS avec la phrase exacte de `CLAUDE.md`.

**Approach:** Tâche 4b, dernière du ticket 5.10. Les essais réels de `ai/tests/` acceptent une déclaration `*.modele.json` (base + adaptateur, ou fusionné, prompt court) ; le notateur du jeu tenu à l'écart devient une fonction testée sans modèle ; puis on mesure, on consigne, et Claude fait la démo dans QGIS avant la recette du propriétaire.

## Boundaries & Constraints

**Always:** Essais de justesse sur GPU (`--features cuda`), mesure de durée sur CPU seul (binaire sans CUDA, `CASSINI_TEST_THREADS=4`). Les deux chemins (base + adaptateur, fusionné) sont mesurés. Le texte des phrases tenues à l'écart n'est jamais imprimé : seuls l'id, la sorte et la forme de l'écart (commande, ordre, paramètre) le sont. Un chiffre sous la barre est consigné tel quel, jamais arrangé. Une déclaration se résout d'une seule façon, dans `cassini-ai`, partagée par la CLI et les essais. Une phrase mal comprise devient une famille d'entraînement à venir, jamais une règle de prompt. Aucune dépendance Rust ajoutée. Décisions du propriétaire (2026-10-23) : sous la barre (score, seuil de questions ou durée), les chiffres sont consignés tels quels, 4b se ferme comme « mesuré », et un ticket de ré-entraînement est proposé (5.10 reste ouvert) ; la démo passe par la sonde seule, qui montre au noyau le seul dossier du modèle ajusté (`CASSINI_MODELS`), sans toucher au dossier des modèles du poste.

**Never:** Ré-entraîner, toucher le prompt court, `SHORT_SYSTEM`, la grammaire, `judge`, le générateur ou ses familles. Lire le texte du jeu tenu à l'écart. Toucher `tests/epreuve-modele-local/` (gelé). Versionner un fichier tiré de `../donnees-test-cassini/`. Écrire dans un fichier de ticket.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Modèle déclaré | `CASSINI_TEST_MODEL=<x>.modele.json` | base, adaptateur et prompt court de la déclaration | déclaration absente ou illisible : l'essai échoue en la nommant |
| GGUF seul | `CASSINI_TEST_MODEL=<x>.gguf` | long prompt, comme avant | — |
| Notation | plan rendu = attendu sauf titre du PDF | juste | — |
| Notation | refus attendu, `Err(Refused)` | juste | — |
| Notation | question attendue, plan rendu | faux, écart « sorte » | — |
| Notation | plan rendu, étapes dans un autre ordre | faux, écart « ordre » | — |
| Notation | même commande, autre valeur de paramètre | faux, écart « paramètre `<nom>` à l'étape k » | — |
| Plafond de questions | prompt court | au plus 1 fausse question sur 25 | au-delà : échec après `SCORE` |
| Plafond de questions | long prompt | au plus 2 (concession 5.5 inchangée) | idem |

</frozen-after-approval>

## Code Map

- `cli/src/main.rs` l.237-262 -- `resolve_model` (déclaration → `ModelFiles` + `PromptStyle`) : à déplacer dans `cassini-ai` (par ex. `ai/src/plan.rs`, `pub fn resolve_model(path) -> Result<(ModelFiles, PromptStyle), AiError>`), la CLI ne garde que la mise en forme de l'erreur (`report`, `models.bad_declaration`). Tests CLI l.801-890 à garder verts.
- `core/src/commands/model_list.rs` -- `is_declaration`, `read_declaration`, `PromptKind` : lecture seule.
- `ai/src/plan.rs` -- `compose_plan_with` (l.424), `model_files_generator` (l.469), `PromptStyle` (l.22) : à réutiliser ; `ai/src/lib.rs` `generate_files_measured` (l.505) pour la durée.
- `ai/tests/ecart_reel.rs` -- notation en ligne dans la boucle (l.190-205) : sortir `fn grade(expected: &Value, got: &Result<Plan, AgentError>) -> Result<(), Gap>` et ses tests sans modèle (matrice) ; imprimer l'écart ; compter les phrases passées par la redemande (appels au générateur par phrase) et celles qui y ont été sauvées.
- `ai/tests/question_reel.rs`, `ai/tests/plan_reel.rs`, `ai/tests/duree_reel.rs` -- passer par le modèle résolu ; `plan_reel` : plafond de fausses questions selon le style ; `duree_reel` : ajouter la phrase de démo exacte de `CLAUDE.md` au `SCENARIO` (id `A0`).
- Nouveau `ai/tests/common/mod.rs` -- `test_model() -> Option<(ModelFiles, PromptStyle)>` lu de `CASSINI_TEST_MODEL`, partagé par les quatre essais.
- `training/entrainement/` -- nouvelle commande `parite_decoupeurs.py` (lancée à la main, dans l'environnement `uv` de la chaîne) : sur 300 prompts du jeu `--format chat`, jetons du découpeur Hugging Face de la base (révision épinglée) contre `llama-tokenize` du GGUF de base (llama.cpp du cache) ; imprime le nombre d'écarts et le premier. Test sans torch de sa comparaison dans `training/entrainement/tests/`.
- Modèles : `C:\Users\kevin\cassini-modeles\ajuste-5-10-t2\` (trois GGUF, deux déclarations) ; 9B : `C:\Users\kevin\cassini-modeles\modeles\Qwen3.5-9B-Q4_K_M.gguf`.
- Démo : sonde jetable hors dépôt sous `python-qgis-ltr.bat` (plateforme offscreen), vrai `cassini.exe serve` par `qgis-plugin/cassini_qgis/client.py`, projet `../donnees-test-cassini/projet-test-villeneuve.qgz` ; elle chronomètre `project.inventory`, écrit l'inventaire traduit dans `../donnees-test-cassini/` (pour `CASSINI_TEST_INVENTORY`), propose la phrase exacte, valide, exécute, relève le PDF et le journal.
- `docs/entrainement-du-modele.md` -- section « Tâche 4 : mesures » ; `deferred-work.md` -- clore les entrées 4b tenues.

## Tasks & Acceptance

**Execution:**
- [x] `ai/src/plan.rs`, `cli/src/main.rs` -- résolution de déclaration déplacée dans `ai`, testée -- une seule façon de lire un modèle
- [x] `ai/tests/common/mod.rs`, les quatre essais réels -- modèle résolu, plafond selon le style, `A0` -- mesurer le modèle ajusté avec son prompt
- [x] `ai/tests/ecart_reel.rs` -- `grade` et ses tests (matrice), écart imprimé, compte des redemandes -- notateur fiable
- [x] `training/entrainement/parite_decoupeurs.py` et son test -- parité des découpeurs
- [x] Mesures (GPU : `ecart_reel`, `question_reel`, `plan_reel` sur les deux déclarations et le 9B pour `ecart_reel` ; CPU 4 fils : `duree_reel` sur l'inventaire réel traduit ; parité) -- consignées dans le document et ce plan
- [x] Démo QGIS de bout en bout -- résultat consigné
- [x] `docs/entrainement-du-modele.md`, `deferred-work.md` -- consigné

**Acceptance Criteria:**
- Given chaque déclaration, when `ecart_reel` tourne sur GPU, then sa ligne `SCORE` est consignée à côté de 32/40, avec le nombre de redemandes et de sauvetages.
- Given chaque déclaration, when `question_reel` et `plan_reel` tournent, then les deux scores sont consignés contre le seuil strict.
- Given la déclaration retenue, when `duree_reel` tourne sur CPU à 4 fils sur l'inventaire réel, then la durée de `A0`, chargement et instantané à part, est consignée contre 30 s.
- Given QGIS et le vrai projet, when Claude envoie la phrase de démo exacte, then le plan, son exécution, le PDF et la durée de l'inventaire sont consignés, réussite ou échec.
- Given la recette complète, when elle tourne, then elle est verte.

## Implementation Notes

- `resolve_model` vit dans `ai/src/plan.rs` et rend `Result<(ModelFiles, PromptStyle), AiError>` ; une variante `AiError::BadDeclaration` (code `models.bad_declaration`) porte le message d'une déclaration illisible, déjà réduit au seul nom du fichier. La CLI ne fait plus que `report(e.code(), e.to_string())` ; ses tests l.801-890 restent verts.
- `grade` renvoie un `Gap` (sorte, titre du PDF absent, nombre d'étapes, ordre, commande, paramètre). L'ordre se reconnaît en remplaçant chaque `step:k.champ` par l'étape k résolue, puis en comparant les étapes comme un multiensemble. Un `Ok(Plan::Refusal)` compte comme refus (jamais rendu par `compose_plan`, sans effet sur le score).
- `duree_reel` : pour que la ligne de A0 soit « chargement et instantané à part », le scénario passe d'abord une amorce (« Décris le projet. ») sur le même inventaire, exclue des moyennes.
- Inventaire réel : mesuré par le noyau et l'extension actuels, 4 007 caractères, 1 458 jetons (2 439 en 4a, chiffre pris avant les correctifs de la relecture de 4a ; aucun filtre de longueur n'explique l'écart). Écrit hors dépôt : `../donnees-test-cassini/inventaire-villeneuve-traduit-4b.json`.
- Démo : noyau de mesure construit avec `--features cassini-ai/cuda` (copie `C:\ct\mesures-4b\cassini-cuda.exe`) ; `CASSINI_MODELS` sur `C:\Users\kevin\cassini-modeles\demo-4b-fusionne\` (liens durs du GGUF fusionné et de sa déclaration), le dossier des modèles du poste n'est pas touché. Sonde versionnée : `qgis-plugin/tests/probe_demo.py` (projet en argument, `CASSINI_CORE`, `CASSINI_MODELS`, `CASSINI_PROBE_PATH` facultatif pour les DLL de CUDA).

### Mesures (2026-10-23)

| Mesure | base + adaptateur | fusionné | Qwen3.5-9B | Barre |
|---|---|---|---|---|
| `ecart_reel` (GPU) | 32/40 (19/26, 8/8, 5/6), redemandes 1, sauvées 0 | 33/40 (20/26, 8/8, 5/6), redemandes 2, sauvées 0 | 32/40 (19/26, 7/8, 6/6), redemandes 1, sauvées 1 | 32/40 |
| `question_reel` (GPU) | 11/12 | 11/12 | — | ≥ 9 |
| `plan_reel` (GPU) | 16/25, 0 question | 16/25, 0 question | — | ≥ 20, ≤ 1 question |
| `duree_reel` A0 (CPU 4 fils, `0x55`) | 43,2 s | 41,4 s | — | < 30 s |
| Parité des découpeurs | 300/300 prompts identiques (205 803 jetons) | | | 0 écart |
| Démo QGIS (fusionné, GPU) | plan de 5 étapes juste, exécuté, PDF `30-layout.export_pdf.pdf` ; `project.inventory` 0,05 s ; composition 24,8 s | | | réussite |

Déclaration retenue : le fusionné. Sous la barre : `plan_reel` (refus de terrain, cadrage, A3, raster jamais appris) et A0 (écriture de 248 jetons à ~6 jetons/s). 4b fermée comme « mesurée », ré-entraînement proposé dans `deferred-work.md`.

Recette finale après les correctifs de relecture (2026-10-23) : `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` (revérification forcée), `cargo test --workspace` (Windows), `cargo deny check`, tests Python de l'extension et de `training/entrainement`, `check_extension.py` : verts. Les essais réels restent `#[ignore]` ; `plan_reel` échoue sa barre des 80 % avec le modèle ajusté (16/25), consigné.

## Plan Change Log

## Review Triage Log

Passe 1 (thorough : quick, blind-hunter, edge-case-hunter, verification-gap, intent-alignment ; CLAUDE.md §8, risque high ; quatre lentilles reprises après une limite d'usage). 0 high, 6 medium, 9 low, 3 false ; aucun intent_gap ni bad_plan.

| # | Verdict | Route | Evidence |
|---|---------|-------|----------|
| 1 | medium | patch | `parite_decoupeurs.py` : `llama-tokenize -f` applique `string_process_escapes` par défaut (`tools/tokenize/tokenize.cpp` l.131) ; aucun des 300 prompts ne porte de barre oblique inverse (vérifié), le 300/300 tient. `--no-escape` ajouté. |
| 2 | medium | patch | `pipeline.py` l.380 ne construit pas `llama-tokenize`, que l'outil de parité dit fourni par la chaîne : mesure non reproductible. Cible ajoutée. |
| 3 | medium | patch | `plan_reel` : l'assertion des 80 % précède celle du plafond de questions ; à 16/25 le plafond strict n'est jamais vérifié (le « 0 question » vient de la ligne imprimée). Les deux vérifiées. |
| 4 | medium | patch | Bilan de durée « à chaud » seulement : la première demande d'une session sur CPU (chargement, lecture entière, A0) n'y figure pas. Ligne ajoutée ; comparaison « 4 jetons/s pour le 9B » sans source, citée ou retirée. |
| 5 | medium | patch | `deferred-work.md` : « les valeurs trop longues ne sont plus gardées » ; aucun code ne borne la longueur (`core/src/agent_inventory.rs`). Énoncé factuel. |
| 6 | medium | patch | Démo non rejouable depuis le dépôt (sonde dans le dossier temporaire). Versionnée en `probe_demo.py`, chemins en arguments. |
| 7 | low | patch | `grade` : repli « inatteignable » étiqueté « ordre » quand deux étapes diffèrent par une autre clé ; `Gap::Title` à l'étape 0. Comparaison de l'étape entière, titre seulement s'il manque. |
| 8 | low | patch | `parite_decoupeurs.py` : `--nombre` nul ou jeu trop court rend `PARITE 0/0` ; ligne illisible en trace brute. Gardes. |
| 9 | low | patch | `duree_reel` : « amorce exclue » imprimé sans amorce ; amorce ratée non signalée ; exemple à 8 fils, masque d'affinité sans commande. Corrigés. |
| 10 | low | patch | En-têtes de `question_reel` et `plan_reel` sans `--features cuda`. Ajouté. |
| 11 | low | defer | Agrégation des moyennes (amorce exclue) et compte `REDEMANDES` sans test sans modèle (verification-gap, disposition defer). |
| 12 | low | rejeté | `plan_reel` ne partage pas `grade` : sa comparaison exacte est celle de 5.3, hors du plan ; aucun score faussé démontré. |
| 13 | low | rejeté | `resolve_model` ne vérifie pas l'existence des GGUF nommés : comportement repris de la CLI, `ModelNotFound` au chargement, déjà suivi pour 5.14. |
| 14 | low | rejeté | Plafond de questions d'un modèle ajusté déclaré sans `prompt` : le plafond suit le prompt, comme le veut la matrice. |
| 15 | low | rejeté | Dossier temporaire du test de déclaration laissé si une assertion échoue ; booléens acceptés par `parse_ids` : sans effet sur une vraie sortie. |
| 16 | false | rejeté | Révélation de H01 comme phrase de démo : écrit dans la doc dès la tâche 1. |
| 17 | false | rejeté | En-tête « in-review » sans triage : relecture en cours, ce tableau la consigne. |
| 18 | false | rejeté | Démo par sonde et non par le panneau : décision du propriétaire (2026-10-23) dans la partie gelée. |

## Verification

Environnement : `core` et `training` sous WSL ; ce qui compile `ai` passe par PowerShell (`LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"`, `CARGO_TARGET_DIR=C:\ct`).

**Commands:**
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `cargo deny check` ; `python3 -I -m unittest discover -s training/entrainement/tests` -- expected: succès
- `CASSINI_TEST_MODEL=<déclaration> cargo test -p cassini-ai --release --features cuda -- --ignored --nocapture ecart_reel` (puis `question_reel`, `plan_reel`) -- expected: lignes `SCORE`
- `CASSINI_TEST_MODEL=<déclaration> CASSINI_TEST_THREADS=4 CASSINI_TEST_INVENTORY=<inventaire traduit> cargo test -p cassini-ai --release -- --ignored --nocapture duree_reel` (sans `cuda`) -- expected: durées par phrase
