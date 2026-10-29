---
title: 'Durée par phrase sous 30 s dans une session QGIS, sur processeur'
type: 'feature'
ticket: '19'
created: '2026-10-24'
status: 'built'
baseline_revision: '4558c3f09f824af618d976f9539fa6cb70885be4'
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

**Problem:** Dans une session QGIS enchaînée, chaque phrase prend 75 à 120 s sur processeur (4 fils) : l'inventaire change à chaque plan exécuté (les nouveaux résultats arrivent en tête, ordre de QGIS), donc il est relu en entier après l'instantané des consignes (~61 jetons/s), puis le plan est écrit à ~6 jetons/s ; la phrase de démo écrit 248 jetons (≈ 40 s) dont la majeure partie recopie des identifiants de couche de QGIS (`parcelles_e9ba733c_8a43_4717_ac73_04344c1b6bb5`).

**Approach:** Mesurer d'abord dans une session enchaînée, puis, du moins coûteux au plus coûteux : (1) ordre stable de l'inventaire (couches de l'utilisateur, puis résultats du plus ancien au plus récent) et instantanés aux frontières de couches, pour ne relire que les couches nouvelles ; (2) identifiants de couche courts vus par le modèle ; (3) budget plus serré si la justesse le permet.

**Décisions (2026-10-24, propriétaire) :**
- Identifiants courts par **traduction dans le noyau**, sans ré-entraînement : le modèle voit `nom_<4 premiers hex>` (forme déjà apprise pour un tiers des couches d'entraînement), le noyau retraduit le plan. Ré-entraînement seulement si la justesse baisse.
- Budget dépassé : **mesurer d'abord** ; après la remesure, Claude propose au propriétaire un réglage chiffré (retrait par paquets ou budget plus serré) avant d'y toucher.
- La **première phrase d'une session** (chargement et première lecture) est **hors critère**, comme en 5.12 ; pas de préchargement.

**Décisions après la remesure (2026-10-29, propriétaire) :**
- Budget dépassé : **retrait par paquets** — les résultats les plus anciens sortent jusqu'à descendre sous 7 000 caractères (moitié du budget de 14 000) ; la phrase du retrait peut dépasser 30 s, les suivantes reprennent à la dernière frontière.
- Identifiants courts **gardés** malgré H11 (32/40 au lieu de 33/40) ; le ré-entraînement de la famille isochrone depuis un point part dans un ticket suivant, hors de 5.19.
- La phrase qui suit un plan à plusieurs résultats (~62 s, lecture unique de ses couches) part dans un **ticket suivant** (couche finale seule vue par le modèle, ou champs allégés) ; l'écart est consigné ici.

## Boundaries & Constraints

**Always:** le noyau ne dépend pas de l'IA (budget en caractères, pas en jetons) ; à invite égale, sortie gloutonne identique avec ou sans instantané ; découpe aux frontières de couches sans changer un jeton par rapport à l'invite lue d'un tenant (prouvé par test au vrai tokeniseur) ; la forme d'une couche reste celle de `training/tests/forme_inventaire.rs` ; le journal, le plan gardé et l'extension ne voient que les vrais identifiants de QGIS ; mesures de durée sur CPU sans CUDA (5800X, `0x55`, 4 fils), justesse sur GPU ; aucune `ai.prompt_too_long`.

**Never:** règle de prompt ou patch par phrase ; changer `SHORT_SYSTEM`, la grammaire, `judge`, `retry_turns` ; instantané sur disque ou mémoire de conversation ; abaisser le critère de 30 s (un écart restant est consigné tel quel).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Phrase suivante, un résultat de plus | inventaire = précédent + 1 couche à la fin | reprise à la dernière frontière commune ; seules la nouvelle couche et la phrase sont lues | — |
| Couche de l'utilisateur ajoutée ou retirée | préfixe changé au milieu | reprise au plus profond instantané encore préfixe (au pire, les consignes) | — |
| Identifiant court | couche QGIS `nom_<uuid>` | le modèle voit `nom_<4 hex>` ; le plan rendu est retraduit vers l'identifiant complet avant vérification | — |
| Collision d'identifiants courts | deux couches donnent le même court | allongé de façon déterministe jusqu'à être unique | — |
| Identifiant inconnu dans le plan | court absent de la table | laissé tel quel, refusé comme aujourd'hui | erreur existante |
| Budget dépassé | résultats au-delà du budget | les plus anciens sortent de la vue du modèle (5.18), sans `prompt_too_long` | — |

</frozen-after-approval>

## Code Map

- `ai/src/lib.rs:345` `Turn` (`cached_head`, `text`) et `:410` `chat_tokens_with` -- points d'instantané = fin de tête, fin du tour système ; à généraliser en parties mises en cache (frontières de couches). `:176` `Checkpoints` (un instantané par `slot`, `best` = plus profond préfixe strict) : garder au plus tête, fin des couches de l'utilisateur, dernière frontière, fin du tour système (taille d'un instantané non négligeable). `:560-620` boucle de lecture par segments, à réutiliser telle quelle.
- `ai/src/plan.rs:311` `short_inventory_text`, `:317` `first_turns` -- découper l'inventaire aux frontières de couches. Frontière sûre pour le découpeur de Qwen (BPE, pré-découpe par classes) : entre la ponctuation `},{"` et les lettres `layer_id` qui suivent, à prouver par le test au vrai tokeniseur.
- `core/src/agent_inventory.rs:70` `within_budget` (garde l'ordre d'entrée) et `:130` `agent_inventory` -- y ajouter l'ordre stable (utilisateur, puis résultats du plus ancien au plus récent, mêmes clés de tri que 5.18) et la table d'identifiants courts (fonctions pures, testées). L'entraînement mélange l'ordre des couches (`training/src/inventories.rs:1405` `finish`) et produit déjà des identifiants `slug_<4 hex>` pour un tiers des couches (`:1239` `layer_id`).
- `core/src/serve/plans.rs:119` `propose` -- appliquer la traduction à l'aller et retraduire les identifiants du plan rendu par le compositeur, avant `verify` et avant de garder le plan.
- `ai/tests/duree_reel.rs`, `qgis-plugin/tests/probe_demo.py` (modèle de la sonde jetable hors dépôt) -- mesures.
- Environnement des essais réels : comme 5.18 (PowerShell depuis WSL, `CARGO_TARGET_DIR=C:\ct`, `LIBCLANG_PATH`, modèle livré `C:\Users\kevin\AppData\Local\Cassini\bin\models\cassini-composition-4b-Q4_K_M.gguf`, script .ps1 avec BOM UTF-8) ; installer le noyau construit dans `%LOCALAPPDATA%\Cassini\bin` avant la recette.

## Tasks & Acceptance

**Execution:**
- [x] sonde jetable (scratchpad) -- recette enchaînée de dix phrases sur CPU, durée par phrase découpée (chargement, lecture, jetons repris, écriture, jetons écrits) -- mesure de départ
- [x] `core/src/agent_inventory.rs` -- ordre stable et identifiants courts, fonctions pures avec tests de la matrice
- [x] `core/src/serve/plans.rs` -- traduction aller et retour ; test par le compositeur factice
- [x] `ai/src/lib.rs`, `ai/src/plan.rs` -- parties mises en cache aux frontières de couches ; tests sans modèle des points, test ignoré au vrai tokeniseur (jetons identiques à la lecture d'un tenant) et reprise effective
- [x] remesure CPU de la recette enchaînée ; si un dépassement du budget casse la reprise au-delà de 30 s, s'arrêter et proposer au propriétaire un réglage chiffré (HITL), puis l'appliquer dans `core/src/agent_inventory.rs` -- HITL fait le 2026-10-29, retrait par paquets appliqué et remesuré
- [x] `docs/mesure-duree-par-phrase.md` -- durées par phrase avant et après ; GPU : `plan_reel`, `question_reel`, `ecart_reel`

**Acceptance Criteria:**
- Given le 5800X bridé à 4 cœurs, sans CUDA, when la recette enchaînée de dix phrases tourne (vrai noyau, vrai modèle, Villeneuve-le-Roi), then chaque plan de la phrase de démo et des phrases courtes est composé en moins de 30 s, durées consignées dans `docs/`, aucune `ai.prompt_too_long`.
- Given le GPU, when `plan_reel`, `question_reel`, `ecart_reel` tournent, then aucun ne baisse par rapport à 5.18 (25/25, 12/12, 33/40).
- Given la recette complète, when elle tourne, then elle est verte.

## Implementation Notes

**Fait (2026-10-29).**

- `core/src/agent_inventory.rs` : `stable_order` (utilisateur dans l'ordre de QGIS, puis résultats
  du plus ancien au plus récent, comparateur exactement inverse de celui de `within_budget`, extrait
  en `newest_first`) ; `ShortIds` (court `nom_<4 hex>` pour un identifiant QGIS `nom_<uuid>`,
  allongé ensemble jusqu'à l'unicité, au-delà de 32 chiffres l'identifiant complet ; `restore` à
  toute profondeur, un texte inconnu reste tel quel) ; `model_inventory` = forme apprise → ordre
  stable → courts (table faite sur toutes les couches, avant le budget) → budget ; `ModelInventory
  {inventory, user_layers}`.
- `core/src/serve/plans.rs` : la fermeture d'inventaire du trait `Composer` rend un `ModelInventory`
  (changement de signature, répercuté dans `cli` et le compositeur factice) ; le plan rendu est
  retraduit avant `verify`, donc avant d'être gardé, montré, exécuté et journalisé.
- `ai/src/lib.rs` : `Turn` gagne `cached_parts` et `anchor` ; `chat_tokens_with` rend des `Cut
  {at, slot}` : une coupe après chaque partie, un instantané aux seules places tête, ancrage,
  dernière partie, tour système (au plus quatre). La découpe est la même avec ou sans reprise.
- `ai/src/plan.rs` : `inventory_parts` coupe l'inventaire une fois par couche, **juste avant sa
  dernière clé** (après le `"` de `,"` ou `{"`), et non entre `},{"` et `layer_id` comme le supposait
  la carte du code : les clés sont triées (`layer_id` n'est pas la première), et couper juste avant
  la fin d'une couche évite de relire la dernière couche entière à la phrase suivante (il ne reste
  que `source":"…"}`). Toutes les couches ou aucune : une forme inattendue se lit d'un tenant.
  `compose_plan_anchored` (le nombre de couches de l'utilisateur) ; `compose_plan_with` inchangé
  (ancrage absent).
- `cli/src/main.rs` : deux variables de **mesure** non prévues au plan, sans effet sans elles :
  `CASSINI_TIMINGS` (une ligne `cassini-timings` par appel sur la sortie d'erreur, vidée et ignorée
  par l'extension) et `CASSINI_THREADS` (sous Windows, `available_parallelism` ignore le masque
  d'affinité : vérifié, 16 sous `0x55`, donc un noyau bridé à 4 cœurs prenait 8 fils). Le défaut du
  produit ne change pas ; corriger `default_threads` sous Windows demanderait une dépendance
  (`windows-sys`), non demandée.
- Essais réels : `plan_reel`, `question_reel`, `ecart_reel` passent par `ShortIds` comme le produit
  (`CASSINI_TEST_FULL_IDS=1` pour comparer) ; nouveau test ignoré `reprise_reel_…` (`ai/src/plan.rs`).

**Vérifié.** `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --check`, `cargo deny check` (PowerShell) verts ; tests Python de l'extension (299) et de
l'entraînement (28) verts, garde de l'extension propre. Au vrai tokeniseur (modèle livré) : 300
prompts engendrés identiques lus par parties (1 357 coupes de couches) et 339 redemandes. Reprise
effective sur GPU : une couche de plus reprend 1 024 jetons (la dernière frontière), sortie identique
sans instantané, un résultat retiré en tête reprend l'ancrage (659 jetons). Mesures CPU et GPU :
`docs/mesure-duree-par-phrase.md`, « Session enchaînée dans QGIS ». Sondes et sorties hors dépôt :
`C:\Users\kevin\AppData\Local\Temp\sonde-5-19\` (`avant.txt`, `apres.txt`), `C:\ct\mesures-5-19\`.

**Résultat de la remesure (CPU, 4 fils, `0x55`).** Phrases 2 à 10 : 779 s → 499 s ; 3 sur 9 sous
30 s (avant 2) ; démo 248 → 165 jetons écrits (43 → 26 s). Reprise à la dernière frontière quand
on ne fait qu'ajouter (phrases 3, 6, 7 : 41,0 / 15,9 / 17,6 s). Restent au-dessus : chaque phrase
où le budget retire des résultats (4, 5, 9, 10 : reprise à la fin de l'utilisateur, 3 500 à 4 600
jetons relus, 71 à 99 s) et la phrase qui suit un plan à quatre résultats (2 : 3 436 jetons
nouveaux, 69,6 s, sans retrait). Aucune `ai.prompt_too_long`.

**Justesse GPU.** `plan_reel` 25/25, `question_reel` 12/12, `ecart_reel` **32/40** avec les
identifiants courts (33/40 avec les complets, comme en 5.18 : les coupes sont neutres). Les courts
coûtent H11 (isochrone depuis un point par la voirie : refus au lieu du plan). La décision du
2026-10-24 prévoit un ré-entraînement « seulement si la justesse baisse » : elle baisse d'une phrase.

**Proposition au propriétaire (HITL, rien d'appliqué).** Chiffres : lecture ≈ 55 jetons/s,
écriture ≈ 6,2 jetons/s ; consignes + couches de l'utilisateur = 1 510 jetons (≈ 3 450 caractères
d'inventaire) ; budget 14 000 caractères ≈ 5 100 jetons, soit ≈ 3 800 jetons de résultats. Pour
tenir 30 s avec un plan court (≈ 40 jetons écrits, 6,5 s), il ne faut pas relire plus d'≈ 1 300
jetons après l'instantané.

1. *Retrait par paquets* (recommandé) : au dépassement, retirer les plus anciens résultats jusqu'à
   descendre sous un seuil bas de **7 000 caractères** (moitié du budget). La phrase de retrait relit
   ≈ (7 000 − 3 450) × 0,364 ≈ 1 300 jetons, ≈ 24 s + écriture (≈ 30 à 50 s selon le plan) ; les
   phrases suivantes reprennent à la dernière frontière jusqu'au prochain dépassement (≈ 2 500
   jetons de marge, deux à quatre couches). Une phrase de retrait sur trois ou quatre environ.
2. *Budget plus serré seul* (9 000 caractères) : chaque phrase au-delà relit encore ≈ 2 000 jetons
   (≈ 36 s + écriture) : insuffisant seul.
3. Aucun des deux ne règle la phrase qui suit un plan à plusieurs résultats (3 400 jetons à lire une
   fois, ≈ 62 s) sans cacher au modèle les résultats les plus récents. Levier hors de ce ticket, à
   décider : ne montrer au modèle que la couche finale de chaque plan (les étapes intermédiaires
   restent dans QGIS), ou alléger les champs des couches d'intersection.

À trancher aussi : garder les identifiants courts (− H11) avec ou sans ré-entraînement, ou y
renoncer (+ 17 s d'écriture sur la démo).

**Après la décision du propriétaire (2026-10-29).** Retrait par paquets appliqué :
`INVENTORY_LOW_CHARS = 7 000` et `within_budget_by_batches` (`core/src/agent_inventory.rs`), appelé
par `model_inventory` à la place de `within_budget` (gardée, avec ses tests). Le noyau ne garde
aucun état d'une phrase à l'autre : la décision est **rejouée** sur les résultats présents, du plus
ancien au plus récent, et vide la vue jusqu'au seuil bas à chaque dépassement ; ajouter des
résultats ne fait que prolonger le rejeu, donc les résultats gardés ne bougent pas jusqu'au
dépassement suivant (tests : sous le budget rien ne sort, dépassement → seuil bas, préfixe gardé
d'étape en étape puis nouveau paquet, ordre d'entrée indifférent, utilisateur jamais retiré).
Le test `the_composer_sees_an_inventory_reduced_to_the_budget` attend désormais les étapes 6 à 10.

Remesure CPU (noyau final) : phrases 2 à 10 en 72,5 / 36,6 / 15,0 / 10,4 / 15,5 / 16,6 / 7,7 /
44,6 / 45,2 s, soit 264 s au total (779 s avant 5.19) et 5 sur 9 sous 30 s ; aucune
`ai.prompt_too_long`. Écarts consignés dans `docs/mesure-duree-par-phrase.md` : phrases 2 et 3
(lecture unique de résultats nouveaux, ticket suivant), phrase 9 (démo en cours de session : 27 s
d'écriture), phrase 10 (phrase du retrait, admise). GPU inchangé : 25/25, 12/12, 32/40 (H11, ticket
de ré-entraînement suivant). Noyau installé dans `%LOCALAPPDATA%\Cassini\bin` (l'ancien gardé en
`cassini.exe.avant-5-19`).

## Plan Change Log

## Review Triage Log

Passe 1 (2026-10-29, lentilles blind-hunter, edge-case-hunter, verification-gap, intent-alignment) : high 0, medium 4, low 12, false 3, maybe-false 0.

| # | Constat | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 1 | Retrait par paquets : si les couches de l'utilisateur dépassent seules 7 000 caractères, la boucle `while size > low` retire tous les résultats, même celui du plan qui vient d'être exécuté (blind, edge) | medium | patch | Réel (`within_budget_by_batches`) : un projet de plus de ~7 couches ne montre plus jamais le dernier résultat ; « ces zones » devient impossible. Ne jamais retirer le plus récent pendant le paquet ; il ne sort que s'il ne tient pas sous le budget. Test. |
| 2 | Identifiants courts visibles par l'utilisateur dans une question ou un refus du modèle (verification-gap autres, edge, blind) | medium | patch | Réel : `restore` ne s'applique qu'à `Plan::Calls` (`serve/plans.rs`). Remplacer aussi les courts dans le texte de la question et du refus, du plus long au plus court. Test. |
| 3 | Aucun test ne vérifie que `ModelComposer` / `ChosenModel::compose` passe l'ancrage et que l'inventaire du noyau est bien coupé (verification-gap, blind : coupe qui s'éteint sans bruit) | medium | patch | Pré-vérifié (verification-gap) ; aucun `preserve_order` dans le dépôt aujourd'hui, mais rien n'en garde. Test dans `cli` : sortie de `model_inventory` passée par `ChosenModel::compose`, `cached_parts.len()` = nombre de couches, `anchor` = `Some(user_layers - 1)`. |
| 4 | `duree_reel` ne passe pas par les identifiants courts, contrairement aux trois autres essais réels (verification-gap) | medium | patch | Pré-vérifié. Passer par `common::shown_inventory` et `common::restored`. |
| 5 | `within_budget` n'est plus appelé en production ; doc du module encore écrite pour lui (verification-gap, edge, blind) | low | patch | Réel. Le retirer avec ses tests (couverts par ceux du retrait par paquets) et mettre à jour la doc du module. |
| 6 | Les commentaires disent « instantané à la fin des couches de l'utilisateur » alors que la coupe est juste avant la dernière clé de la dernière couche (blind) | low | patch | Réel, correction de commentaire. |
| 7 | Phrase 9 (démo en cours de session, 44,6 s, écriture 27 s) et phrase 3 (36,6 s, sélection portant tous les champs du PLU) au-dessus de 30 s sans décision ni entrée reportée (intent, blind) | medium | defer | Réel, critère d'acceptation 1 non tenu pour elles ; deux entrées ajoutées à `deferred-work.md`. Présenté au propriétaire. |
| 8 | Changement d'issue de la phrase 9 (question en 5.18, plan en 5.19) non commenté dans la doc (blind) | low | patch | Ajout d'une phrase dans `docs/mesure-duree-par-phrase.md`. |
| 9 | Ligne de commentaire trop longue dans `propose` (blind) | low | patch | Correction directe. |
| 10 | `cassini plan` (CLI) ne passe ni par l'ordre stable ni par les courts (blind) | low | rejet | Préexistant : avant 5.19 il passait déjà l'inventaire du fichier tel quel, sans `agent_inventory` ni budget (`4558c3f:cli/src/main.rs:550`). |
| 11 | Un paramètre texte égal à un court est réécrit ; un court dans un titre libre reste court (blind, edge) | low | rejet | Coïncidence improbable ; avant 5.19 le titre aurait porté l'identifiant complet. |
| 12 | Une collision de courts à l'arrivée d'une couche change le court d'une couche existante et casse le préfixe (edge, blind) | low | rejet | Il faut deux couches au même nom et aux 4 premiers chiffres égaux (1 sur 65 536) ; le prix est une relecture, pas une erreur. |
| 13 | `low >= budget` non vérifié (edge) | false | rejet | Seul appelant : les constantes 14 000 / 7 000. |
| 14 | `CASSINI_THREADS` invalide ignoré sans avertissement (edge, blind) | low | rejet | Variable de mesure seulement, sans effet en usage normal. |
| 15 | Instantané `SLOT_LAST` périmé gardé quand l'ancrage est la dernière partie (edge) | low | rejet | `best` ne reprend qu'un préfixe exact : aucune sortie fausse, seulement de la mémoire. |
| 16 | Coût mémoire des quatre instantanés non mesuré (blind) | low | rejet | Même constat rejeté en 5.12 (#8) ; estimation déjà écrite là-bas. |
| 17 | Coupes sans instantané : raison et coût en lots non écrits (blind) | low | rejet | La découpe identique avec ou sans reprise est déjà la règle de 5.12, écrite près de `segment_ends`. |
| 18 | Pas de test `serve` qu'un court inconnu reste et que le plan est refusé ; ancre hors bornes non vérifiée (blind) | low | rejet | `restore` laissant un inconnu est testé ; le refus est le contrôle existant. Ancre toujours calculée bornée (`inventory_parts`). |
| 19 | Justesse non mesurée sur un inventaire de session ordonné et réduit (intent) | low | rejet | Déjà reporté par 5.18 (deferred-work, justesse sur long inventaire) ; la recette QGIS montre la démo juste en 9ᵉ position. |
| 20 | `ecart_reel` 32/40 : critère 2 non tenu (edge) | false | rejet | Décision du propriétaire du 2026-10-29 (bloc figé) : identifiants courts gardés, H11 reportée. |
| 21 | Tâches cochées alors que le critère 1 échoue (edge) | false | rejet | La doc et le rapport disent l'écart ; le statut sera présenté au propriétaire, pas déclaré tenu. |

Patches 1 à 6, 8 et 9 appliqués (2026-10-29) ; #7 consigné dans `deferred-work.md`. Vérification
relancée après les patches : `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
warnings`, `cargo test --workspace`, `cargo deny check` verts (PowerShell) ; extension 299 tests,
entraînement 28 tests, garde de l'extension propre.

## Design Notes

Ordre de grandeur (5.10 t. 4b, CPU 4 fils) : lecture ~61 jetons/s, écriture ~6 jetons/s. Une couche d'intersection à 34 champs pèse ~724 jetons, soit ~12 s de lecture ; un identifiant complet ~25 jetons recopiés, ~4 s d'écriture chacun. La phrase de démo en recopie six.

## Verification

**Commands:**
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `cargo deny check` (PowerShell) -- expected: verts.
- tests Python de l'extension (WSL) -- expected: verts.

**Manual checks (if no CLI):**
- Recette enchaînée de dix phrases par Claude (sonde jetable), puis par le propriétaire dans QGIS : aucune phrase au-delà de 30 s.
