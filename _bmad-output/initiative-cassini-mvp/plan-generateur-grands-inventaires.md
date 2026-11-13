---
title: 'Générateur : inventaires larges et jeu de 8 600 exemples'
type: 'feature'
ticket: ''
created: '2026-11-10'
status: 'built'
baseline_revision: '8e724747489f65c089f94b85fa7681dfdf802d1c'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** les quatre candidats de la nuit du 2026-11-10 ratent « décompte des gares par commune avec le nom » à Annecy : `label_field` vaut `nom` ou `commune` au lieu de `nom_officiel`, dans une couche des communes de 17 champs, alors que les inventaires engendrés n'en portent que quelques-uns. Et les familles ajoutées depuis `ajuste-revue-11-23` diluent les autres dans un jeu de taille fixe.

**Approach:** décisions du propriétaire du 2026-11-10 : une part notable des couches des thèmes qui en ont dans la réalité (communes, routes, bâti, gares, parcelles, PLU) porte 15 à 20 champs réalistes, de types variés, style BD TOPO / cadastre / CNIG, le champ de nom noyé au milieu ; le jeu passe de 8 000 à environ 8 600 exemples, décision datée écrite dans la doc d'entraînement.

## Boundaries & Constraints

**Always:** graines du générateur inchangées, `training/tests/recouvrement.rs` vert ; les plans ne citent que le champ de rôle ; longueur des exemples mesurée avant et après, sous les 2 048 jetons de `MAX_LENGTH` ; parts des familles comparées à `47ef2f4` (`ajuste-revue-11-23`) et à `b53ff4c`.

**Never:** recopier une phrase du jeu tenu à l'écart, des essais réels (`ai/tests/`) ou de la recette d'Annecy ; nouvelle dépendance ; lancer un entraînement, installer un modèle, pousser ou fusionner.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Nom dans une longue liste | communes élargies à 17 champs, nom `nom_officiel` au milieu, leurre `nom_officiel_en_majuscules` | `label_field: "nom_officiel"` | — |
| Champ absent | bâtiments élargis sans champ de hauteur | question « aucun champ de hauteur » ; aucun remplissage n'est une hauteur | — |
| Doublon | remplissage `cleabs` et leurre `cleabs` | un seul `cleabs` (casse comprise) | — |

</frozen-after-approval>

## Code Map

- `training/src/inventories.rs` -- `FILL_TEXT`/`FILL_INT`/`FILL_NUM` (rôle `leurre`), tables `WIDE_*` par thème, `wide_fields`, `widen` ; `Builder::add` élargit (2/5, au plus `MAX_WIDE_LAYERS` = 2 par inventaire) ; `Builder::wide_next` force la couche suivante.
- `training/src/families.rs` -- `decompte_par_zone` : `wide_next` une fois sur deux quand le nom est demandé ; test `a_label_field_is_…` compte les noms pris dans une couche large.
- `training/src/lib.rs` `DEFAULT_COUNT`, `training/entrainement/pipeline.py` `TRAINING_COUNT` -- 8 000 → 8 600.
- `docs/entrainement-du-modele.md` -- section « Inventaires larges et jeu de 8 600 exemples », commandes à 8 600.

## Tasks & Acceptance

**Execution:**
- [x] `training/src/inventories.rs` -- remplissage, élargissement, tests
- [x] `training/src/families.rs` -- `decompte_par_zone`, test
- [x] `training/src/lib.rs`, `training/entrainement/pipeline.py` -- 8 600
- [x] `docs/entrainement-du-modele.md` -- décision et mesures

**Acceptance Criteria:**
- Given le jeu par défaut, when on le mesure, then environ 40 % des couches des six thèmes portent 15 à 20 champs, aucun exemple ne dépasse 2 048 jetons, et les familles à poids inchangé retrouvent au moins leur compte de `47ef2f4` en somme.
- Given le dépôt, when `cargo test -p cassini-training`, fmt et clippy, then tout passe, `recouvrement` compris.

## Implementation Notes

- Implémenté directement dans la session (contexte déjà chargé). Le point d'approbation du plan s'appuie sur les décisions du propriétaire transmises dans la demande.
- Mesures finales (graine 1, après relecture) : couches des six thèmes à 15 champs ou plus 0 % → 38 % ; `label_field` 41 (`47ef2f4`) → 173 (`b53ff4c`) → 194, dont 65 en couche large ; familles à poids inchangé 7 044 → 6 637 → 7 150 ; longueur moyenne 770 → 773 → 830 jetons, maximum 1 629 → 1 661 → 1 773, aucun au-delà de 2 048 (validation, graine 2 : max 1 628) ; jetons du jeu 6,16 M → 6,19 M → 7,14 M.
- Première mesure : 1 838 puis 1 937 jetons au plus (inventaires longs à deux couches élargies) ; les distractions d'un inventaire long ne s'élargissent plus.
- `date_maj` renommé `date_de_mise_a_jour` : le test le prenait pour un leurre de nom (`_maj`).

## Plan Change Log

## Review Triage Log

Boucle 0, relecture quick. Verdicts : high 0, medium 0, low 7, false 0, maybe-false 1. Corrigés 7, rejeté 1.

| Constat | Verdict | Route | Preuve / action |
|---|---|---|---|
| `main.rs` documente encore `--count 8000` | low | patch | 8600 |
| Commentaire de `MAX_LENGTH` (« moins de 1 400 jetons ») faux | low | patch | 1 773 mesurés, commentaire à jour |
| `with_decoys` compte `cleabs`/`code_postal` de remplissage | low | patch | ne compte que les couches non élargies |
| `wide_next` perdu sur les quartiers, doc inexacte | low | patch | doc : « la couche des communes » ; sans effet sur un thème sans remplissage, dit |
| `force` masque le paramètre `force` de `add` | low | patch | renommé `forced_wide` |
| `FILL_TEXT` identique à `DECOY` | low | patch | rôle `remplissage`, distinct des leurres |
| `symbole`, `idzone` du zonage proches du type de zone | maybe-false | patch | `symbole` (code de figuré tiré du type de zone) retiré ; `idzone` est un identifiant, gardé |
| Durée : risque de carte pleine, « 8 % » faux | low | patch | 7,5 % ; pic attendu dans la plage mesurée (1 768 jetons, 5,85 Gio), à vérifier par `--longs-d-abord` |

Constat ajouté hors relecture : deux couches élargies dans un inventaire long donnaient 1 937 jetons ; corrigé (voir notes).

## Verification

**Commands:**
- `cargo test -p cassini-training`, `cargo fmt --all --check`, `cargo clippy -p cassini-training --all-targets -- -D warnings` -- expected: vert.
- `python3 -I -m unittest discover -s training/entrainement/tests` -- expected: vert.
