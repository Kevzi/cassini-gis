---
title: "Noms d'outils MCP sans point, acceptés par tous les clients"
type: 'feature'
ticket: '30'
created: '2026-10-31'
status: 'built'
baseline_revision: 'c25de65270beb891d6acb0fc63bdd29b45df3645'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/docs/canal-local.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `cassini mcp` expose les identifiants du registre tels quels (`analysis.buffer`) ; Antigravity CLI, successeur de Gemini CLI, refuse tout nom hors `^[a-zA-Z0-9_-]{1,64}$` et écarte les 11 outils de Cassini, si bien que la recette Gemini de 5.27 reste inachevée.

**Approach:** Le serveur MCP expose chaque commande sous un nom sans point (`analysis_buffer`), le même pour tous les clients et dans les deux modes, et ramène ce nom à l'identifiant du registre à l'appel ; l'ancien nom à point reste accepté en appel sans jamais figurer au catalogue.

## Boundaries & Constraints

**Always:**
- Un seul nom par commande, pour tous les clients (AD-29) ; même règle en relais et en autonome.
- Le registre, le canal d'IA externe (`command.run`), le journal et le rejeu gardent l'identifiant à point.
- Nom ≤ 52 caractères (64 moins le préfixe `mcp_cassini_` d'Antigravity), motif `^[a-zA-Z0-9_-]+$`.
- `docs/canal-local.md` (relais vu de `cassini mcp`) et le README (fiche Antigravity CLI, noms cités dans la recette) suivent.

**Never:**
- Aucune ligne propre à un fournisseur, aucune option de nommage, aucun argument propre à un client.
- Aucun changement du registre, du contrat des identifiants (`core/src/contract.rs`), du protocole du canal ni de l'extension QGIS.
- Aucune dépendance ajoutée.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Catalogue | `tools/list`, relais ou autonome | noms `domaine_verbe`, aucun point | — |
| Appel par nom sans point | `tools/call analysis_buffer` | `command.run` / registre reçoit `analysis.buffer` ; journal sous `analysis.buffer` | — |
| Appel par ancien nom | `tools/call system.about` | exécuté comme `system_about` | — |
| Nom inconnu | `tools/call foo_bar` ou `foo.bar` | erreur de protocole `-32602`, comme aujourd'hui | message nomme le nom reçu |
| Collision dans un catalogue de session | deux `id` disponibles donnant le même nom | seul le premier est exposé | — (impossible avec `Registry::standard()`, garanti par test) |

</frozen-after-approval>

## Code Map

- `cli/src/mcp.rs:55-93` -- `catalogue` et `catalogue_from_session` : seuls endroits où les `Tool` sont nommés ; y appliquer une fonction unique `tool_name(id)` (`.` → `_`).
- `cli/src/mcp.rs:208-235` -- `decide` rend `(Mode, Vec<Tool>)` ; il doit aussi porter, pour chaque outil, son identifiant du registre (par exemple une liste d'entrées `{id, tool}` ou deux vecteurs parallèles) ; `fingerprint` et `list_tools` ne gardent que les `Tool`.
- `cli/src/mcp.rs:259-290` -- `call` : résoudre `request.name` en identifiant (égal au nom exposé, ou égal à l'identifiant lui-même pour l'ancien nom) ; le reste (`call_autonomous`, `call_relay`) reçoit l'identifiant à point, inchangé. Message « Outil inconnu » gardé.
- `cli/src/mcp.rs:520-620` -- tests unitaires du catalogue : comparent `tool.name` aux `spec.id` et appellent `registry.spec(&tool.name)` ; à adapter au nom sans point.
- `core/src/contract.rs:226-238` -- `is_valid_id` : un identifiant a exactement un point et deux segments snake_case ; un nom sans point n'est donc jamais un identifiant. Lecture seule.
- `cli/tests/mcp.rs` -- tests d'intégration : `tool["name"] == spec.id` (l. 46, 59), `call_tool("system.about", …)` (l. 73-147), `tool_names`/`core_names` (l. 372-390), relais `analysis.buffer`/`layer.describe` (l. 201, 396-465) : passer aux noms sans point, garder un appel par l'ancien nom.
- `docs/canal-local.md:464-497` -- « nom `id` » et ligne `tools/call` de la table de traduction.
- `README.md:369-450` -- section « Brancher un client MCP » : ajouter la fiche Antigravity CLI (`~/.gemini/config/mcp_config.json`, autorisation `mcp(cassini/*)` dans `~/.gemini/antigravity-cli/settings.json`, constat de 5.27) ; noms de la recette (étape 2) et des modes (`layer_describe`, `analysis_buffer`, `system_about`, `model_list`).
- `qgis-plugin/tests/probe_external.py` -- sonde de 5.27, réutilisée telle quelle pour la recette.

## Tasks & Acceptance

**Execution:**
- [x] `cli/src/mcp.rs` -- `tool_name`, catalogues nommés sans point, résolution nom → identifiant dans `call`, collision ignorée au-delà du premier ; tests unitaires : motif et longueur ≤ 52 pour tout `Registry::standard()` (deux modes), injectivité sur tous les identifiants du registre, résolution des deux noms, collision de session -- cœur du ticket.
- [x] `cli/tests/mcp.rs` -- adapter les noms ; ajouter : autonome avec `--project`, appel `system_about` journalisé sous `system.about` ; relais, `analysis_buffer` arrive à la session comme `command.run analysis.buffer` ; ancien nom `system.about` exécuté et absent de `tools/list` -- preuve par processus réel.
- [x] `docs/canal-local.md`, `README.md` -- règle de nommage, fiche Antigravity CLI, noms cités -- documentation.
- [ ] Installation puis recette -- noyau `--features gpu` et extension installés à jour ; `probe_external.py` avec Antigravity CLI (`agy`), puis Claude Code -- résultats dans Implementation Notes.

**Acceptance Criteria:**
- Given un client MCP en autonome ou en relais, when il lit `tools/list`, then chaque nom respecte `^[a-zA-Z0-9_-]+$`, tient en 52 caractères, et aucun ne contient de point.
- Given la sonde lancée et Antigravity CLI branché par sa fiche, when la phrase de démo lui est donnée, then la sonde imprime des appels réussis, une couche ajoutée, un PDF lisible et un rejeu `identical: true`.
- Given Claude Code branché comme en 5.27, when la même recette est rejouée, then elle reste verte.
- Given les changements, when `cargo fmt --check`, `cargo clippy` et `cargo test` tournent, then ils passent sans avertissement.

## Implementation Notes

- `cli/src/mcp.rs` : `tool_name(id)` (`.` → `_`) ; les deux catalogues rendent des `Exposed {id, tool}`, filtrés par `first_per_name` (collision : la première entrée seule) ; `resolve(served, name)` cherche d'abord parmi les noms servis, puis parmi les `id` (ancien nom à point). `call_autonomous` et `call_relay` reçoivent l'`id` à point ; `fingerprint` et `list_tools` ne voient que les `Tool`. Message « Outil inconnu » inchangé, il nomme le nom reçu.
- Tests unitaires ajoutés : `every_served_name_fits_every_client` (motif, ≤ 52, deux modes sur tout `Registry::standard()`), `no_two_registry_ids_share_a_name`, `both_names_resolve_to_the_registry_id`, `a_session_name_collision_keeps_the_first_entry`. Tests d'intégration : noms sans point partout, `the_dotted_name_still_runs_but_is_never_listed` (autonome `--project`, journal sous `system.about` pour les deux formes), relais `analysis_buffer` → ordre `analysis.buffer`, inconnus `foo_bar`/`foo.bar`/`session_hello` en `-32602`. `cli/tests/parity.rs` appelle toujours par l'`id` à point : il couvre l'ancien nom, laissé tel quel.
- Vérification (Windows, `CARGO_TARGET_DIR=C:\ct`, `LIBCLANG_PATH` d'outils-cassini) : `cargo fmt --check` propre, `cargo clippy --workspace --all-targets -- -D warnings` propre, `cargo test -p cassini` vert (47 unitaires, 16 MCP, parité, bout en bout). Sous WSL, clippy échoue sur GTK de `/app` : sans rapport.
- Installation (2026-10-31, 16 h 50) : `cargo build --release -p cassini --features gpu`, `cassini.exe` copié dans `%LOCALAPPDATA%\Cassini\bin` (ancien gardé en `cassini.exe.avant-5-30`). Trois `cassini mcp` de sessions Claude Code tenaient l'exe : il a été renommé `cassini.exe.enservice-5-30` (copie identique de `avant-5-30`, à supprimer une fois ces processus finis), sans tuer les processus. DLL identiques, non recopiées. Extension resynchronisée. `model.list` : trois modèles, `cassini/model=cassini-composition-4b.modele.json` présent. `tools/list` du binaire installé : `model_list`, `system_about`.
- **Antigravity CLI (`agy`, Gemini 3.8 Flash, `agy -p`, dossier neutre, 16 h 55 et 16 h 58)** : le défaut de 5.27 est levé. Plus aucun « violates » au `cli.log`, et les 11 outils du relais sont chargés (`~/.gemini/antigravity-cli/mcp/cassini/analysis_buffer.json`, …, `system_about.json`, commandes QGIS comprises). Mais **aucun appel** : aux deux essais (phrase seule, puis précédée de « Dans mon projet QGIS ouvert, avec les outils Cassini : »), le modèle commence par un `RunCommand`. Le mode `-p` refuse ce geste sans demander, et le tour s'arrête. Sonde : verdict échec (aucun appel, ni couche, ni PDF). Deux essais : arrêt. Le critère « Antigravity » reste ouvert. Pistes pour le propriétaire : la session interactive d'`agy` (refuser la commande, le modèle se rabat sur les outils), ou une règle `command(...)` étroite. `--dangerously-skip-permissions` n'a pas été employé.
- **Claude Code (`claude -p`, dossier neutre, `--mcp-config` d'un fichier temporaire + `--strict-mcp-config`, `--allowedTools mcp__cassini__*`, 17 h 05)** : **réussite**. Cassini n'était plus déclaré dans la configuration Windows de Claude Code. Le premier essai, sans `--mcp-config`, n'a vu aucun outil. 7 appels, tous réussis : `project.inventory`, `analysis.select_by_attribute` ×2, `analysis.buffer`, `analysis.intersect` ×2, `layout.export_pdf`. Cinq couches ajoutées, PDF lisible, journal `origin: external` sous les `id` à point. Le rejeu rend `identical: true` (9 étapes).

## Plan Change Log

## Review Triage Log

Relecture `quick`, 2026-10-31. Comptes : high 0, medium 2, low 4, false 0 ; rejetés 2.

| # | Constat | Verdict | Preuve | Suite |
|---|---|---|---|---|
| 1 | Critère Antigravity (appels, couche, PDF, rejeu) non tenu | medium | Implementation Notes : outils chargés, aucun appel, `RunCommand` refusé en `-p` ; pas un défaut du code, la suite est une recette à rejouer autrement | décision du propriétaire (voir présentation) |
| 2 | Recette Claude Code jouée par `--mcp-config` et non par la fiche | low | la fiche `claude mcp add` écrit la même entrée ; la différence tient à la configuration du client, pas à Cassini ; correctif = rejouer, pas de code | rejeté |
| 3 | Fiche Antigravity : « la voie de la famille Gemini » non prouvée, constat `RunCommand` absent | medium | `README.md`, bloc Antigravity, contre Implementation Notes | patch |
| 4 | Relecture `quick` au lieu de `thorough` (CLAUDE.md §8) | low | §8 vise les entrées de risque `high` ; 5.30 est `medium`, le protocole du canal ne change pas | rejeté |
| 5 | « un `id` a exactement un point » énoncé sans condition pour le catalogue de session | low | `catalogue_from_session` ne revérifie pas les `id` ; la session les tire du registre (`is_valid_id`) | patch (formulation) |
| 6 | Ligne de 107 caractères dans `docs/canal-local.md` | low | puce « Relais », l. 470 | patch |

## Design Notes

Le remplacement `.` → `_` n'est pas réversible en général (`a_b.c` et `a.b_c`), d'où la table identifiant ↔ nom tenue avec le catalogue servi plutôt qu'une fonction inverse. Comme tout identifiant contient exactement un point, un nom exposé n'est jamais un identifiant : chercher d'abord parmi les noms exposés, puis parmi les identifiants, ne peut pas se tromper de commande.

## Verification

**Commands:**
- `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` -- expected: propre
- `cargo test -p cassini` -- expected: vert, dont les nouveaux tests du catalogue
- `python-qgis-ltr.bat qgis-plugin/tests/probe_external.py <projet> antigravity` -- expected: code 0, rejeu identique
