---
title: 'Selecteur de modele dans les Reglages du panneau'
type: 'feature'
ticket: '14'
created: '2026-10-23'
status: done
baseline_revision: '467b2ceded260479e4860b9c844fe9872823fd33'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: ['{project-root}/docs/canal-local.md', '{project-root}/_bmad-output/initiative-cassini-mvp/ux-panneau-cassini/EXPERIENCE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** La ligne Modèle des Réglages est grisée : le noyau prend toujours le premier modèle du dossier, si bien que l'utilisateur ne peut pas choisir entre le 9B et le modèle ajusté (déclarations `*.modele.json` de 5.10).

**Approach:** Le noyau seul découvre les modèles et les rend par une nouvelle méthode du canal `agent.models` (noms de fichiers et tailles, ni dossier ni chemin, non journalisée) ; `plan.propose` accepte `model` (un nom de cette liste, absent = premier, inconnu = `models.unknown`). Le panneau affiche la liste dans les Réglages, mémorise le choix dans `QSettings` `cassini/model` et l'envoie à chaque proposition ; le changement de clé recharge le modèle (entrée 12, déjà en place).

## Boundaries & Constraints

**Always:** Écart assumé à AD-15 (décision du 2026-10-18) : `agent.models` hors registre, jamais `command.run model.list` côté extension. Aucun chemin absolu à l'écran ni dans un message du canal (AD-9) : les erreurs de découverte passent par le masquage du compositeur de `cli`. `agent.models` exige une session ouverte, rend `agent.unavailable` sans compositeur et `session.busy` pendant un ordre, comme `plan.propose`. Le nom de `model` est comparé tel quel aux `file` de la liste. Un choix mémorisé absent de la liste reçue est oublié et le premier modèle est pris. Logique de choix dans un module Python sans Qt, testé par `unittest` ; Qt par `qgis.PyQt` seulement. Microcopy FR/EN, mêmes clés.

**Décisions du propriétaire (2026-10-23) :** plan gardé entier (~2 600 jetons). Le diagnostic des déclarations (`deferred-work.md`, « 5.14 — diagnostic ») est inclus : chaque entrée d'`agent.models` porte un `problem` facultatif (« déclaration illisible », « fichier manquant : x.gguf »), une déclaration illisible reste listée avec son problème au lieu de disparaître, le sélecteur grise ces entrées avec leur raison, « absent = premier » devient le premier modèle sans problème, et choisir un modèle à problème rend `models.unusable` avec la raison.

**Never:** Changer `model.list` ni son résultat journalisé (le diagnostic vit dans une fonction publique de `model_list.rs` à côté de `discover`, que `model.list` n'utilise pas). Ouvrir un GGUF pour lister. Réparer une déclaration. Nouvelle dépendance. Serveur MCP (5.7).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Liste | `agent.models` | `{"models":[{"file","size_bytes"}…]}` trié par nom, rien au journal | — |
| Dossier absent | `CASSINI_MODELS` vers un dossier absent | erreur `models.no_folder`, chemin remplacé par « (dossier des modèles) » | Réglages : liste grisée, message et code |
| Dossier vide | aucun GGUF | liste vide | Réglages : liste grisée, « Aucun modèle… » |
| Choix | `plan.propose {request, model:"b.gguf"}` | plan composé par `b.gguf`, modèle rechargé si la clé change | — |
| Sans choix | `model` absent | premier de la liste, comme avant | — |
| Inconnu | `model:"x.gguf"` absent de la liste | erreur `models.unknown`, aucun ordre d'inventaire | le panneau redemande la liste |
| Mauvais type | `model: 3` | `-32602` paramètres invalides | — |
| Choix disparu | mémorisé `b.gguf`, liste sans lui | choix oublié dans `QSettings`, premier affiché | — |
| Déclaration cassée | `a.modele.json` illisible, `b.modele.json` dont un GGUF manque, `c.gguf` | trois entrées, `a` et `b` avec `problem` ; `model` absent → `c.gguf` | Réglages : `a`, `b` grisés avec leur raison |
| Modèle inutilisable | `model:"b.modele.json"` à problème | erreur `models.unusable`, raison, aucun inventaire | — |
| Pas d'agent | noyau sans compositeur | `agent.unavailable` | Réglages : grisé avec le message |

</frozen-after-approval>

## Code Map

- `core/src/serve/plans.rs` -- trait `Composer` (l.22) : ajouter `models(&self, registry) -> Result<Vec<Value>, ErrorReport>` et le paramètre `model: Option<&str>` à `compose` ; `propose` (l.95) lit `model` (texte ou absent, sinon `invalid_params`) ; nouvelle fonction `models(state)` pour `agent.models`.
- `core/src/serve.rs` -- `dispatch` (l.364) : route `agent.models` ; `answer_during_order` (l.450) : l'ajouter à la liste `session.busy` ; module doc (l.1-11). Tests : `FixedComposer` (l.~1093) à adapter ; motifs d'échange `exchange`/client de test existants.
- `cli/src/main.rs` -- `ModelComposer` (l.148) : extraire la découverte (appel `model.list`, `hidden`, `scrub`) dans une fonction partagée par `compose` et `models` ; passe par la fonction de diagnostic au lieu de `registry.call("model.list")` ; `models` rend `file`, `size_bytes` et `problem` (masqué) ; `compose` prend le premier sans problème, cherche `model` dans la liste, sinon `models.unknown`, `models.unusable` s'il a un problème (« Le modèle « x » n'est pas dans le dossier des modèles. »), avant tout inventaire. Tests `cli` existants (l.770-880) : motif de dossier temporaire.
- `core/src/commands/model_list.rs` -- `discover` (l.144) et `ModelEntry` inchangés pour `model.list` ; nouvelle fonction publique de diagnostic (même choix de dossier, même parcours) rendant `{file, size_bytes, problem}`, où une déclaration illisible (`read_declaration` en `Err`) et un GGUF nommé absent deviennent un `problem` ; partager le parcours plutôt que le dupliquer. Tests à côté de `a_declaration_naming_a_missing_file_is_still_listed`.
- `ai/src/lib.rs` -- lecture seule : modèle gardé par clé (chemin, taille, date) ; rien à changer pour recharger.
- `qgis-plugin/cassini_qgis/client.py` -- `list_models()` (step `models`, garde `_can_send`), événements `MODELS {models}` et `MODELS_FAILED {code, message}` ; `propose(request, model=None)` n'envoie `model` que s'il est un texte ; `_handle` (l.276) route la réponse `models`.
- `qgis-plugin/cassini_qgis/models.py` (nouveau, sans Qt) -- `choose(listed, remembered) -> (selected, forget)`, libellé d'une entrée (nom + taille lisible), raison de la liste grisée ; une entrée à `problem` n'est jamais choisie (choix mémorisé à problème : oublié).
- `qgis-plugin/cassini_qgis/panel.py` -- Réglages (l.266-281) : `model_label` devient libellé + `QComboBox` `model_box` (nom accessible) + raison ; `_open_settings` (l.1014) demande la liste ; `currentIndexChanged` mémorise `cassini/model` ; `_send` passe le choix à `propose` ; sur `PLAN_FAILED` `models.unknown`, redemande la liste.
- `qgis-plugin/cassini_qgis/plugin.py` -- action `list_models()` ; `propose(text, model)` ; `_on_event` (l.162) route `MODELS*` vers le panneau ; demande la liste à `READY`.
- `qgis-plugin/cassini_qgis/texts.py` -- `settings_model` (l.13, l.134) devient « Modèle » ; clés `models_none`, `models_error` (« {message} ({code}) »).
- `qgis-plugin/tests/test_plan_client.py`, `test_panel_logic.py` -- motifs `FakeProcess` et import sans Qt.
- `docs/canal-local.md` -- section « Plans de l'agent » (l.95) : `agent.models`, `model`, `models.unknown`.
- `qgis-plugin/README.md` -- recette manuelle.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/serve/plans.rs`, `core/src/serve.rs` -- trait, `agent.models`, `model` sur `plan.propose` ; tests : liste rendue et non journalisée, `agent.unavailable`, `session.busy`, `model` transmis au compositeur, mauvais type -- lignes Liste, Mauvais type, Pas d'agent
- [x] `core/src/commands/model_list.rs` -- fonction de diagnostic ; tests : illisible listée avec problème, GGUF manquant nommé, `model.list` inchangé -- ligne Déclaration cassée
- [x] `cli/src/main.rs` -- découverte partagée, `models`, choix et `models.unknown` ; tests sur dossier temporaire à deux GGUF : liste sans chemin, erreur masquée, inconnu sans inventaire, choix du second -- lignes Dossier absent/vide, Choix, Sans choix, Inconnu, Modèle inutilisable
- [x] `qgis-plugin/cassini_qgis/client.py`, `models.py` + `qgis-plugin/tests/test_models.py`, `test_plan_client.py` -- envoi, événements, choix et oubli -- ligne Choix disparu
- [x] `qgis-plugin/cassini_qgis/panel.py`, `plugin.py`, `texts.py` -- liste dans les Réglages, mémorisation, routage
- [x] `docs/canal-local.md`, `qgis-plugin/README.md` -- contrat et recette manuelle (hitl)
- [x] `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- marquer close l'entrée « 5.14 — diagnostic » ; entrée : reporter l'écart à AD-15 dans l'architecture (`bmad-architecture`)

**Acceptance Criteria:**
- Given deux modèles dans le dossier des modèles, when le propriétaire choisit le second dans Réglages et envoie une demande, then le plan est composé avec ce fichier, le choix survit à la réouverture de QGIS, et aucun chemin absolu n'apparaît à l'écran ni dans les messages du canal.
- Given le vrai modèle et `cassini serve` (`--features cuda`), when deux `plan.propose` changent de `model`, then le second recharge le modèle (`Timings.load` repart au-dessus de 0,1 s) et aucun message ne porte de chemin absolu.
- Given la suite sans modèle, when `cargo test --workspace`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `python -I -m unittest discover -s qgis-plugin/tests` et `python -I qgis-plugin/tests/check_extension.py` tournent, then tout passe.

## Implementation Notes

- `model_list.rs` : `choose_folder` et `scan` partagés par `discover` (`model.list`, sortie inchangée) et `diagnose` (public, `ModelDiagnosis {folder, models: [DiagnosedModel {file, size_bytes, problem}]}`). Le `problem` d'une déclaration illisible est le message de `read_declaration` (sans chemin) ; un GGUF absent donne « fichier manquant : a.gguf, b.gguf ».
- `cli` : `ModelComposer` porte `setting` (`CASSINI_MODELS`) et `exe_dir`, lus une fois par `from_env` ; les tests passent un dossier temporaire sans toucher l'environnement. `choose` refuse `models.unknown` / `models.unusable` avant tout inventaire ; sans `model`, un dossier sans modèle utilisable rend `ai.no_model`. Le test de bout en bout `plan_propose_with_a_declaration_naming_a_missing_base_names_the_file_only` change de sens : sans choix, la déclaration à problème n'est plus prise (`ai.no_model`), choisie elle rend `models.unusable` ; plus aucun ordre d'inventaire.
- `model: null` sur `plan.propose` vaut absent.
- Extension : `agent.models` compte comme la requête en vol du client (`_can_send`) ; une demande envoyée pendant la liste (fenêtre courte, à `READY` et à l'ouverture des Réglages) ne part pas, comme pendant toute autre requête.
- Vérification réelle (GPU, `--features cassini-ai/cuda`, `CASSINI_MODELS` vers le dossier source à trois GGUF) : `agent.models` sans chemin ; Ministral Q4 7,4 s, puis Qwen 9,7 s (rechargement), puis Qwen 3,1 s (gardé) ; aucun chemin absolu dans les messages. `Timings.load` n'est pas exposé par le canal : la mesure est le temps de bout en bout de `plan.propose`. Le binaire CPU de `C:\ct\release` a été reconstruit ensuite.

## Plan Change Log

## Review Triage Log

| # | Lentille | Constat | Verdict | Route | Preuve |
|---|---|---|---|---|---|
| 1 | blind | Une déclaration lisible à problème (adaptateur absent) cache encore son GGUF de base présent dans `diagnose` | medium | patch | `scan` retire tout GGUF nommé par une déclaration lisible ; test `diagnose_keeps…` : `present.gguf` absent. Avec `Qwen.gguf` + `ajuste.modele.json` sans adaptateur, rien d'utilisable → `ai.no_model`. Correctif limité à `diagnose`, `model.list` inchangé |
| 2 | edge, blind | `problem` peut porter un chemin absolu (AD-9) et le détail serde en anglais | medium | patch | `read_declaration` écrit « nomme « {bad} » » avec la valeur déclarée telle quelle, et `({e})` de serde, qui peut citer une valeur ; `hide` ne masque que le dossier |
| 3 | edge | Entrée à problème affichée « (0 octets) » | low | patch | `diagnose` rend `size_bytes: 0` pour une déclaration illisible ; `label` affiche toute taille non nulle-None ; correction directe |
| 4 | verif-gap | `model: null` documenté comme absent, non testé | low | patch | Aucun test n'envoie `"model": null` ; le motif `Some(Value::Null)` peut disparaître sans échec |
| 5 | blind | README : « 5,7 Go » contre 5,6 Go ailleurs ; « le premier est sélectionné » au lieu de « le premier utilisable » | low | patch | `qgis-plugin/README.md`, recette ticket 14, étape 1 ; correction directe |
| 6 | verif-gap, blind, intent | Câblage du sélecteur dans `panel.py` (envoi du choix, oubli `QSettings`, redemande sur `models.unknown`) sans test | medium | defer | Aucun test ne charge `panel.py` (`FakePanel` dans `test_tracer.py`) ; le dépôt vérifie le Qt par recette manuelle |
| 7 | blind, edge | Casse : `retain` insensible, contrôle des fichiers manquants sensible | low | reject | Poste cible Windows, système de fichiers insensible à la casse ; rare, correctif non direct |
| 8 | blind | Le panneau envoie le modèle par défaut comme un choix explicite | low | reject | Seul effet : fichier retiré entre la liste et la demande → `models.unknown`, puis liste redemandée ; rare |
| 9 | blind | Envoi ignoré sans message pendant `agent.models` | low | reject | Fenêtre de quelques ms (parcours d'un dossier, aucun GGUF ouvert) ; correctif non direct |
| 10 | blind, edge | Liste grisée sans raison si `list_models()` ne part pas à `READY` | low | reject | Rien n'occupe le client à `READY` ; l'ouverture des Réglages redemande ; rare |
| 11 | blind | `problem` et erreurs de liste en français dans l'interface anglaise | low | reject | Même règle que tous les messages du noyau déjà affichés ; aucun nouveau défaut de clés FR/EN de l'extension |
| 12 | blind | Le modèle qui a composé un plan n'est pas journalisé | false | reject | Le journal porte les commandes exécutées et le rejeu les rejoue telles quelles, sans le modèle ; `plan.propose` n'est pas journalisé |
| 13 | blind | Le critère `Timings.load` contredit par les notes | — | reject | Correctif = modifier le plan de ce build ; signalé au propriétaire à la présentation |
| 14 | blind | Le test « rien au journal » ne prouve rien si le fichier n'est jamais créé | low | reject | `agent_models_lists_the_composer_models_without_journal_or_pending_change` (noyau) le prouve sur le journal en mémoire |
| 15 | blind | Un choix mémorisé est oublié sur un état passager du dossier | false | reject | L'intention l'exige : « Un choix mémorisé absent de la liste reçue est oublié » |
| 16 | blind | Doc d'`agent.models` incomplète (params, redemande, taille 0) | low | reject | Cosmétique ; contrat principal documenté |
| 17 | edge | `models.unknown` répète un chemin absolu fourni par l'appelant | low | reject | Le texte vient de l'appelant, pas du noyau ; l'extension n'envoie que des noms de la liste |
| 18 | edge | Noyau ancien : `-32601` affiché « Méthode inconnue. (core.unknown) » | low | reject | Message lisible ; versions couplées par le paquet Cassini |
| 19 | edge | `size_text` : 999 999 octets → « 1000,0 ko » | low | reject | Les modèles pèsent des Go ; rare |
| 20 | edge | `MODELS_FAILED` après une liste réussie grise une liste encore valable | low | reject | Rare (dossier qui disparaît en cours de session) ; le message d'erreur dit pourquoi |
| 21 | triage | Le correctif n° 1 rendait seul un adaptateur LoRA nommé par une déclaration à problème, pris alors par défaut | medium | patch | Constaté sur le test de bout en bout, dont `lora.gguf` avait été retiré ; règle : un GGUF nommé `adapter` par une déclaration lisible reste caché ; `lora.gguf` remis dans le test |

## Design Notes

Moment de la liste : à `READY` puis à chaque ouverture des Réglages, si le client est libre (une seule requête en vol) ; sinon la dernière liste reste affichée. Libellé : `Qwen3.5-9B-Q4_K_M.gguf (5,6 Go)` ; une déclaration s'affiche par son nom `ajuste.modele.json`.

## Verification

**Commands:**
- `cargo test --workspace` -- expected: succès
- `cargo clippy --all-targets -- -D warnings` et `cargo fmt --check` -- expected: aucun avertissement
- `python -I -m unittest discover -s qgis-plugin/tests` et `python -I qgis-plugin/tests/check_extension.py` -- expected: succès, code 0

**Manual checks (if no CLI):**
- QGIS 3.44, deux modèles : choisir le second, demande composée, réouverture de QGIS, choix gardé (recette du propriétaire, hitl).
