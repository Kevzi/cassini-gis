---
title: 'Modele ajuste, tache 3 : prompt court et base + adaptateur charges par ai'
type: 'feature'
ticket: '10'
created: '2026-10-22'
status: 'built'
baseline_revision: 'b7104a503f53b1a157ebd28009d6c8586cc30760'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['quick', 'blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: ['{project-root}/docs/entrainement-du-modele.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** La tâche 2 a sorti un modèle ajusté (base Qwen3.5-4B Q4_K_M + adaptateur LoRA, et un fusionné), mais `ai/` ne sait lui parler qu'avec le long prompt du 9B et ne charge qu'un seul fichier GGUF. Le modèle ajusté, qui n'a appris que le prompt court, ne peut donc pas servir.

**Approach:** Tâche 3 sur 4 du ticket 5.10. `ai/` apprend le prompt court, identique au jeton près à celui de l'entraînement, et sait charger une base avec un adaptateur. Un modèle déclare lui-même le prompt qu'il attend et l'adaptateur qui va sur sa base ; le noyau le liste comme un modèle parmi les autres, prêt pour le sélecteur de 5.14.

## Boundaries & Constraints

**Always:** Un GGUF sans déclaration garde le long prompt actuel, à l'identique (Qwen3.5-9B inchangé). Le texte du prompt court est celui de `training/src/prompt.rs` : un test le compare octet par octet sur le jeu engendré, et un test avec le vrai modèle compare les jetons de `ai` au découpage d'un tenant de l'entraînement (ferme l'entrée différée « parité au jeton »). Même grammaire, même `judge`, même redemande unique (AD-23). Le noyau seul découvre les modèles (AD-15) ; aucun chemin absolu dans un message (AD-9). Un seul adaptateur par modèle. Aucune nouvelle dépendance externe. Démo réelle avant de rendre : `cassini plan` sur la base + adaptateur, avec `--features cuda`. Décisions du propriétaire (2026-10-22) : un modèle se déclare par un petit fichier `*.modele.json` posé à côté des GGUF, par exemple `{"model":"qwen3.5-4b-base-Q4_K_M.gguf","adapter":"cassini-composition-adaptateur-f16.gguf","prompt":"short"}` (`adapter` facultatif, noms de fichiers seuls, même dossier) ; la chaîne d'entraînement l'écrit ; les GGUF qu'il nomme disparaissent de la liste ; plan gardé entier malgré ses ~1 950 jetons.

**Never:** Ré-entraîner ou changer le texte du prompt court (la précision « en comptant à partir de 1 » attend le prochain entraînement). Toucher `tests/epreuve-modele-local/` ou `ai/tests/tenu_a_l_ecart/`. Construire le sélecteur de 5.14 (`agent.models`, paramètre `model` de `plan.propose`). Mesurer le jeu tenu à l'écart ou la durée (tâche 4). Lier `cassini-training` au binaire du produit.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Modèle ordinaire | `Qwen3.5-9B-Q4_K_M.gguf` seul | long prompt, comme avant | — |
| Base + adaptateur | déclaration → base + adaptateur, prompt court | base chargée, adaptateur posé sur chaque contexte, prompt court | — |
| Fusionné | déclaration → un GGUF, prompt court | prompt court, pas d'adaptateur | — |
| Fichier nommé absent | la déclaration cite un GGUF manquant | modèle introuvable, nom du fichier seul | `ai.model_not_found` |
| Adaptateur illisible | ce n'est pas un adaptateur de cette base | erreur typée, rien de gardé | `ai.adapter` |
| Autre adaptateur, même base | — | instantanés oubliés, tout relu | — |

</frozen-after-approval>

## Code Map

- `ai/src/lib.rs` -- `Loaded { key: ModelKey, model, checkpoints }`, `generate_measured` : la clé doit couvrir base **et** adaptateur ; l'adaptateur s'initialise une fois (`LlamaModel::lora_adapter_init`, `&mut self`) et se pose sur chaque contexte neuf (`LlamaContext::lora_adapter_set(&mut adapter, 1.0)`), avant la reprise d'un instantané. API vérifiée dans llama-cpp-2 0.1.159 (`context.rs` l.368, `model.rs` l.366). `chat_tokens` (privé) : découpe par segment, `special=false` pour le texte.
- `ai/src/plan.rs` -- `first_turns` l.256, `fixed_system_text` l.230, `inventory_text` l.245, `compose_plan` l.358, `model_generator` l.396. Le prompt court se coupe : tête en cache = `SYSTEM` de training, reste = `"\n\nINVENTAIRE DU PROJET :\n{inv}\n"` ; tour utilisateur = la phrase. `judge`, `plan_grammar`, `retry_turns` inchangés.
- `training/src/prompt.rs` -- `SYSTEM`, `render_prompt`, `system_text` : la référence. Lecture seule.
- `core/src/commands/model_list.rs` -- `discover` l.52 : liste les `*.gguf` d'un niveau, triés ; `ModelEntry { file, size_bytes }`. Tests l.150+.
- `cli/src/main.rs` -- `ModelComposer::compose` l.150 (premier de la liste, masque des chemins), `default_model` l.323, `plan` l.343 (`--model`).
- `training/entrainement/pipeline.py` -- `BASE_NAME`, `ADAPTER_NAME`, `MERGED_NAME` l.92-94 ; `launch` l.484 écrit `rapport.json` ; `--resume` saute ce qui existe. Tests `unittest` sans torch dans `training/entrainement/tests/`.
- Sorties réelles : `~/cassini-entrainement/5-10-t2/` (WSL) : les trois GGUF, pas encore de déclaration.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/commands/model_list.rs` -- lire les déclarations du dossier ; une déclaration devient une entrée de la liste ; un GGUF qu'elle nomme n'est plus listé seul ; une déclaration illisible ou qui nomme un chemin (séparateur, `..`) est ignorée ; tests -- le noyau seul découvre (AD-15)
- [x] `ai/src/lib.rs` -- `ModelFiles { model, adapter: Option }`, clé élargie, adaptateur gardé avec le modèle, posé sur chaque contexte ; `AiError::Adapter` (`ai.adapter`) ; tests sans modèle sur la clé -- base + adaptateur
- [x] `ai/src/plan.rs`, `ai/Cargo.toml` -- `PromptStyle { Full, Short }`, tours du prompt court ; `cassini-training` en dépendance de développement seulement ; test : pour chaque exemple engendré, les tours concaténés valent `render_prompt` ; test réel (variable `CASSINI_TEST_MODEL`) : jetons de `chat_tokens` = découpage d'un tenant avec `special=true` -- parité
- [x] `cli/src/main.rs` -- `--model` et le compositeur acceptent une déclaration ; masque des chemins de chaque fichier ; tests -- branchement
- [x] `training/entrainement/pipeline.py` + test -- écrire les deux déclarations (base + adaptateur, fusionné), aussi à la reprise -- la chaîne livre un modèle prêt
- [x] `docs/entrainement-du-modele.md` -- section tâche 3 : déclaration, installation dans `models/`, démo -- consigné

**Acceptance Criteria:**
- Given le dossier de sortie de la tâche 2 repris par la chaîne, when on copie son contenu dans un dossier de modèles, then `model.list` rend deux entrées (base + adaptateur, fusionné) et aucun GGUF nu.
- Given ces deux entrées, when `cassini plan` compose « Trouve les parcelles de la zone U à moins de 500 m d'une gare » sur un inventaire de démo, then chacune rend un plan valide, et le fichier du 9B rend toujours un plan avec le long prompt.
- Given `cargo test --workspace`, when il tourne sans modèle, then tout passe, test de parité du texte compris.

## Review Triage Log

Passe 1 (thorough, risque high : quick, blind-hunter, edge-case-hunter, verification-gap, intent-alignment). Avant la relecture, le contrôle des cas a ajouté le test réel `a_file_that_is_not_an_adapter_of_this_base_is_an_adapter_error_and_keeps_nothing` (ligne « Adaptateur illisible » sans test). 0 high, 4 medium, 17 low, 2 false ; aucun intent_gap ni bad_plan.

| # | Verdict | Route | Evidence |
|---|---------|-------|----------|
| 1 | medium | patch | verification-gap : le style de prompt d'une déclaration n'est vérifié à aucun des deux sites de `cli` (remplacer `chosen.style` par `Full` passe tous les tests). Méthode commune `ChosenModel::compose` et test au générateur scripté. |
| 2 | medium | patch | blind-hunter, verification-gap, intent-alignment : rien ne montre qu'un vrai adaptateur est posé (supprimer `lora_adapter_set` passe tout). Test réel sous `CASSINI_TEST_MODEL` + `CASSINI_TEST_ADAPTER` : base seule ≠ base + adaptateur, deux appels identiques (reprise d'instantané). |
| 3 | medium | patch + defer | intent-alignment, quick : la résolution « parité au jeton » de `deferred-work.md` revendique la parité avec l'entraînement, le test compare au découpage de llama.cpp, pas au tokenizer Hugging Face de l'entraînement. Résolution reformulée ; comparaison HF / llama.cpp reportée à la tâche 4. |
| 4 | medium | defer | blind-hunter : la redemande (sortie rejetée + écarts) n'a jamais été apprise par le modèle ajusté ; à mesurer à la tâche 4. Garder la redemande unique reste la règle (AD-23). |
| 5 | low | patch | quick, blind-hunter : un échec de `lora_adapter_set` laisse `Loaded` gardé ; chaque appel suivant échoue pareil. `*loaded = None` avant l'erreur. |
| 6 | low | patch | quatre lentilles : le masquage des GGUF nommés compare à la casse près, l'extension et NTFS non. `eq_ignore_ascii_case`. |
| 7 | low | patch | blind-hunter : `"C:\m.gguf"` et `"..\a.gguf"` sont des échappements JSON invalides : ces cas échouent avant le contrôle de nom. Antislash doublé. |
| 8 | low | patch | blind-hunter, edge-case-hunter : le test de parité charge un second modèle à côté de celui du moteur, et `points[0] < ours.len()` ne prouve rien. Moteur vidé avant, point de coupe comparé d'un exemple à l'autre. |
| 9 | low | patch | edge-case-hunter, verification-gap : `a_missing_adapter_is_typed…` teste une base absente, pas un adaptateur absent. Base présente, adaptateur absent. |
| 10 | low | patch | edge-case-hunter : `--model` vers une déclaration absente rend `models.bad_declaration` au lieu de `ai.model_not_found`. |
| 11 | low | patch | quick : `cassini plan` imprime le chemin complet de la base absente d'une déclaration ; la matrice dit « nom du fichier seul ». `scrub` avec `chosen.hidden()`. |
| 12 | low | patch | blind-hunter : la doc du module dit « aucun GGUF n'est ouvert » sans dire que les déclarations sont lues. |
| 13 | medium | defer | blind-hunter, edge-case-hunter, quick : une déclaration illisible est écartée sans trace (ses GGUF, adaptateur compris, reviennent nus dans la liste) ; une déclaration dont la base manque reste listée avec une taille partielle et, première par son nom, bloquerait le canal. Diagnostic des déclarations à porter par 5.14 (sélecteur). |
| 14 | low | rejeté | blind-hunter : un GGUF nommé par une déclaration, passé directement à `--model`, prend le long prompt. Usage de développeur, invisible au canal (le GGUF n'est pas listé) ; la garde ajouterait une recherche de déclarations sœurs. |
| 15 | low | rejeté | blind-hunter : la ligne « autre adaptateur » n'est testée qu'au niveau de la clé. `drop_if_other_key` est testé, la clé couvre l'adaptateur : la composition suffit. |
| 16 | low | rejeté | blind-hunter : `PromptStyle` double `PromptKind`. La conversion est un `match` exhaustif : une variante ajoutée ne compile pas sans elle, pas de dérive possible. |
| 17 | low | rejeté | blind-hunter : chemins d'erreur sans test de bout en bout (adaptateur absent par `serve`, `bad_declaration` par `serve`). Même `scrub` que le cas couvert. |
| 18 | low | rejeté | blind-hunter, edge-case-hunter : `..` interdit au milieu d'un nom (`modele..v2.gguf`). Prudent, nom improbable ; noms réservés de Windows : le chargement échoue proprement. |
| 19 | low | rejeté | blind-hunter : la doc ne compare pas les durées de démo au but de 30 s ni ne recommande d'entrée par défaut : c'est la tâche 4 et 5.14. |
| 20 | low | rejeté | edge-case-hunter : même fichier en modèle et en adaptateur : improbable, échoue en `ai.adapter`. |
| 21 | low | rejeté | verification-gap : les tests réels passent à vide sans modèle. Motif existant du dépôt, commandes listées dans Verification. |
| 22 | low | rejeté | verification-gap : l'appel de `write_declarations` par `launch` n'est pas testé (torch requis) ; vérifié par la reprise réelle. |
| 23 | low | rejeté | verification-gap : `model-check --model` ne prend qu'un GGUF ; usage documenté, outil de diagnostic. |
| 24 | false | rejeté | quick : les descriptions de `model.list` parlent encore de « fichiers GGUF ». `model.list` entre au catalogue du long prompt : la changer décalerait le prompt du 9B, que l'Always garde à l'identique ; voulu. |
| 25 | false | rejeté | edge-case-hunter : tests parallèles qui partageraient un modèle gardé. Aucun test de `lib` ne garde un vrai modèle dans le moteur (les tests réels chargent à part ou échouent avant) : `loaded` reste vide. |

## Design Notes

Dépendance de développement plutôt que normale : le binaire du produit n'embarque pas le générateur, et `ai` reste publiable. Le texte du prompt court est donc écrit deux fois, mais un test le compare au jeton près sur tout le jeu engendré : un écart d'un espace échoue la recette.

Le prompt court se coupe au même endroit que le long (fin des consignes = point d'instantané) : le cache de 5.12 reste valable, la tête étant fixe.

## Verification

**Commands:**
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` (WSL) -- expected: succès
- `python3 -I -m unittest discover -s training/entrainement/tests` -- expected: succès
- `CASSINI_TEST_MODEL=<base Q4_K_M> cargo test -p cassini-ai --features cuda -- --nocapture` -- expected: parité au jeton
- `cassini plan … --model <déclaration>` sur les deux entrées, `--features cuda` -- expected: plans valides
- PowerShell `cargo deny check` -- expected: succès
