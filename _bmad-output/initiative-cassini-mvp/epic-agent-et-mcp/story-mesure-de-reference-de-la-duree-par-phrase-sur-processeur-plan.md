---
title: 'Mesure de référence de la durée par phrase sur processeur'
type: 'chore'
ticket: '11'
created: '2026-10-17'
status: 'built'
baseline_revision: 'd9de48e9b307049ef0980caea903c355ccddf224'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** L'exigence du propriétaire (une longue phrase résolue en moins de 30 s sur un poste sans carte graphique) n'a aucun point de départ chiffré pour le binaire réel : la seule mesure CPU (8.6) passait par `llama-completion.exe`, pas par le noyau, et ne séparait pas les phases telles que le noyau les enchaîne (modèle rechargé à chaque appel, redemande comprise).

**Approach:** Une génération mesurée dans `cassini-ai` (même chemin que `generate`, nombre de fils réglable, durées de chargement, de lecture du prompt et d'écriture rendues avec les nombres de jetons), et un essai réel ignoré par défaut qui passe les 25 phrases de l'épreuve par `compose_plan` et imprime un tableau par phrase. Mesure lancée à 8 fils puis à 4 fils sur 4 cœurs physiques, binaire sans CUDA ; chiffres consignés dans `docs/` ; relevé A à E du scénario Villeneuve-le-Roi noté dans ce plan, avec un inventaire tenu hors dépôt.

## Boundaries & Constraints

**Always:** Binaire compilé sans `--features cuda`, en `--release`. `generate` garde exactement son comportement actuel (fils par défaut de llama.cpp, rendu inchangé) : la génération mesurée est le même code, `generate` l'appelle et jette les durées. Durées en temps réel (`Instant`), par appel au modèle, additionnées par phrase avec le nombre d'appels (1, ou 2 quand il y a une redemande). Aucune assertion sur une durée : c'est une mesure de référence ; l'essai n'échoue que si une phrase ne rend aucune ligne. L'inventaire Villeneuve-le-Roi et tout fichier tiré de `../donnees-test-cassini/` restent hors dépôt ; seuls le texte des phrases A à E et les chiffres sont versionnés. Aucune nouvelle dépendance, hors ligne. Décisions du propriétaire (2026-10-17) : l'inventaire Villeneuve-le-Roi est construit par Claude avec un script jetable hors dépôt (couches du `.qgz`, champs des GeoPackage par `sqlite3`, format de l'inventaire de l'épreuve, champs compris) dans `../donnees-test-cassini/inventaire-villeneuve.json` ; Claude lance lui-même la mesure sur ce poste (le 5800X), par PowerShell.

**Never:** Garder le modèle chargé ou mettre le prompt en cache (ticket 5.12). Changer le nombre de fils utilisé par le produit (constat à reporter au 5.12). Toucher le prompt, la grammaire, `judge` ou `compose_plan`. Toucher `tests/epreuve-modele-local/` (gelé). Mesurer la justesse : le score reste porté par `plan_reel` sur GPU.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Sans modèle | `CASSINI_TEST_MODEL` absent | essai sauté, message sur `stderr` | — |
| Épreuve | modèle, `CASSINI_TEST_THREADS=8` | 25 lignes : id, appels, chargement, lecture du prompt (jetons), écriture (jetons), total, issue | — |
| Redemande | 2 appels au modèle | les trois durées additionnées, `appels = 2` | — |
| Scénario | `CASSINI_TEST_INVENTORY=<json>` | phrases A, B1, B2, B3, C, D, E sur cet inventaire | fichier illisible → panique avec le chemin |
| Erreur de l'agent | refus, question, plan invalide | ligne écrite quand même, issue notée | — |

</frozen-after-approval>

## Code Map

- `ai/src/lib.rs` -- `generate` (l.209) : charge le modèle, crée le contexte (`LlamaContextParams`, l.223, fils non réglés = `GGML_DEFAULT_N_THREADS` = 4 dans llama.cpp, `llama-context.cpp` l.3766), décode le prompt d'un bloc (l.237), puis boucle d'écriture (l.252). Y découper trois chronos sans changer l'ordre des opérations. `with_n_threads` et `with_n_threads_batch` existent sur `LlamaContextParams` (llama-cpp-2 0.1.159, `context/params/get_set.rs` l.172). `Completion` (l.152) reste tel quel : ses constructeurs dans les tests de `plan.rs` (`done`) ne bougent pas.
- `ai/src/plan.rs` -- `compose_plan` (l.344) prend un `&mut dyn FnMut(&GenerationRequest)` : l'essai y branche son propre générateur, qui appelle la génération mesurée et empile les durées. `model_generator` (l.382) inchangé.
- `ai/tests/plan_reel.rs` -- modèle à reprendre : `include_str!` de `inventory.json` et `sentences.json`, garde `CASSINI_TEST_MODEL`, `#[ignore]`.
- `ai/tests/duree_reel.rs` (nouveau) -- l'essai de mesure.
- `docs/mesure-duree-par-phrase.md` (nouveau) -- commandes de lancement (PowerShell, `CARGO_TARGET_DIR=C:\ct`, affinité), tableaux 8 et 4 fils, lecture des chiffres, constat des 4 fils par défaut.
- Référence : `tests/epreuve-modele-local/resultats-8-6-cpu.json` (8 fils, `llama-completion.exe`) : chargement 1,9 s, prompt 6,0 s pour 4 873 jetons, écriture 18,6 s pour 105 jetons — point de comparaison, à citer, pas à modifier.
- Machine : Ryzen 7 5800X, 8 cœurs et 16 fils, cœurs frères numérotés par paires (0-1, 2-3…) : 4 cœurs physiques = masque d'affinité `0x55`, 8 = `0x5555`.

## Tasks & Acceptance

**Execution:**
- [x] `ai/src/lib.rs` -- `pub struct Timings { load, prompt, prompt_tokens, write, written_tokens }` et `pub fn generate_measured(model, turns, grammar, max_new_tokens, threads: Option<NonZeroU32>) -> Result<(Completion, Timings), AiError>` ; `generate` = `generate_measured(.., None)` sans les durées ; test unitaire : modèle absent → `ModelNotFound` aussi par la voie mesurée -- une seule implémentation, aucun écart de comportement
- [x] `ai/tests/duree_reel.rs` -- essai `#[ignore]` : fils lus dans `CASSINI_TEST_THREADS` (défaut : llama.cpp), phrases de l'épreuve ou, avec `CASSINI_TEST_INVENTORY`, les phrases A à E ; imprime un tableau Markdown et une ligne de moyennes -- l'outil de mesure du ticket
- [x] `docs/mesure-duree-par-phrase.md` -- mode opératoire, tableaux relevés à 8 et 4 fils, lecture -- « consigné dans le dépôt »
- [x] ce plan, `## Implementation Notes` -- relevé A à E -- verify du ticket

**Acceptance Criteria:**
- Given le 5800X, binaire sans CUDA, when l'essai tourne à 8 fils puis à 4 fils sur 4 cœurs physiques, then `docs/mesure-duree-par-phrase.md` porte pour chacun un tableau de 25 lignes avec chargement, lecture du prompt et écriture.
- Given l'inventaire Villeneuve-le-Roi hors dépôt, when l'essai tourne avec `CASSINI_TEST_INVENTORY`, then le relevé des phrases A à E, aux deux réglages, est noté dans ce plan.
- Given la suite sans modèle, when `cargo test --workspace` tourne, then elle passe sans charger de modèle.

## Implementation Notes

Mesure lancée le 2026-10-17 sur le 5800X, binaire `--release` sans CUDA, par PowerShell
(`Start-Process` puis `ProcessorAffinity`). Résultats bruts hors dépôt dans `C:\ct\mesure-11\`.

**Inventaire Villeneuve-le-Roi** : construit par un script jetable hors dépôt (couches du
`.qgz`, champs des GeoPackage lus en lecture seule par `sqlite3`, valeurs listées pour les
champs texte à 8 valeurs distinctes au plus), écrit dans
`../donnees-test-cassini/inventaire-villeneuve.json` (6 couches, dont un MNT). L'invite du scénario compte environ 5 850 jetons, contre
5 130 pour l'inventaire de l'épreuve.

**Relevé A à E, 8 fils (`0x5555`)** — moyennes : chargement 4,0 s, lecture du prompt 101,4 s,
écriture 19,9 s, total 125,3 s.

| id | appels | chargement (s) | lecture du prompt (s) | jetons lus | écriture (s) | jetons écrits | total (s) | issue |
|---|---|---|---|---|---|---|---|---|
| A | 1 | 3,9 | 101,0 | 5873 | 43,1 | 182 | 148,0 | plan, 4 étape(s) |
| B1 | 1 | 4,0 | 100,4 | 5841 | 16,8 | 71 | 121,2 | plan, 1 étape(s) |
| B2 | 1 | 4,1 | 101,7 | 5852 | 28,2 | 120 | 134,0 | plan, 2 étape(s) |
| B3 | 1 | 4,0 | 101,1 | 5841 | 3,9 | 19 | 109,0 | plan, 1 étape(s) |
| C | 1 | 4,0 | 103,0 | 5847 | 29,3 | 124 | 136,2 | plan, 2 étape(s) |
| D | 1 | 4,0 | 101,3 | 5841 | 5,6 | 24 | 110,8 | question |
| E | 1 | 4,0 | 101,5 | 5854 | 12,7 | 53 | 118,2 | question |

**Relevé A à E, 4 fils (`0x55`)** — moyennes : chargement 3,9 s, lecture du prompt 176,0 s,
écriture 19,9 s, total 199,8 s.

| id | appels | chargement (s) | lecture du prompt (s) | jetons lus | écriture (s) | jetons écrits | total (s) | issue |
|---|---|---|---|---|---|---|---|---|
| A | 1 | 4,0 | 177,8 | 5873 | 42,6 | 182 | 224,4 | plan, 4 étape(s) |
| B1 | 1 | 3,9 | 175,7 | 5841 | 16,6 | 71 | 196,2 | plan, 1 étape(s) |
| B2 | 1 | 4,0 | 175,8 | 5852 | 28,1 | 120 | 207,9 | plan, 2 étape(s) |
| B3 | 1 | 3,9 | 175,6 | 5841 | 4,9 | 23 | 184,4 | plan, 1 étape(s) |
| C | 1 | 4,0 | 175,9 | 5847 | 29,1 | 124 | 209,0 | plan, 2 étape(s) |
| D | 1 | 3,9 | 175,4 | 5841 | 5,6 | 24 | 185,0 | question |
| E | 1 | 3,9 | 175,8 | 5854 | 12,3 | 52 | 192,0 | refus |

Lecture : la phrase de démo A demande 148 s à 8 fils et 224 s à 4 fils, dont 43 s
d'écriture pour un plan de 4 étapes ; le découpage B1 + B2 + B3 coûte 364 s à 8 fils, trois
fois la lecture du prompt. D et E rendent une question (E : refus à 4 fils) : le projet n'a
pas de couche d'écoles, et la justesse n'est pas mesurée ici. Aucune redemande.

Écarts au plan : aucun sur le code. Pour les passes à 8 fils, l'affinité a été posée sur
les 8 cœurs physiques (`0x5555`), comme le prévoit la Code Map. Constat : la mesure 8.6
n'était pas purement CPU (llama.cpp compilé CUDA, `ngl 0`, pic VRAM 1,5 Go) ; le document
le dit et ne compare que l'écriture.

Audit de la matrice (après le rapport de l'implémenteur) : les lignes « Redemande », « Erreur
de l'agent » et « Scénario, fichier illisible » n'avaient aucun test qui tourne sans modèle.
`read_inventory` est sorti de l'essai et trois tests sans modèle sont ajoutés dans
`duree_reel.rs` : `a_retry_adds_both_calls_to_the_row`,
`each_outcome_is_named_and_an_agent_error_still_gives_a_line`,
`an_unreadable_inventory_panics_with_its_path`. Ligne « Sans modèle » : `--ignored` sans
`CASSINI_TEST_MODEL` imprime « essai ignore » et passe. Sous Windows : `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings` et `cargo test --workspace` passent (core 178).

## Plan Change Log

## Review Triage Log

Passe 1 (quick). 0 high, 1 medium, 3 low, 1 false.

| # | Verdict | Route | Evidence |
|---|---------|-------|----------|
| 1 | medium | patch | `assert_eq!(rows.len(), …)` ne peut pas échouer : `rows.push` tourne à chaque tour, un modèle qui ne charge pas donne 25 lignes à `appels = 0` et l'essai passe. Assertion `calls >= 1` par ligne ajoutée. |
| 2 | low | patch | Un appel en erreur n'est ni compté ni chronométré (`Timings` rendu sur `Ok` seulement) ; cas réel : `PromptTooLong` à la redemande, avalé par `compose_plan` (`plan.rs` l.364) après le chargement. Aucune redemande dans les mesures ; limite écrite dans l'essai et le document, API inchangée. |
| 3 | low | patch | `start` est pris avant `backend()?.lock()` : « chargement » inclut le démarrage unique du moteur et l'attente du verrou ; la libération du modèle n'est dans aucune colonne. Écart visible au plus sur S01 (4,1 s contre 3,9) ; précisé dans le document. |
| 4 | low | patch | « tourne comme la colonne 4 fils » : la colonne est épinglée sur 4 cœurs physiques (`0x55`), le produit tourne 4 fils sans affinité. Reformulé. Le défaut de 4 fils est vérifié dans `llama-context.cpp` l.3766 (`GGML_DEFAULT_N_THREADS`). |
| 5 | false | rejeté | « 73 min » ne vient pas du tableau mais du temps mur relevé (`epreuve-4.fin.txt` : 01:12:43), arrondi juste. |

## Design Notes

Phrases du scénario (dans l'essai, texte seulement) : A « Trouve les parcelles constructibles du PLU (zones U) à moins de 500 m d'une gare à Villeneuve-le-Roi et sors-moi la carte en PDF. » ; B1 « Garde les zones U du PLU. », B2 « Trouve les parcelles dans ces zones à moins de 500 m d'une gare. », B3 « Sors-moi la carte en PDF. » (chaque demande mesurée à part, sans mémoire entre elles : il n'y en a pas encore) ; C « Trouve les bâtiments à moins de 50 m d'une route. » ; D « Trouve les parcelles près des écoles. » ; E « Trouve les parcelles à 10 minutes à pied de la gare de Villeneuve-le-Roi. »

Brider à 4 fils sur 4 cœurs physiques d'un 5800X ne reproduit ni la mémoire ni la fréquence d'un portable : c'est un substitut, le document le dit.

## Verification

**Commands:**
- `cargo test --workspace`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` -- expected: succès
- `CASSINI_TEST_MODEL=<gguf> CASSINI_TEST_THREADS=8 cargo test -p cassini-ai --release -- --ignored --nocapture duree_reel` (puis 4 fils, affinité `0x55`) -- expected: deux tableaux de 25 lignes
