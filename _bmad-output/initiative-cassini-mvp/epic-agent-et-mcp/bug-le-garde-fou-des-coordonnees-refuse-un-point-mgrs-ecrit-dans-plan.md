---
title: 'Le garde-fou des coordonnees refuse un point MGRS ecrit dans la demande'
type: 'bugfix'
ticket: '60'
created: '2026-11-16'
status: done
baseline_revision: '5169921a5fd60b294e94c9115bb87f935546eb2c'
route: 'full'
route_source: 'auto'
risk: 'high'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-garde-fou-une-coordonnee-du-plan-vient-de-la-demande-jamais-plan.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** La demande « Convertis le point 32T KR 81280 88316 (MGRS) en WGS84, reprojette-le vers le systeme du MNT, calcule le champ de vision cumule depuis ce point et sors la carte en PDF A3 avec un carroyage UTM de 1 km » rend la question du garde-fou (« Il manque a la demande un point en coordonnees ») au lieu d'un plan. Le point MGRS est ecrit dans la demande.

**Approach:** Obtenir le plan reel que le modele compose pour cette demande, trouver la regle du garde-fou qui le refuse, et la corriger pour qu'un point venu de la demande, MGRS compris, passe le garde-fou sans affaiblir sa protection contre une coordonnee inventee.

## Boundaries & Constraints

**Always:** le garde-fou reste une verification pure sur le plan, sans etape ajoutee (AD-23 etendu) ; « en cas de doute, c'est la question » ; le noyau ne reprojette pas (AD-19) ; un test echoue sur le defaut et passe apres, avec un test pour chaque cas voisin touche ; relecture thorough (risque high).

**Never:** pas de correctif par phrase ; pas de changement du prompt, de `SHORT_SYSTEM`, du juge, de la grammaire ni de `training/` ; pas de journal de provenance (AD-11) ; pas de garde cote MCP ; pas de re-entrainement ni d'installation de modele. Si le plan reel montre que le garde-fou a raison (le modele invente une coordonnee), le ticket s'arrete avec ce constat et le proprietaire decide : le defaut n'est alors pas dans le garde-fou.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Point MGRS de la demande | la demande du bug, plan du modele | le plan passe le garde-fou | — |
| Coordonnee inventee | x et y du plan absents de la demande | question du garde-fou, comme avant | question |
| MGRS du plan absent de la demande | chaine MGRS differente de celle ecrite | question du garde-fou | question |
| Reference a une etape | point passe par `step:N.champ` du bon type | accepte (inchange) | — |

</frozen-after-approval>

## Code Map

- `core/src/point.rs` -- garde : `invents_a_point` (755-859), `walk` (628-688), `reference_is_a` (693-718), `written_mgrs` (517-584), `without_used_mgrs` (588), `request_numbers` (249-297), `utm_codes` (370) ; tests dans `mod tests` (869 et suivantes), `calls` (884), `is_guard` (888), `convert` (1506), tests MGRS 1555-1680.
- `ai/src/plan.rs` -- appelants `guard_coordinates` (743) et `guard_rejected` (753) ; tests vers 917, 1013, 1038.
- `core/src/commands/coord_convert.rs`, `terrain_viewshed_cumulative.rs` -- sorties `wgs84`, `utm`, `mgrs`, `point` ; `observers: Vec<Point>`.
- Hypotheses a trancher par le plan reel (non verifiees) : A, le modele ecrit un Point litteral (chiffres MGRS ou WGS84 calcules) dans la reprojection ou `observers` ; B, une reference `step:` mal typee (`step:1.point` vers `coord.convert`, ou `step:N.x` dans un tableau).
- Cas freres, hors correctif sauf s'ils partagent la meme racine : UTM « 31T 123456 4567890 » (aucune zone reconnue), degres-minutes.

## Tasks & Acceptance

**Execution:**
- [ ] `ai/src/plan.rs` ou un test du noyau -- obtenir le plan brut du modele installe pour la demande du bug (`CASSINI_TEST_MODEL`), le consigner dans les notes ; trancher A ou B ; hors du garde-fou, s'arreter et rendre la main
- [ ] `core/src/point.rs` -- corriger la regle refusant, sans autre changement de comportement
- [ ] `core/src/point.rs` (tests) -- le plan reel du bug passe ; une coordonnee inventee, un MGRS absent de la demande et une reference mal typee restent refuses

**Acceptance Criteria:**
- Given la demande du bug et le plan reel du modele, when le garde-fou le verifie, then il passe sans question.
- Given un plan dont une coordonnee n'est pas dans la demande, when le garde-fou le verifie, then il rend la question comme avant.
- Given le correctif, when les tests du noyau et de `ai` tournent, then ils passent, dont un test qui echoue sans le correctif.
- Given le noyau reconstruit et installe sur le poste, when la sonde de 11.14 est rejouee, then le garde-fou ne repond plus, et le proprietaire confirme.

## Implementation Notes

Plan reel du modele (2026-11-16, modele `ajuste-continu-5-58-1`, `cassini plan` avec l'inventaire d'Annecy, sortie brute relevee avant le garde-fou par un print temporaire, retire) :
`coord.convert {mgrs:"32TKR8128088316", to:["wgs84"]}` ; `data.reproject {layer_id:"step:1.point", target_epsg:2154}` ; `terrain.viewshed {dem:"mnt/mnt-74-25m.tif", observer:"step:2.point"}` ; `layout.export_pdf {grid:{epsg:32632, interval:{unit:"km",value:1}, kind:"projected"}, page_size:"a3", title:"..."}`.
Verdict : hypothese B. Le MGRS de l'etape 1 passe (cle `32TKR8128088316` retrouvee dans la demande). Le refus vient de `observer: "step:2.point"` : l'etape 2 est `data.reproject` (reprojection de couche, sortie `layer_id`, `name`, `feature_count`, `crs`, aucun `point`), donc la reference n'est pas un `Point`. Le garde-fou a raison de refuser : aucune coordonnee inventee, mais le modele a choisi la mauvaise commande (`data.reproject` au lieu de `data.transform_point`) et `terrain.viewshed` au lieu de `terrain.viewshed_cumulative`. Le defaut n'est pas dans le garde-fou : ticket arrete, aucun code modifie, le proprietaire decide (Never).

## Plan Change Log

## Review Triage Log

## Verification

**Commands:**
- `cargo test -p cassini-core point` -- expected: tests du garde-fou verts
- `cargo test -p cassini-ai plan` -- expected: tests de composition verts
- `cargo fmt --check` et `cargo clippy --all-targets -- -D warnings` -- expected: sans avertissement

**Manual checks (if no CLI):**
- Rejouer `outils-cassini/scripts/demo-11-14.ps1` apres reconstruction du noyau : plus de question `source: "guard"`.

**Cloture (2026-11-16).** Aucun code change : le garde-fou refuse a juste titre (le defaut est dans la composition du modele). Decision du proprietaire : entrainer (ticket 5.61, famille de la chaine point converti, reprojete, champ de vision cumule) et corriger le message du garde-fou (ticket 5.62). 11.14 attend 5.61.
