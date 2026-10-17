---
title: 'Ouvrir un GeoPackage, un Shapefile et un GeoJSON par QGIS'
type: 'feature'
ticket: '5'
created: '2026-10-16'
status: done
baseline_revision: 'fb76e5b10d70939ca534cdee50506dccf1263c47'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/docs/canal-local.md'
  - '{project-root}/docs/regle-de-redaction-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le registre ne sait pas ouvrir un fichier de l'utilisateur : aucune commande ne met une couche dans le projet QGIS, donc rien ne peut ensuite l'analyser.

**Approach:** ajouter `layer.add`, commande **déléguée à QGIS** (noyau : déclaration seule ; extension : geste), qui ajoute au projet une couche issue d'un `.gpkg`, `.shp` ou `.geojson`, et rend son id de couche, son SCR et si QGIS la reprojette. Empreinte du fichier = **taille + date de modification** (décidé par le propriétaire le 2026-10-16), déjà relevée par le journal du noyau.

## Boundaries & Constraints

**Always:** noyau sans QGIS ni GDAL ; Qt et PyQGIS via `qgis.PyQt` / `qgis.core` seulement dans l'extension ; la commande déclare `.path_params(&["path"])` (chemin relatif au projet, AD-9) et `.runs_in(RunsIn::Qgis)` ; le fichier n'est jamais ouvert en écriture ni modifié (taille et date identiques avant/après) ; SCR rendu en code EPSG explicite, reprojection faite par QGIS, jamais par le noyau (AD-19) ; l'id de couche n'est qu'une valeur de session, jamais une clé de rejeu (AD-9) ; aucun chemin absolu dans un message d'erreur ; geste asynchrone, jamais bloquant (contrat de `gestures.py`) ; logique testable sans QGIS (couche injectable).

**Never:** CSV et GeoParquet (9.6) ; inventaire du projet (9.7) ; sélection de sous-couche d'un GeoPackage (la première couche est prise) ; choix de symbologie ; téléchargement ; modifier `docs/canal-local.md` ; somme de contrôle du fichier.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Ouvrir un GeoJSON | `path` d'un `.geojson` valide | couche ajoutée ; `{layer_id, name, source_crs:"EPSG:4326", project_crs, reprojected}` | — |
| GeoPackage et Shapefile | `.gpkg`, `.shp` valides | idem, première couche du GeoPackage | — |
| SCR différent du projet | source 4326, projet 2154 | `reprojected: true`, couche gardée dans son SCR (QGIS reprojette à l'affichage) | — |
| Format inconnu | `.txt` | rien n'est ajouté | `layer.unsupported_format` |
| Fichier illisible | couche invalide ou absente | rien n'est ajouté | `layer.invalid_source`, sans chemin absolu |
| Fichier intact | avant/après | taille et date identiques | — |
| Annulation | `cancel()` avant la fin | `done` rappelé, rien d'ajouté | — |
| Non annoncée | extension sans le geste | `registry.unavailable` | côté noyau |

</frozen-after-approval>

## Code Map

- `core/src/commands/layer_add.rs` -- NOUVEAU : `LayerAdd` (`impl Delegated`), params `{path}`, sortie typée ; modèle : `test_commands.rs` (`TestQgisProbe`) ; module déclaré par `core/build.rs`, aucun fichier partagé à toucher.
- `qgis-plugin/cassini_qgis/layer_add.py` -- NOUVEAU : `LayerAddGesture` (`start`/`cancel`/`done`, `GestureError`), couche QGIS injectable ; seul fichier à importer `qgis.core`.
- `qgis-plugin/cassini_qgis/plugin.py` (~l.148) -- `registry.register("layer.add", ...)` à côté de `test.qgis_probe`.
- `qgis-plugin/cassini_qgis/gestures.py`, `client.py` -- contrat et `order.execute` ; lecture seule.
- `qgis-plugin/tests/test_layer_add.py` -- NOUVEAU, doublure de QGIS ; `fixtures/` -- un GeoJSON synthétique.
- `core/src/journal.rs` (`input_entry`) -- taille et date déjà journalisées ; lecture seule.
- `tests/epreuve-modele-local/catalog.json` -- vérifier si la commande doit y figurer (`check_catalog.py`).

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/layer_add.rs` -- déclarer `layer.add` (FR/EN, `path_params`, `cancellable(true)`, test `layer_add_is_delegated`) -- contrat du registre
- [ ] `qgis-plugin/cassini_qgis/layer_add.py`, `plugin.py` -- geste : format par extension, couche en lecture seule, SCR, erreurs typées -- exécution par QGIS
- [ ] `qgis-plugin/tests/test_layer_add.py`, `fixtures/` -- matrice complète, empreinte avant/après sur un vrai fichier -- preuve sans QGIS
- [ ] `qgis-plugin/README.md` -- recette manuelle : ouvrir un fichier de chaque format depuis le panneau

**Acceptance Criteria:**
- Given un noyau et une extension démarrés, when `session.hello` répond, then `layer.add` est annoncée et le mode expert propose un champ `path`.
- Given un fichier de chacun des trois formats, when `layer.add` s'exécute, then la couche apparaît dans le projet et la taille et la date du fichier n'ont pas changé.
- Given `cargo test` et `python -I -m unittest discover -s qgis-plugin/tests`, then tout passe sans QGIS.

## Implementation Notes

## Plan Change Log

## Review Triage Log

| # | Verdict | Route | Evidence |
|---|---------|-------|----------|
| 1 | false | rejete | catalog.json est le catalogue factice jetable de l'epreuve du modele (epic 8), pas le registre ; son `source` n'est pas le contrat de `layer.add`. |
| 2 | medium (non verifie) | defer | Shapefile sans .prj : `authid()` vide. Refuser ou accepter est un choix d'intention (AD-19), a trancher. |
| 3 | low | patch (fait) | Ouverture GPKG : aucun changement constate sur le vrai QGIS, mais `forceReadOnly` ajoute pour tenir « jamais ouvert en ecriture ». |
| 4 | low | rejete | Branche d'annulation morte et code `qgis.cancelled` : sans effet, le correctif ajouterait du code. |
| 5 | low | rejete | Fichier absent et fichier corrompu donnent le meme message : voulu par la matrice. |
| 6 | low | rejete | Annonce par session.hello deja couverte par les tests du client ; verifiee a la main. |
| 7 | low | rejete | Double import de `GestureError` : latent, aucun echec constate. |

## Design Notes

Le journal du noyau relève déjà `size` et `modified` avant l'appel (`input_entry`) : le geste n'ajoute que la preuve « après » dans ses tests. Le résultat n'est pas revalidé par le noyau : le geste doit respecter le schéma `Output`.

## Verification

**Commands:**
- `cargo test -p cassini-core` -- expected: OK
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: OK
- `python -I qgis-plugin/tests/check_extension.py` -- expected: code 0

**Manual checks (if no CLI):**
- Dans QGIS, mode expert, `layer.add` avec un chemin relatif au projet pour un `.gpkg`, un `.shp`, un `.geojson` : la couche apparaît, le fichier reste intact.
