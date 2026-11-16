---
title: 'La question du garde-fou dit qu''un point manque quand une reference ne designe pas un point'
type: 'bugfix'
ticket: '62'
created: '2026-11-16'
status: done
baseline_revision: '394c8944dd510e400488e7e86b9fcd240de68888'
route: 'oneshot'
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

**Problem:** Pour la demande de 11.14, le garde-fou refuse `observer = step:2.point` parce que l'etape 2 (`data.reproject`) ne produit aucun point, puis affiche « Il manque a la demande un point en coordonnees : quels sont x, y et le systeme ». Aucune coordonnee ne manque dans la demande : c'est une reference du plan qui ne designe pas un point.

**Approach:** Le garde-fou distingue deux causes : une coordonnee absente de la demande (question inchangee, `GUARD_QUESTION`) et une reference du plan vers un champ qui n'est ni un `Point` ni une reference MGRS (nouvelle question, qui le dit et ne demande ni x, ni y, ni systeme). Le refus ne change pas, seul le message. La source reste `guard`, la question finit par « ? ».

**Never:** assouplir un refus ; toucher au prompt, au juge, a la grammaire, a `training/`, a `GUARD_QUESTION` ; ajouter une etape au plan.

</frozen-after-approval>

## Implementation Notes

Oneshot : un seul fichier (`core/src/point.rs`, `invents_a_point` rend la cause au lieu d'un booleen, nouvelle constante de question, tests), moins de 100 lignes. Les tests existants de references refusees (`a_reference_to_a_field_that_is_not_a_mgrs_is_a_guard_question`, vers 1680) passent a la nouvelle question ; un test garde `GUARD_QUESTION` pour une coordonnee inventee ; un test rend la nouvelle question pour le plan du bug (reproject puis `step:2.point`).

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 0 high, 0 medium, 6 low.

| Verdict | Route | Evidence |
|---|---|---|
| low | patch (fait) | `guard_rejected` renvoie la nouvelle question sans test : test ajoute (plan du bug sur une sortie rejetee). |
| low | patch (fait) | Litteral de la demande du test avec ~13 espaces (continuation cassee) : corrige. |
| low | patch (fait) | Pas de ligne vide avant le premier test ajoute ; docs de `guard_coordinates` : les deux questions et l'ordre sont dits. |
| low | rejete | Un plan a la fois mal reference et a coordonnee inventee rend la question de la premiere cause rencontree : les deux sont des refus, « en cas de doute, c'est la question », et corriger l'une revele l'autre. Dit dans la doc. |
| low | rejete | Une reference mal formee ou vers l'avant rend aussi la question de la reference : « ne donne pas de point » reste vrai pour une etape qui n'en donne pas ; pas de troisieme message pour un cas que le modele ne produit pas. |

## Verification

**Commands:**
- `cargo test -p cassini-core point` -- expected: tests du garde-fou verts
- `cargo test -p cassini-ai plan` -- expected: tests de composition verts
- `cargo fmt --check` et `cargo clippy --all-targets -- -D warnings` -- expected: sans avertissement
