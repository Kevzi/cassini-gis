---
title: "Garde-fou 5.57 : un champ absent d'une couche détaillée de la vue triée donne une question"
type: 'bugfix'
ticket: ''
created: '2026-11-13'
status: 'built'
baseline_revision: '99706ed9d24d928925f550cec4c7ae520cab05a7'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter','edge-case-hunter','verification-gap','intent-alignment']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le garde-fou des attributs (5.57) ne contrôle un champ absent que hors vue triée, parce que la vue triée d'un gros projet coupe les champs d'une couche détaillée sans le dire. Sur le projet de 63 couches, le plan de la phrase GA06 de la recette 5.50 écrit `cpx_classement_administratif` sur `route_numerotee_ou_nommee`, qui n'a pas ce champ, et l'utilisateur reçoit un plan qui échouera ou rendra une carte vide.

**Approach:** `capped` (vue triée) note les couches dont elle a retiré des champs ; cette liste voyage avec `ModelInventory` jusqu'au garde-fou, sans rien changer au texte que le modèle lit. Le garde-fou contrôle alors un champ absent sur une couche détaillée **non coupée**, et ne le fait jamais sur une couche coupée.

## Boundaries & Constraints

**Always:** le texte de l'inventaire montré au modèle reste identique à l'octet (aucune clé nouvelle : le modèle est entraîné sur cette forme) ; en cas de doute, pas de question (une couche dont on ignore si elle est coupée n'est pas contrôlée) ; la question reste celle de `field_question`, provenance `guard` ; les appelants qui ne donnent pas la liste (inventaire écrit à la main, `cassini plan`) gardent exactement le comportement actuel ; AD-23 et AD-11 respectés (la question n'est pas journalisée).

**Never:** pas de clé ajoutée à l'inventaire du modèle ; pas de changement des plafonds (30 champs, 3 000 caractères) ; pas de redemande au modèle ; pas de dépendance nouvelle ; ne pas toucher à `training/` ni au prompt.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Champ absent, couche entière | vue triée, couche non coupée, champ inconnu | question `guard` nommant couche, champ, champs | — |
| Champ absent, couche coupée | vue triée, couche dans la liste des coupées | plan inchangé | — |
| Champ présent | vue triée, champ dans `fields` | contrôle de valeur comme aujourd'hui | — |
| Liste non fournie | `index` présent, aucune liste (appelant sans elle) | plan inchangé (comportement actuel) | — |
| Vue non triée | pas de `index` | question comme aujourd'hui, liste ignorée | — |
| Couche non détaillée | couche seulement dans `index` | plan inchangé | — |

</frozen-after-approval>

## Code Map

- `core/src/agent_inventory/sorted_view.rs:358` -- `capped` : dit si elle a retiré des champs ; l'appelant (`:199`) range l'identifiant court de la couche dans l'ensemble des coupées ; `ModelInventory` construit à `:214`.
- `core/src/agent_inventory.rs:417,458` -- `ModelInventory` : ajouter `cut_layers: BTreeSet<String>` (identifiants tels que montrés au modèle) ; vide dans la vue non triée.
- `core/src/attribute_guard.rs:44,80-110` -- `guard_attributes` et `fault` : le test `inventory.get("index").is_some()` devient « et la couche n'est pas connue comme entière ». Garder `guard_attributes` (3 arguments, ancien comportement) et ajouter `guard_attributes_with_cuts(.., cut_layers: Option<&BTreeSet<String>>)`.
- `ai/src/plan.rs:679,720` -- `compose_plan_anchored` appelle le garde-fou ; ajouter une variante qui reçoit la liste, `compose_plan_anchored` la délègue avec `None`.
- `cli/src/main.rs:376,1644` -- appel réel : passer `view.cut_layers` ; l'autre construction de `ModelInventory` laisse l'ensemble vide.
- `core/src/serve.rs:1818` -- compositeur de test : passer la liste de la vue reçue.
- Ne pas toucher : `ai/src/plan.rs` prompt (`inventory_parts`), `training/`.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/agent_inventory.rs`, `core/src/agent_inventory/sorted_view.rs` -- `cut_layers` rempli par `capped` -- source de vérité de la coupe
- [ ] `core/src/attribute_guard.rs` -- `guard_attributes_with_cuts` et contrôle du champ absent sur couche entière -- cœur du correctif
- [ ] `ai/src/plan.rs`, `cli/src/main.rs`, `core/src/serve.rs` -- faire passer la liste jusqu'au garde-fou
- [ ] tests dans `attribute_guard.rs` (une ligne de la matrice chacune) et `sorted_view.rs` (une couche coupée est dans `cut_layers`, une couche entière non ; le texte de la vue est inchangé par rapport à avant)

**Acceptance Criteria:**
- Given une vue triée de 63 couches et un plan qui écrit un champ inexistant sur une couche entière, when le garde-fou s'applique, then le plan devient une question `guard`.
- Given la même vue et un champ absent sur une couche coupée, when le garde-fou s'applique, then le plan est inchangé.
- Given les recettes existantes (`cargo test -p cassini-core -p cassini-ai`), when elles tournent, then elles passent sans changement d'attendus et le prompt de la vue triée est identique à l'octet.

## Implementation Notes

- Implémenté par un sous-agent selon le plan : `capped` rend `(Value, bool)`, `ModelInventory.cut_layers`, `guard_attributes_with_cuts`, `compose_plan_anchored_cut`, `ChosenModel::compose_cut` ; `guard_attributes` et `compose_plan_anchored` gardent leur comportement (`None`). Texte du modèle inchangé : la liste n'est pas dans le JSON de l'inventaire.
- Preuve réelle (2026-11-13) : recette v2 du gros projet, GPU, noyau reconstruit. GA06 donne maintenant la question `guard` « La couche route_numerotee_ou_nommee n'a aucun champ cpx_classement_administratif (champs : …) ». Score inchangé, 15/20 (GA06 reste raté au décompte : question sur phrase claire).
- Vérification : `cargo test -p cassini-core -p cassini-ai` vert, clippy et fmt sans écart.

## Plan Change Log

## Review Triage Log

Passe 1 (2026-11-13), quatre lentilles (blind-hunter, edge-case-hunter, verification-gap, intent-alignment). Verdicts : high 0, medium 1, low 2, false 5.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Aucun test ne relie la vue triée réelle au garde-fou (liste et `index` écrits à la main) | medium | patch | test `the_real_view_drives_the_attribute_guard` ajouté |
| Commande de vérification du plan invalide (deux filtres) | low | patch | corrigée |
| Câblage de `ModelComposer` (`Some(&view.cut_layers)`) sans test dans `cli` | low | defer | prouvé par la recette réelle (GA06 devient une question `guard`) ; un test d'intégration `cli` reste à écrire |
| `is_none_or` demande Rust 1.82 | false | — | `rust-version = "1.89"` au `Cargo.toml` |
| Couche coupée sans `layer_id` non enregistrée | false | — | le garde-fou retrouve la couche par `layer_id` ; sans lui il ne la contrôle pas |
| Champs retirés après `capped` non suivis | false | — | le budget n'écarte que des couches entières (`continue`), aucun retrait de champ après `capped` |
| Texte du modèle pas prouvé identique à l'octet | false | — | la liste est un champ de `ModelInventory`, jamais dans le JSON ; les tests d'octets de la vue passent sans changement |
| Appelants restés sur `None` (`cassini plan`, inventaires écrits à la main) | false | — | exigé par le plan : comportement actuel conservé |

## Design Notes

Une couche « coupée » se reconnaît mal de l'extérieur : `capped` s'arrête au premier champ qui dépasse un plafond, et ce champ peut être grand ; aucune règle sur la taille de la couche montrée ne sépare sûrement les deux cas. D'où la liste produite par `capped` elle-même. Le score de la recette 5.50 ne monte pas avec ce correctif : GA06 passera de « plan non conforme » à « question sur une phrase claire », ce qui protège l'utilisateur d'un plan faux.

## Verification

**Commands:**
- `cargo test -p cassini-core -- attribute_guard sorted_view` -- expected: vert
- `cargo test -p cassini-core -p cassini-ai` -- expected: vert
- `cargo clippy --workspace --all-targets` -- expected: aucun avertissement
- `cargo fmt --check` -- expected: aucun écart
