---
title: 'Modèle ré-éprouvé et couverture mesurée sur le catalogue étendu'
type: 'feature'
ticket: '17'
created: '2026-11-20'
status: 'built'
baseline_revision: '5dca80fc76bc99d2a89970b3463ae14d090d9313'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
  - '{project-root}/docs/couverture-phrases-reelles.md'
  - '{project-root}/docs/mesure-duree-par-phrase.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Les commandes ajoutées par l'epic 10 n'ont pour la plupart aucune famille d'exemples dans le générateur (`analysis.join_table`, `difference`, `area`, `statistics_by_category`, `sum_line_lengths`, `layout.export_atlas`, `network.prepare`, `network.shortest_path`, `geocode.ban`…). Le modèle installé (continu-5-61b) ne les compose donc pas, et le catalogue complet n'a jamais été mesuré de bout en bout.

**Approach:** Écrire les familles manquantes (hors phrases tenues à l'écart), ré-entraîner par la chaîne 5.10, installer, puis consigner la couverture des 44 phrases réelles, le taux de plans conformes et la durée d'une phrase longue sur processeur.

## Boundaries & Constraints

**Always:** Barres du ticket : couverture agent ≥ 70 % des phrases réelles ; plans conformes ≥ la référence de l'entrée 8 ; phrase longue sur CPU < 30 s, catalogue complet. Garde anti-fuite (`training/tests/recouvrement.rs`) verte : aucune phrase d'épreuve dans le générateur. Rapports dans `docs/`. Noyau Apache-2.0, aucune dépendance nouvelle.

**Never:** Pas de commande ni de changement de catalogue. Pas d'installation du modèle ni de `git push` sans le propriétaire. Ne pas supprimer `C:\ct`, `C:\ct-5-57g`, `target`, `.venv`. Rien sur le module défense.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Famille ajoutée | `--seed 1`, nouvelle famille | exemples dont le plan passe `check_plan` | test de famille rouge |
| Fuite | phrase d'épreuve proche d'un exemple | garde rouge | reformuler l'exemple |
| Barre manquée | mesure sous une barre | rapport l'écrit, ticket non clos | diagnostic, pas de barre abaissée |

**Décisions du propriétaire (2026-11-21) :**
1. Plans conformes : plancher 44/48 (déjà validé pour 10.8).
2. Durée : l'amorce (première phrase) compte. On mesure la phrase de démo et l'amorce, et on consigne les deux ; au-delà de 30 s, on cherche un préchauffage du modèle, on ne sort pas l'amorce de la mesure.
3. Nuit d'entraînement : lancée par le propriétaire ; l'assistant prépare familles, garde et commandes ; l'installation reste au propriétaire.
4. Portée : un seul ticket, ouvert par un diagnostic classant les phrases qui tombent par commande ; des familles seulement pour celles qui pèsent sur les 31 phrases nécessaires. 10.33 est construit avant le ré-entraînement.
5. Écarts hors catalogue (CV14, CV18, CV28, CV29 ; CV37, CV39, CV42) : consignés au rapport, ticket de garde-fou à ouvrir, non bloquants.

</frozen-after-approval>

## Code Map

- `training/src/families.rs` -- familles d'exemples ; `plus_proche_voisin` l.3549, `isochrone` l.3321, `courbes_de_niveau` l.4963 comme modèles
- `training/tests/recouvrement.rs`, `training/src/barres.rs` -- garde anti-fuite et barres
- `training/entrainement/pipeline.py`, `nuit-5-55.sh` -- entraînement reproductible (WSL, RTX 3070 Ti, ~5-6 h)
- `ai/tests/plan_reel.rs`, `ai/tests/couverture_reelle_reel.rs`, `ai/tests/duree_reel.rs` -- mesures (GGUF via `CASSINI_TEST_MODEL`, feature `gpu` sauf durée)
- `ai/tests/par_commande/phrases.json` -- NEA1-3, CHE1-3, PRE1-3, jamais passées contre un vrai modèle
- `docs/couverture-phrases-reelles.md`, `docs/mesure-duree-par-phrase.md`, `docs/entrainement-du-modele.md` -- rapports (sections ajoutées, existant intact)

## Tasks & Acceptance

**Portée de cette session de construction :** diagnostic, familles, garde, préparation des commandes de la nuit, docs de procédure. Le ré-entraînement, l'installation du modèle et les mesures qui en dépendent (couverture, `plan_reel`, durée) sont au propriétaire : l'implémenteur les laisse décochées, écrit les commandes exactes à lancer et s'arrête. Les mesures sans modèle neuf (diagnostic sur le modèle installé) se font. 10.33 doit être construit avant la nuit : le signaler au rapport.

**Execution:**
- [ ] `docs/couverture-phrases-reelles.md` -- diagnostiquer 44→41/48 (PL04, PL08, PL17, PL22, S24) et classer par commande les phrases non rendues -- le plan de 10.8 l'exige, et cela borne les familles à écrire
- [ ] `training/src/families.rs` -- une famille par commande retenue par le diagnostic, avec test -- rend les commandes composables
- [ ] `training/tests/recouvrement.rs` -- garde étendue -- aucune fuite
- [ ] `docs/entrainement-du-modele.md` -- section « Ré-entraînement de 10.17 » : commandes, seed, résultat de non-oubli
- [ ] `docs/couverture-phrases-reelles.md` -- mesure après ré-entraînement, `CASSINI_PLANS_CONFORMES` renseigné
- [ ] `docs/mesure-duree-par-phrase.md` -- durée sur le catalogue complet, phrase longue, CPU 4 cœurs

**Acceptance Criteria:**
- Given le modèle ré-entraîné, when le harnais passe les 44 phrases, then ≥ 31 sont couvertes et bien rendues.
- Given les 48 plans de `plan_reel`, when ils sont mesurés, then le score ≥ 44/48.
- Given le catalogue complet, when la phrase longue tourne sur CPU, then la phrase de démo et l'amorce prennent chacune moins de 30 s.
- Given `cargo test --workspace`, then tout est vert, clippy et fmt sans avertissement.

## Implementation Notes

Construction partielle voulue : diagnostic par commande, deux familles (`longueurs_par_zone`, `fusion_par_champ`), docs de la nuit. Nuit, installation, mesures et diagnostic du `plan_reel` restent au propriétaire ; 10.33 à construire avant.

## Plan Change Log

## Review Triage Log

Passe 1 (lentilles quick + cas limites) : 4 medium, 3 low, 3 false, 1 non résolu.

| Verdict | Route | Constat et preuve |
|---|---|---|
| medium | patch | Accord fautif de `fusion_par_champ` (« le même section ») : article masculin figé, étiquettes féminines ; gabarits rendus neutres. |
| medium | patch | Gabarits proches des phrases tenues à l'écart (« Quel linéaire… y a-t-il dans chaque », « qui partagent le même type de zone ») ; reformulés ; garde `recouvrement` verte (8/8). |
| medium | patch | Chemin d'inventaire inexistant dans la doc de durée ; remplacé par l'emplacement hors dépôt. |
| medium | patch | Arithmétique de la barre : CV35 comptée sans preuve ; plafond écrit 29 ou 30. |
| low | patch | Seuil `two > 10` fragile ; abaissé. Amorce nommée A0 à tort, stderr non redirigé : corrigés. |
| low | rejeté | Durée « 5-6 h » du plan contre « 52 min » de la doc : documentaire, sans effet. |
| false | rejeté | « Garde étendue absente » : `recouvrement.rs` couvre déjà toutes les familles et les 44 phrases. |
| false | rejeté | Commande de nuit non vérifiée : `nuit-5-55.sh` accepte `continu <familles>`, `PARENT`, `NOM_CONTINU` (tracé). |
| false | rejeté | Ids et paramètres inventés : conformes au registre (`deny_unknown_fields`). |
| medium | non fait | Diagnostic PL04, PL08, PL17, PL22, S24 : exige de passer `plan_reel` sur le modèle installé (GPU) ; laissé au propriétaire. |

## Design Notes

Le registre couvre déjà 34/44 (77,3 %) ; l'enjeu est l'agent (21/44 = 47,7 % au 2026-11-20). La durée tangente : 28,5 s (phrase de démo, 10.5), amorce 33,1 s.

## Verification

**Commands:**
- `cargo test -p cassini-training` -- expected: familles et garde vertes
- `cargo test -p cassini-ai --features gpu --test plan_reel -- --ignored --nocapture` -- expected: ligne `SCORE n/48`
- `cargo test --workspace` -- expected: vert

**Manual checks (if no CLI):**
- Rapports relus : trois barres chiffrées, modèle et date nommés.
