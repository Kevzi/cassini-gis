---
title: 'Générateur après quatre causes : nom recopié, reprojection, retrait de couche'
type: 'feature'
ticket: ''
created: '2026-11-10'
status: 'built'
baseline_revision: '5f01dbd'
route: 'full'
route_source: 'pinned'
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

**Problem:** `ajuste-quatre-causes` a reculé sur Annecy (1/5) : le champ de nom se déduisait du thème (`nom_<thème>`, le modèle a rendu `commune` au lieu de `nom_officiel`), seulement 58 exemples sur 8 000 portaient `label_field`, et les plans avec `data.reproject` étaient tombés de 318 à 294 ; `retrait_couche` ne connaissait ni raster, ni plusieurs couches, ni verbe seul.

**Approach:** décisions du propriétaire du 2026-11-10 : noms du rôle `nom` tirés d'un pool sans lien au thème, avec champs leurres (majuscules, codes) dans l'inventaire, et bien plus d'exemples `label_field` ; poids de la reprojection remonté (≥ 318 plans, ≥ 201 reprojection puis analyse) ; `retrait_couche` gagne le raster, plusieurs couches (une étape par couche) et le verbe seul → question ; deux décisions écrites dans la doc d'entraînement (barre Annecy ≥ 4/5, deux graines par nuit) ; script de nuit à deux graines préparé.

## Boundaries & Constraints

**Always:** graine du générateur inchangée, `training/tests/recouvrement.rs` vert ; le `label_field` attendu est le nom exact du champ de rôle `nom` de l'inventaire, jamais un leurre ; jeu de 8 000 exemples ; tableau des familles comparé à `ajuste-revue-11-23` (commit `47ef2f4`) rapporté.

**Never:** recopier une phrase du jeu tenu à l'écart, de `plan_reel` ou des recettes réelles (Annecy compris) ; « la couche des <pluriel> du projet », « ne sert plus » ; nouvelle dépendance ; changer la taille du jeu ; lancer un entraînement depuis ce plan.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Nom recopié | zones avec `nom_officiel` et leurre `nom_officiel_en_majuscules`, `code_insee` | `label_field: "nom_officiel"` | — |
| Pas de nom thématique | toute couche | aucun nom de champ `nom_<thème>` ni `nom_commune`, `nom_voie`, `nom_etab` | — |
| Retrait d'un raster | « retire le MNT du projet » | une étape `layer.remove` sur le MNT | — |
| Plusieurs couches | « retire les gares et les écoles du projet » | une étape par couche, dans l'ordre de la phrase | — |
| Verbe seul | « supprime les gares » | question : retirer la couche du projet ? | — |

</frozen-after-approval>

## Code Map

- `training/src/inventories.rs` -- `NOM` (l. ~153) et les `FieldDef` de rôle `nom` des communes (l. ~355), routes (l. ~508), cours d'eau (l. ~686), écoles (l. ~904) : un pool commun `NAME_FIELDS` ; `Builder::add` (l. ~1856) : retirer le suffixe `nom_<thème>`, ajouter des leurres juste après le champ de nom (rôle `leurre`, sans valeurs).
- `training/src/families.rs` -- `decompte_par_zone` (l. ~3157) : `label_field` trois fois sur quatre ; poids dans `FAMILIES` ; `reprojection_avant_analyse` poids 6 → 8 ; `retrait_couche` (l. ~5888) : branches raster, plusieurs couches, verbe seul ; test `a_label_field_is_…` (l. ~9931) réécrit ; tests de la famille.
- `docs/entrainement-du-modele.md` -- lignes de familles, décisions du 2026-11-10.
- `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- entrée « famille `retrait_couche` à compléter » close.
- Hors dépôt : `~/cassini-entrainement/nuit-11-10.sh`, `%TEMP%\essais-nuit-11-10.ps1`, `%TEMP%\recette-nuit-11-10\sonde.py` ; seconde graine par l'option `--graine-torch` de `training/entrainement/pipeline.py` (graine torch seule ; le générateur garde ses graines 1 et 2).

## Tasks & Acceptance

**Execution:**
- [x] `training/src/inventories.rs` -- pool de noms et leurres
- [x] `training/src/families.rs` -- `label_field`, poids, `retrait_couche`, tests
- [x] `training/entrainement/pipeline.py` -- graine torch réglable par `--graine-torch`, au rapport, reprise refusée sous une autre graine
- [x] `docs/entrainement-du-modele.md`, `deferred-work.md`
- [x] scripts de nuit hors dépôt

**Acceptance Criteria:**
- Given le jeu par défaut, when on le compare à celui d'`ajuste-revue-11-23`, then `data.reproject` ≥ 318, reprojection puis analyse ≥ 201, `label_field` ≥ 150, et le tableau des familles est rapporté.
- Given le dépôt, when `cargo test --workspace`, fmt et clippy, then tout passe, `recouvrement` compris.

## Implementation Notes

- Implémenté directement dans la session (contexte déjà chargé), sans sous-agent d'implémentation.
- Poids : `decompte_par_zone` 4 → 8 (nom trois fois sur quatre), `reprojection_avant_analyse` 6 → 8, `analyse_couche_metrique` 4 → 5 (contrepoids, ajouté après la première mesure : il tombait de 123 à 95), `retrait_couche` 3 → 6 (quatre formes : une couche vecteur 4/10, raster 2/10, plusieurs 2/10, verbe seul 2/10).
- Recouvrement : le nouveau tirage rendait F03 d'`epic11_reel` (« Fais un PDF de la carte avec un carroyage UTM au kilomètre. ») ; la tête devient « Fais-moi un PDF de la carte » devant un carroyage, comme pour S09 et F01. Graine inchangée.
- Graine de l'entraînement : `pipeline.py --graine-torch N` (défaut 1), gardée au rapport ; les graines du générateur (1 et 2) ne bougent pas.
- Mesure (graine 1, 8 000), revue `47ef2f4` → avant (`5f01dbd`) → après : `data.reproject` 318 → 308 → 351 ; reprojection puis analyse 201 → 187 → 246 ; `label_field` 41 → 64 → 173 ; `layer.remove` 0 → 86 → 131 (35 à plusieurs couches, 23 sur raster) ; questions du verbe seul 38. Le jeu garde 8 000 exemples (`TRAINING_COUNT`, décision du propriétaire en 5.15) : les autres familles perdent environ 6 % en part de tirage, et jusqu'à 25 % par le hasard du tirage ; tableau complet au rapport.

## Plan Change Log

## Review Triage Log

Boucle 0, relecture quick. Verdicts : high 0, medium 1, low 6, false 1, maybe-false 0. Corrigés 7, rejetés 1.

| Constat | Verdict | Route | Preuve / action |
|---|---|---|---|
| Leurre `code_insee` / `INSEE_COM` en double du vrai champ INSEE des communes | medium | patch | les codes leurres excluent tout nom d'un champ du thème, casse comprise |
| `--reprendre` sous une autre `--graine-torch` passe en silence | low | patch | reprise refusée (`PipelineStop`) si la graine du rapport diffère |
| Scripts de nuit sur un worktree pas encore remis sur le commit | false | rejet | voulu : les worktrees se remettent sur le commit au lancement de la nuit, après les tests ; `CASSINI-nuit` créé |
| Code Map : `CASSINI_TORCH_SEED` au lieu de `--graine-torch` | low | patch | plan corrigé |
| Entrée de `deferred-work.md` supprimée au lieu d'être close | low | patch | entrée rétablie avec `status: close` et `resolution:` (de même pour celle du refus de couche non enregistrée) |
| Test des causes après 11.23 compte `nom_` | low | patch | compte les `label_field` du pool `NAME_FIELDS` |
| Test « aucun nom thématique » trop étroit | low | patch | tous les thèmes à champ de nom, nom dans le pool, aucun doublon de nom (casse comprise) |
| Compte des leurres faussé par le vrai code INSEE | low | patch | liste des codes du test réduite aux leurres |

## Verification

**Commands:**
- `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` sous Windows -- expected: vert.
- `cargo run -p cassini-training --release -- --format chat --seed 1 --count 8000 --out …` puis la comparaison -- expected: seuils tenus.
