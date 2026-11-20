---
title: 'Géocodage sur la Base Adresse Nationale, dans le noyau'
type: 'feature'
ticket: '16'
created: '2026-11-20'
status: 'built'
baseline_revision: 'ba45750dfbf42add7b444706cd10482a96376449'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick, edge-cases]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Un bureau d'études qui a un CSV d'adresses ne peut pas les placer sur la carte : QGIS ne fait qu'une jointure sur écriture identique, et aucune commande n'apparie « 12 av. de la Gare » à « 12 Avenue de la Gare ».

**Approach:** Ajouter `geocode.ban`, exécutée par le noyau (AD-26) : elle apparie un CSV d'adresses à un CSV de la BAN, écrit un seul CSV déterministe avec `lon` et `lat` en EPSG:4326, et rend `path`, `crs`, `x_field`, `y_field` que reprend un `layer.add` du plan par `step:k`.

## Boundaries & Constraints

**Always:** Entrées par `path_params` (`addresses`, `ban`) : le journal garde déjà taille et date, le rejeu signale un fichier changé (AD-11). Commande `runs_in: core`, `cancellable(false)` (AD-12), sortie `.csv` nommée par le noyau (AD-10). BAN lue comme `adresses-XX.csv` décompressé : séparateur `;`, UTF-8, colonnes `numero`, `rep`, `nom_voie`, `code_postal`, `nom_commune`, `lon`, `lat` (constaté sur `adresses-2A.csv`, 2026-11-20). Appariement déterministe : accents, casse, ponctuation et abréviations usuelles (av, bd, rue, chemin, imp, all, pl, st/ste) normalisés ; le code postal ou la commune, s'ils sont donnés, départagent. Une adresse sans correspondance unique reste dans le CSV de sortie, `lon` et `lat` vides, comptée dans `unmatched_count`. Les fichiers d'entrée ne sont jamais modifiés. Aucune reprojection (AD-19) ; un seul format lu : le CSV.

**Décisions du propriétaire (2026-11-20) :** lecture par la crate `csv` (Unlicense OR MIT), ajoutée à `core/Cargo.toml` avec commentaire daté ; recette sur fixtures synthétiques seules, aucun département BAN déposé dans `../donnees-test-cassini/`.

**Never:** Réseau, téléchargement ou décompression du `.gz` ; lecture d'un autre format ; flou typographique (fautes de frappe) ; toucher `layer.add`, `RasterPath` ou le plugin QGIS ; ré-entraîner le modèle (groupé au ticket 17) ; ajouter une crate sans validation du propriétaire (AD-16).

## I/O & Edge-Case Matrix

| Scénario | Entrée / État | Résultat attendu | Erreur |
|---|---|---|---|
| Exacte | « 12 Avenue de la Gare, 74000, Annecy » | `lon`/`lat` de la BAN | aucune |
| Autrement écrite | « 12 av. de la gare » sans accents ni casse | même point | aucune |
| Introuvable | rue absente de la BAN | ligne gardée, `lon`/`lat` vides, comptée | aucune |
| Ambiguë | même texte, deux communes, sans filtre | non appariée | aucune |
| Rejeu | `addresses` modifié depuis le journal | rejeu s'arrête, fichier signalé | `Diverged` existant |
| Colonne absente | `address_field` inconnu | refus nommant la colonne | code de paramètre invalide |
| Déterminisme | deux exécutions | CSV identique octet pour octet | aucune |

</frozen-after-approval>

## Code Map

- `core/src/commands/geocode_ban.rs` (nouveau) -- modèle : `terrain_slope.rs` (`Command`, `Params`/`Output`, `inventory::submit!`, `.runs_in(Core).cancellable(false).path_params(&["addresses","ban"]).output_params(&["out"]).output_ext(".csv").example().tests()`) ; `mod.rs` automatique (build.rs)
- `core/src/commands/test_support.rs` -- `fixture()`, `Scratch` ; chaque test de `.tests()` doit exister
- `core/src/journal/mod.rs` (1313-1370, 1572-1608, 2005) -- empreinte et contrôle au rejeu déjà faits, ne rien changer
- `core/src/plan.rs` (370-415, 515) -- `check_reference` compare `normalize(expected)` et le champ de sortie : `String` contre `Option<String>` (`layer.add.path`, `crs`, `x_field`, `y_field`) risque `plan.type_mismatch` ; à constater par un test, corriger par un type nul accepté si besoin
- `_bmad-output/.../spec-cassini-mvp/catalogue-des-commandes.md` (2275-2310) -- fiche `planifiee` → `faite` (`core/tests/catalogue.rs`)
- `fixtures/` -- créer `adresses-test.csv`, `ban-test.csv` synthétiques (format BAN réel), les décrire dans `fixtures/README.md`
- `core/Cargo.toml`, `deny.toml` -- ajouter `csv` (accord du 2026-11-20), vérifier `cargo deny`
- `ai/`, `training/src/families.rs`, `docs/couverture-phrases-reelles.md` (CV35) -- l'agent la voit par le registre ; test d'un plan `geocode.ban` puis `layer.add` ; famille d'exemples hors ticket (17)
- À ne pas toucher : `qgis-plugin/` (commande du noyau, rien à déléguer)

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/geocode_ban.rs` -- lecture CSV, normalisation, appariement, écriture déterministe, résultat typé, tests de la matrice -- contrat du registre
- [ ] `fixtures/adresses-test.csv`, `fixtures/ban-test.csv`, `fixtures/README.md` -- jeu synthétique -- tests sans QGIS
- [ ] `core/src/plan.rs` -- seulement si le test de chaîne `step:k.path` échoue : accepter une sortie `String` dans un paramètre facultatif -- plan valide
- [ ] `core/src/journal/tests.rs` -- rejeu identique, puis rejeu après modification d'un CSV d'entrée signalé -- AD-11
- [ ] catalogue-des-commandes.md -- fiche `faite`, `README` et docs -- cohérence
- [ ] `ai/tests/plan_reel.rs` ou test voisin -- l'agent compose `geocode.ban` puis `layer.add` par `step:1.*` dans un plan valide

**Acceptance Criteria:**
- Given la fixture BAN synthétique, when `geocode.ban` s'exécute, then exactes, autrement écrites et introuvables donnent le résultat de la matrice et les deux fichiers d'entrée restent intacts.
- Given un plan `geocode.ban` puis `layer.add` avec `step:1.path`, `crs`, `x_field`, `y_field`, when le noyau le valide, then il est accepté.
- Given un journal rejoué après modification d'un CSV d'entrée, when on rejoue, then le rejeu le signale.

## Implementation Notes

`core/src/commands/geocode_ban.rs` (commande, 11 tests de la matrice), crate `csv` 1.x, fixtures `adresses-test.csv` et `ban-test.csv`. `plan.rs` : le test de chaîne a échoué comme prévu (`String` contre `Option<String>`), d'où `accepts()` ; la règle « exactement un de » de `layer.add` refusait aussi le plan une fois la référence ôtée, d'où une seconde passe où une chaîne tient lieu de chaque paramètre texte libre référencé (elle n'élargit que ce qui était refusé pour cette seule raison). Hors liste du plan, nécessaires pour garder les tests verts : `ai/tests/common/mod.rs`, jeux de phrases `couverture_reelle` (CV35 devient un plan), `plan_reel` (PL22 remplacé) et `par_commande` (GC1 à GC3), `cli/tests/parity.rs`, `training/src/barres.rs` et le texte de refus `NO_GEOCODING` de `families.rs` (une adresse isolée seulement) ; aucune famille d'exemples ni ré-entraînement, ticket 17. `path` rend le chemin de sortie du dossier de travail tel que le noyau l'a nommé, comme `terrain.slope` ; aucun essai sous QGIS (rien n'est délégué).

## Plan Change Log

## Review Triage Log

Passe 1 (quick + cas limites, deux lentilles pour un risque `high`) : 3 constats corrigés, 2 écartés pour cause, le reste différé ou rejeté.

| Verdict | Route | Constat et preuve |
|---|---|---|
| medium | patch | Sortie écrite avec le séparateur de l'entrée : une entrée à tabulation donne un CSV que `layer.add` (`delimiter=,;`) ne lit pas ; la sortie passe en `;` |
| medium | patch | « 12, rue de la Gare » (virgule collée au numéro) donnait une adresse sans voie, donc non appariée en silence ; la virgule est ignorée, test ajouté |
| low | patch | `Ÿ` majuscule non repliée et accents combinants (texte décomposé) devenus des espaces (« e glise ») ; repli ajouté, test ajouté |
| low | defer | « bis » contre « b », lettre séparée, « 12-14 », code postal « 74000.0 », commune déléguée, lon/lat comparés en texte : adresse non appariée, jamais un mauvais point ; consigné dans deferred-work |
| low | defer | messages trompeurs (CSV vide ou irrégulier) et lignes BAN illisibles écartées sans compte ; consigné |
| false | rejeté | Seconde passe de `plan.rs` « élargit tout » : un test négatif (`path: step:k` avec un `table` littéral) passe déjà avant ce ticket, car la validation ôtait la référence ; la seconde passe n'accepte que ce que la première refusait pour la seule absence d'un paramètre référencé |
| false | rejeté | `accepts()` laisse passer une chaîne dans tout `Option<String>` : `check_reference` ne contrôle que la forme du type, jamais le sens (chemin, SCR), et ne l'a jamais fait |
| false | rejeté | `path` pas « relatif au projet » : même forme que `terrain.slope` (`file`), le journal le relativise ; `layer.add` reçoit le chemin du noyau |
| low | rejeté | Fichiers hors liste du plan (`ai/tests`, `families.rs`, `barres.rs`) : exigés par les tests existants, sans ré-entraînement ; noté dans Implementation Notes |
| low | rejeté | `out` égal à une entrée, relance sur sa propre sortie (`lon` déjà présent, refusé), écriture partielle : `out` est donné par le journal, refus voulu, commande non annulable |
| low | rejeté | Docs `couverture-phrases-reelles.md` : chiffres de tête non remesurés, dit dans la note ajoutée ; la mesure est celle du ticket 17 |

## Design Notes

Rejeu de `step:k.path` : seul un `RasterPath` entier devient un repère `out:<j>.1` ; ici `path` rend le nom du CSV du dossier de travail, que le rejeu relit tel quel. Le CSV étant déterministe et déclaré en entrée de `layer.add`, un changement est détecté.

Plan type : `[{"command":"geocode.ban","params":{"addresses":"adresses.csv","ban":"adresses-74.csv","address_field":"adresse","postcode_field":"cp"}},{"command":"layer.add","params":{"path":"step:1.path","crs":"step:1.crs","x_field":"step:1.x_field","y_field":"step:1.y_field"}}]`

## Verification

**Commands:**
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement
- `cargo test -p cassini-core -p cassini -p cassini-ai --locked` -- expected: vert
