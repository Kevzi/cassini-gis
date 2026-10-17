---
title: 'Executer un plan valide etape par etape par le canal'
type: 'feature'
ticket: '4'
created: '2026-10-17'
status: 'built'
baseline_revision: '9846ce9ca22927be472b6e99f68afe4198d848e1'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md', '{project-root}/_bmad-output/initiative-cassini-mvp/ux-panneau-cassini/EXPERIENCE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `compose_plan` (5.3) rend un plan valide, mais le canal ne sait ni le proposer a l'extension, ni le faire valider, ni l'executer : l'agent n'a aucun chemin jusqu'a QGIS.

**Approach:** Trois methodes sur le canal. `plan.propose {request}` : le noyau demande l'inventaire a l'extension, compose par un compositeur injecte, verifie, garde le plan en attente et le rend avec un `plan_id`. `plan.execute {plan_id}` (la validation de l'utilisateur) : le noyau execute le plan garde, une etape a la fois par `Session::call`, en resolvant les `step:k` depuis les resultats deja obtenus. `plan.reject {plan_id}` l'oublie.

## Boundaries & Constraints

**Always:** Le noyau garde le plan ; l'extension ne renvoie jamais un plan, seulement son `plan_id` (le plan montre est le plan execute, AD-23). Un plan en attente a la fois ; une nouvelle proposition remplace l'ancienne ; `execute` le consomme. Avant d'executer, `check_plan` et la disponibilite (AD-7) sont reverifies ; un plan qui nomme une commande indisponible est refuse a la proposition, jamais repare. Chaque etape passe par `Session::call` (journal, chemins, annulation : AD-10, AD-11, AD-12) ; le journal garde les valeurs resolues, pas `step:k`, pour que le rejeu lie les `layer_id` comme aujourd'hui. Premiere erreur ou annulation : les etapes suivantes ne partent pas, l'erreur porte `step` (numero dans le plan). `request.cancel` sur l'id de `plan.execute` annule l'etape en vol (`registry.cancelled`), avec les regles de 2.11 (non annulable = refus). Une notification `plan.step {plan_id, step, command}` part avant chaque etape. `core` ne depend pas de `ai` : le compositeur est un trait de `core`, implemente dans `cli` par `cassini_ai::plan::compose_plan`. Sans compositeur ou sans modele, `plan.propose` rend une erreur typee, le reste du canal ne change pas. Aucun chemin absolu dans un message rendu. Aucune nouvelle dependance.

**Never:** Variante `question` (5.5), panneau Python (5.6), serveur MCP (5.7). Changer `compose_plan`, `check_plan` ou le format des plans. Journaliser l'inventaire demande pour composer. Rendre annulable la composition elle-meme. Enchainer depuis l'extension.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Plan de trois etapes | propose puis execute, ordres repondus | trois ordres dans l'ordre, chacun apres la reponse du precedent, `step:1.layer_id` remplace par l'id rendu ; trois lignes au journal | — |
| Etape en erreur | etape 2 repond une erreur | erreur au `step: 2`, etape 3 jamais envoyee, deux lignes au journal | code de l'extension garde |
| Annulation | `request.cancel` sur l'id d'execute pendant l'etape 2 annulable | `registry.cancelled`, `step: 2`, etape 3 jamais envoyee, ligne `cancelled` | sorties partielles nettoyees (2.11) |
| plan_id inconnu, deja execute ou rejete | execute / reject | rien n'est execute | `plan.unknown` |
| Pas de compositeur / modele | propose | rien n'est garde | `agent.unavailable` / `ai.*`, sans chemin absolu |
| Refus, plan invalide | compositeur rend l'erreur | rien n'est garde | `agent.refused` / `agent.plan_invalid` |
| Commande indisponible | plan valide nommant une commande non annoncee | rien n'est garde | `agent.plan_invalid`, l'etape nommee |
| Pendant une etape | propose / execute / run / hello | refuses | `session.busy` |

</frozen-after-approval>

## Code Map

- `core/src/serve.rs` -- `dispatch` (ajouter les trois methodes), `handle_line` (`is_run` doit couvrir `plan.execute` pour que `run_id` vise sa requete), `Channel::answer_during_order` (busy pour les methodes `plan.*`), `request_cancel` (reutilise tel quel, il lit `flight.command` de l'etape en vol), `is_available`, `State` (ajouter plan en attente, compteur `plan-<n>`, compositeur). Inventaire : `Channel{..}.execute("project.inventory", json!({}))` directement, hors `Session` donc hors journal ; non annonce -> erreur de la commande. 1416 lignes : un sous-module (`core/src/serve/` ou `core/src/plan_run.rs`) est permis.
- `core/src/plan.rs` -- `Plan`, `PlannedCall`, `parse_reference` (prive) : ajouter une fonction publique de resolution des references d'un `params` contre les sorties deja obtenues (`step:k` -> sortie entiere, `step:k.champ` -> champ) ; ne pas toucher `check_plan`.
- `core/src/journal/mod.rs` -- `Session::call(id, params) -> Result<Value, ErrorReport>` ; journalise `cancelled`, nettoie les sorties. Lecture seule.
- `core/src/error.rs` -- `ErrorReport::at_step`.
- `cli/src/main.rs` -- `serve()` appelle `cassini_core::serve::serve` ; y brancher un compositeur base sur `default_model` (l.257) et `cassini_ai::plan::{model_generator, compose_plan}`, modele cherche a chaque proposition. `default_model` met le dossier absolu dans son message : pour le canal, un message sans chemin.
- `core/src/serve.rs` tests (l.655+) -- `Client` interactif (l.1036), `scripted_registry` (l.1018), `reply_ok` : base des nouveaux tests ; un compositeur de test rend un plan fixe.
- `cli/tests/{parity,end_to_end}.rs` -- compteurs en dur : verifier s'ils comptent les methodes.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/plan.rs` -- resolution publique des references + tests (sortie entiere, champ, champ absent a l'execution = erreur typee, non-reference intacte)
- [x] `core/src/serve.rs` (ou sous-module) -- trait compositeur, `serve_with_composer` (garder `serve`), `plan.propose` / `plan.execute` / `plan.reject`, notification `plan.step`, busy et annulation etendus
- [x] `core/src/serve.rs` tests -- toutes les lignes de la matrice par le `Client` interactif
- [x] `cli/src/main.rs` -- compositeur reel branche sur `serve` ; test CLI : `plan.propose` sans modele rend une erreur typee sans chemin absolu
- [x] `docs/` ou doc du module -- decrire les trois methodes et la notification (protocole lu par 5.6)

**Acceptance Criteria:**
- Given un plan de trois etapes propose et execute, when l'extension repond a chaque ordre, then chaque ordre ne part qu'apres la reponse au precedent, et `journal.replay` rend `identical: true`.
- Given un plan execute, when on rappelle `plan.execute` avec le meme `plan_id`, then `plan.unknown` et rien n'est envoye.
- Given le binaire sans modele, when `plan.propose`, then une erreur typee, et `command.run` marche toujours dans la meme session.

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment ; CLAUDE.md §8 l'emporte sur le `quick` epingle). 0 high, 1 medium (non verifie), 8 low corriges, reste rejete.

| # | Verdict | Route | Evidence |
|---|---------|-------|----------|
| 1 | low | patch | `propose` effacait le plan en attente apres la lecture de `request` : une proposition malformee le laissait. Efface en premier ; test `a_failed_proposal_still_erases_the_pending_plan`. |
| 2 | low | patch | `relative_to_work` doublait `journal::paths::relativize`. Supprime, `relativize` reutilise (pub(crate)). |
| 3 | low | patch | Aucun test canal ne prouvait la relativisation d'un chemin de sortie. Test `a_plan_result_makes_an_output_path_relative_to_the_work_folder`. |
| 4 | low | patch | `CASSINI_MODELS` relatif remplace comme sous-chaine dans les messages. Masque seulement s'il est absolu. |
| 5 | low | patch | Masquage du chemin du modele et garde phrase vide jamais exerces sur le canal. Tests bout en bout : faux `x.gguf`, `request` blanc -> `ai.empty_prompt` sans ordre. |
| 6 | low | patch | Doc : `registry.unavailable` (propose), `session.io`, `ai.empty_prompt`, sens de `plan.step`. Ajoutes. |
| 7 | maybe-false | defer | Un message de llama.cpp qui ecrirait le chemin du modele autrement (forme canonique, autres separateurs) echapperait au masquage ; a regler par un essai sur un GGUF corrompu sous Windows. Medium si vrai. |
| 8 | low | rejete | Annulation et `session.busy` pendant une etape executee dans le noyau : rien ne lit le canal, la requete attend l'etape suivante ou la fin du plan. Les commandes noyau actuelles sont instantanees et l'extension n'envoie rien pendant un plan (AD-5) ; drainer entre les etapes ajoute une logique. A reprendre a l'epic 6 (terrain, etapes noyau longues). |
| 9 | false | rejete | Second `session.hello` qui garderait le plan : refuse `session.already_started`. |
| 10 | false | rejete | Re-verification a l'execution qui consommerait le plan : annonces et registre figes pour la session, elle ne peut pas echouer aujourd'hui. |
| 11 | low | rejete | Plan vide stocke : la grammaire impose au moins une etape ; seul un compositeur de test peut le rendre. |
| 12 | false | rejete | Erreurs d'etape avec chemin absolu : meme chemin que `command.run`, deja teste sans chemin absolu (2.11). |
| 13 | low | rejete | `plan.execute` en notification s'execute sans reponse : meme comportement que `command.run`, l'extension envoie des requetes. |
| 14 | low | rejete | `ModelComposer` ne reutilise pas `default_model` : le message de `default_model` porte le dossier absolu ; la reutiliser demande de refondre la fonction. |
| 15 | low | rejete | Resultats partiels absents de l'erreur : les couches sont dans QGIS, les etapes au journal ; rien dans l'intention ne le demande. |
| 16 | low | rejete | Erreur de reference a l'execution non testee par le canal : `check_plan` verifie le champ contre le schema, seul un executant hors schema la declenche ; teste sur `resolve_references`. |
| 17 | false | rejete | Branche `Refusal` d'`execute` et `expect` de session : inatteignables (un refus n'est jamais garde ; `dispatch` verifie la session). |
| 18 | low | rejete | `assert!` du compositeur de test dans le fil du noyau : test seulement. |
| 19 | false | rejete | Le compositeur choisit quand demander l'inventaire : le seul compositeur reel le demande toujours apres avoir trouve le modele ; aucun dommage nomme. |
| 20 | false | rejete | References imbriquees laissees litterales : coherent avec `check_plan`, premier niveau seulement. |
| 21 | low | rejete | Plan propose renvoye tel quel avec d'eventuels chemins du modele : deja reporte (5.3, finding 3, AD-23 chemins connus). |
| 22 | low | rejete | Ligne « modele » de la matrice ne cite pas `models.*` : la correction edite le plan fige. |

## Design Notes

Decisions prises sans redemander (consigne du proprietaire : trancher) : l'inventaire vient de `project.inventory` demande par le noyau (AD-5 : l'extension transmet la demande, elle ne prepare rien) ; `plan.execute` est la validation, pas de methode `plan.validate` separee ; la composition bloque la boucle mais pas la lecture (le fil lecteur continue, les requetes recues attendent la fin), donc non annulable — a reporter dans `deferred-work.md`.

Resultat de `plan.execute` : `{"plan_id", "steps": [{"step", "command", "result"}]}`. Resultat de `plan.propose` : `{"plan_id", "plan"}`.

## Verification

**Commands:**
- `cargo test --workspace` -- expected: tout passe
- `cargo clippy --all-targets -- -D warnings` -- expected: aucun avertissement
- `cargo fmt --check` et `cargo deny check` -- expected: succes
