---
title: "Rejeu d'un inventaire demandé après la création de couches"
type: 'bugfix'
ticket: '31'
created: '2026-10-31'
status: 'done'
baseline_revision: 'b53880531b6e9bd65cf3e12970739df14834bf02'
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

**Problem:** Au rejeu, seul le `layer_id` de premier niveau d'un résultat est écarté de la comparaison. Un `project.inventory` demandé après la création de couches liste leurs `layer_id` (suffixe aléatoire, différent à chaque exécution) dans `layers[]` : le rejeu rend `replay.diverged`, comme à l'étape 13 du journal Antigravity de 5.30.

**Approach:** Avant de comparer un résultat rejoué à celui du journal, le noyau remplace, à toutes les profondeurs, chaque identifiant de couche obtenu par ce rejeu par son équivalent d'origine, d'après la correspondance que le rejeu tient déjà (`bound`) ; les chemins sous `replay/` sont déjà ramenés à leur nom par `relativize`, à toutes les profondeurs. Le format du journal ne change pas.

## Boundaries & Constraints

**Always:**
- Les journaux déjà écrits restent rejouables tels quels, ceux de 5.30 compris.
- Remplacement d'une chaîne entière égale à un identifiant lié, jamais d'une sous-chaîne (même règle que `substitute`).
- Une couche sans équivalent (ouverte à la main entre deux appels) reste un écart réel : le rejeu diverge toujours.
- Un résultat réellement différent diverge toujours, qu'il soit gardé tel quel ou résumé par empreinte.

**Never:**
- Aucun changement du format du journal, du canal, de l'extension QGIS ni du registre.
- Aucune clé ignorée en plus (pas de liste de champs « volatils ») : seule la correspondance du rejeu sert.
- Aucune dépendance ajoutée.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Inventaire après création | journal : inventaire, tampon, inventaire (résultat résumé par empreinte) | `identical: true` | — |
| Inventaire court | même suite, résultat ≤ 200 octets gardé tel quel | `identical: true` | — |
| Couche sans équivalent | l'inventaire rejoué contient une couche absente à l'origine | `replay.diverged` à cette étape | message nommant `project.inventory` |
| Résultat différent | inventaire rejoué avec un autre `feature_count` | `replay.diverged`, empreinte ou non | idem |

</frozen-after-approval>

## Code Map

- `core/src/journal/mod.rs:634-728` -- `Replay::run` : `bound` (ancien id → nouveau) est rempli après chaque étape qui rend un `layer_id` ; la comparaison est `same_result(&relativize(value, &self.replay_dir), kept)` l. 672. Y appliquer, avant `same_result`, la correspondance inverse (nouveau → ancien) au résultat relativisé.
- `core/src/journal/mod.rs:731-744` -- `substitute(value, map)` : remplace les chaînes entières à toutes les profondeurs ; à réutiliser avec la table inverse, sans nouvelle fonction de parcours.
- `core/src/journal/summary.rs` -- `same_result`, `summarize`, `without_layer_id` : inchangés. L'empreinte d'origine porte les ids d'origine imbriqués ; une fois ramenés, taille et empreinte coïncident.
- `core/src/journal/paths.rs` -- `relativize` traite déjà toute chaîne absolue sous le dossier de sortie, à toutes les profondeurs ; lecture seule.
- `qgis-plugin/cassini_qgis/inventory.py:224-241` -- l'inventaire rend `source` en nom de fichier seul (`source_name`) : aucun chemin sous `replay/` n'y apparaît ; lecture seule.
- `core/src/journal/tests.rs:522-587` -- `SessionQgis` (exécutant factice, ids `couche_g{génération}_{n}`) et `qgis_journal` : à étendre d'un `project.inventory` qui liste les couches créées par la session.
- `../donnees-test-cassini/projet-sonde-5-27-antigravity.cassini/journal.jsonl` (19 étapes, inventaires aux étapes 2, 13, 18) et `projet-sonde-5-27-claude-code.cassini/journal.jsonl` -- journaux de la recette, à rejouer sans les réécrire.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/journal/mod.rs` -- dans `Replay::run`, construire la table inverse de `bound` et l'appliquer par `substitute` au résultat relativisé avant `same_result` ; mettre à jour le commentaire de `bound` -- cœur du ticket.
- [x] `core/src/journal/tests.rs` -- `SessionQgis` rend un `project.inventory` (couches créées, avec `layer_id` et source sous le dossier de sortie, option de couche en plus, `feature_count` réglable, option de taille pour passer sous ou au-dessus de 200 octets) ; tests des quatre lignes de la matrice -- preuve.
- [x] Installation puis recette -- noyau `--features gpu` et extension installés à jour (mémoire « Mettre à jour QGIS et le noyau ») ; dans QGIS, rouvrir chacun des deux projets de sonde depuis sa copie et lancer `journal.replay` (même geste que `probe_external.py`, l. 248-263) -- résultats dans Implementation Notes.

**Acceptance Criteria:**
- Given le journal Antigravity de 5.30, inchangé, when le propriétaire (ou la sonde) le rejoue dans QGIS, then le rejeu rend `identical: true` sur ses 19 étapes.
- Given le journal Claude Code de 5.30, when il est rejoué, then il rend toujours `identical: true`.
- Given les changements, when `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` et `cargo test -p cassini-core` tournent, then ils passent sans avertissement.

## Implementation Notes

- `Replay::run` : avant `same_result`, la table inverse de `bound` (neuf → ancien) est appliquée par `substitute` au résultat relativisé. Commentaires de `bound` et de `substitute` mis à jour. Format du journal, canal, extension et registre inchangés.
- `SessionQgis` : suit les couches qu'il crée (`created`, source de `analysis.buffer` absolue sous le dossier de sortie), rend `project.inventory` (couche de projet stable `communes_0` + couches créées), avec `inventory_feature_count`, `short_inventory` (≤ 200 octets, gardé tel quel) et `stranger` (couche ouverte à la main après le tampon). `analysis.buffer` vérifie que la couche vient de la session courante au lieu de l'id 1 figé.
- Tests : `an_inventory_after_created_layers_replays_identically` (empreinte et court), `an_inventory_with_a_layer_without_equivalent_diverges` (étape 4, `project.inventory`, les deux tailles), `an_inventory_with_another_feature_count_diverges` (les deux tailles). Le premier échoue sur le code de `b538805` (vérifié en remettant `mod.rs` d'origine), passe avec le correctif.
- `cargo fmt --check`, `cargo clippy -p cassini-core --all-targets -- -D warnings`, `cargo test -p cassini-core` (257 + 31) : verts. Clippy du workspace entier non lancé sous WSL (libclang absent pour `cassini-ai`) ; le build `--features gpu` sous Windows passe.
- Installation : noyau `--features gpu` reconstruit (`C:\ct\release`) et copié dans `%LOCALAPPDATA%\Cassini\bin` (ancien gardé en `cassini.exe.avant-5-31` ; DLL identiques, inchangées) ; `model.list` voit les trois modèles, `cassini/model` pointe sur `cassini-composition-4b.modele.json` ; extension recopiée dans le profil.
- Recette (2026-10-31), sous le Python de QGIS 3.44.15, script de rejeu seul hors dépôt (mêmes gestes que `probe_external.py`, l. 248-263) : `projet-sonde-5-27-antigravity.qgz` → `{'steps': 20, 'identical': True}` ; `projet-sonde-5-27-claude-code.qgz` → `{'steps': 10, 'identical': True}`. Le compte dépasse de 1 les 19 et 9 attendus : à l'ouverture, la poignée de main du client (`system.about`) s'ajoute au journal avant le rejeu, qui la rejoue aussi. Les deux journaux ont été remis à l'identique depuis une copie prise avant la recette.
- **Contrôle extérieur (2026-10-31, 20 h 15)**, fait par l'orchestrateur, et non par le propriétaire (absent de son poste), indépendamment de l'implémenteur. Noyau installé identique au binaire construit après la dernière modification du code. Journaux sauvegardés, puis remis à l'identique après chaque rejeu (empreintes inchangées). Antigravity : `{'steps': 20, 'identical': True}` ; Claude Code : `{'steps': 10, 'identical': True}`. Contre-essai avec l'ancien noyau (`cassini.exe.avant-5-31`) sur le journal Antigravity : `replay.diverged` à l'étape 13 (`project.inventory`), le défaut de 5.30 reproduit.

## Plan Change Log

## Review Triage Log

Relecture `thorough`, 2026-10-31 (quatre lentilles). Comptes : high 0, medium 1, low 5, false 7 ; rejetés 12, reporté 1. Vérification complémentaire faite par l'orchestrateur : `cargo fmt --check` et `cargo clippy --workspace --all-targets -- -D warnings` sous Windows (`CARGO_TARGET_DIR=C:\ct`, `LIBCLANG_PATH` d'outils-cassini) : propres ; `cargo test -p cassini-core` : 257 + 31 verts.

| # | Constat | Verdict | Preuve | Suite |
|---|---|---|---|---|
| 1 | Table inverse : deux anciens ids liés au même neuf, choix selon l'ordre du `HashMap` | false | `bound` n'est rempli que par un `layer_id` de premier niveau ; seules les commandes qui créent une couche en rendent un (`layer.add`, `analysis.*`) ; `layer.describe` n'en rend pas (ligne 3 du journal Antigravity : pas de `layer_id`). Une même couche d'origine garde la même clé. | rejeté |
| 2 | L'id neuf de l'étape en cours, imbriqué dans son propre résultat, n'est pas ramené | false | Aucune commande du registre ne répète son propre id ailleurs qu'au premier niveau, déjà écarté par `without_layer_id` | rejeté |
| 3 | Un id neuf égal à un id d'origine non lié (couche du projet) serait réécrit | false | Les ids de QGIS portent un UUID ; collision impossible en pratique | rejeté |
| 4 | Couche créée dont l'id n'est qu'imbriqué : jamais liée | low | Aucune commande actuelle dans ce cas ; correctif = nouvelle règle de liaison | rejeté |
| 5 | Le contrôle de `analysis.buffer` du faux QGIS est affaibli | false | Une seule couche créée avant le tampon dans `qgis_journal` et `inventory_journal` : les deux formes sont aussi strictes | rejeté |
| 6 | Les deux tests de divergence passent aussi sans le correctif | low | Voulu : ils gardent contre un rejeu trop indulgent ; le test positif, lui, échoue sans le correctif | rejeté |
| 7 | Aucun test d'une correspondance inverse fausse (ids permutés) | low | La table vient de `bound`, exacte ; un ordre permuté change l'empreinte. Ajouter une option au faux QGIS dépasse une correction directe | rejeté |
| 8 | Sens « couche présente à l'origine, absente au rejeu » non testé | low | Même mécanisme que `stranger` : le contenu diffère, l'empreinte aussi ; correctif = nouvelle option de test | rejeté |
| 9 | Critère « 19 et 9 étapes, journal inchangé » : obtenu 20 et 10 | medium | L'ouverture du projet journalise la poignée de main `system.about` (comportement antérieur à 5.31) ; les 19 et 9 étapes d'origine rejouent à l'identique, journaux remis depuis copie. Le correctif reviendrait à éditer le plan. | reporté (journalisation de la poignée de main) |
| 10 | Clippy du workspace non lancé | false | Lancé sous Windows par l'orchestrateur : propre | rejeté |
| 11 | `back` reconstruit à chaque étape | low | Table de quelques entrées ; aucun effet mesurable | rejeté |
| 12 | Le remplacement touche toute chaîne, pas seulement les champs d'id | false | Seule une chaîne entière égale à un id neuf (UUID) est touchée ; elle valait l'id d'origine dans le journal | rejeté |
| 13 | Faux QGIS : source absolue, ids de même longueur, pas de `project_name`/`extent` | false | Le rejeu ramène la chaîne d'origine exacte, donc taille et empreinte identiques ; le cas réel est prouvé par la recette QGIS | rejeté |

## Design Notes

La table inverse est sûre : chaque nouvel id provient d'une étape distincte du rejeu et l'exécutant en tire un neuf à chaque fois. Le `layer_id` de premier niveau de l'étape en cours n'est pas encore lié au moment de la comparaison : il reste écarté par `without_layer_id`, comme aujourd'hui. Un id d'origine non lié (couche du projet, stable car le projet est rouvert depuis sa copie) n'est pas touché.

## Verification

**Commands:**
- `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` -- expected: propre (sous Windows, `/app` fait échouer clippy sous WSL)
- `cargo test -p cassini-core` -- expected: vert, dont les nouveaux tests du rejeu

**Manual checks (if no CLI):**
- Rejeu dans QGIS des deux journaux de sonde : `identical: true`, 19 et 9 étapes.
