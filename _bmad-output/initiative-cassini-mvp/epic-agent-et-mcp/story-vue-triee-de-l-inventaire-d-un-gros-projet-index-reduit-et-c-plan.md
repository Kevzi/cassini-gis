---
title: "Vue triée de l'inventaire d'un gros projet : index réduit et couches détaillées par la demande"
type: 'feature'
ticket: '46'
created: '2026-11-10'
status: 'built'
baseline_revision: '1b3ea90ef448b6eda8dd2635211b95ce847f904d'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-inventaire-des-gros-projets/change-inventaire-des-gros-projets.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** un projet réel d'environ 60 couches (BD TOPO 74 + PLUi + MNT, 84 463 caractères d'inventaire) dépasse six fois le budget de 14 000 caractères ; les couches de l'utilisateur ne sortent jamais de la vue, donc l'agent rend `ai.prompt_too_long`.

**Approach:** une fonction pure du noyau, de (inventaire brut, fichiers de travail, intermédiaires, dernier plan exécuté, demande), qui rend la vue du modèle : sous un seuil, exactement celle d'aujourd'hui ; au-dessus, un `index` réduit de toutes les couches et des `layers` détaillées choisies par la demande (changement de cap du 2026-11-10, §3 points 1 à 6). Pas encore branchée à `plan.propose` (48).

Décisions du propriétaire (2026-11-10) : forme réduite `{layer_id, name, geometry}` ; seuil ≈ 9 000 caractères ; couche détaillée au plus 30 champs ou 3 000 caractères (à revoir en 48) ; la question entre couches proches n'est pas écrite dans le noyau, le tri détaille seulement les deux couches.

## Boundaries & Constraints

**Always:** Rust sans dépendance nouvelle ; accents ôtés par une table écrite à la main ; règle générique, rien de propre à la BD TOPO ; déterministe (aucun ordre de `HashSet`/`HashMap` en sortie) ; identifiants courts de `ShortIds` dans toute la vue, table faite sur toutes les couches ; résultats sans valeurs et intermédiaires retirés comme aujourd'hui (5.22, 5.34) ; tests sur fixtures synthétiques seulement.

**Never:** brancher la fonction à la composition, au canal, à la CLI ou aux mesures ; changer `model_inventory`, `agent_inventory` ou la sortie de `within_budget_by_batches` ; poser une question depuis le noyau ; lire le disque ou le journal dans la fonction ; copier une donnée réelle dans le dépôt.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Sous le seuil | couches de l'utilisateur ≤ 9 000 car. | vue et table égales à `model_inventory`, octet pour octet | — |
| Pluriel | « les gares », couche `gare_sncf` | `gare_sncf` détaillée | — |
| Valeur courte | « zones U », valeurs `U` / `AU` / `Ua` | seule la couche à valeur `U` est désignée par la valeur | — |
| Couche large | 88 champs désignée | ≤ 30 champs et ≤ 3 000 car., champs désignés et à valeurs d'abord | — |
| Index stable | deux demandes différentes | `index` identique | — |
| Gros projet | fixture synthétique de même forme que la BD TOPO 74 (63 couches) | vue ≤ 14 000 car. | — |
| Couches proches | deux couches de gares | les deux détaillées | — |
| Index trop long | index > budget d'index | les résultats les plus anciens sortent de l'index par paquets, rendus dans `left_out` ; jamais une couche de l'utilisateur | contrôle exact de `cassini-ai` en dernier filet |

</frozen-after-approval>

## Code Map

- `core/src/agent_inventory.rs` -- `model_inventory` (chemin sous le seuil, à rappeler tel quel), `stable_order`, `ShortIds`, `without_intermediates`, `results_without_values`, `result_date`, `within_budget_by_batches` (règle des paquets à reprendre pour l'index), `json_chars`, `INVENTORY_BUDGET_CHARS`, `INVENTORY_LOW_CHARS`, `ModelInventory` (`user_layers`, `left_out`).
- `core/src/journal/mod.rs` -- `intermediate_files` : modèle de lecture du journal ; ne pas toucher (la lecture du dernier plan vient en 48).
- `core/src/serve/plans.rs`, `cli/src/main.rs`, `ai/src/plan.rs` -- appelants de `model_inventory` ; ne pas toucher.
- `serde_json` sans `preserve_order` : les clés sortent triées, donc `index` avant `layers` dans le texte JSON.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/agent_inventory/sorted_view.rs` (nouveau, `mod sorted_view` + réexports dans `agent_inventory.rs`, doc du module mise à jour) -- `LastPlan { results: HashSet<String> /* fichiers */, cited: HashSet<String> /* layer_id */ }`, `ViewLimits` (seuil 9 000, budget 14 000, index 7 000 / bas 3 500, 30 champs, 3 000 car.), `sorted_view(raw, work_files, intermediates, last_plan, request, limits) -> (ModelInventory, ShortIds)` ; normalisation et correspondance ; plafond des champs ; index et paquets -- cœur du ticket.
- [x] même fichier, `#[cfg(test)]` -- tests de la matrice, déterminisme, couche de 88 champs, fixture synthétique de 63 couches décrite par une table de formes (nombre de champs, entiers, réels, champs à valeurs, nombre de valeurs par couche, mesurées le 2026-11-10) dont la taille complète est vérifiée à ±10 % de 84 463 car. -- preuve sans données réelles.

**Acceptance Criteria:**
- Given un gros projet, when la vue est rendue, then chaque couche de l'index et de `layers` porte l'identifiant court, et `layers` suit l'ordre stable (utilisateur d'abord, résultats du plus ancien au plus récent).
- Given un dernier plan exécuté, when la vue est rendue au-dessus du seuil, then ses résultats (sans valeurs) et les couches qu'il cite sont détaillés après les couches désignées, puis les rasters, jusqu'au budget.
- Given `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` et `cargo test --workspace`, then tout est vert.

## Design Notes

- Priorité de détail : désignées (rang nom 0, champ 1, valeur 2 ; à égalité ordre stable), résultats du dernier plan, couches citées, rasters ; dédoublonnées ; on s'arrête à la première qui ne tient pas (« on s'arrête au budget »).
- Champs d'une couche détaillée : rangs désigné, texte à valeurs, numérique, autre ; ordre de QGIS dans chaque rang ; ajout jusqu'à la première qui dépasse ; rendus dans l'ordre de QGIS.
- Normalisation : minuscules, table d'accents, découpe sur tout non-alphanumérique, mots vides, `s`/`x` final ôté au-delà de 3 lettres. Mot de la demande ≥ 3 lettres contre les mots du nom, du radical de l'identifiant (`qgis_uuid_id`) et du radical du fichier (dernier segment de `source`, avant `|`, sans extension). Valeur de 1 ou 2 signes : égale à un mot brut de la demande, casse comprise.
- `user_layers` d'une vue triée : couches de l'utilisateur en tête de `layers`. Le budget d'index (7 000 / 3 500) n'est pas une décision du propriétaire : réglage à revoir en 48 avec les autres.

## Verification

Dépôt de travail : le worktree `/home/kevin/cassini-worktrees/vue-triee` (branche `story/5-46-vue-triee`) ; `{project-root}` y désigne ce dossier. Avant toute compilation du workspace (llama.cpp, sous WSL) : `export LIBCLANG_PATH=/tmp/claude-1000/-mnt-c-Users-kevin-Downloads-Projet-code-CASSINI/38f6e1c7-3a62-4e6d-a1e2-bc44d994e1ac/scratchpad/clang/x/libclang-18.1.1.data/platlib/clang/native BINDGEN_EXTRA_CLANG_ARGS=-I/usr/lib/gcc/x86_64-linux-gnu/13/include`. Ne pas committer ; la mesure sur données réelles est faite par l'orchestrateur, pas par l'implémenteur.

**Commands:**
- `cargo test -p cassini-core agent_inventory` -- expected: vert
- `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` -- expected: vert
- Mesure hors dépôt (scratchpad) sur l'inventaire réel de 63 couches -- expected: vue ≤ 14 000 car., chiffres consignés dans les Implementation Notes

## Implementation Notes

- `within_budget_by_batches` : sa règle est sortie dans `kept_by_batches` (tailles seules) et `results_oldest_first`, partagés avec l'index ; sortie inchangée (tests 5.18/5.19 verts sans retouche).
- Seuil : mesuré sur l'inventaire réduit aux couches de l'utilisateur (forme apprise, identifiants courts, en-tête `project` et `unloaded_files` compris) ; `≤ threshold` ⇒ `model_inventory(raw, work_files, intermediates, budget, low)` tel quel. `ViewLimits` porte aussi `low` (7 000) pour ce chemin.
- Index : `{layer_id, name, geometry}` (clés présentes seulement ; raster `geometry: null`), budget mesuré sur la liste JSON seule ; les couches sorties de l'index ne sont jamais détaillées ; `left_out` réutilise `left_out_by_budget`.
- Correspondance : mot de demande retenu s'il a ≥ 3 signes après normalisation et au moins une lettre (« 500 » ignoré) ; mots vides comparés avant le pluriel ôté ; même normalisation pour les mots des couches. Un champ « désigné » l'est par son nom ou une de ses valeurs.
- Résultats du dernier plan : reconnus par le nom de fichier (`result_date`) ; couches citées par l'identifiant QGIS complet. Ordre stable à l'intérieur de chaque groupe.
- Fixture synthétique : 84 802 car. (réel 84 463, +0,4 %) ; index 5 994 car. ; vue « la couche46 » 9 339 car. (couche de 86 champs plafonnée à 2 960), vue où tout est désigné 13 703 car., demande vide 6 378 car. (les deux MNT).
- Vérification : `cargo fmt --all --check`, `cargo clippy --workspace --exclude cassini-app --all-targets -D warnings`, `cargo test --workspace --exclude cassini-app` verts. `cassini-app` (Tauri) exclu : libdbus-1-dev absent sous ce WSL, sans rapport avec le ticket. Mesure sur données réelles laissée à l'orchestrateur.

**Mesure sur l'inventaire réel (orchestrateur, 2026-11-10, hors dépôt).** Projet BD TOPO 74 + PLUi + MNT, 63 couches. Vue d'aujourd'hui : 84 446 caractères. Vue triée : index 5 700 caractères (63 couches) ; vue entière 13 117 à 13 842 caractères pour six phrases réelles (5 ou 6 couches détaillées), 6 063 pour une demande vide ; toujours sous 14 000. Constat : la phrase de démo ne détaille pas le zonage du PLUi (voir deferred-work, pour 48).
Vérification complète : `cargo fmt --all --check`, `cargo clippy --workspace --exclude cassini-app --all-targets -- -D warnings`, `cargo test --workspace --exclude cassini-app` verts ; `cassini-app` (Tauri) exclu, `libdbus-1-dev` absent sous ce WSL (environnement, pas ce changement).

## Plan Change Log

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment). Verdicts : high 0, medium 4, low 7, false 3, maybe-false 1.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| `layername=` d'un GeoPackage ignoré : le nom de table ne désigne pas sa couche, un mot du fichier partagé désigne les 54 couches | medium | patch | `file_stem` prend `<t>` de `|layername=<t>` ; test étendu |
| Seuil mesuré sur les seules couches de l'utilisateur, aucun test ne le distingue de la vue entière | medium | patch | test `the_threshold_measures_the_user_layers_only` |
| Champ désigné par une valeur jamais testé dans une couche plafonnée | medium | patch | test `a_field_designated_by_a_value_is_kept_in_a_capped_layer` |
| Mots vides gardés dans les mots bruts : « de », « a » désignent une valeur courte en minuscules | low | patch | mots bruts égaux à un mot vide (casse comprise) retirés ; assertion ajoutée |
| Commentaire « les deux MNT sont toujours détaillés » sans assertion, faux pour les demandes larges | low | patch | commentaire corrigé |
| « six fois le budget » contre une assertion à 5× | low | patch | assertion à 6× |
| Index dit stable « d'une demande à l'autre », ordre `index` avant `layers` dû à serde_json | low | patch | docs précisées |
| Mots communs (« carte », « pdf », « calcule ») désignent par valeur des couches sans rapport ; « PLU » ne trouve pas « PLUi » ; la phrase de démo ne détaille pas le zonage (mesure réelle) | medium | defer | deferred-work, pour 48 |
| Arrêt à la première couche qui ne tient pas : rasters privés, budget libre | low | defer | règle du changement de cap appliquée à la lettre ; deferred-work pour 48 |
| Index des seules couches de l'utilisateur au-delà du budget (≈ 150 couches) : vue hors budget | low | defer | voulu (matrice : `cassini-ai` en dernier filet) ; deferred-work pour 48 |
| `LastPlan.cited` par id complet, id peut-être renouvelé à la réouverture | maybe-false (medium si vrai) | defer | à vérifier en 48 en lisant le journal après relance |
| Champ désigné seul au-delà de 3 000 car. : couche sans champ ; couche sans champ déjà au-delà du plafond | low | reject | improbable (≤ 10 valeurs par champ), correctif ajoute une branche |
| Double calcul sous le seuil | low | reject | quelques ms, correctif ajoute de la complexité |
| Refactor `kept_by_batches` non prouvé identique | false | reject | les tests 5.18/5.19 appellent `within_budget_by_batches` et `model_inventory` directement et passent |
| Point 5 (redemande élargie) absent | false | reject | entrée 48, hors 46 par le ticket |
| Projet réel jamais joué | false | reject | mesuré hors dépôt, voir Implementation Notes |
