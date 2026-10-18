---
title: 'Modèle chargé une fois et partie fixe du prompt gardée en cache'
type: 'feature'
ticket: '12'
created: '2026-10-18'
status: 'built'
baseline_revision: '6990f08a7ae0784018077eb95821ce5e56c17817'
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

**Problem:** Chaque appel au modèle recharge le GGUF (4 s), crée un contexte et relit les ~5 130 jetons de l'invite (155 s à 4 fils, mesure 5.11), alors que les consignes, le catalogue et les exemples ne changent jamais et que l'inventaire change rarement ; une redemande double ce coût (constat 6 de 5.3), et le produit tourne à 4 fils quelle que soit la machine.

**Approach:** Garder dans le processus du noyau le modèle chargé (clé : chemin, taille, date du fichier ; autre clé → déchargé puis rechargé) et, par llama.cpp, des instantanés de l'état de la séquence pris à des points de l'invite : après la partie fixe (consignes, catalogue, exemples, désormais avant l'inventaire) et après le tour système (partie fixe + inventaire). Une demande reprend le plus profond instantané dont les jetons sont un préfixe des siens et ne lit que la suite. Le nombre de fils par défaut suit le nombre de cœurs de la machine.

## Boundaries & Constraints

**Always:** Une seule implémentation de la génération (`generate` = `generate_measured(.., None)`). Restauration par `state_seq_get` / `state_seq_set` (le modèle `qwen35` est hybride, à état récurrent : `clear_kv_cache_seq` ne sait pas le rembobiner). Le premier appel découpe la lecture de l'invite aux mêmes points que les appels suivants, pour qu'à invite égale la sortie gloutonne soit identique avec ou sans instantané. Validation, grammaire et refus de réparer inchangés (AD-15, AD-23). Hors ligne. Mesures de durée sur CPU sans CUDA, justesse sur GPU (`--features cuda`). Décisions du propriétaire (2026-10-18) : le nombre de fils par défaut est le nombre de cœurs physiques donné par la crate `num_cpus` (`get_physical`, MIT OR Apache-2.0), seule dépendance ajoutée, dans `ai/` ; le plan reste entier malgré ses ~1 900 jetons.

**Never:** Garder l'état d'une demande à l'autre au-delà du tour système (pas de mémoire de conversation). Un serveur, une socket, un fichier d'instantané sur disque. Toucher `tests/epreuve-modele-local/` (gelé), `judge`, `check_plan` ou la grammaire. Abaisser le critère de 30 s : s'il n'est pas tenu, l'écart est consigné et porté par l'entrée 10.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Première demande | aucun modèle gardé | charge, lit l'invite par segments, prend les instantanés | — |
| Même projet | même modèle, même inventaire | restaure l'instantané du tour système, ne lit que la phrase | — |
| Inventaire changé | même modèle | restaure l'instantané de la partie fixe, lit inventaire + phrase, remplace l'instantané du tour système | — |
| Redemande | 2ᵉ appel de `compose_plan` | restaure l'instantané du tour système, lit sortie rejetée + écarts | — |
| Autre modèle | chemin, taille ou date différents | ancien modèle et instantanés libérés, nouveau chargé | chargement échoué → erreur typée, rien de gardé |
| Restauration refusée | `state_seq_set` en erreur | instantanés vidés, lecture complète | jamais de panique |

</frozen-after-approval>

## Code Map

- `ai/src/lib.rs` -- `backend()` (l.88) : `static Mutex<LlamaBackend>` ; y joindre l'état gardé (`Mutex<Engine { backend, loaded: Option<Loaded> }>`, `Loaded { key, model: LlamaModel, checkpoints }`). `LlamaModel` est `Send + Sync`. Le contexte emprunte le modèle (`LlamaContext<'a>`) : il reste créé à chaque appel, seul l'instantané (`SeqState`, opaque, `Clone`) est gardé. `chat_tokens` (l.164) : rendre aussi les positions des points d'instantané. `generate_measured` (l.239) : chargement conditionnel, restauration, lecture par segments, `Timings` gagne `cached_tokens`.
- API llama-cpp-2 0.1.159 : `LlamaContext::state_seq_get(0, LlamaStateSeqFlags::empty())` / `state_seq_set(&s, 0)` (`context/session.rs` l.586 et l.632) ; `with_n_threads` / `with_n_threads_batch` déjà utilisés.
- `ai/src/plan.rs` -- `system_text` (l.230) : partie fixe d'abord (SYSTEM, CATALOGUE, EXEMPLES), puis INVENTAIRE ; `first_turns` (l.246) marque la fin de la partie fixe comme point d'instantané. `compose_plan`, `judge`, `retry_turns`, `model_generator` : comportement inchangé.
- `cli/src/main.rs` -- `ModelComposer::compose` (l.150) et `cassini plan` (l.393) passent par `model_generator` : rien à changer, ils profitent du modèle gardé.
- `ai/tests/duree_reel.rs` -- ajouter la colonne « jetons repris » ; le modèle n'est plus rechargé (en-tête à corriger).
- `ai/tests/plan_reel.rs` -- inchangé, sert à re-mesurer le score (≥ 22/25).
- `docs/mesure-duree-par-phrase.md` -- ajouter la section « après le cache (ticket 12) ».
- `_bmad-output/initiative-cassini-mvp/deferred-work.md` l.175 -- constat « recharger le GGUF à chaque tentative » : réglé par ce ticket.
- Taille d'un instantané estimée : 8 couches d'attention × K+V × 4 têtes × 256 × 2 octets ≈ 32 Ko par jeton, soit ~135 Mo (partie fixe) et ~165 Mo (tour système).

## Tasks & Acceptance

**Execution:**
- [x] `ai/src/lib.rs` -- `Turn` gagne une tête mise en cache (`Turn::with_cached_head(role, head, rest)`, `Turn::new` inchangé) ; points d'instantané = fin d'une tête, fin d'un tour système ; modèle gardé par clé ; jusqu'à un instantané par point, le plus profond préfixe gagne ; fils par défaut = `num_cpus::get_physical()` (au moins 1) ; tests sans modèle : choix de l'instantané (préfixe, plus profond, aucun), clé de modèle (chemin, taille, date), fils par défaut ≥ 1 -- le cœur du ticket
- [x] `ai/Cargo.toml` -- `num_cpus = "1"`, commentaire de licence comme pour `llama-cpp-2` -- décision du propriétaire
- [x] `ai/src/plan.rs` -- ordre de `system_text`, tête fixe dans `first_turns` ; tests : la tête est identique pour deux inventaires, l'inventaire vient après les exemples -- la partie fixe doit précéder ce qui change
- [x] `ai/tests/cache_reel.rs` (nouveau, `#[ignore]`) -- même demande deux fois : sortie identique, 2ᵉ appel `cached_tokens` > 0 et sans rechargement ; inventaire changé : reprise à la partie fixe -- preuve sur le vrai modèle
- [x] `ai/tests/duree_reel.rs` -- colonne « jetons repris », en-tête corrigé -- outil de mesure
- [x] `docs/mesure-duree-par-phrase.md` -- tableau 25 phrases à 4 fils sur 4 cœurs (`0x55`), premier appel noté à part, verdict S17 contre 30 s -- verify du ticket
- [x] `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- marquer réglé le constat du rechargement -- ménage

**Acceptance Criteria:**
- Given le 5800X bridé à 4 cœurs, binaire sans CUDA, l'inventaire de l'épreuve, when `duree_reel` passe les 25 phrases dans un même processus, then le document porte la durée de S17 et dit si elle est sous 30 s ; sinon l'écart et sa cause (écriture) sont consignés pour l'entrée 10.
- Given le GPU, when `plan_reel` tourne avec `--features cuda`, then au moins 22 des 25 phrases sont conformes.
- Given la suite sans modèle, when `cargo test --workspace`, `cargo clippy --all-targets -- -D warnings` et `cargo fmt --check` tournent, then tout passe.

## Implementation Notes

- `Turn` gagne le champ public `cached_head` (vide pour `Turn::new`) ; `chat_tokens` découpe la tête à part et rend les points d'instantané. Instantanés gardés par numéro de point (`slot`), au plus un par point ; un instantané qui couvrirait toute l'invite n'est jamais repris (il faut au moins un jeton lu).
- Restauration refusée : instantanés vidés, `clear_kv_cache`, lecture complète. Prise d'instantané en échec : ignorée, sans erreur.
- `Timings.load` vaut ~0,1 s quand le modèle est gardé ; la reprise et la prise d'instantanés sont comptées dans `prompt`.
- `default_threads()` est public (utilisé par `duree_reel` pour afficher le réglage).
- Mesures (2026-10-18) : CPU 4 fils `0x55`, S02-S25 17,4 s en moyenne, S17 61,4 s (écriture 59,1 s) : au-dessus de 30 s, écart porté par l'entrée 10. GPU : `plan_reel` 23/25, `cache_reel` vert (5 109 puis 4 173 jetons repris).
- Audit de la matrice (après le rapport de l'implémenteur) : « Restauration refusée », « Redemande » et « Autre modèle » n'avaient aucun test, et « chargement échoué → rien de gardé » non plus. La reprise sort dans `Checkpoints::restore` (testée sans modèle : `a_refused_restore_forgets_every_checkpoint_and_reads_everything`) ; `a_file_that_is_not_gguf_is_a_load_error` vérifie qu'aucun modèle n'est gardé ; `cache_reel` ajoute la redemande (5 019 jetons repris, le tour système) et un lien dur vers le GGUF comme autre modèle (0 repris, puis 0 au retour : rechargé). Sous Windows : fmt, clippy `-D warnings`, `cargo test --workspace` verts ; `cache_reel` sur GPU vert (67 s).
- Après la passe 1 de relecture (5 correctifs) : sous Windows, fmt, clippy `-D warnings` et `cargo test --workspace` verts (cassini-ai 35 tests, core 178) ; sur GPU, `cache_reel` vert (rechargement 3,7 s contre 35 ms) et `plan_reel` 23/25 (1 question).

## Plan Change Log

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment). 0 high, 3 medium, 9 low, 6 false, 0 maybe-false.

| # | Verdict | Route | Evidence |
|---|---------|-------|----------|
| 1 | medium | patch | `default_threads` : `num_cpus::get_physical` (Windows : `GetLogicalProcessorInformation`) ignore l'affinité ; sous `0x55` le défaut serait 8 fils sur 4 cœurs, et le test `>= 1` est vrai par type. `min(get_physical, available_parallelism)`, test par égalité. |
| 2 | medium | patch | Aucune décision « autre clé → libérer avant de charger » n'est testée sans modèle : le moteur est toujours vide dans la suite, l'assertion `loaded.is_none()` ne peut pas échouer. Décision extraite en fonction pure et testée. |
| 3 | medium | patch | Les points d'instantané de `chat_tokens` ne sont vérifiés que par `cache_reel` (ignoré) : retirer le point de fin du tour système laisse la suite verte. Calcul des points rendu générique sur la découpe, testé avec un faux découpeur. |
| 4 | low | patch | `cache_reel` : un panic entre `hard_link` et `remove_file` laisse un lien près du modèle ; le rechargement n'est prouvé que par `cached_tokens == 0`. Garde `Drop` et assertion sur `load`. |
| 5 | low | patch | Document : « une fois par session » vaut pour `cassini serve` seul (`cassini plan` relance un processus) ; « il ne reste à lire que la phrase » vaut pour un inventaire inchangé (~936 jetons, ~28 s sinon, non mesuré) ; la découpe séparée de la tête est une seconde cause de l'écart S05/S10. Précisé. |
| 6 | low | defer | Premier appel d'une session à 173 s : aucun préchargement au démarrage de `serve`. Hors intention (note du ticket : budget compté dès la 2ᵉ demande, premier chargement signalé par l'indicateur) ; reporté. |
| 7 | low | defer | Aucun test ne prouve que `serve` garde le modèle entre deux `plan.propose` réels ni que l'inventaire envoyé deux fois par QGIS donne les mêmes jetons ; raisonné (`model_generator` → `generate`, `serde_json` sans `preserve_order` sérialise dans un ordre stable). Reporté à l'entrée 9. |
| 8 | low | rejeté | Mémoire gardée (~300 Mo d'instantanés) non mesurée et jamais libérée : estimation écrite dans le plan et le document ; mesurer demande une instrumentation nouvelle, peu utile au quotidien. |
| 9 | low | rejeté | Reprise refusée ou instantané manqué invisibles (pas de compteur dans `Timings`) : ajout de surface publique pour un cas jamais observé. |
| 10 | low | rejeté | Redemande prouvée par des tours construits à la main, pas par `retry_turns` : forcer une sortie rejetée du vrai modèle demande un montage lourd ; le test de `plan.rs` vérifie que la redemande garde le tour système. |
| 11 | low | rejeté | `ModelKey` : deux `modified == None` sont égaux, et fenêtre entre la clé et le chargement : la date est toujours fournie sous Windows et Linux, et remplacer le GGUF pendant un chargement n'arrive pas en usage normal. |
| 12 | low | rejeté | `first_call` de `cache_reel` masque l'erreur de `generate_measured` derrière « au moins un appel » : diagnostic de test seulement. |
| 13 | false | rejeté | Reprise refusée suivie de `clear_kv_cache` : `llama_memory_clear(mem, true)` vide aussi l'état récurrent pour une mémoire hybride (`context/kv_cache.rs` l.103, « clear both metadata and data ») ; le contexte est de plus neuf à chaque appel. |
| 14 | false | rejeté | Instantanés rangés par numéro de point, écrasés par un appelant d'une autre forme : seuls `compose_plan` (tour système à tête) et `complete` (un tour utilisateur, aucun point) appellent la génération. |
| 15 | false | rejeté | Un tour système après un tour utilisateur ferait garder la phrase : aucun appelant ne construit cet ordre. |
| 16 | false | rejeté | Instantané préfixe sans être un point de la nouvelle invite : un instantané finit sur la fin de la tête fixe ou sur `<|im_end|>` du tour système, qui sont toujours des points de toute invite qui les contient en préfixe. |
| 17 | false | rejeté | `SeqState` vide rendu pour une séquence décodée : un instantané n'est pris qu'après un `decode` réussi d'au moins un jeton ; `cache_reel` montre la reprise effective (sortie identique). |
| 18 | false | rejeté | « Journal de triage vide » et `status: close` : le triage est écrit ici ; `close` est le terme de toutes les entrées closes de `deferred-work.md` (l.14, 20, 26…). |

## Design Notes

À écrire par avance : S17 écrit 247 jetons à environ 4 jetons/s (58 s, mesure 5.11), borné par la mémoire et non par le nombre de fils. Même avec une invite entièrement reprise, S17 restera au-dessus de 30 s sur ce poste ; le cache gagne la lecture (~155 s), pas l'écriture. C'est le cas prévu par la note `unknown` du ticket.

Lecture par segments : `[0, p1)`, instantané, `[p1, p2)`, instantané, `[p2, fin)` ; la restauration d'un instantané en `pk` reprend la même découpe à partir de `pk`.

## Verification

**Commands:**
- `cargo test --workspace`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` -- expected: succès
- PowerShell, `CARGO_TARGET_DIR=C:\ct`, sans CUDA : `duree_reel` à `CASSINI_TEST_THREADS=4`, affinité `0x55` -- expected: 25 lignes, S17 relevé
- PowerShell, `--features cuda` : `plan_reel` puis `cache_reel` -- expected: ≥ 22/25, sorties identiques
