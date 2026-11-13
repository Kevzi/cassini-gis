---
title: 'layer.remove refuse une couche non enregistrée (layer.unsaved)'
type: 'feature'
ticket: ''
created: '2026-11-10'
status: 'built'
baseline_revision: 'f851d37c112aeb93f70f17ea83476fc157f665bf'
route: 'full'
route_source: 'pinned'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-refus-retrait-couche-non-enregistree/change-refus-retrait-couche-non-enregistree.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `layer.remove` (10.27) détruit sans recours une couche en mémoire ou les modifications non enregistrées d'une couche en édition, parce que `QgsProject.removeMapLayer` détruit la couche.

**Approach:** décision du propriétaire du 2026-11-10 (changement de cap `change-refus-retrait-couche-non-enregistree`, approuvé) : le geste refuse ces deux cas par le code `layer.unsaved`, avec un message clair en français ; la couche reste au projet, intacte. Fiche du catalogue mise à jour.

## Boundaries & Constraints

**Always:** le refus est décidé avant tout retrait ; la couche refusée n'est ni retirée ni modifiée ; rejeu par le chemin générique d'une étape QGIS refusée (AD-11) ; tests de l'extension par fond doublure ; seul `QgisBackend` importe `qgis.core`.

**Never:** nouveau paramètre (pas de `force`) ; enregistrer ou annuler soi-même les modifications ; changer le générateur ; nouvelle dépendance ; toucher au modèle.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Couche en mémoire | fournisseur `memory` | refus ; couche au projet, entités intactes | `layer.unsaved`, message qui la nomme et dit « sans fichier » |
| Édition modifiée | couche en édition, `isModified()` | refus ; modifications toujours là | `layer.unsaved`, message qui dit enregistrer ou annuler |
| Édition sans modification | en édition, rien de modifié | retirée comme d'habitude | — |
| Couche en fichier | Shapefile, raster | retirée, fichier intact (inchangé) | — |
| Couche absente | id inconnu | même résultat sans erreur (inchangé) | — |
| Rejeu même session | journal avec l'étape refusée, couche toujours non enregistrée | rejeu identique | même code |
| Rejeu, couche enregistrée entre-temps ou session neuve sans la couche | retrait réussi au rejeu | `replay.diverged` à cette étape | — |

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/layer_remove.py` -- `LayerRemoveGesture._open` : ajouter le contrôle entre `backend.layer` et `backend.remove` ; `QgisBackend` : nouvelle méthode `unsaved(layer)` qui rend `"memory"`, `"edits"` ou `None` (`layer.providerType() == "memory"` ; `isEditable()` et `isModified()` lus par `getattr`, un raster n'en a pas toujours). Constante `UNSAVED = "layer.unsaved"` sur le modèle de `CRS_CONFLICT` (`layer_add.py` l. 36). Le nom gardé pour l'affichage (`_REMOVED_NAMES`) n'est rempli qu'après le contrôle.
- `qgis-plugin/tests/test_layer_remove.py` -- `FakeBackend` : couches `(nom, fichier, état)` ; tests des cas de la matrice ; doublure de `unsaved`.
- `core/src/journal/tests.rs` l. ~3641 (`RemoveQgis`) -- ajouter un ensemble de couches non enregistrées ; test : refus journalisé, couche toujours au projet, rejeu même session identique, session neuve sans la couche → `replay.diverged`. Le noyau ne déclare aucun code de geste (aucune liste à mettre à jour).
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` l. ~1659 -- phrase des erreurs de la fiche (texte du changement de cap §4).
- `_bmad-output/initiative-cassini-mvp/deferred-work.md` l. ~923-925 -- l'entrée « décision du propriétaire — couche en mémoire ou en édition » sort (tranchée).
- Sonde QGIS : `C:\Users\kevin\Downloads\Projet code\donnees-test-cassini\_archives\sondes\recette-10-27-layer-remove\sonde.py` -- ajouter une couche `memory` à 2 entités et une copie de couche fichier mise en édition et modifiée ; après refus, compter les entités et `isModified()`.

## Tasks & Acceptance

**Execution:**
- [x] `qgis-plugin/cassini_qgis/layer_remove.py` -- refus `layer.unsaved` -- perte de données évitée
- [x] `qgis-plugin/tests/test_layer_remove.py` -- cas de la matrice côté extension
- [x] `core/src/journal/tests.rs` -- rejeu d'une étape refusée
- [x] catalogue, `deferred-work.md` -- fiche et entrée tranchée
- [x] sonde QGIS rejouée, archivée

**Acceptance Criteria:**
- Given le dépôt, when `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` et les tests de l'extension, then tout passe.
- Given QGIS 3.44 et la copie du projet d'Annecy, when la sonde retire une couche en mémoire puis une couche en édition modifiée, then les deux sont refusées par `layer.unsaved`, restent au projet avec leurs entités et leurs modifications, et les 22 contrôles d'avant restent verts.

## Implementation Notes

- Implémenté directement dans la session (changement de petite taille, contexte déjà chargé), sans sous-agent d'implémentation.
- Code : `layer.unsaved`, forme `<domaine>.<raison>` des codes des gestes ; déclaré dans l'extension seule, le noyau ne liste aucun code de geste.
- Message en mémoire : « Rendez-la d'abord permanente en l'enregistrant dans un fichier (icône à côté de son nom, dans le panneau des couches) » — sans chemin de menu figé, l'extension servant QGIS 3.44 et 4.x.
- Sonde QGIS réelle (2026-11-10, QGIS 3.44.15, noyau installé inchangé, extension recopiée dans le profil) : phase 3 ajoutée à la sonde de 10.27, dans la session neuve — couche `memory` à 2 entités et copie GeoPackage de `plans-d-eau-74` en édition, une entité supprimée, jamais enregistrée. Les deux retraits sont refusés par `layer.unsaved`, les couches restent avec leurs entités et leur modification, la copie n'est pas écrite ; rejeu des 5 étapes en même session `identical: true`. 37/37 contrôles (22 d'avant + 15), rejouée après les corrections de relecture : 37/37. Archive : `donnees-test-cassini/_archives/sondes/recette-10-27-layer-remove/` (`sonde.py`, `sortie.txt` ; versions d'avant sous `*-avant-refus.*`).

## Plan Change Log

## Review Triage Log

Boucle 0, relecture thorough (blind-hunter, edge-case-hunter, verification-gap, intent-alignment). Verdicts : high 0, medium 1, low 8, false 5, maybe-false 0. Corrigés 5, rejetés 9.

| Constat | Verdict | Route | Preuve / action |
|---|---|---|---|
| Texte de la fiche plus riche que le §4 du changement de cap (phrase de rejeu) | low | patch | phrase recopiée dans le §4 du changement de cap |
| Paragraphe de la fiche non recoupé (lignes très longues) | low | patch | les deux paragraphes recoupés à 95 colonnes |
| Conseil « clic droit, Exporter » vague et lié aux menus | low | patch | message : « Rendez-la d'abord permanente… (icône à côté de son nom) », sans chemin de menu |
| Nom de test `editing_layer_without_changes` trompeur | low | patch | renommé `a_layer_without_unsaved_content_is_removed`, le cas réel est dans le test du vrai fond |
| Assertion sur le fichier sans pouvoir d'échec (test des modifications) | low | patch | retirée |
| Journal d'un refus en mémoire jamais rejouable en session neuve (CLAUDE.md §6) | medium | rejet (signalé au propriétaire) | voulu par la matrice gelée et le changement de cap : la couche en mémoire n'existe plus en session neuve, le rejeu diverge honnêtement à l'étape au lieu de la sauter |
| Couche virtuelle ou de greffon sans fichier non refusée | low | rejet | hors de l'intention (en mémoire, ou modifications non enregistrées) ; rare pour le public visé, la refuser ajoute des branches ; une couche virtuelle se recrée depuis ses sources |
| Aucun test d'un refus au milieu d'un plan | low | rejet | arrêt du plan sur une étape échouée : chemin générique, déjà testé ; rien de propre à ce code |
| Panneau non testé avec le message du refus | low | rejet | le panneau montre tout message de geste par le même chemin, déjà testé ; rien de propre à ce code |
| Sonde QGIS absente du diff (trois lentilles) | false | rejet | jouée, hors dépôt par règle (données réelles) : 37/37, archivée ; voir Implementation Notes |
| Fichier du plan hors du diff | false | rejet | écarté du diff relu par choix ; commité avec le changement |
| Entrée retirée de `deferred-work.md` sans trace | false | rejet | la décision est consignée dans le plan de 10.27 (« Décisions du propriétaire ») et dans le changement de cap |
| Lien code extension ↔ test du noyau par chaîne littérale | false | rejet | le noyau ne connaît aucun code de geste (AD : codes nés dans l'extension) ; le bout en bout est la sonde réelle |
| Prédicat jamais confronté à l'API PyQGIS | false | rejet | confronté dans la sonde réelle (`providerType`, `isEditable`, `isModified` sur de vraies couches) |

## Verification

**Commands:**
- `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test --workspace'` (de même fmt, clippy) -- expected: vert.
- `python3 -I -m unittest discover -s qgis-plugin/tests` et `python3 -I qgis-plugin/tests/check_extension.py` -- expected: vert.
