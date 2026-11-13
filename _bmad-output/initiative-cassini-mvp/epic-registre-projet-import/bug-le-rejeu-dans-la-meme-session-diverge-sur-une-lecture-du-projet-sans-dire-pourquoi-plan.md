---
title: 'Rejeu en même session : une lecture du projet diverge sans dire pourquoi'
type: 'bugfix'
ticket: '12'
created: '2026-11-10'
status: 'built'
baseline_revision: '962a1d4cabdfbd0147c2608c5c364883dbdfcf30'
route: 'full'
route_source: 'pinned'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** la recette d'Annecy (5.45) rejoue son journal en fin de session ; `project.inventory`, première ligne, voit les couches posées, rangées ou retirées depuis, et le rejeu s'arrête à l'étape 1 sur `replay.diverged` avec un motif muet. Le rejeu identique promis (R5) part de l'état d'ouverture du projet.

**Approach:** décision tranchée en party le 2026-11-10, à confirmer par le propriétaire : la lecture reste journalisée, rejouée et comparée. Le noyau dit la cause et le remède quand le résultat rejoué d'une ligne divergente nomme une couche posée par une exécution précédente ; la sonde de recette rejoue dans la session (indicatif), puis rouvre la copie du projet sans l'enregistrer et rejoue (compté).

## Boundaries & Constraints

**Always:** code `replay.diverged` et étape inchangés, seul le motif change ; une vraie divergence garde le motif ordinaire ; tests de rejeu existants verts ; noyau sans dépendance nouvelle ; `cargo fmt`, `cargo clippy -D warnings`.

**Never:** ne pas comparer ou ne pas journaliser une lecture ; nouveau paramètre au catalogue, nouveau code d'erreur, changement du protocole ; toucher au modèle ou au générateur ; installer un noyau.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Session neuve | journal inventaire, couche, tampon, inventaire ; QGIS neuf | `identical: true` (comme aujourd'hui) | — |
| Même session | même journal ; le projet porte encore les couches de l'origine | `replay.diverged` étape 1, motif : la couche « couche_g1_… » posée par une exécution précédente, rouvrir sans enregistrer | motif nommé |
| Couche d'un rejeu antérieur | id d'un rejeu précédent (`replayed_layers`) vu par la lecture | même motif nommé | motif nommé |
| Vraie divergence | session neuve, autre nombre d'objets ou couche sans équivalent | motif ordinaire, sans le remède | motif ordinaire |

</frozen-after-approval>

## Code Map

- `core/src/journal/mod.rs` `Replay::run` (l. ~1267-1430) -- `leave_out` (ids du journal et des rejeux antérieurs, figé avant l'étape 1), `outcome` brut avant `relativize`/`substitute(back)`, closure `diverged`, motif « la commande {id} ne redonne pas le même résultat ou la même erreur ». Chercher l'id dans le résultat **brut** : après la substitution inverse, les ids de ce rejeu redeviennent ceux du journal.
- `core/src/journal/mod.rs` `JournalError::Diverged { step, why }` -- message « Le rejeu diverge du journal à l'étape {step} : {why} » ; à réutiliser tel quel.
- `core/src/journal/tests.rs` `SessionQgis` (l. 525), `inventory_journal`, `replay_inventory` (l. ~656-700) -- modèle ; préremplir `created` d'une génération 2 avec les couches g1 simule la même session.
- `qgis-plugin/tests/probe_recette.py` `next_phrase`, `on_event` (`REPLAY_DONE`, `REPLAY_FAILED`), code de sortie -- deux rejeux ; `project.clear()` puis `project.read(PROJECT)` entre les deux (la copie n'est jamais enregistrée).
- `docs/entrainement-du-modele.md` l. ~665-672 -- RA13 et rejeu de la mesure de référence.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/journal/mod.rs` -- sur une ligne divergente dont le résultat brut contient un id de `leave_out`, motif qui nomme cet id et dit de fermer puis rouvrir le projet sans l'enregistrer avant de rejouer -- cause et remède.
- [x] `core/src/journal/tests.rs` -- tests de la matrice (même session, rejeu antérieur, vraie divergence sans remède).
- [x] `qgis-plugin/tests/probe_recette.py` -- rejeu en session (indicatif, imprimé), réouverture de la copie, second rejeu compté ; en-tête d'usage mis à jour.
- [x] `docs/entrainement-du-modele.md` -- RA13 : règle stricte gardée (party) ; rejeu de la recette après réouverture.

**Acceptance Criteria:**
- Given la recette d'Annecy rejouée avec un noyau de la branche, when la sonde finit, then le rejeu après réouverture est `identical: true` et le score égale la mesure de référence à la variance du modèle près ; le rejeu en session imprime le motif nommé.

## Implementation Notes

- Recette réelle du 2026-11-10, QGIS 3.44.15, modèle installé (`cassini-composition-4b-Q4_K_M.gguf`, par le dossier de 5.45), noyau construit sous Windows depuis la branche (`C:\ct-rejeu\release\cassini.exe`, `--features gpu`, non installé ; première construction en échec dans cmake à 16 tâches, passée avec `NUM_JOBS=4`) : **17/20**, ratés RA07, RA09, RA13 comme la mesure de référence. Rejeu en session : `replay.diverged` à l'étape 1, motif nommé (`8_data_reproject_…`, « Rouvrez le projet… »). Rejeu après réouverture : `{"steps": 29, "identical": true}`, code de sortie 0. Sortie hors dépôt : `%TEMP%\recette-2-12\sortie.txt`.

## Plan Change Log

## Review Triage Log

Passe 1 (lentilles blind-hunter, edge-case-hunter, verification-gap, intent-alignment) : 1 medium, 6 low corrigés, 10 rejetés (false ou low hors usage courant).

| Constat | Verdict | Route | Preuve ou action |
|---|---|---|---|
| Filtre `known` : en même session, une lecture placée après les créations du journal garde le motif muet (verification-gap, blind, edge) | medium | patch | Reproduit à la main (journal layer.add, buffer, inventaire court) ; en session neuve le résultat brut ne porte jamais un id de `leave_out` : filtre retiré, test `a_read_after_the_creations_in_the_same_session_names_the_earlier_layer` (court et long). |
| Remède faux si le projet a été enregistré avec les couches (blind, edge) | low | patch | Motif récrit : « Rouvrez le projet tel qu'il était avant ces étapes, sans l'enregistrer ». |
| Tests de vraie divergence et de rejeu antérieur trop lâches (blind) | low | patch | Code, étape, « exécution précédente » et « Rouvrez » vérifiés, contexte sur chaque assert. |
| Sonde : réouverture dans le rappel du client ; « non joué » si le noyau meurt pendant le rejeu en session ; motif en session jamais contrôlé (blind, edge) | low | patch | `QTimer.singleShot` gardé par `shielded`, `in_session` = « interrompu » au lancement, ligne ANOMALIE indicative. |
| Doc : décision donnée pour définitive, ancien paragraphe contradictoire, retours à la ligne (blind) | low | patch | « à confirmer par le propriétaire », constat au passé renvoyant à 2.12, lignes à ~100 colonnes. |
| Couches rangées ou retirées : motif ordinaire (blind, intent) | low | reject | L'approche de l'intention borne le motif nommé à une couche posée par une exécution précédente ; un id disparu ne se lit pas dans un résultat résumé. |
| Une vraie divergence en session non neuve reçoit le remède (blind, edge) | low | reject | Voulu : l'état du projet suffit à expliquer l'écart ; après réouverture la vraie divergence reparaît avec le motif ordinaire. |
| Recherche dans toutes les chaînes du résultat (blind) | low | reject | Ids de session à suffixe UUID : collision avec un nom ou une valeur hors usage réel. |
| Erreur (Err) causée par une couche ancienne (edge) | false | reject | Aucune commande ne rend une erreur qui nomme un id ; les couches se désignent par id, pas par nom. |
| Ordre de la couche nommée, une seule nommée (edge) | false | reject | `layers` est un tableau parcouru dans l'ordre ; nommer une couche suffit au remède. |
| `replayed_layers` perdu entre deux processus du noyau (edge) | low | reject | Antérieur au changement (5.36) ; la réouverture du projet efface ces couches. |
| Cache d'inventaire du noyau après réouverture (edge) | false | reject | Le rejeu lit l'inventaire en direct (`read_inventory(router)`), sans cache. |
| Délai de 90 min pour deux rejeux (blind) | false | reject | Un rejeu dure quelques minutes ; un délai dépassé rend le code 1, comme avant. |
| Chemins de poste dans la Vérification du plan (blind) | low | reject | Correctif = éditer le plan de ce build, exclu par la règle de tri. |
| Plan incomplet, `ticket: '12'` (blind) | false | reject | Plan relu en cours de relecture ; `ticket` porte l'id de l'entrée, comme le gabarit le veut. |
| Le remède n'atteint pas l'utilisateur (intent D) | false | reject | Le panneau affiche le `message` du noyau (`thread.replay_outcome` puis `replay_text`). |
| La sonde n'a pas de test automatique (verification-gap, intent) | low | reject | Sonde manuelle par nature ; recette réelle jouée après la relecture (Implementation Notes). |

## Verification

**Commands:**
- Tout se fait dans le worktree `/home/kevin/cassini-worktrees/rejeu-lectures` (branche `story/rejeu-des-lectures`), jamais dans le dépôt principal. Sous WSL, avant `cargo` : `export LIBCLANG_PATH=/tmp/claude-1000/-mnt-c-Users-kevin-Downloads-Projet-code-CASSINI/38f6e1c7-3a62-4e6d-a1e2-bc44d994e1ac/scratchpad/clang/x/libclang-18.1.1.data/platlib/clang/native BINDGEN_EXTRA_CLANG_ARGS=-I/usr/lib/gcc/x86_64-linux-gnu/13/include`. Ne rien commiter : l'orchestrateur relit puis commite.
- `cargo test -p cassini-core` -- expected: vert
- `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` -- expected: sans avertissement
- `python3 -I -m unittest discover -s qgis-plugin/tests` et `python3 -I qgis-plugin/tests/check_extension.py` -- expected: OK, code 0

**Manual checks (if no CLI):**
- sonde `probe_recette.py` sous `python-qgis-ltr.bat`, noyau de la branche (`CASSINI_CORE`), `CASSINI_MODELS` = dossier du seul modèle installé -- score et deux verdicts de rejeu.
