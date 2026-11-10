---
title: 'Mise en page : carroyage MGRS étiqueté, en-tête et pied de page, sources détaillées'
type: 'feature'
ticket: '31'
created: '2026-11-10'
status: 'done'
baseline_revision: 'e1a2633e211f8f1572356836011c045fce39ccd3'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-complements-geometrie-et-mise-en-page/change-complements-geometrie-et-mise-en-page.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** l'export PDF n'a ni carroyage étiqueté en MGRS (cartes de secours, grille USNG), ni texte libre en haut et en bas de page (bureaux d'études), et sa ligne des sources ne dit ni la résolution ni la date des données.

**Approach:** ajouter à `layout.export_pdf`, exactement comme ses `planned_params` et `planned_result` (D7 à D9) : `header` et `footer` (une ligne de 120 caractères au plus) ; la forme de grille `mgrs` (1, 10 ou 100 km), dont `epsg` et `squares` sont fournis par le noyau d'après `extent.lonlat`, journalisés, et imprimés par l'extension sans aucun calcul MGRS, les lignes étiquetées par un format de grille fixe de QGIS ; `source_details` (résolution, date et son origine), repris sur la ligne des sources, `sources` inchangé. Familles du générateur `carroyage_mgrs` et `entete_pied_de_page`.

## Boundaries & Constraints

**Always:** l'extension ne calcule rien de MGRS ; un journal ancien (sans `source_details`) se rejoue à l'identique ; rien d'inventé quand QGIS ne sait pas ; décisions de détail tranchées en party le 2026-11-10, à confirmer par le propriétaire.

**Never:** `epsg` ou `squares` écrits par le modèle ou l'appelant ; nouvelle dépendance ; changement de `sources`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| MGRS 1 km | `grid` mgrs, emprise d'Annecy | zone 32632 et carrés posés par le noyau, journalisés ; étiquettes à deux chiffres, cadre des carrés | — |
| MGRS sans emprise | inventaire sans `extent.lonlat` | rien exporté, rien journalisé | `layout.no_extent` |
| Intervalle hors liste | 5 km | refusé par le noyau | `registry.invalid_params` |
| En-tête trop long | 121 caractères, ou saut de ligne | refusé | `registry.invalid_params` |
| Journal ancien | ligne sans `source_details` | rejeu identique | — |
| Sortie de Cassini | source sous `<projet>.cassini` | `date` null | — |

</frozen-after-approval>

## Code Map

- `core/src/commands/layout_export_pdf.rs` -- variante `Mgrs`, `header`/`footer`, `SourceDetail`, `mgrs_grid_of` (zone du centre, carrés de l'emprise échantillonnée au kilomètre, 12 puis « … »).
- `core/src/geodesy.rs` -- `Mgrs::square_in_zone` (désignation de zone et de carré dans une zone imposée).
- `core/src/registry.rs` -- métadonnée `mgrs_grid_param`.
- `core/src/journal/mod.rs` -- `fill_mgrs_grid` : ôte `epsg`/`squares` de l'appelant, lit l'inventaire, pose zone et carrés avant journalisation ; le rejeu repasse les valeurs journalisées.
- `core/src/journal/summary.rs` -- `LATE_FIELDS` : une ligne sans `source_details` ne le compare pas.
- `ai/src/plan.rs` -- catalogue du modèle : la forme `mgrs` montrée sans `epsg`/`squares`.
- `qgis-plugin/cassini_qgis/layout_export.py` -- forme `mgrs` (expression d'étiquette fixe), en-tête, pied, cadre des carrés, `source_details`.
- `training/src/families.rs` -- familles `carroyage_mgrs`, `entete_pied_de_page`.
- `qgis-plugin/tests/probe_mgrs_layout.py` -- sonde réelle, Annecy A3.

## Tasks & Acceptance

**Execution:**
- [x] noyau : paramètres, résultat, complétion du carroyage, rejeu des lignes anciennes
- [x] extension : geste et tests
- [x] catalogue : fiche à jour (`params` et `result`), `skipped_layers` reste planifié
- [x] générateur : deux familles

**Acceptance Criteria:**
- Given un export avec grille mgrs à 1 km d'Annecy, when il est rendu en image, then les étiquettes à deux chiffres et le cadre des carrés sont lisibles et ne se chevauchent pas.
- Given `core/tests/catalogue.rs`, when il tourne, then il est vert.
- Given `check_plan`, when il lit les exemples des deux familles, then il les accepte.

## Implementation Notes

- Sonde réelle (`probe_mgrs_layout.py`, QGIS 3.44.15, Annecy, A3) : 19/19, rejeu identique ; PDF rendus en PNG et regardés : en-tête, pied, cadre des carrés (32T KR, KS, LR, LS), étiquettes à deux chiffres lisibles à 1 km (1:25 000) et 10 km ; deux corrections après le premier rendu (cadre ajusté à son texte, sources en 7 pt au-dessus du pied).

- Décisions de la party du 2026-11-10 (à confirmer) : P1 pas d'étiquette à 100 km ; P2 en-tête et pied centrés, 7 pt ; P3 pas de date pour une sortie de Cassini ; P4 métadonnées publication > révision > création ; P5 résolution `null` en degrés ; P6 zone du centre, 12 carrés puis « … », plus de deux fuseaux : `layout.no_extent`.
- Les carrés sont ceux de l'emprise du projet (D7), pas du cadre d'un export à l'échelle : à 1:25 000, le cadre peut lister des carrés hors de la carte.

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 11 constats — medium 3, low 5, false 0, rejetés 2, différés 1.

| # | Verdict | Sort | Constat et preuve |
|---|---|---|---|
| 1 | medium | patch | Le carroyage MGRS lisait l'inventaire avant la validation : un intervalle de 5 km rendait `layout.no_extent` au lieu de `registry.invalid_params`. Validation d'abord, test du journal ajouté. |
| 2 | rejeté | — | Passer par `read_inventory` donnerait le message de la mise en forme topographique (`style.unavailable`), faux ici ; l'erreur du routeur nomme le geste manquant. |
| 3 | low | patch | L'extension imprimait l'en-tête rogné : imprimé tel quel (D8), test ajouté. |
| 4 | low | patch | Seuls `\n` et `\r` étaient refusés : tout caractère de contrôle et les séparateurs de ligne Unicode le sont, au noyau et à l'extension. |
| 5 | rejeté | — | Étiquette négative au sud de l'équateur dans une zone nord : hors des territoires visés, emprise de part et d'autre de l'équateur seulement. |
| 6 | medium | patch | Au-delà de 60 lignes sur un côté de la carte, les étiquettes MGRS se chevaucheraient : elles ne sont pas posées (les lignes et le cadre restent). |
| 7 | low | patch | Une emprise sur deux fuseaux perdait des carrés sans le dire : « … » en fin de liste, test ajouté. |
| 8 | medium | defer | `date_origin: file` rend le rejeu sensible à la date de modification d'une source non journalisée (D9 la croyait journalisée par AD-11) : diverge sans message « source modifiée ». |
| 9 | medium | patch | Pas de test dédié aux deux familles : `valid(…, 600)` et vérifications (intervalle écrit, zone et carrés jamais, textes recopiés). |
| 10 | low | patch | Forme de la ligne des sources : le catalogue suit l'implémentation (« mnt_5m.tif (5 m, fichier du …) »). |
| 11 | low | patch | Recette manuelle de `qgis-plugin/README.md` mise à jour (6 bis). |

## Verification

**Commands:**
- `cargo test -p cassini-core -p cassini-ai -p cassini -p cassini-training --release` -- vert
- `cargo clippy … -- -D warnings` -- propre
- `python3 -m unittest discover -p "test_*.py"` (dans `qgis-plugin/tests`) -- vert
- `python -I qgis-plugin/tests/check_extension.py` -- propre
