---
title: 'Phrase qui suit un plan à plusieurs résultats sous 30 s, sur processeur'
type: 'feature'
ticket: '22'
created: '2026-10-29'
status: 'built'
baseline_revision: 'ac0cdef31e97ae18e7a02bff28b07630817dcc56'
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

**Problem:** Dans la recette enchaînée de 5.19 (CPU, 4 fils), la phrase 2 lit une fois les quatre couches produites par le plan de démo (3 432 jetons, 72,5 s) et la phrase 3 lit la sélection des zones U, qui porte tous les champs du PLU et leurs valeurs (~1 100 jetons, 36,6 s) : deux phrases au-dessus de 30 s.

**Approach:** Réduire, dans la vue du modèle seulement, ce qu'une couche de résultat coûte à lire, par les deux leviers : cacher au modèle les couches intermédiaires d'un plan, et retirer les listes de valeurs des champs des couches de résultat. Puis remesurer sur CPU (durée) et GPU (justesse).

**Décision (2026-10-29, propriétaire) :** les deux leviers (C). Estimations sur les couches réelles de la session 5.19 (sonde hors dépôt `C:\Users\kevin\AppData\Local\Temp\sonde-5-22\`) : phrase 2 ≈ 690 jetons nouveaux (≈ 20 s, contre 72,5 s), phrase 3 ≈ 630 (≈ 23 s, contre 36,6 s) ; couche finale seule : phrase 3 inchangée ; sans valeurs seul : phrase 2 ≈ 43 s. Coût accepté : le modèle ne voit plus les intermédiaires d'un plan, ni les valeurs des champs texte des résultats (celles des couches de l'utilisateur restent).

## Boundaries & Constraints

**Always:** seule la vue du modèle change : QGIS, le journal, le plan gardé et l'extension gardent toutes les couches et tous les champs ; la forme d'une couche reste celle de `training/tests/forme_inventaire.rs` (un champ sans `values` est une forme apprise) ; l'ordre stable, les identifiants courts (table faite sur toutes les couches) et le retrait par paquets de 5.19 restent ; à résultats inchangés, la vue est identique d'une phrase à l'autre (la reprise à la dernière frontière doit continuer de fonctionner) ; mesures de durée sur CPU sans CUDA (5800X, `0x55`, `CASSINI_THREADS=4`), justesse sur GPU ; aucune `ai.prompt_too_long`.

**Never:** règle de prompt ou patch par phrase ; changer `SHORT_SYSTEM`, la grammaire, `judge`, `retry_turns` ; toucher aux couches de l'utilisateur ; état sur disque ou nouveau champ du journal ; abaisser le critère de 30 s (un écart restant est consigné tel quel).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Plan à quatre résultats chaînés | démo : sélection, tampon, deux intersections, PDF | seule la dernière intersection vue, sans `values` | — |
| Plan d'une étape | « Garde les zones U » | la sélection reste vue (couche finale), sans `values` | — |
| Plan arrêté en cours | échec à l'étape 3 | une sortie n'est cachée que si une étape exécutée l'a consommée | — |
| Noyau relancé (QGIS rouvert) | résultats d'une session précédente | tous vus une fois (la mémoire des intermédiaires est celle de la session) | — |
| Phrase qui cite un intermédiaire | « Montre le tampon de 500 m » après la démo | issue consignée (nouveau plan, question ou refus) | issue existante |

</frozen-after-approval>

## Code Map

- `core/src/agent_inventory.rs:370` `model_inventory` -- y ajouter, avant le budget, le retrait des couches dont le `layer_id` est dans un ensemble d'intermédiaires passé en paramètre, puis le retrait de `values` des champs des couches de résultat (`result_date` / `results_of` les reconnaissent). Fonctions pures et testées ; `ShortIds::of_inventory` reste faite sur toutes les couches (avant tout retrait).
- `core/src/serve/plans.rs:245` `execute` -- quand `resolve_references` consomme `step:k`, la sortie `outputs[k-1]["layer_id"]` devient intermédiaire ; à garder dans `State` (`core/src/serve.rs:155`, à côté de `pending`), en mémoire seulement. `:165` passer l'ensemble à `model_inventory`. Repérer les `k` consommés depuis les paramètres non résolus (`core/src/plan.rs:128` pour la syntaxe `step:k.champ`).
- `core/src/serve.rs:1995` `the_composer_sees_an_inventory_reduced_to_the_budget` -- attentes du budget à revoir (les résultats sans valeurs sont plus petits) ; ajouter un test bout en bout : plan à résultats chaînés exécuté par le compositeur factice, puis la vue de la proposition suivante.
- `cli/src/main.rs:1234`, tests de `agent_inventory.rs` (`:956`, `:972`) -- appels de `model_inventory` à mettre à jour.
- Mesures : sonde de 5.19 `C:\Users\kevin\AppData\Local\Temp\sonde-5-19\sonde.py` et `run.ps1` (à copier dans `sonde-5-22\`) ; environnement des essais réels comme 5.19 (PowerShell depuis WSL, `CARGO_TARGET_DIR=C:\ct`, modèle livré, script .ps1 avec BOM) ; essais GPU `plan_reel`, `question_reel`, `ecart_reel` (`--features cuda`).

## Tasks & Acceptance

**Execution:**
- [x] `core/src/agent_inventory.rs` -- les deux leviers, fonctions pures, tests de la matrice (vue identique d'une phrase à l'autre, utilisateur intact, forme apprise)
- [x] `core/src/serve.rs`, `core/src/serve/plans.rs` -- ensemble des intermédiaires en mémoire, alimenté par `execute`, passé à `model_inventory` ; test bout en bout par le compositeur factice
- [x] `cli/src/main.rs` -- appel mis à jour
- [x] sonde jetable `sonde-5-22` -- recette enchaînée de dix phrases sur CPU, plus une onzième qui cite un résultat caché ; installer le noyau dans `%LOCALAPPDATA%\Cassini\bin` avant
- [x] `docs/mesure-duree-par-phrase.md` -- durées après 5.22, issue de la phrase qui cite un résultat caché ; GPU : `plan_reel`, `question_reel`, `ecart_reel`

**Acceptance Criteria:**
- Given le 5800X bridé à 4 cœurs, sans CUDA, when la recette enchaînée de dix phrases de 5.19 tourne, then les phrases 2 et 3 sont composées en moins de 30 s, durées consignées dans `docs/mesure-duree-par-phrase.md`, aucune `ai.prompt_too_long`.
- Given le GPU, when `plan_reel`, `question_reel`, `ecart_reel` tournent, then aucun ne baisse par rapport à 5.19 (25/25, 12/12, 32/40).
- Given une phrase qui cite une couche cachée ou une valeur retirée, when elle est composée, then son issue est consignée dans la doc.

## Implementation Notes

- `agent_inventory.rs` : `without_intermediates` (ne retire qu'une couche de résultat, jamais une couche de l'utilisateur, même citée par l'ensemble — cas `layer.add` puis `step:1`) et `results_without_values` ; `model_inventory` prend `&HashSet<String>` d'identifiants complets, retire après `ShortIds::of_inventory` et avant `shorten` et le budget.
- `serve/plans.rs` : `consumed_layers` lit les références de premier niveau des paramètres non résolus ; l'ensemble n'est alimenté qu'après le succès de l'étape consommatrice (plan arrêté : la sortie non consommée reste vue). `State.intermediates` en mémoire, jamais vidé (identifiants QGIS uniques).
- Le test du budget (`the_composer_sees_an_inventory_reduced_to_the_budget`) n'a pas eu à changer : ses résultats n'ont pas de valeurs.
- Recette CPU (sonde `sonde-5-22`, 11 phrases) : phrase 2 15,1 s, phrase 3 21,8 s ; seule la phrase 9 (écriture du plan de démo, 30 s) reste au-dessus (38,1 s). Phrase 11 : question hors de propos. Phrase 3 relie « ces zones » au PLU entier. Valeur retirée (`formdomi` 0300 de la sélection) : question, 10,8 s. GPU : 25/25, 12/12, 32/40. Relecture : seule une étape qui produit une couche consomme, et seulement par `step:k` ou `step:k.layer_id`.

## Plan Change Log

## Review Triage Log

Passe 1 (2026-10-29, lentilles blind-hunter, edge-case-hunter, verification-gap, intent-alignment) : high 0, medium 4, low 9, false 2, maybe-false 1.

| # | Constat | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 1 | Une étape qui ne produit pas de couche (`layer.describe`, `layout.export_pdf`) et cite `step:k` cache la sortie k, parfois le seul résultat du plan (blind, edge) | medium | patch | Réel : `layer.describe` est une commande du registre (`core/src/commands/layer_describe.rs`) ; `consumed_layers` marque sans regarder la sortie de l'étape. Ne marquer que si la sortie de l'étape qui consomme porte elle-même un `layer_id`. Test. |
| 2 | La couche finale de la démo n'est épargnée que parce que `"layers": ["step:4"]` du PDF est un tableau, non lu (intent a) | medium | patch | Même effet que #1 par une autre cause (forme de paramètre) ; le correctif #1 l'épargne quelle que soit la forme. Test d'un plan qui finit par un PDF citant la dernière couche en scalaire. |
| 3 | `step:k.<champ>` autre que `layer_id` (ex. `feature_count`) marque la couche k (verification-gap autres) | low | patch | Réel ; correction directe : ne compter que `step:k` nu ou `step:k.layer_id`. |
| 4 | Aucun test ne prouve que l'allègement passe avant le budget (verification-gap) | medium | patch | Pré-vérifié. Test : vue brute au-delà de `INVENTORY_BUDGET_CHARS`, vue allégée en deçà, aucun résultat non caché retiré. |
| 5 | Critère 3 (« valeur retirée ») coché sans phrase qui en cite une (blind, edge) | medium | patch | Réel (`docs/mesure-duree-par-phrase.md`) : courte recette CPU (démo, « Garde les zones U du PLU. », puis une phrase qui cite une valeur de la sélection, ex. un libellé), issue consignée. |
| 6 | La doc attribue l'erreur de liaison de la phrase 3 à un défaut ancien, sans séparer les leviers (blind) | maybe-false | patch | Possible que le retrait des valeurs y contribue ; la doc ne doit pas trancher la cause : reformuler (cause non séparée). |
| 7 | « 9 sur 10 » contre « 5 sur 9 » : bases différentes (blind) | low | patch | Réel ; sur les phrases 2 à 10 : 8 sur 9. Corriger la doc. |
| 8 | Tableau 5.22 sans les durées de 5.19 en regard (blind) | low | patch | Correction directe : colonne 5.19. |
| 9 | Test bout en bout trie les identifiants : l'ordre stable n'est pas vérifié par `propose` (blind) | low | patch | Correction directe : comparer sans trier. |
| 10 | Justesse GPU mesurée sur des inventaires sans résultat : la vue allégée n'est pas jugée (blind, intent b) | medium | defer | Réel, mais l'essai de justesse sur session enchaînée est déjà reporté depuis 5.18 ; la doc le dit. Entrée dans `deferred-work.md`. |
| 11 | Relance du noyau ou `journal.replay` : les intermédiaires restent vus toute la session et peuvent ramener le retrait par paquets (blind, edge, intent d) | low | defer | Accepté par la matrice figée (mémoire du processus) ; le rejeu n'est pas cité. Reconstruction depuis le journal à décider plus tard. Entrée dans `deferred-work.md`. |
| 12 | Phrase qui cite une couche cachée : question hors de propos (blind) | low | defer | Coût accepté par le propriétaire ; piste d'entraînement (5.10 : couche introuvable → question sur la couche). Entrée dans `deferred-work.md`. |
| 13 | `a_one_step_plan_…` n'est pas un plan d'une étape au niveau du service (blind) | low | rejet | Le test bout en bout couvre une sortie non consommée restée vue (r3, r4) ; un ensemble vide est exactement l'état d'un plan d'une étape. |
| 14 | Source d'un intermédiaire supprimée : la couche redevient « utilisateur », valeurs comprises (edge) | low | rejet | Il faut effacer à la main un fichier du dossier de travail pendant la session ; le prix est une relecture, pas une erreur. |
| 15 | Résultat dont la source n'est pas un fichier de travail (sortie mémoire) (edge) | false | rejet | Toutes les commandes à sortie écrivent un `.gpkg` dans le dossier de travail (`output_ext`), reconnu par `result_date` depuis 5.18. |
| 16 | Tests à `source` égal au nom du fichier : reconnaissance réelle non testée (intent c) | low | rejet | Mécanisme de 5.18, inchangé ; la recette CPU le montre (jetons en baisse). |
| 17 | Journal de tri vide alors que `lenses_ran` est rempli (blind) | false | rejet | La relecture était en cours ; ce tableau la consigne. |

Patches 1 à 9 appliqués (2026-10-29) ; #10 à #12 consignés dans `deferred-work.md`. Vérification relancée après les patches : `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check` verts (PowerShell). Noyau reconstruit avec les patches et installé dans `%LOCALAPPDATA%\Cassini\bin` (ancien : `cassini.exe.avant-5-22`).

## Design Notes

Couche intermédiaire = sortie d'une étape consommée par une étape **exécutée** du même plan (`step:k`). Une sortie reprise par une phrase suivante (« ces zones ») reste vue : seul le plan qui l'a produite et consommée la cache. La mémoire est celle du processus du noyau, comme les instantanés : à la réouverture, tout est relu une fois, coût de la première phrase (hors critère).

## Verification

**Commands:**
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `cargo deny check` (PowerShell) -- expected: verts.
- tests Python de l'extension (WSL) -- expected: verts (aucun changement attendu).

**Manual checks (if no CLI):**
- Recette enchaînée par Claude (sonde jetable), puis par le propriétaire dans QGIS : dix phrases, dont une qui reprend un résultat intermédiaire ; durée et issue confirmées.
