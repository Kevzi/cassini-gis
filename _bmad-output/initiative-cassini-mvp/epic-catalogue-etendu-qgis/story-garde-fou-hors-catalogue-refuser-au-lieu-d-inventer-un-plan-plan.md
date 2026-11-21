---
title: 'Garde-fou hors catalogue : refuser au lieu d''inventer un plan'
type: 'feature'
ticket: '35'
created: '2026-11-21'
status: 'built'
baseline_revision: '330f7000a787983d4e780cdd8b3e2c094de247ff'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Quand la demande porte sur une opération absente du catalogue, le modèle rend un plan inventé (CV39 étiquettes, CV40 découpe du MNT, CV41 profil d'altitude), un plan invalide (CV42, S24) ou une question de coordonnées (PL22), au lieu de refuser.

**Approach:** Un garde-fou déterministe dans le noyau (`core`), sans ré-entraînement : une table de familles d'opérations absentes (mots-clés de la demande vers mot-clé `missing`) qui refuse avec `Plan::Refusal`. Une famille ne vaut que tant qu'aucun identifiant du registre ne porte son mot-clé `missing` : si le catalogue l'acquiert, le garde se tait.

## Boundaries & Constraints

**Always:** le garde ne change ni le catalogue ni ne crée de commande ; il est pur (demande + registre), sans réseau ; l'ordre de ses motifs est fixe ; un refus nomme l'opération absente en français ; tout nouveau motif a un test positif ET un test négatif sur une phrase couverte voisine ; le garde s'applique avant toute génération du modèle (aucun appel inutile).

**Never:** pas de ré-entraînement ni de famille d'exemples de refus (réservé au ré-entraînement unique de fin d'epic 10) ; pas de refus d'une phrase couverte (CV hors liste, plans conformes : 44/48 de `plan_reel` et 32/44 de couverture agent ne baissent pas) ; pas de modification des phrases attendues ni des fixtures pour faire passer ; pas de dépendance nouvelle.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Absente | CV39, CV40, CV41, CV42, PL22, S24 | `AgentError::Refused` nommant l'opération (`label`, `clip`, `profile`, `raster_calculator`, `watershed`, `visibility_stat`) | Aucun appel au modèle |
| Couverte voisine | « Découpe les parcelles par la commune », « Soustrais la zone bâtie des parcelles » (analyses vectorielles) | Garde silencieux, le modèle compose | — |
| Déjà refusée | CV37, CV43 | Refus du modèle inchangé | — |
| Catalogue étendu | registre contenant un id portant `clip` | Garde de la famille `clip` muet | — |

</frozen-after-approval>

## Code Map

- `core/src/point.rs`, `core/src/attribute_guard.rs` -- modèles de garde pur à imiter (style, doc, tests) ; `core/src/lib.rs` -- déclarer et exporter le nouveau module.
- `core/src/plan.rs:34` -- `Plan::Refusal { reason }` ; pas de champ `missing` à l'exécution (il n'existe que dans les fixtures attendues).
- `ai/src/plan.rs:765-831` -- `compose_plan_anchored_cut` : appeler le garde juste après le contrôle de demande vide, avant `plan_grammar_built` ; un `Plan::Refusal` devient `Err(AgentError::Refused(reason))`.
- `ai/tests/couverture_reelle/phrases.json`, `ai/tests/plan_reel/phrases.json`, `tests/epreuve-modele-local/sentences.json` -- phrases de mesure, à ne pas toucher.
- `ai/tests/common/mod.rs:282` `refusals_now_wrong` -- vérifie que les mots-clés `missing` ne sont pas dans les ids du registre ; même règle pour le garde.
- `docs/couverture-phrases-reelles.md:69-99` -- commandes de mesure et relevé de `base-17-1` ; y ajouter le relevé.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/out_of_catalogue.rs` -- `guard_out_of_catalogue(registry, request) -> Option<String>` : table de six familles (étiquette/label, découpe du MNT/emprise, profil d'altitude, soustraction de rasters, bassin versant, moyenne/statistique par zone d'un champ de vision), normalisation sans casse ni accents, mutisme si un id du registre porte le mot-clé -- garde déterministe sans re-entraînement.
- [x] `core/src/lib.rs` -- déclarer le module -- accès depuis `ai`.
- [x] `ai/src/plan.rs` -- appel du garde avant la génération ; refus en `AgentError::Refused` -- PL22 compte comme refus vrai.
- [x] Tests unitaires (`core`, motifs positifs et voisines couvertes ; `ai`, générateur factice jamais appelé) -- CV39-42, PL22, S24 refusés ; couvertes voisines intactes.
- [ ] `docs/couverture-phrases-reelles.md` -- relevé mesuré sur le modèle réel installé (jamais écrit à la main).

**Acceptance Criteria:**
- Given CV39, CV40, CV41, CV42, PL22 et S24, when l'agent compose, then il rend un refus sans plan et sans appel au modèle.
- Given CV37 et CV43, when l'agent compose, then ils restent refusés.
- Given `plan_reel` et la couverture agent mesurées sur un modèle réel, when le garde est actif, then au moins 44/48 et 32/44.
- Given `cargo test -p cassini-training --test recouvrement`, then vert.

## Implementation Notes

## Plan Change Log

## Review Triage Log

| Verdict | Route | Constat |
|---|---|---|
| high | patch | RC2 « Découpe le raster de coût en deux classes » refusée par la famille `clip` (vérifié) ; emprise désormais exigée, test de corpus ajouté (`ai/tests/garde_hors_catalogue.rs`). |
| medium | patch | `label` et `visibility_stat` trop larges (« étiquette » de légende, « moyens ») ; contenu à afficher exigé, `moyen` retiré, voisines couvertes ajoutées aux tests. |
| medium | defer | Relevé réel `plan_reel` 44/48 et couverture 32/44 non mesuré (GPU, modèle installé) : à faire avant de clore le ticket. |
| low | rejeté | CV37/CV43 non rejoués sur le vrai jeu : le garde se tait dessus (test de corpus sur toutes les phrases non refusées). |
| false | rejeté | Câblage non vérifié : `compose_plan` appelle `compose_plan_anchored_cut`, test `an_absent_operation_is_refused_without_calling_the_model` à 0 appel. |

## Design Notes

Choix tranché : garde du noyau seul. Une table de familles absentes est plus fiable qu'un entraînement, ne coûte pas une nuit ni un juge de non-oubli, et se mesure sur le modèle installé. Le risque (refuser à tort une phrase couverte) est borné par des motifs étroits (verbe + objet) et des tests de voisines couvertes ; les familles de refus du générateur restent possibles au ré-entraînement de fin d'epic 10.

## Verification

**Commands:**
- `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` -- expected: sans avertissement
- `cargo test -p cassini-core -p cassini-ai` -- expected: vert
- `cargo test -p cassini-training --test recouvrement` -- expected: vert
- `cargo test -p cassini-ai --features gpu --test plan_reel -- --ignored --nocapture` puis `couverture_reelle_reel` (voir docs/couverture-phrases-reelles.md) -- expected: >= 44/48 et >= 32/44
