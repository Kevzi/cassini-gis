---
title: 'Contrat de réseau unique et isochrones paramétrées'
type: 'feature'
ticket: '6'
created: '2026-11-19'
status: 'built'
baseline_revision: 'c34b0d161cab32e24a070ecb2f4f63c03889533d'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [quick, edge-cases]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `analysis.isochrone` garde en dur dans l'extension les vitesses (marche 5 km/h, voiture 50 km/h), les deux sens et l'accrochage de l'origine à 100 m. AD-28 veut un seul contrat de réseau, défini une fois en Rust, que liront aussi `network.shortest_path` (13) et `network.prepare` (14).

**Approach:** Un type Rust `NetworkOptions` (`speed`, `speed_field`, `direction`, `tolerance`), tel que fixé par le catalogue des commandes (« Contrat de réseau (AD-28) »), entre dans `analysis.isochrone` comme paramètre facultatif `network`. Le noyau résout les valeurs par défaut et les écrit dans les paramètres envoyés et journalisés, comme il le fait pour `work_epsg` ; l'extension les lit et n'a plus aucune constante.

## Boundaries & Constraints

**Always:** type défini une seule fois, sans s'inscrire au registre ; champs facultatifs, forme exacte du catalogue (`speed {value > 0, unit "km/h"}`, `speed_field` prioritaire sur `speed`, `direction {field, forward, backward}`, `tolerance: Distance`) ; défauts (5 marche, 50 voiture, deux sens, 100 m) dans le schéma et le code Rust, jamais dans la ligne de table ni dans une constante Python ; un plan sans `network` donne le résultat d'aujourd'hui ; fiche `analysis.isochrone` du catalogue mise à jour (`network` passe de `planned_params` à `params`).

**Never:** changer la chaîne (`native:serviceareafrompoint` puis `native:convexhull`), `chain_version`, le résultat, les gardes ni le plan `work_epsg` du ticket suivant ; modifier `Distance` autrement que pour le partager ; dupliquer `Distance`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Sans `network` | plan d'aujourd'hui | mêmes appels QGIS qu'avant (5 ou 50 km/h, deux sens, 100 m) | — |
| Vitesse donnée | `speed` 20 km/h | `DEFAULT_SPEED` 20, résultat différent de la marche | `value` <= 0 refusé par le schéma |
| Champ de vitesse | `speed_field` | `SPEED_FIELD` posé, prioritaire sur `speed` | champ absent de la couche refusé |
| Tolérance | `tolerance` 250 m | accrochage de l'origine à 250 m | origine plus loin : refus avant lancement |
| Sens | `direction` avec champ et valeurs | `DIRECTION_FIELD` et valeurs transmis | champ inconnu refusé |
| Rejeu | journal avec valeurs résolues | mêmes valeurs rejouées | — |

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_isochrone.rs` -- params, descriptions FR/EN (texte « vitesses fixes… 100 m »), tests `.tests(&[…])` ; ajouter `network: Option<NetworkOptions>`.
- `core/src/commands/network_options.rs` (nouveau, déclaré dans `commands/mod.rs`) -- `NetworkOptions` ; modèle `summary_row.rs` ; réutilise `Distance` (`analysis_buffer.rs` l.5-60), à partager sans le dupliquer.
- `core/src/registry.rs` -- mécanisme de remplissage de `work_epsg` (`work_crs_param`, l.147-151, 329) : modèle pour écrire les valeurs résolues dans les paramètres.
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` -- l.205-227 (type) et fiche l.656-686 ; `core/tests/catalogue.rs` compare le registre à la fiche.
- `qgis-plugin/cassini_qgis/analysis.py` -- `translate_isochrone` (l.149), constantes l.132-135, `_check_origin` (l.1384), en-tête l.470-480 : lire `network`, supprimer les constantes.
- `qgis-plugin/tests/test_analysis.py` -- `IsochroneTest` (l.869-1010), `FakeBackend.near_call` ; `probe_isochrone.py`, `fixtures/reseau-grille-2154.geojson`.
- À ne pas toucher : `fixtures/resultats-extension.json`, `table_annulable.json`, `ai/`, `training/` (champ facultatif).

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/network_options.rs`, `commands/mod.rs` -- type partagé, schéma fermé, tests de schéma et d'aller-retour
- [ ] `core/src/commands/analysis_isochrone.rs`, `core/src/registry.rs` -- champ `network`, résolution des défauts écrite dans les paramètres, descriptions mises à jour, tests (défauts du schéma, plan sans `network` inchangé)
- [ ] catalogue des commandes -- fiche `analysis.isochrone` avec `network`, type `NetworkOptions`
- [ ] `qgis-plugin/cassini_qgis/analysis.py`, `tests/test_analysis.py` -- lecture de `network`, plus de constante ; tests : vitesse, champ de vitesse, sens, tolérance changent l'appel et le résultat ; ligne de table sans 5, 50 ni 100
- [ ] `qgis-plugin/tests/probe_isochrone.py` -- cas avec `network.speed` et `tolerance` sous vrai QGIS

**Acceptance Criteria:**
- Given une isochrone avec une vitesse et une tolérance données par le plan, when on l'exécute, then le résultat change.
- Given un plan sans `network`, when on l'exécute, then le résultat est celui d'avant.
- Given les valeurs par défaut, when on lit le code, then elles vivent dans le schéma Rust et pas dans la table.

## Implementation Notes

- Le contexte `architecture-noyau-et-facade-qgis.md` n'a pas été chargé par l'implémenteur ; AD-28 est résumé dans le catalogue des commandes, que le plan suit.
- `NetworkOptions::resolve(mode)` est idempotent. Le contrat résolu est écrit dans les paramètres journalisés (`fill_network`) et, pour les journaux anciens sans `network`, au rejeu (`resolve_network`).
- `TOLERANCE` de `native:serviceareafrompoint` reste 0 : `network.tolerance` ne règle que le contrôle d'accrochage de l'origine.

## Plan Change Log

## Review Triage Log

Passe 1 (lentilles : quick, edge-cases). Verdicts : 1 high, 1 low patchés, 3 medium/low différés, 9 rejetés.

| Verdict | Route | Constat et preuve |
|---|---|---|
| high | patch (fait) | Une ligne d'isochrone écrite avant ce ticket n'a pas de `network` ; `Replay::run` n'appelait jamais `fill_network` (seulement `Session::call_from`, `journal/mod.rs`), donc l'extension la refusait en `analysis.bad_params` et le rejeu divergeait. Le rejeu résout maintenant le contrat (`resolve_network`, fonction libre partagée) ; test d'un journal ancien rejoué à l'identique. |
| low | patch (fait) | Le commentaire de `fill_work_crs` était séparé de sa fonction par l'insertion de `fill_network` ; remis en place. |
| medium | defer | Le modèle voit le paramètre `network` dans le prompt et la grammaire, sans exemple ni entraînement : il peut en émettre sans que la phrase le demande. Non mesuré ; à traiter au ré-entraînement de fin d'epic. |
| low | defer | Le résumé (`summary.py::_isochrone`) décrit « à pied » ou « en voiture » même avec une vitesse donnée par `network`. |
| low | defer | `contract.rs` ne vérifie pas le nom du paramètre `network_param` (une faute de frappe n'apparaît qu'à l'exécution), contrairement à `check_work_crs_param`. |
| low | rejeté | `fill_network` avale l'erreur de désérialisation (`unwrap_or_default`) : inatteignable, `validate_params` vient de valider le même type. |
| low | rejeté | `tolerance` ne règle que l'accrochage de l'origine, `TOLERANCE` de l'algorithme reste 0 : conforme au ticket (« accrochage de l'origine ») et à la matrice du plan. |
| low | rejeté | `speed_field` vide, `direction` à champs vides ou égaux, tolérance énorme : cas improbables ; le champ inconnu est refusé avant lancement. |
| low | rejeté | Sonde : `check_tolerance` ne compare pas d'étendue et un commentaire est inexact ; l'effet observable (100 m refusé, 250 m accepté) est testé. |
| false | rejeté | `chain_version` à 1 : la chaîne ne change pas, seuls ses paramètres ; l'empreinte du registre n'est pas comparée au rejeu. |
| false | rejeté | `_check_network` avant `_check_origin`, formatage `%d` de la distance, conversion locale `m/km` : sans effet sur un réseau en mètres, seul cas accepté aujourd'hui. |

## Design Notes

Le noyau écrit les valeurs résolues dans les paramètres journalisés : le rejeu rejoue donc les mêmes valeurs même si un défaut change plus tard. `network.shortest_path` (13) lira le même type ; `network.prepare` (14) ne réutilise que `Distance`.

## Verification

**Commands:**
- `cargo test -p cassini-core --locked` et `cargo test -p cassini --locked` -- expected: succès
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement
- `python -I qgis-plugin/tests/check_extension.py` puis `python -I -m unittest discover -s qgis-plugin/tests` -- expected: succès

**Manual checks (if no CLI):**
- `probe_isochrone.py` sous vrai QGIS (`python-qgis-ltr.bat`) : verdict OK, étendues différentes avec vitesse et tolérance.
