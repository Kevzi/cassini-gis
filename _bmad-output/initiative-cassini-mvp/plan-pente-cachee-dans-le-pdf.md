---
title: 'Le PDF d''un plan cache les couches produites par les plans précédents'
type: 'bugfix'
ticket: ''
created: '2026-11-13'
status: 'built'
baseline_revision: '4d0a4c811ceb69027b798ef26e8612d96aeadb9d'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['correctness', 'edge-cases']
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-mise-en-evidence-du-resultat-dans-le-pdf/change-mise-en-evidence-du-resultat-dans-le-pdf.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** dans un second plan du même projet (la pente du MNT après les parcelles), `focus.hidden` est vide : les sorties du plan 1 restent visibles dans le PDF et recouvrent le résultat (essai réel du 2026-11-13, `9-layout.export_pdf.pdf`, pente dans la légende mais cachée sur la carte).

**Approach:** décision du propriétaire du 2026-11-13 : le PDF d'un plan montre les couches de l'utilisateur et le résultat du plan en cours ; les sorties des plans précédents de la session sont ajoutées à `focus.hidden` (carte, légende, sources), restent dans le projet, et ne sont jamais des `results` du plan en cours.

## Boundaries & Constraints

**Always:** changement dans `fill_focus` (`core/src/journal/mod.rs`) et une fonction pure de `core/src/focus.rs` ; `results`, `frame`, `framed_on` et le rejeu inchangés (`focus` est journalisé et relu, jamais recalculé, AD-11) ; une couche ouverte sur un fichier de l'utilisateur n'est jamais cachée (réutiliser `produced_layers`) ; une couche retirée (`layer.remove`) ou déjà dans `hidden` n'est pas dupliquée ; aucun identifiant ni paramètre visible du catalogue ne change, ni le texte lu par le modèle ; documents mis à jour : fiche `layout.export_pdf` du catalogue, précision datée sous AD-24, §2-§3 de la décision de 7.9, en-tête de `focus.rs`.

**Never:** modifier l'extension ; caler le résultat par l'ordre des couches ; changer le cas sans résultat (`focus_of` rend `None`, rien n'est caché) ni l'export hors plan ou à l'étape 1 ; supprimer des couches du projet de l'utilisateur.

## I/O & Edge-Case Matrix

| Scénario | Entrée / état | Comportement attendu | Gestion d'erreur |
|----------|---------------|----------------------|------------------|
| Second plan | plan 1 (analyses + export), plan 2 (pente + export) | `hidden` du second export contient les sorties du plan 1 ; `results` = la pente seule | — |
| Couche de l'utilisateur | `layer.open_raster` sur un fichier de l'utilisateur au plan 1 | jamais dans `hidden` | — |
| Sortie retirée | `layer.remove` d'une sortie du plan 1 | absente de `hidden` (aucun doublon) | — |
| Plan unique | un seul plan | `hidden` inchangé (intermédiaires du plan) | — |
| Résultat issu d'un plan précédent | une sortie du plan 1 est un `result` du plan 2 | n'est pas cachée | — |
| Export hors plan / étape 1 | `plan_step` absent ou 1 | pas de `focus`, comportement actuel | — |

</frozen-after-approval>

## Code Map

- `core/src/journal/mod.rs` -- `fill_focus` (l.468-520) : après `current_plan` (l.1055), relire le journal, garder les lignes d'index < `plan_start` (`Session.plan_start`, l.171, posé l.360), calculer les sorties précédentes, les ajouter à `focus.hidden` sans doublon et hors `results` ; `fill_focus` ne s'exécute pas si `plan_step` est `None` ou ≤ 1 (l.476).
- `core/src/focus.rs` -- ajouter `earlier_outputs(registry, lines)` : `produced_layers(...)` (l.~175) puis les ids, sans les couches retirées ; `focus_of` (l.544), `split` (l.299-312), `hidden` (l.623) ne changent pas ; garder `an_earlier_export_of_the_plan_consumes_nothing` (l.1148). En-tête l.1-22 à mettre à jour.
- `core/src/journal/tests.rs` (~l.5040-5240, `demo_plan`, `call_in_plan`, `exports.borrow()[0]["focus"]`) et `core/src/focus.rs` `mod tests` (helper `ok(...)`) -- tests.
- Documents : `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` (fiche `layout.export_pdf`, l.729 et l.779-796) ; `architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md` (AD-24 l.745) ; la décision de 7.9.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/focus.rs` -- `earlier_outputs`, tests purs (sortie d'analyse listée, fichier de l'utilisateur exclu, `layer.remove` exclu) -- règle pure
- [ ] `core/src/journal/mod.rs` -- `fill_focus` ajoute les sorties précédentes à `hidden` -- le correctif
- [ ] `core/src/journal/tests.rs` -- plan 1 + plan 2 : `hidden` du second export ; plan unique inchangé ; résultat issu d'un plan précédent non caché ; rejeu identique -- couvre la matrice
- [ ] catalogue, AD-24, décision de 7.9 -- précision datée du 2026-11-13 -- documents à jour

**Acceptance Criteria:**
- Given deux plans dans un projet, when le second exporte, then `focus.hidden` contient les sorties du premier et le PDF ne les dessine ni ne les liste.
- Given un journal avec les deux plans, when il est rejoué, then le résultat est identique étape par étape (`focus` relu tel quel).
- Given les essais réels de la sonde de démo, when elle rend `OK`, then le second PDF montre la pente sur la carte, sans les couches intermédiaires du plan 1.

## Implementation Notes

Essai réel du 2026-11-13 (noyau gpu recompilé dans `C:\ct-5-57g`, QGIS 3.44.15, modèle installé) : le second export cache les 4 sorties du plan 1 (`hidden` à 4 au lieu de 0) ; PDF ouvert avec Ghostscript : légende sans `N-analysis`, plus aucune couche du plan 1 sur la carte. Tests du noyau : 895 OK, fmt et clippy propres.
Deuxième cause, hors périmètre du noyau : 7.9 range un résultat raster « sous les vecteurs de l'utilisateur » (`_focus_order`, `layout_export.py`) ; les polygones pleins des fixtures de démo cachaient donc encore la pente. Corrigé dans la sonde (polygones de la démo en contour seul), pas dans l'extension. La sonde remplace son contrôle « le second export a plus de couches » par « aucune sortie du plan 1 dans les sources du PDF ». Seconde observation : un cadre noir de 1 cellule autour de la pente (bords sans valeur de la méthode de Horn, le MNT noir du dessous apparaît).

## Plan Change Log

## Review Triage Log

Passe 1 (deux lentilles : justesse, cas limites). Justesse : aucun critère non satisfait ; cas limites : 5 constats retenus.

- `layer.remove` d'une sortie faite dans le plan en cours ou hors plan non vu (id retiré dans `hidden`) -- medium, patch : `earlier_outputs` prend aussi ces lignes, deux tests de journal.
- Garde « hors des `results`, sans doublon » en ligne et sans test (ligne de matrice « résultat issu d'un plan précédent ») -- medium, patch : `merge_hidden` extraite et testée.
- Test `a_plan_started_mid_journal_reads_only_its_own_lines` au nom périmé -- low, patch : renommé, assertion « hors plan non caché » séparée.
- Assertion de rejeu trop faible (`contains("f_g2_")`) -- low, patch : égalité exacte des quatre ids.
- Pas de test à trois plans -- low, patch : ajouté.
- Libellé « de la session » alors que le code lit le journal du projet -- low, patch : « du journal du projet » dans les trois documents et le code ; les sorties hors plan ne sont pas cachées, dit.
- Plan sans résultat (`focus_of` rend `None`) -- rejeté : comportement inchangé voulu par le plan (Never).
- Journal relu en entier à chaque export (O(n²)) -- rejeté : taille de journal d'un projet, sans effet mesurable.
- Ligne de journal illisible ignorée alors que `current_plan` s'arrête -- rejeté : improbable, correction plus lourde que le défaut.
- Frame et hidden qui se recouvrent -- maybe-false, non atteignable : sans résultat, `focus_of` rend `None` et rien n'est caché.

## Design Notes

Il n'existe pas d'identifiant de plan : un plan est une suite de lignes à `plan_step` croissant et `plan_step == 1` en ouvre un nouveau. `Session.plan_start` (numéro de ligne) sépare donc le plan en cours des précédents : toute ligne d'index inférieur est d'un plan précédent. Les ids journalisés d'une sortie sont ceux du moment de l'export, ce qui suffit puisque `focus` est relu tel quel au rejeu.

## Verification

**Commands:**
- `cargo test -p cassini-core` -- expected: tous verts
- `cargo fmt --check` et `cargo clippy -p cassini-core -- -D warnings` -- expected: aucun avertissement
- Construire le noyau gpu dans `C:\ct-5-57g` (`VULKAN_SDK`, `LIBCLANG_PATH`, `CMAKE_GENERATOR=Ninja`, `cargo build --release -p cassini --features gpu`), puis `probe_demo.py` sous le Python de QGIS -- expected: `VERDICT : OK`

**Manual checks (if no CLI):**
- Ouvrir `9-layout.export_pdf.pdf` (Ghostscript de QGIS) : la pente est visible sur la carte, aucune couche `N-analysis.*` dans la légende ni les sources.
