---
title: 'Famille d''entrainement : convertir un point puis le reprojeter, avant un champ de vision cumule'
type: 'feature'
ticket: '61'
created: '2026-11-16'
status: 'built'
baseline_revision: '302df024402ea12209f570fb1ee81434fdff3fb8'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/docs/entrainement-du-modele.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Pour la demande de la demonstration de 11.14 (point MGRS, reprojection vers le systeme du MNT, champ de vision cumule, PDF A3 carroye), le modele installe compose `data.reproject` (une couche) au lieu de `data.transform_point` (un point) et `terrain.viewshed` au lieu de `terrain.viewshed_cumulative` ; le garde-fou refuse a juste titre la reference `step:2.point`. Aucun exemple du generateur ne combine MGRS, champ de vision cumule et export PDF.

**Approach:** Lot 1 (cette session) : une famille du generateur qui ecrit la chaine `coord.convert`, `data.transform_point`, `terrain.viewshed_cumulative`, `layout.export_pdf`, avec des formulations variees, ses tests et sa doc. Lot 2 (session suivante) : une seance continue repart du modele installe avec cette famille ; elle finit avec le verdict du juge de non-oubli et les mesures habituelles (Annecy, gros projet, v4) ; rien n'est installe sans decision du proprietaire.

## Boundaries & Constraints

**Always:** la famille ne reprend aucune phrase des recettes tenues (annecy, gros-annecy v1 a v4, `ai/tests`) : le garde de recouvrement passe sans allonger `FOCUS_EXCLUDED` ; elle ne rejette aucun exemple ; le jeu par defaut garde ses parts (gros projets, planchers) ; chaque exemple passe la validation du registre et le garde-fou des coordonnees ; installation du modele sur decision du proprietaire seulement, juge « installable » et Annecy a 16/20 au moins.

**Never:** pas de reformulation de la demande de 11.14 pour la faire passer ; pas de changement du garde-fou, du prompt, du juge, de la grammaire ; pas de nouvelle commande ; pas de relevement de `DEFAULT_COUNT`, `FOCUS_COUNT`, `RECALL_FLOOR` ni `RECALL_VERSION` sans que le test l'exige ; la phrase exacte de 11.14 n'est pas copiee dans le generateur.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Exemple de la famille | tirage quelconque | plan : `coord.convert` (MGRS vers wgs84), `data.transform_point` (point `step:1.wgs84`, systeme du MNT), `terrain.viewshed_cumulative` (observers `["step:2.point"]`), `layout.export_pdf` (a3, grille projetee 1 km) | — |
| Recouvrement | famille tiree seule a `FOCUS_SEED` | aucune phrase egale a une recette tenue | le test de recouvrement echoue, la tournure fautive est retiree |
| Jeu par defaut | graine 1, 8 600 exemples | aucun rejet ; planchers et part des gros projets tenus | le plancher est ajuste comme en 5.58 si le test l'exige |

</frozen-after-approval>

## Code Map

- `training/src/families.rs` -- `Family`, `FAMILIES` (65, 512-525), modele `champ_de_vision_cumule` (~5464), `foreign_terrain_point` (~8167, MGRS puis `transform_call`), `transform_call` (~6489), `mgrs_ref` (~6452), grille projetee (`carroyage_utm_projet` ~2763), tests `valid(fn, 600)`.
- `training/src/inventories.rs` -- `add_dem`, `terrain_epsg` : le MNT et son systeme.
- `training/src/lib.rs` -- constantes (31, 323, 391) ; planchers du bloc « 5.58 » (~978-990) ; part des gros projets (~935, 1670-1690) ; `rejected == 0` (~999).
- `training/tests/recouvrement.rs`, `training/src/barres.rs` -- garde de recouvrement et sources tenues.
- `training/entrainement/pipeline.py:467` -- les noms de familles y sont lus par regex dans `families.rs` : respecter `Family { name: "x"`.
- `docs/entrainement-du-modele.md` -- tableau des familles (~109) et section datee (modele : « Trois familles (5.58) », ~781).
- Reference : commits `8caf85d` et `3cabf06` (ajout d'une famille).

## Tasks & Acceptance

**Execution:**
- [ ] `training/src/families.rs` -- fonction de la famille et entree dans `FAMILIES`, reutilisant `foreign_terrain_point`, `transform_call`, la grille projetee -- poids comparable aux familles voisines
- [ ] `training/src/families.rs` (tests) -- ordre des commandes, `step:1.wgs84`, `observers == ["step:2.point"]`, `a3`, grille 1 km, aucun rejet sur 600 tirages
- [ ] `training/src/lib.rs` -- plancher de la famille dans le bloc des planchers ; ajuster la part des gros projets seulement si le test l'exige
- [ ] `docs/entrainement-du-modele.md` -- ligne au tableau et section datee

**Acceptance Criteria:**
- Given le jeu par defaut (graine 1), when il est engendre, then la famille y figure au-dessus de son plancher et aucun exemple n'est rejete.
- Given la famille tiree seule, when le test de recouvrement tourne, then aucune phrase ne copie une recette tenue et `FOCUS_EXCLUDED` reste vide.
- Given un exemple de la famille, when le registre et le garde-fou des coordonnees le verifient, then il passe sans question.
- Given le correctif, when `cargo test -p cassini-training` tourne, then tout passe.

## Implementation Notes

Lot 1 fait le 2026-11-16 : famille `mgrs_vers_vision_cumulee_pdf` (poids 4), test sur 600 tirages, plancher de 40, doc. Tests de `cassini-training` verts, clippy et fmt propres. Lot 2 (seance continue, juge, mesures) : a faire.

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 0 high, 2 medium, 2 low.

| Verdict | Route | Evidence |
|---|---|---|
| medium | decision du proprietaire (a confirmer) | Le rapport questions / recopies du carroyage UTM passe de 50 a 48 % et la famille n'y est plus comptee : barre assouplie hors du plan. Consigne dans la doc ; le correctif de fond (poids de la famille) n'est pas sur d'en eviter une autre. |
| medium | patch (fait) | `conversion_format` reecrit pour « la couche des … » (collision avec DC1 apres decalage des tirages) : hors du plan, absent de la doc. Doc completee ; `FOCUS_EXCLUDED` reste vide. |
| low | rejete | Plancher de 40 hors de la boucle de 5.58 : structure sans effet ; valeur coherente avec le poids 4. |
| low | patch (fait) | Le test ne verifie pas que la grille est sur le fuseau du MNT : par construction elle tire un fuseau ; doc ramenee a « un fuseau UTM tire par le generateur ». |

## Verification

**Commands:**
- `cargo test -p cassini-training` -- expected: tout vert (recouvrement, planchers, familles)
- `cargo fmt --check` et `cargo clippy --all-targets -- -D warnings` -- expected: sans avertissement
- `python -I -m unittest discover -s training/entrainement/tests` -- expected: le pipeline lit toujours les familles
