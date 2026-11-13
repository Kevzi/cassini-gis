---
title: 'La sonde de démo juge la phrase de démo sur des fixtures synthétiques'
type: 'feature'
ticket: ''
created: '2026-11-10'
status: 'done'
baseline_revision: '47ef2f44baa217b2d2d69a6ad689c457fe1cdd5e'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `qgis-plugin/tests/probe_demo.py` exige un projet réel hors dépôt et ne fait qu'imprimer ce qui se passe : elle ne juge pas si la phrase de démo a donné la bonne réponse, et son code de sortie ne tient qu'au PDF.

**Approach:** Ajouter aux fixtures trois couches synthétiques en EPSG:2154 (zonage du PLU U et N, parcelles, gares) dont la sonde assemble par défaut un projet de démo dans un dossier temporaire ; un projet donné en argument reste possible. La sonde envoie la phrase de démo au vrai modèle, valide, exécute, puis rend `VERDICT : OK` ou `VERDICT : ECHEC — <raison>` avec un code de sortie non nul en cas d'échec.

## Boundaries & Constraints

**Always:** fixtures écrites dans l'emprise du MNT `fixtures/mnt-colline-2154.tif` (652000–652500, 6850000–6850500) pour l'entrée 4 ; noms de couches et de champs repris de l'inventaire de l'épreuve (`Zonage du PLU`/`typezone`, `Parcelles cadastrales`/`id_parcelle`, `section`, `Gares`/`nom`) ; logique du verdict dans un module pur testé en CI sans QGIS ; les fichiers d'origine (fixtures, projet passé en argument) ne sont jamais modifiés, les sorties vont dans `<projet>.cassini/`.

**Never:** aucune donnée réelle commitée ; aucune nouvelle dépendance ; aucune modification de l'extension (`cassini_qgis/`) ni du noyau ; pas de rejeu (entrée 2), pas de lecture de la mise en page (entrée 3), pas de MNT (entrée 4).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Fixtures, plan juste | pas d'argument | plan composé, étapes réussies, couche retenue non vide, chaque entité en zone U et à ≤ 500 m d'une gare, PDF `%PDF-` | `VERDICT : OK`, code 0 |
| Projet donné | `probe_demo.py <projet.qgz>` | mêmes contrôles sur ce projet | idem |
| Étape échouée | `CASSINI_DEMO_FAIL_STEP=<id de commande>` | le geste répond une erreur, le noyau arrête le plan | `ECHEC — étape k <id> : <code>`, code 1 |
| Question ou refus du modèle | phrase non comprise | pas de plan | `ECHEC — question : …`, code 1 |
| Mauvaise réponse | entité en zone N, ou à > 500 m, ou couche vide | | `ECHEC` nommant l'entité fautive, code 1 |
| Pas de PDF / PDF illisible | plan sans `layout.export_pdf` | | `ECHEC` nommant le PDF, code 1 |
| Couche PLU ou gares introuvable | projet sans champ `typezone` ou sans couche « gare » | | `ECHEC` nommant la couche, code 1 |

</frozen-after-approval>

## Code Map

- `qgis-plugin/tests/probe_demo.py` -- sonde actuelle : `check_phrase`, `core_path`, `Timed`/`TimedRegistry`, `on_event`, `pdf_path`. À réécrire en gardant phrase, variables `CASSINI_CORE`/`CASSINI_MODELS`/`CASSINI_PROBE_PATH`, chronométrage.
- `qgis-plugin/cassini_qgis/client.py` -- événements `READY`, `PLAN_PROPOSED`, `PLAN_DONE` (`{"steps"}`), `PLAN_EXEC_FAILED` (`{"code","message","step","outputs"}`), `PLAN_QUESTION`, `PLAN_FAILED`, `PLAN_CANCELLED`, `FAILED`, `DIED`. Ne pas modifier.
- `qgis-plugin/cassini_qgis/gestures.py` -- `GestureError(code, message)`, contrat `done(result=…, error=…)`.
- `qgis-plugin/tests/probe_end_to_end.py` -- modèle d'un projet assemblé dans `tempfile.mkdtemp` puis `project.write`.
- `ai/tests/plan_reel.rs` S17 -- plan attendu de la phrase de démo : sélection `typezone = U`, tampon 500 m des gares, deux intersections, export PDF ; le résultat d'analyse porte `layer_id`.
- `fixtures/README.md` -- tableau des fixtures à compléter.
- CI : `python -I -m unittest discover -s qgis-plugin/tests` (sans QGIS) ; `check_extension.py` ne garde que `cassini_qgis/`.

## Tasks & Acceptance

**Execution:**
- [x] `fixtures/demo/plu-zonage-2154.geojson`, `fixtures/demo/parcelles-2154.geojson`, `fixtures/demo/gares-2154.geojson` -- zone U = moitié ouest (652000–652250), zone N = moitié est ; 16 parcelles de 125 m (grille 4×4) ; gares en (651700, 6850050) et (652800, 6850100). Attendu : 7 parcelles U à ≤ 500 m ; la parcelle U du coin nord-est de la moitié ouest exclue par la distance ; des parcelles N à ≤ 500 m exclues par la zone.
- [x] `qgis-plugin/tests/demo_verdict.py` -- module pur : couche retenue (dernière étape avant l'export portant `layer_id`), nom du PDF, `judge_done(steps, facts)` et `failure_reason(kind, payload)` rendant la raison d'ECHEC ou `None`.
- [x] `qgis-plugin/tests/probe_demo.py` -- argument facultatif ; sans lui, projet assemblé en dossier temporaire depuis `fixtures/demo/` ; injection d'échec par `CASSINI_DEMO_FAIL_STEP` ; relevé des faits sous QGIS (zone `typezone` au point intérieur de chaque entité retenue, distance à la gare la plus proche, cinq premiers octets du PDF) ; verdict par `demo_verdict`.
- [x] `qgis-plugin/tests/test_demo_verdict.py` -- cas de la matrice sur des charges factices, et contrôle des fixtures : SCR 2154, emprise du MNT colline pour zonage et parcelles, réponse attendue de 7 parcelles calculée à la main sur rectangles.
- [x] `fixtures/README.md` -- trois lignes au tableau.

**Acceptance Criteria:**
- Given la CI sans QGIS, when `unittest discover` tourne, then les tests du verdict et des fixtures passent.
- Given le poste du propriétaire, carte graphique libre, when la sonde tourne sans argument, then elle rend `VERDICT : OK` ; with `CASSINI_DEMO_FAIL_STEP=layout.export_pdf`, then `VERDICT : ECHEC` et code non nul.

## Implementation Notes

- Essai sous le vrai QGIS 3.44.15 (2026-11-13, poste du propriétaire, noyau gpu `C:\ct-5-57g`, modèle installé) : `VERDICT : OK`, code 0. Plan de 5 étapes composé en 56 s, exécuté en 0,8 s, 7 parcelles retenues (P01, P02, P05, P06, P09, P10, P14), toutes en zone U et à 240–437 m de la gare, PDF produit. Cette entrée reste `built` : le ticket de l'arbre a été fusionné avec 6.2 et 6.3 le 2026-11-12, et le rejeu du journal après réouverture, la lecture de la mise en page (titre, légende, échelle, flèche du nord, sources) et les deux échecs nommés (journal altéré, légende perdue) restent à construire.

- Couches de référence (PLU : première couche vectorielle à champ `typezone` ; gares : première couche de points dont le nom contient « gare ») relevées **avant** l'exécution : les sorties d'intersection reprennent le champ `typezone` et ne doivent pas être prises pour le PLU. Absentes → ECHEC avant même de lancer le noyau.
- Faits relevés dans le SCR du projet (EPSG:2154 si le projet est géographique) ; tolérance de 0,01 m sur les 500 m (le tampon de QGIS est inscrit dans le cercle).
- Échec injecté : code `qgis.demo_injected`, réponse différée d'un tour de boucle Qt.
- Fixtures générées par un script jetable (non versionné) ; leur contenu est contrôlé par `test_demo_verdict.py`.
- Sonde sous QGIS non jouée par l'agent d'implémentation : carte graphique occupée (97 %, 7 Go) et PowerShell refusé dans son bac à sable.
- Après relecture (correctifs) : les gares de la tâche 1 sortaient de l'emprise du MNT, contre la contrainte « Always » ; remplacées par **une** gare dans l'emprise, au coin sud-est (652490, 6850010). Les 7 parcelles attendues tiennent ; la parcelle exclue par la distance est désormais celle du coin nord-ouest (516 m). Couche retenue = dernière **analyse** avant l'export (une étape de style rend la couche qu'elle met en forme) ; la couche retenue doit porter un champ propre aux parcelles ; couche des parcelles (polygones nommés « parcelle ») exigée parmi les références ; un plan mené au bout malgré `CASSINI_DEMO_FAIL_STEP` rend ECHEC ; géométries vides ignorées ; gestes pris de `delegated.build_registry` (style compris), comme le panneau.
- Tests Rust (régression, aucun fichier Rust touché) : `cargo.exe` Windows lancé depuis WSL par `rtk proxy`, `CARGO_TARGET_DIR=C:\ct-6-1` : clippy vert, 1062 tests verts ; `cargo fmt --check` vert.

- **Étape restante (verify du ticket)** : la sonde n'a pas encore tourné avec le vrai modèle dans QGIS ; la carte graphique était prise par l'entraînement `ajuste-nuit-11-10-a` (99 %, 7 Go) le 2026-11-10 à midi. À jouer une fois la carte libre : sans argument → `VERDICT : OK` ; avec `CASSINI_DEMO_FAIL_STEP=layout.export_pdf` → `VERDICT : ECHEC`, code 1.

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : high 0, medium 3, low 4, false 0, maybe-false 0 ; 2 rejets.

| # | Constat | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 1 | Un plan qui ne touche pas aux parcelles (zonage U ∩ tampon) passe OK | medium | patch | Vérifié : rien ne reliait la couche retenue aux parcelles. Ajout du fait `parcel_fields` et de la couche de référence des parcelles. Le sous-point « 1 parcelle sur 7 suffit » : low, rejeté (le ticket demande « des parcelles en zone U retenues »). |
| 2 | `retained_layer_id` peut prendre une étape de style | medium | patch | Vérifié : `style_gestures.py:975` rend `{"layer_id": …}` de la couche stylée. Ne retient plus que les commandes `analysis.*` ; test ajouté. |
| 3 | Échec injecté sur une commande sans geste ignoré en silence | medium | patch | Vérifié : `Failing` ne remplaçait qu'un geste existant. PLAN_DONE avec `injected` rend ECHEC ; test ajouté. |
| 4 | Gares hors de l'emprise du MNT, contre la contrainte figée | low | patch | Vérifié. Une gare dans l'emprise, discrimination gardée ; test d'emprise étendu aux gares. |
| 5 | README : provenance des gares « écrit à la main » | low | patch | Corrigé en « calculé ». |
| 6 | Géométries vides non filtrées | low | patch | Correction directe : `isEmpty()` ignoré avec `isNull()`. |
| 7 | SCR projeté non conforme (3857) fausse les distances | low | rejet | Projets visés en 2154 (CLAUDE.md §5) ; garde à ajouter, peu probable en usage. |
| 8 | La sonde n'enregistre pas les gestes de style | low | patch | Préexistant mais rend le verdict faux si le modèle met en forme : registre pris de `delegated.build_registry`. |
| 9 | `sys.exit` sans `exitQgis`, dossier temporaire jamais effacé | low | rejet | Le processus se termine ; le dossier garde le PDF et le journal pour l'examen, voulu. |

## Design Notes

Projet assemblé à l'exécution plutôt que `.qgz` commité : pas de binaire lié à une version de QGIS, et les sorties (`<projet>.cassini/`, journal) ne tombent jamais dans `fixtures/`. Le contrôle est géométrique, non une liste d'identifiants : il tient quel que soit l'ordre des intersections choisi par le modèle et attrape un plan qui oublie la zone ou le tampon. Décision prise sans le propriétaire (« go » du 2026-11-10), à signaler dans le rapport.

## Verification

**Commands:**
- `python3 -I -m unittest discover -s qgis-plugin/tests` -- expected: tout passe
- `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` (Windows, `CARGO_TARGET_DIR=C:\ct-6-1`) -- expected: vert

**Manual checks (if no CLI):**
- Sous QGIS, carte libre : `probe_demo.py` sans argument → `VERDICT : OK` ; avec `CASSINI_DEMO_FAIL_STEP=layout.export_pdf` → ECHEC, code 1.
