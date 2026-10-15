---
title: 'Registre factice et jeu de phrases de reference — partie 1 : catalogue et inventaire'
type: 'chore'
ticket: '2'
created: '2026-10-15'
status: 'built'
baseline_revision: 'f729b1b09c26d58f7b0f5e0812eee8c90dc20c65'
route: 'oneshot'
route_source: 'pinned'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** l'épreuve du modèle local (entrées 8.3 à 8.5) ne peut rien mesurer sans un catalogue de commandes typées à montrer au modèle et un inventaire de projet à lui décrire. L'entrée 8.2 les fournit avec les 25 phrases et le validateur ; elle est livrée en deux sessions, et celle-ci en est la première moitié.

**Approach:** écrire dans `tests/epreuve-modele-local/` un catalogue de 16 commandes factices (`catalog.json`) et un inventaire de projet factice (`inventory.json`), aux conventions du contrat d'architecture, plus un court README qui fixe le sous-ensemble de JSON Schema employé, pour que le validateur de la seconde session ne soit écrit que pour lui. Les 25 phrases et le validateur sont hors de cette session.

## Boundaries & Constraints

**Always:**
- Code jetable, hors des crates et de `qgis-plugin` (epic 8, Done when 5) ; aucun fichier de modèle au dépôt.
- Commandes nommées `domaine.verbe`, anglais, snake_case (AD-6) ; description courte en français et en anglais ; champ `runs_in` valant `core` ou `qgis`.
- Schémas JSON Schema draft 2020-12, dialecte déclaré, écrits à la main ici faute de types Rust (le vrai registre les engendrera, AD-6).
- Paramètres neutres : un filtre est `{field, operator, value}`, jamais une expression ; aucun nom d'algorithme QGIS (AD-22).
- Aucun paramètre de chemin ni de nom de sortie : le noyau les décide (AD-10). Une couche se désigne par son identifiant opaque, ou par `step:k` pour la sortie de l'étape *k* (AD-9, AD-11).
- Toute distance, hauteur ou surface est `{value, unit}` ; toute coordonnée porte son EPSG (AD-19, Consistency Conventions).
- Aucune collection de géométries non bornée (AD-8) : `maxItems` partout.
- Aucune donnée réelle : les noms de couches de l'inventaire sont inventés, aucun jeu d'Esri.
- Bibliothèque standard seule pour tout outil de contrôle ; commentaires et README en français.

**Never:**
- Écrire les 25 phrases ou le validateur dans cette session.
- Ajouter une fonction de fonction réservée au module privé ou toute fonction à usage militaire (CLAUDE.md §2.9) : les commandes de terrain restent les trois du MVP.
- Employer `oneOf`, `anyOf`, `allOf`, `if/then` ou `$ref` hors `#/$defs/` : le validateur de 8.2 (seconde session) ne les connaîtra pas.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Catalogue complet | `catalog.json` lu | 16 commandes, ids uniques, chacune avec `id`, `description_fr`, `description_en`, `runs_in`, `params`, `result` | Un contrôle de cohérence échoue sur un id dupliqué ou un champ manquant |
| Inventaire cohérent | `inventory.json` lu | couches à identifiants opaques, chacune avec type, EPSG, champs typés | Le contrôle échoue sur un identifiant lisible ou un chemin absolu |
| Hors sous-ensemble | un mot-clé absent de la liste du README dans un schéma | rejeté par le contrôle | message nommant le mot-clé |

</frozen-after-approval>

## Code Map

- `tests/epreuve-modele-local/catalog.json` -- à créer : tableau de 16 commandes.
- `tests/epreuve-modele-local/inventory.json` -- à créer : un projet factice, 6 couches vecteur et 1 raster.
- `tests/epreuve-modele-local/README.md` -- à créer : rôle du dossier, conventions, sous-ensemble de JSON Schema, état (partie 1).
- `tests/epreuve-modele-local/check_catalog.py` -- à créer : contrôle de cohérence jetable, bibliothèque standard seule.
- `tests/README.md` -- à ne pas modifier : l'épreuve est hors du chemin du produit.
- `_bmad-output/.../architecture-noyau-et-facade-qgis.md` -- AD-6, AD-8 à AD-11, AD-19, AD-22, Consistency Conventions : seule source des conventions.
- Aucun code existant à réutiliser ; Python 3.11 disponible sur le poste.

## Tasks & Acceptance

**Execution:**
- [x] `tests/epreuve-modele-local/catalog.json` -- 16 commandes : `project.inventory`, `layer.add`, `layer.describe`, `layer.remove`, `layer.reproject`, `layer.select_by_attribute`, `layer.export`, `analysis.buffer`, `analysis.intersect`, `analysis.zonal_statistics`, `analysis.isochrone`, `map.zoom_to_layer`, `layout.export_pdf`, `terrain.viewshed`, `terrain.line_of_sight`, `terrain.slope` -- couvre la phrase de démo de CLAUDE.md §1 et les trois commandes de terrain du MVP.
- [x] `tests/epreuve-modele-local/inventory.json` -- projet factice, EPSG 2154, couches à identifiants opaques de forme `nom_8hex_…` -- ce que `project.inventory` rendrait, le modèle n'ayant jamais besoin des données.
- [x] `tests/epreuve-modele-local/check_catalog.py` -- vérifie ids, champs, mots-clés autorisés, absence de chemin ou d'expression dans les paramètres, cohérence de l'inventaire -- c'est la preuve que le catalogue tient ses propres règles.
- [x] `tests/epreuve-modele-local/README.md` -- rôle, conventions, liste des mots-clés JSON Schema permis, rappel que les phrases et le validateur viennent ensuite.

**Acceptance Criteria:**
- Given le dossier `tests/epreuve-modele-local/`, when `python check_catalog.py` s'exécute, then il sort 0 et imprime le nombre de commandes (16) et de couches.
- Given un schéma qui emploie un mot-clé hors liste ou un paramètre nommé `path` ou `expression`, when le contrôle s'exécute, then il sort non nul en nommant l'intrus.
- Given le catalogue, when on cherche la phrase de démo de CLAUDE.md §1, then ses étapes (sélection, zone tampon, intersection, export PDF) existent toutes comme commandes.

## Implementation Notes

Route oneshot épinglée : l'estimation dépasse 100 lignes (le catalogue en fait davantage), mais c'est de la donnée sans logique, et aucun sous-agent n'a été autorisé pour cette session ; l'implémentation se fait donc en direct.

Fait : `catalog.json` (16 commandes, `$defs` partagés), `inventory.json` (7 couches, 3 fichiers non ouverts), `check_catalog.py` (contrôle + 7 mutations d'auto-test), `README.md`. `python -I check_catalog.py` sort 0. Écarts au plan : (1) l'identifiant de couche suit la forme réelle de QGIS (nom + UUID à underscores), pas la forme courte du plan, pour que la copie par le modèle soit mesurée sur du réaliste ; (2) `layer.add` reçoit un chemin d'entrée relatif au projet, le « aucun paramètre de chemin » du plan visant les sorties (AD-10) ; (3) le contrôle refuse le paramètre nommé `path` mais `source` passe, garde-fou contre un chemin de sortie. Aucun fichier touché hors `tests/epreuve-modele-local/`.

Ajouts du 2026-10-15, hors plan initial.

- Après un essai à la main de la phrase de démo sur Qwen3.5-9B Q4_K_M (invite libre, sans grammaire, 6 s) : les champs de catégorie de l'inventaire portent `values` (valeurs d'exemple, 12 au plus), car le modèle avait inventé `typezone contains "constructible"` ; la forme `step:k` est notée « à éprouver » au README. Deuxième essai avec les valeurs : plus de valeur inventée, mais le modèle filtre à tort `parcelles.section = "AB"`, laisse de côté le zonage, ouvre trois fichiers inutiles et choisit `format: pdf`, hors énumération ; la numérotation des étapes y est, cette fois, juste. Deux essais isolés, donc un bruit à mesurer en 8.3, pas un constat.
- Décidé avec le propriétaire pour la seconde session : en plus des 25 phrases françaises, une dizaine de phrases **anglaises**, écrites sur les mêmes cas que dix des phrases françaises et portant le même plan attendu. Elles vivent dans un fichier à part (`sentences_en.json`) et se mesurent à part, sans entrer dans le taux des 25, pour savoir si la langue change le résultat (descriptions des commandes déjà bilingues, CAP-6). Le bloc figé et `tickets.toml` ne sont pas modifiés : c'est un ajout de périmètre, la contrainte « vingt-cinq phrases » reste tenue.

## Design Notes

Référence de couche : chaîne de motif `^(step:[1-9][0-9]*|<identifiant QGIS>)$`, où l'identifiant suit la forme réelle de QGIS (nom puis UUID à underscores) ; un résultat de commande ne porte que l'identifiant concret (`$defs.layer_id`). Le contrôle d'appartenance à l'inventaire est fait par le harnais de 8.3, pas par le schéma. La forme de cette référence est inventée ici pour la mesure ; la forme du plan rendu est tranchée par 8.6.

Mesure : `{"value": number > 0, "unit": enum}`. Coordonnée : `{"x", "y", "epsg"}` avec `epsg` dans `[2154, 4326, 3857, 4258]`.

## Verification

**Commands:**
- `python tests/epreuve-modele-local/check_catalog.py` -- expected: sortie 0, « 16 commandes »

## Review Triage Log

Relecture quick, 10 constats.
- medium, patch : `project.inventory` ne rendait que deux compteurs, pas l'inventaire. Résultat remplacé par `layers` et `unloaded_files`, bornés (`$defs.inventory_layer`).
- medium, patch : `layer_result` exigeait `feature_count` (faux pour un raster) et acceptait `step:k` comme résultat. `feature_count` devient facultatif, `layer_id` un identifiant concret.
- low, patch : liste de refus des paramètres trop courte (`file`, `filename`, `dest`, `folder`…) ; allongée.
- low, patch : `"type": ["object", …]` contournait les contrôles d'objet et de tableau ; corrigé, avec mutation d'auto-test.
- low, patch : contrôles d'inventaire partiels (types de champs, EPSG du projet, 6 vecteurs + 1 raster) ; ajoutés.
- low, patch : docstring parlait d'une option `--self-test` inexistante ; reformulée.
- low, patch : cases et statut du plan incohérents ; mis à jour.
- false : « valeur de filtre = nombre nu interdit ». Une valeur de filtre est une donnée, pas une mesure ; le README le précise.
- low, rejeté : `$` Python accepte un saut de ligne final. Le harnais rejouera les motifs en `fullmatch`, sans risque réel pour des chemins de recette.
- à signaler au propriétaire, hors correction : `layer.add` reçoit un chemin d'entrée, ce que la lettre du bloc figé (« aucun paramètre de chemin ») exclut. Choix défendable (AD-9 le prévoit pour les fichiers), consigné en Implementation Notes.
