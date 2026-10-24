---
title: 'Plusieurs phrases enchaînées dans une session sans saturer le contexte'
type: 'bugfix'
ticket: '18'
created: '2026-10-24'
status: 'built'
baseline_revision: 'b0e8548af977d0b72e8fc3fc4b5b4600fe20edb0'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Dans une même session QGIS, chaque plan exécuté ajoute ses couches de résultat au projet, donc à l'inventaire lu par le modèle ; dès la 4e phrase de la recette de 5.17, `plan.propose` échoue en `ai.prompt_too_long` (N_CTX 8192), et le message accuse la demande alors que l'inventaire est en cause.

**Approach:** Budget d'inventaire dans le noyau : le modèle voit toujours toutes les couches de l'utilisateur, plus les couches de résultat Cassini les plus récentes qui tiennent dans le budget ; les plus anciennes sont retirées de la vue du modèle seulement (rien n'est retiré de QGIS). Le message de `ai.prompt_too_long` nomme l'inventaire.

**Decision (2026-10-24, propriétaire) :** budget d'inventaire choisi, contre un contexte plus grand (qui repousse seulement le problème et allonge chaque phrase) et contre les deux à la fois. N_CTX reste à 8192.

## Boundaries & Constraints

**Always:** correctif du noyau, sans ré-entraînement ; la forme de chaque couche gardée est inchangée (même clés, mêmes champs, mêmes valeurs) : le test `training/tests/forme_inventaire.rs` reste vert ; les couches de l'utilisateur ne sont jamais retirées ; une couche de résultat est une couche dont le fichier source est dans le dossier de travail Cassini du projet (`<projet>.cassini/`) ; parmi elles, les plus récentes (date du fichier) restent ; le retrait est déterministe ; l'inventaire n'est toujours pas journalisé.

**Never:** aucune règle de prompt, aucun changement de `SHORT_SYSTEM`, de la grammaire, de `judge` ni du tour de redemande ; ne pas retirer de champs ni couper de valeurs d'une couche gardée ; ne pas toucher à la reprise de « ces zones » (famille d'entraînement, hors ticket) ; ne pas changer ce que l'extension ajoute au projet.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Sous le budget | inventaire de Villeneuve, aucune couche de résultat | inventaire vu par le modèle identique à aujourd'hui | aucune |
| Au-delà du budget | couches de l'utilisateur + 10 résultats | toutes les couches de l'utilisateur, puis les résultats les plus récents qui tiennent ; les plus anciens absents | aucune |
| Couches utilisateur seules trop grosses | aucune couche de résultat, inventaire > contexte | `ai.prompt_too_long` | message qui nomme l'inventaire du projet et conseille de retirer des couches |
| Pas de dossier de travail | session sans projet enregistré | aucune couche reconnue comme résultat, rien retiré | aucune |
| Plan qui cite un résultat retiré | `layer_id` absent de l'inventaire vu | refusé comme aujourd'hui (AD-23) | erreur existante |

</frozen-after-approval>

## Code Map

- `core/src/serve/plans.rs:119` `propose` -- ferme l'inventaire par `agent_inventory(&raw)` ; y appliquer le budget, avec le dossier de travail de la session (`state.session`, `Session::work_dir`, `core/src/journal/mod.rs:196`).
- `core/src/agent_inventory.rs` -- traduction vers la forme apprise ; y ajouter la fonction pure de budget (entrée : inventaire traduit, noms des fichiers de résultat datés, budget ; sortie : inventaire réduit) et ses tests. `source` est un nom de fichier seul (`core/src/commands/project_inventory.rs:38`), à confronter aux fichiers du dossier de travail (ex. `21-analysis.intersect.gpkg`).
- `ai/src/lib.rs:27` `N_CTX` 8192, `:576` contrôle `PromptTooLong`, `:71` message à corriger ; `ai/src/plan.rs:43` `MAX_PLAN_TOKENS` 1024, `:47` `MAX_ECHO_CHARS` 1500 (redemande), `:311` `short_inventory_text`.
- `training/src/inventories.rs:1405` `finish` -- l'entraînement mélange l'ordre des couches : mettre les résultats après les couches de l'utilisateur ne sort pas de la forme apprise.
- `qgis-plugin/tests/probe_demo.py` -- modèle de la sonde jetable de la recette enchaînée (le script de 5.17 n'existe plus).
- Environnement des essais à modèle réel (Windows, depuis WSL) : `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; $env:CASSINI_TEST_MODEL="<gguf>"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test -p cassini-ai --features cuda ...'` ; modèle livré : `C:\Users\kevin\AppData\Local\Cassini\bin\models\cassini-composition-4b-Q4_K_M.gguf` (déclaration `cassini-composition-4b.modele.json` à côté, prompt court). Script .ps1 écrit depuis WSL : préfixer par le BOM UTF-8. `cargo fmt` et les tests Python se lancent sous WSL.
- À ne pas changer : `SHORT_SYSTEM`, `fixed_system_text`, `judge`, `retry_turns`, la forme de `agent_inventory` pour une couche.

## Tasks & Acceptance

**Execution:**
- [x] `ai/` -- mesurer avec le vrai tokeniseur (test ignoré, modèle livré) le nombre de jetons par caractère de l'inventaire de Villeneuve et d'une couche d'intersection à 34 champs ; en tirer un budget en caractères qui laisse la place à la réponse (1024) et à la redemande -- le noyau ne tokenise pas.
- [x] `core/src/agent_inventory.rs` -- budget en caractères (constante documentée par la mesure) et fonction de réduction pure, avec tests de la matrice.
- [x] `core/src/serve/plans.rs` -- lister les fichiers datés du dossier de travail et appliquer la réduction à l'inventaire traduit ; test par le compositeur factice qu'un inventaire de dix résultats arrive réduit et dans l'ordre attendu.
- [x] `ai/src/lib.rs` -- message de `PromptTooLong` qui nomme l'inventaire du projet.
- [x] `docs/` -- consigner la mesure et le budget là où sont documentés les chiffres du modèle.

**Acceptance Criteria:**
- Given la recette enchaînée de dix phrases (sonde jetable, vrai noyau, vrai modèle, projet de Villeneuve), when elle tourne, then aucune phrase ne rend `ai.prompt_too_long` et les phrases 1 à 3 rendent les mêmes plans qu'à la recette de 5.17.
- Given le jeu de mesures sur GPU, when il tourne, then `plan_reel`, `question_reel` et `ecart_reel` ne baissent pas par rapport à 5.17.
- Given la recette complète, when elle tourne, then elle est verte.

## Implementation Notes

- Mesure (tokeniseur du modèle livré, test ignoré `the_inventory_budget_fits_the_context_with_the_real_tokenizer`, `ai/src/plan.rs`) : Villeneuve 4 007 car. / 1 458 jetons (0,364) ; intersection à 34 champs 1 999 / 724 (0,362) ; hors inventaire 359 jetons, redemande au pire +1 106. Budget `INVENTORY_BUDGET_CHARS` = 14 000 caractères (JSON compact) : au budget, redemande 6 501 + 1 024 = 7 525 / 8 192. Le test vérifie ce pire cas.
- `within_budget` (pure, `core/src/agent_inventory.rs`) : sous le budget, inventaire rendu tel quel (ordre compris) ; au-delà, seuls des résultats sont retirés, l'ordre de l'inventaire restant celui d'entrée. Choix des plus récents : date décroissante, numéro d'étape du nom décroissant (sans numéro en dernier), nom décroissant, rang ; arrêt au premier résultat qui ne tient pas (aucun résultat plus ancien n'est gardé à sa place). Couches de l'utilisateur seules au-delà du budget : toutes gardées, l'IA rend `ai.prompt_too_long`.
- `plans.rs` lit `read_dir` du dossier de travail et de `replay/` après la réponse de `project.inventory` ; dossier illisible : aucun fichier, rien retiré.
- Recette enchaînée (10 phrases de 5.17, noyau CPU du produit construit, modèle livré, sonde jetable hors dépôt) : aucune `ai.prompt_too_long`, phrases 1 à 3 identiques à 5.17. GPU : `plan_reel` 25/25, `question_reel` 12/12, `ecart_reel` 33/40, identiques à 5.17.
- Le noyau construit n'est pas installé dans `%LOCALAPPDATA%\Cassini\bin` : à faire avant la recette du propriétaire.

## Plan Change Log

## Review Triage Log

Passe 1 (2026-10-24, lentilles blind-hunter, edge-case-hunter, verification-gap, intent-alignment) : high 0, medium 4, low 5, false 3, maybe-false 1.

| # | Constat | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 1 | Égalité de date départagée par le nom en texte : « 9-… » passe devant « 10-… » (blind, edge) | low | patch | Réel ; dates NTFS fines, égalité rare. Départager par le numéro d'étape en tête du nom, puis le nom. |
| 2 | Au-delà du budget, l'ordre des couches change (utilisateur d'abord) (blind) | low | patch | Réel ; l'entraînement mélange l'ordre, donc sans effet mesuré, mais filtrer sur place est plus simple et ne déplace rien. |
| 3 | Le message de `PromptTooLong` accuse seulement l'inventaire, alors que `complete`, le long prompt ou une longue demande le rendent aussi (blind, edge, intent) | medium | patch | Réel : `AiError::PromptTooLong` est partagé (`ai/src/lib.rs:576`). Message qui nomme l'inventaire comme cause habituelle sans exclure la demande. |
| 4 | Aucun garde sans modèle sur le budget : changer `N_CTX`, `MAX_PLAN_TOKENS`, `MAX_ECHO_CHARS`, `MAX_REPORTED` ou les consignes ne casse aucun test lancé d'office (blind, verification-gap) | medium | patch | Réel. Test sans modèle qui fige ces constantes et la longueur de `SHORT_SYSTEM` mesurées avec le budget, avec renvoi au test ignoré. |
| 5 | Les résultats d'un rejeu, écrits sous `<projet>.cassini/replay/`, ne sont pas reconnus (edge) | medium | patch | Réel : `core/src/journal/mod.rs:564`, `read_dir` non récursif. Lister aussi `replay/`. |
| 6 | Le test de `serve.rs` n'épingle pas le nombre de résultats gardés (`kept > 0 && kept < 10`) (blind) | low | patch | Réel. Épingler le nombre exact. |
| 7 | La doc dit qu'un plan qui cite une couche retirée est refusé ; aucun contrôle d'appartenance à l'inventaire n'existe (verification-gap, edge) | medium | patch | Réel : `verify` ne compare pas les `layer_id` à l'inventaire. Corriger la phrase : la couche existe toujours dans QGIS, le plan s'exécute. |
| 8 | Ligne vide dans le commentaire de module, base du chemin `../donnees-test-cassini` ambiguë (blind) | low | patch | Cosmétique, correction directe. |
| 9 | Les couches retirées ne sont signalées ni au modèle ni à l'utilisateur (blind) | low | defer | Réel, demande une surface nouvelle (panneau). Hors intention. |
| 10 | La date d'un fichier de résultat peut changer quand QGIS l'ouvre (WAL de GeoPackage) (edge) | maybe-false | defer | Medium s'il est vrai. À trancher : comparer la date d'un `.gpkg` avant et après ouverture et fermeture dans QGIS. Le dossier réel montre des numéros d'étape réécrits après une remise à zéro du journal (18 à 21 plus récents que 66 à 70) : la date reste meilleure que le numéro. |
| 11 | `unloaded_files` peut grandir sans borne (blind, edge) | false | rejet | `agent_inventory` le rend toujours vide (`core/src/agent_inventory.rs`, « l'extension ne parcourt pas le disque »). |
| 12 | Une couche de l'utilisateur au nom de fichier identique à un résultat serait prise pour un résultat (blind, edge) | low | rejet | Exige un fichier nommé `<n>-<commande>.gpkg` hors du dossier de travail ; correction plus lourde (chemins complets). |
| 13 | Sans session, rien n'est retiré (edge) | false | rejet | Comportement voulu, ligne « Pas de dossier de travail » de la matrice. |
| 14 | Pas de borne sur la longueur de la demande, marge mesurée sur un seul projet (blind) | low | rejet | Préexistant ; l'erreur typée reste le filet ; seuil de 0,41 jeton par caractère consigné dans la doc. |
| 15 | L'issue réelle (session QGIS enchaînée) n'est couverte par aucun test automatique (intent) | low | rejet | Voulu : recette manuelle par sonde jetable, rejouée par Claude puis par le propriétaire (Verification du plan). |

## Design Notes

Un budget en caractères plutôt qu'en jetons : le noyau ne dépend pas de l'IA (CLAUDE.md §9), et la mesure du rapport jetons/caractères fixe une marge sûre. Le contrôle exact de `ai/` reste le dernier filet. Le budget vise le prompt court du modèle livré ; le long prompt garde son erreur typée.

## Verification

**Commands:**
- `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check` -- expected: verts.
- `cargo deny check` (PowerShell) -- expected: vert.
- tests Python de l'extension (WSL) -- expected: verts.

**Manual checks (if no CLI):**
- Recette enchaînée de dix phrases dans QGIS par Claude, puis recette du propriétaire.
