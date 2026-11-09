---
title: 'Rejeu : la mise en forme topographique diverge après des couches créées'
type: 'bugfix'
ticket: ''
created: '2026-11-09'
status: 'done'
baseline_revision: '5be0591'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [adversarial, edge-cases]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** recette du 2026-11-09 sur Annecy (nouveau modèle 11.23c) : le rejeu du journal diverge à l'étape 15, `map.topographic_style` (« la commande map.topographic_style ne redonne pas le même résultat ou la même erreur »). Avant elle, la session a créé des couches (étape 2 `data.reproject`, étape 4 pente, étape 7 champ de vision, étape 14 champ de vision cumulé). Le résultat journalisé de l'étape 15 est résumé (`{"bytes": 2803, "fingerprint": "38fe5423cc1216a5", "truncated": true}`) et contient les identifiants de ces couches dans ses groupes (ex. `2_data_reproject_d4d35309…` dans « Autres couches »). Journal réel copié dans `/tmp/claude-1000/-mnt-c-Users-kevin-Downloads-Projet-code-CASSINI/29e203d1-8d13-4dd3-a117-ffb32ec569e9/scratchpad/rejeu/journal-recette-11-23c.jsonl`.

**Approach:** trouver pourquoi la comparaison échoue (empreinte d'un résultat résumé calculée avant la substitution des ids, ordre des couches ou des groupes, couches d'une exécution précédente présentes ou absentes, raster ouvert par ordre dicté dont l'id n'est pas lié, nom de couche différent…), reproduire par un test du noyau sans QGIS, et corriger la comparaison du rejeu (ou la forme du résultat) pour qu'un rejeu fidèle soit déclaré identique, sans masquer une vraie divergence.

## Boundaries & Constraints

**Always:** un vrai changement de résultat reste une divergence (le test le montre) ; les ids d'une exécution précédente restent exclus (`leave_out`) ; la correction vit dans le noyau (`core/src/journal/`), testée sans QGIS ; tests de rejeu existants verts ; sonde QGIS : rejouer le journal d'une session Annecy qui crée une couche puis applique `map.topographic_style` → `identical: true`.

**Never:** comparer moins que le résultat (pas d'ignorer `map.topographic_style` au rejeu) ; changer le protocole ; toucher au modèle ou au générateur.

</frozen-after-approval>

## Code Map

- `core/src/journal/mod.rs` l. ~1091-1250 -- `Replay::run` : `leave_out`, table `bound` (ancien → neuf, et neuf → ancien pour les résultats), `relativize`, `substitute`, `same_result` ; l. 1188 `replayed_layers`.
- résumé des résultats longs (empreinte, `truncated`) : chercher `fingerprint` dans `core/src/journal/` (et `summary.rs` côté noyau) — comparer l'empreinte d'un résultat rejoué après substitution des ids.
- `core/src/commands/map_topographic_style.rs` (ou nom voisin) et `core/src/style.rs` -- forme du résultat (groupes, couches), ordre.
- `core/src/journal/tests.rs` l. ~702 `an_inventory_after_created_layers_replays_identically` -- modèle de test à suivre.

## Tasks & Acceptance

**Execution:**
- [x] Reproduire dans un test du noyau (couche créée puis `map.topographic_style`, résultat assez long pour être résumé).
- [x] Corriger ; test qui montre qu'un vrai changement reste une divergence.
- [x] Sonde QGIS sur Annecy (copie du projet, archivée ensuite) : session qui reprojette les gares puis met les couleurs topographiques, puis rejeu → identique.

**Acceptance Criteria:**
- Given le journal de la recette du 2026-11-09, when on le rejoue dans QGIS, then le rejeu ne diverge plus à l'étape de mise en forme.
- Given le dépôt, when `cargo test --workspace`, then tout passe.

## Implementation Notes

**Cause (prouvée sous QGIS).** Ni l'empreinte ni la substitution des ids : le résultat de `map.topographic_style` suit l'ordre de la légende (`styled`, `unchanged` dans l'ordre de l'inventaire, ordre dans un groupe à forme égale). Le `layer.arrange` dicté par l'exécution d'origine remanie la légende de la session QGIS (ici « communes » remonte au-dessus de « batiments »). Un rejeu dans cette même session relit donc un inventaire aux mêmes couches mais dans un autre ordre : `styled` change d'ordre, l'empreinte aussi, et le rejeu diverge. Sonde de diagnostic sur le journal de la recette (copie) : 1er rejeu dans une session neuve `identical: true` ; 2e rejeu dans la même session, divergence à l'étape 15, `styled` relu avec `communes` en 6e place au lieu de 9e. Les couches créées ne sont pas en cause : elles sont posées en haut de la même façon à l'origine et au rejeu.

**Correction (`core/src/journal/mod.rs`).**
- À l'exécution d'une commande à `inventory_param`, la ligne garde, sous la clé réservée `replay`, `legend` : les ids de l'inventaire dans l'ordre de la légende, de haut en bas (seulement les ids, l'inventaire n'entre pas au journal ; le test existant qui interdit « inventory » dans le journal reste vert). La clé `replay` peut maintenant porter `center` et `legend`.
- Au rejeu, l'inventaire relu (sans les couches d'une exécution précédente) est remis dans cet ordre (`in_legend_order`), après substitution ancien → neuf des ids liés : les couches nommées reprennent leur ordre relatif aux places qu'elles occupent, les autres ne bougent pas. Seul l'ordre change ; une couche en plus, en moins ou d'un autre thème donne toujours un autre résultat, donc une divergence.
- Journal plus ancien sans `replay.legend` (celui de la recette) : l'ordre vient des `layer_id` des ordres dictés qui suivent la ligne (les `layer.set_style`, donnés dans l'ordre de l'inventaire) ; les couches qu'ils ne nomment pas (couches non reconnues) gardent leur place. Limite connue : pour un tel journal ancien, les couches qu'aucun `layer.set_style` ne nomme (non reconnues, ou laissées hors de `layers`) gardent leur place relue ; si le rangement les a déplacées, le rejeu dans la même session peut encore diverger. Les journaux nouveaux n'ont pas cette limite.
- Docs : `docs/canal-local.md` (clé `replay`), en-tête de `core/src/commands/map_topographic_style.rs`.

**Tests (`core/src/journal/tests.rs`).** `StyleQgis` gagne `rearranged` (légende remaniée : couches du projet inversées, couches posées en haut) et `dropped` (couche du projet absente). `a_styling_replays_identically_when_the_legend_was_rearranged` (journal nouveau et journal ancien sans la clé ; résultat résumé, `truncated: true` ; ligne avec `replay.legend` attendu) : il échouait avant la correction (`replay.diverged` à l'étape 2). `a_styling_on_a_changed_project_still_diverges_with_a_rearranged_legend` : couche retirée, divergence à l'étape 2, pour les deux formes de journal.

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture thorough : adversarial, edge-cases) — aucun défaut bloquant ; corrigés 5, acceptés 5, reportés 2.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Repli des anciens journaux : limite plus large que décrite (couches hors de `layers`) | medium | patch (doc) | limite réécrite ici et dans `docs/canal-local.md` |
| `replay.legend` mal formé désactive aussi le repli | low | patch | traité comme absent ; `a_malformed_legend_falls_back_to_the_dictated_orders` |
| Repli qui avale des lignes dictées au-delà du rangement | low | patch | arrêt au `layer.arrange`, seuls les `layer.set_style` |
| Test « diverge toujours » limité à une couche retirée | medium | patch | couche ajoutée et couche renommée (nom et fichier), deux formes de journal |
| `in_legend_order` sans test unitaire | low | patch | `a_legend_reorders_only_the_layers_it_names_at_their_own_places` |
| Réordonner la légende à la main n'est plus un écart | low | accepté, documenté | le rangement rejoué vient du journal ; dit dans `docs/canal-local.md` |
| Ids de toutes les couches au journal (un id QGIS porte le nom) | low | accepté, documenté | AD-9 tenu (aucun chemin) ; dit dans `docs/canal-local.md` |
| Taille : ~50 octets par couche hors résumé | low | accepté | 10 Ko pour 200 couches, sans effet mesuré |
| Ligne en erreur sans légende | low | accepté | seul le code d'erreur est comparé, indépendant de l'ordre |
| Renommer seulement une couche ne diverge pas | — | sans objet | la reconnaissance suit aussi le fichier : même thème, même résultat, même carte |
| Rejeu même session avec `replayed_layers` non vide non testé avec la légende | low | report | couvert par la sonde QGIS (trois rejeux) |
| Garde contre un `replay_param` nommé `legend` | low | report | aucune commande ne le fait aujourd'hui |

## Verification

**Fait (2026-11-09)** : après la relecture, `cargo test --workspace` de nouveau vert (633 tests du noyau), fmt et clippy propres. Avant : `cargo test --workspace` vert (aucun échec), `cargo fmt --all -- --check` propre, `cargo clippy --workspace --all-targets -- -D warnings` sans avertissement. Noyau `--features gpu` reconstruit et installé dans `%LOCALAPPDATA%\Cassini\bin\cassini.exe` (l'ancien gardé sous `cassini.exe.avant-rejeu-style`). Sondes QGIS 3.44.15 (`QT_QPA_PLATFORM=offscreen`, sans modèle), copies archivées dans `donnees-test-cassini/_archives/sondes/` :
- journal de la recette 11.23c (copie `projet-sonde-rejeu-style`) rejoué trois fois dans la même session QGIS : `{"steps": 32, "identical": true}` trois fois (avant : 2e rejeu en divergence à l'étape 15) ;
- session neuve sur Annecy (`projet-sonde-rejeu-style-session`) : `data.reproject` des gares, puis `map.topographic_style` (résultat résumé, ligne avec `replay.legend`), puis deux rejeux dans la même session, légende déjà remaniée : `{"steps": 15, "identical": true}` deux fois.
Scripts jetables : `%TEMP%\sonde-rejeu-style\diag.py` et `session.py`.

**Commands:** Rust sous Windows seulement : `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test --workspace'` (de même pour fmt et clippy).
