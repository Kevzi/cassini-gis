---
title: 'Gestes layer.set_style et layer.arrange dans l''extension, et démonstration'
type: 'feature'
ticket: '21'
created: '2026-11-08'
status: 'done'
baseline_revision: '5d85b18175630948ec3d03113fe334c2ea5b264b'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le noyau compose les styles et le rangement (11.20) et dicte `layer.set_style` et `layer.arrange`. Mais l'extension QGIS ne sait pas les appliquer : la carte reste aux couleurs au hasard de QGIS. Depuis 11.20, l'essai qui confronte la table des gestes au catalogue échoue, puisque les deux fiches sont `faite`.

**Approach:**
- Deux gestes de la table fermée (`delegated.py`), d'entrée `project` et de sortie `none`.
- `layer.set_style` traduit chaque forme de `StyleSpec` en rendu QGIS :
  - `single` : symbole unique, de surface, de ligne ou de point selon la géométrie ;
  - `categories` : catégories par valeur ;
  - `graduated` : classes calculées par QGIS (intervalles égaux, quantiles ou seuils naturels) et dégradé entre deux couleurs ;
  - `elevation_ramp` : pseudo-couleur interpolée ;
  - `hillshade` : gris en fondu « produit », avec son opacité.
- `layer.arrange` crée les groupes en tête de l'arbre des couches, dans l'ordre, et y range les couches. Les couches absentes de la liste restent au-dessus.
- Aucun fichier de données n'est touché : seul le style de la couche dans le projet change.
- Les erreurs suivent le catalogue : `qgis.layer_missing`, `style.invalid`, `style.field_missing`, `style.geometry_mismatch`, `arrange.invalid`.
- La partie qui traduit le style est testée sans QGIS, avec un faux back-end. Une sonde sous QGIS 3.44 enchaîne, sur des copies des projets de Villeneuve et d'Annecy et avec le vrai noyau :
  - `map.topographic_style` ;
  - un `map.graduated_style` ;
  - l'export PDF, rendu en image et regardé ;
  - le rejeu, qui doit être identique.
- Le propriétaire confirme la carte et tranche la couleur des bois.

</frozen-after-approval>

## Boundaries & Constraints

**Always:**
- Qt par `qgis.PyQt` seulement (AD-18). Aucun module réseau : `check_extension.py` reste vert.
- Le `QgisBackend` est le seul code qui importe `qgis.core`, à l'intérieur de ses méthodes, comme `raster_output.py`.
- Les APIs de rendu choisies existent en 3.44 et en 4.x. Une API propre à une série passe par une seule fonction de compatibilité, notée.
- Les copies de projet de la sonde sont faites dans un dossier temporaire. Les projets et les données de `../donnees-test-cassini/` ne sont jamais modifiés.
- Chaque PDF de la sonde est rendu en PNG et regardé à l'œil, en `QT_QPA_PLATFORM=windows`, pas `offscreen` (mémoire « vérifier le PDF à l'œil »).
- Les 453 essais de l'extension et le nouvel essai restent verts, `test_delegated.py` compris.

**Never:**
- Pas de décision de thème ni de couleur dans l'extension (AD-5) : elle applique ce qu'on lui dicte.
- Pas de nouvelle dépendance ; pas de changement du noyau, sauf correctif d'un défaut trouvé par la sonde, noté.
- Pas de version de chaîne inventée ici : elle relève du ticket 10.3.
- Pas d'installation du noyau ou de l'extension chez le propriétaire sans le dire. Si la sonde l'exige, le noyau en place est sauvegardé d'abord, comme en 11.18.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|---|---|---|---|
| Symbole unique | `single` sur une couche de surface, de ligne, de points | Symbole du bon type, couleurs et épaisseur posées | — |
| Surface sans contour | `single` avec `fill` seul | Contour absent | — |
| Catégories | `categories` sur `typezone`, avec `other` | Une catégorie par valeur listée, plus un symbole pour les autres | `style.field_missing` si le champ manque |
| Gradué | `graduated` jenks, 5 classes, sur un champ numérique | 5 classes calculées par QGIS, du `from` au `to` | `style.field_missing` si le champ manque ou n'est pas numérique |
| Rampe | `elevation_ramp` sur un MNT | Pseudo-couleur interpolée, paliers croissants | `style.geometry_mismatch` sur un vecteur |
| Ombrage | `hillshade` sur un raster d'ombrage | Gris, mode produit, opacité | `style.geometry_mismatch` sur un vecteur |
| Mauvais type | `single` sur un raster | Rien n'est changé | `style.geometry_mismatch` |
| Couche absente | `layer_id` inconnu | Rien n'est changé | `qgis.layer_missing` |
| Rangement | 3 groupes, couches listées | Groupes en tête, dans l'ordre ; couches dedans, de haut en bas ; couches non listées au-dessus | `arrange.invalid` pour un groupe vide ou un doublon |
| Rangement répété | Deux `layer.arrange` successifs | Les groupes du premier sont réutilisés, pas dupliqués ; un groupe devenu vide est retiré | — |
| Sonde Villeneuve | Copie du projet de test, vrai noyau | PLU aux couleurs CNIG, gares, MNT en rampe dessous ; PDF lisible ; rejeu identique | — |
| Sonde Annecy | Copie du projet de test | Eau bleue, bâti par usage, routes par importance, ombrage sur la rampe ; PDF lisible ; rejeu identique | — |

## Code Map

- `qgis-plugin/cassini_qgis/delegated.py:9-45` -- table fermée : ajouter les deux lignes `registry.register(...)`, et le module au bloc d'import (paquet, puis repli à plat).
- `qgis-plugin/cassini_qgis/gestures.py:17-39` -- contrat `start(params, done)` et `cancel()`, et `GestureError(code, message)`.
- `qgis-plugin/cassini_qgis/open_output.py:27-63`, `raster_output.py:29-63` -- modèle d'un geste hors traitement : `_run` planifié par `QTimer.singleShot`, `GestureError` vers `done(error=...)`, toute autre exception vers `qgis.gesture_failed`. `QgisBackend` isolé.
- `qgis-plugin/cassini_qgis/inventory.py:71-74, 412-418` -- recherche d'une couche par son id (`QgsProject.instance().mapLayer`). Le code `qgis.layer_missing` n'existe pas encore côté Python : l'ajouter dans le nouveau module.
- `qgis-plugin/cassini_qgis/style_gestures.py` (nouveau) -- `SetStyleGesture`, `ArrangeGesture`, la traduction pure du `StyleSpec` en appels au back-end, et `QgisBackend` (`QgsSingleSymbolRenderer`, `QgsCategorizedSymbolRenderer`, `QgsGraduatedSymbolRenderer` avec `QgsClassificationEqualInterval`, `QgsClassificationQuantile`, `QgsClassificationJenks` et `QgsGradientColorRamp`, `QgsSingleBandPseudoColorRenderer` avec `QgsColorRampShader`, `QgsSingleBandGrayRenderer` avec `setBlendMode`, `QgsLayerTreeGroup`).
- `core/src/style/spec.rs:23-65` -- la forme exacte du `StyleSpec` reçu (`kind` en `snake_case`, champs refusés s'ils sont inconnus).
- `qgis-plugin/tests/test_delegated.py` -- confronte la table aux fiches `faite` et `qgis` : vert dès que les deux lignes sont posées.
- `qgis-plugin/tests/test_raster_output.py:15-52`, `test_layer_add.py:17-60` -- modèle des essais avec faux back-end et `schedule=lambda fn: fn()`. Nouveau `test_style_gestures.py`.
- `qgis-plugin/tests/probe_end_to_end.py` -- modèle de sonde qui lance le vrai noyau par `QProcess` et rejoue. `probe_layout_export.py:22-38, 201-220` -- chaînage par `QTimer`, export et contrôle du PDF. Nouvelle `probe_style.py` (copie du projet en dossier temporaire, puis `map.topographic_style`, `map.graduated_style`, `layout.export_pdf`, rejeu).
- `qgis-plugin/tests/check_extension.py:16-41` -- modules interdits ; à garder vert.
- `README.md:25-60` -- liste des sondes : y ajouter `probe_style.py`.

## Tasks & Acceptance

**Execution:**
- [ ] `qgis-plugin/cassini_qgis/style_gestures.py` -- les deux gestes, la traduction du `StyleSpec`, la validation des types et le `QgisBackend` -- le cœur du ticket.
- [ ] `qgis-plugin/cassini_qgis/delegated.py` -- les deux lignes de la table -- annonce au `hello`, `test_delegated` vert.
- [ ] `qgis-plugin/tests/test_style_gestures.py` -- chaque ligne de la matrice, sauf les sondes, avec un faux back-end.
- [ ] `qgis-plugin/tests/probe_style.py` et `README.md` -- sonde sous QGIS 3.44 avec le vrai noyau construit dans ce worktree, sur Villeneuve et Annecy : PDF, rendu PNG, rejeu.
- [ ] Rendus PNG montrés au propriétaire, qui confirme et tranche la couleur des bois.

**Acceptance Criteria:**
- Given le projet de Villeneuve ou d'Annecy copié, when la sonde lance `map.topographic_style` par le vrai noyau, then chaque couche reconnue prend son style, les groupes sont posés relief en dessous, et le PDF rendu en image le montre.
- Given le journal de la sonde, when il est rejoué, then le rapport est `identical`.
- Given les fichiers de `../donnees-test-cassini/`, when la sonde a fini, then aucun n'a changé (date et taille).

## Implementation Notes

- **Construction :** dans le worktree `{project-root}` (branche `story/style-topographique`). Pas de `git switch` ni de commit : le workflow commite après la relecture. Le dossier voisin `CASSINI` n'est jamais touché, parce qu'un entraînement y tourne.
- **Noyau de la sonde :** construire `cassini.exe` depuis ce worktree sous Windows. Utiliser `powershell.exe` avec `LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"`, `CARGO_TARGET_DIR="C:\ct-wt"`, puis `cargo build --release -p cassini`, sans GPU. La sonde le désigne par `CASSINI_CORE`.
- **Bois :** couleur du Plan IGN gardée (`#dfe8d5`), décision du propriétaire du 2026-11-08.
- **Ombrage :** sur un raster que QGIS dessine déjà en relief ombré (le `relief_ombre` des deux projets de test est le MNT au rendu d'ombrage), `layer.set_style` `hillshade` garde ce rendu et ne change que l'opacité et le fondu « produit » ; sinon, rendu en gris. Cartes approuvées par le propriétaire le 2026-11-08 ; écrit au catalogue, ligne `hillshade` de `StyleSpec`.
- **Rendu des PDF :** `uv run --with pypdfium2 --with pillow`, une page en PNG à environ 100 dpi. Les PNG sont écrits à côté du PDF, dans le dossier temporaire de la sonde ; donner leurs chemins dans le rapport.

## Verification

**Commands:**
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python -I qgis-plugin/tests/check_extension.py` -- expected: vert
- `"C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat" qgis-plugin/tests/probe_style.py` -- expected: `RÉSULTAT : OK`

**Manual checks:**
- Les PNG des deux PDF : eau bleue, bois verts, routes classées, PLU aux couleurs CNIG, relief dessous ; légende et coordonnées lisibles.

Confirmé par le propriétaire le 2026-11-08 (« ok la carte me va ») sur les rendus de Villeneuve et d'Annecy, recopiés hors du dépôt dans `C:\Users\kevin\Downloads\cartes-cassini-11-21\`. Ajouts qu'il a demandés pendant le ticket, chacun écrit au point 8 du changement de cap :
- `max_scale` par couche et par classe ;
- seuil du détail au 1:100 000 ;
- légende continue de l'altitude ;
- PLU à 40 % de transparence ;
- basses altitudes en crème ;
- ombrage à 0,4.

## Review Triage Log

Relecture `thorough` : blind-hunter (15), edge-case-hunter (12), verification-gap (2 + 2), intent-alignment (descriptif). Relecture `quick` ensuite, sur les ajouts du propriétaire (6 constats). Tous les correctifs sont appliqués par l'agent de construction. Vérifié :
- 494 essais de l'extension ;
- `check_extension.py` propre ;
- 610 + 32 + 27 + 11 essais du noyau ;
- `fmt` et `clippy` propres ;
- `probe_layout_export.py` OK ;
- `probe_style.py` OK (66 sur 66), rejeux identiques, projet rouvert juste.

- medium, patch : `layer.arrange` pouvait laisser l'arbre à moitié rangé quand une couche du projet n'avait pas de nœud. Les nœuds sont maintenant résolus avant tout geste ; sinon `qgis.layer_missing`.
- medium, patch : les groupes réutilisés sont clonés par QGIS, et le retrait des sous-groupes vidés suivait l'identité Python. Il suit maintenant le chemin des noms. La doublure clone comme QGIS, et trois essais couvrent les groupes imbriqués.
- medium, patch : des couches non listées dans un groupe réutilisé descendaient sous les couches dictées. Elles remontent maintenant en tête de la racine.
- medium, patch : `normalize_style` acceptait ce que le noyau refuse :
  - `kind` non textuel ;
  - classes vides ;
  - champ blanc.

  Une fixture commune, `fixtures/style-cases.json`, est maintenant lue des deux côtés. La valeur `""` est refusée par l'extension et acceptée par le noyau, écart noté dans la fixture.
- medium, patch : un dégradé sans classe passait pour un succès. L'ancien rendu est remis, avec `style.field_missing`.
- medium, patch : la sonde ne vérifiait que le type de rendu. Elle vérifie maintenant les couleurs, la catégorie « autres », le fondu, l'opacité et l'échelle, avec un compte des contrôles calculé et l'en-tête `%PDF` vérifié au rejeu.
- medium, patch : la légende de l'altitude se perdait à la réouverture du projet. Elle est maintenant gardée dans des propriétés de la couche, et réinstallée avant l'export.
- medium, patch : la légende repliée était mesurée sur un filtre périmé. Le filtre est maintenant forcé avant la mesure, et un cas de sonde à 300 classes le couvre.
- low, patch : la légende « ne déborde jamais » n'était pas vrai. Le cadre est maintenant borné à sa colonne et le contenu rogné ; un cas de sonde à 1 500 couches le couvre.
- low, patch : les statistiques de la bande parcouraient tout le MNT. Elles sont maintenant échantillonnées (250 000 cellules au plus), et ne servent qu'à la légende.
- low, patch : l'ombrage garde un rendu QGIS d'ombrage déjà posé. C'est accepté, et écrit dans le catalogue et la docstring.
- low, patch : la compatibilité 4.x était affirmée sans preuve. La docstring dit maintenant « vérifié sous 3.44 ». Lignes longues et README complétés.
- defer : QGIS 4.x non essayé. La légende Python et le moment du filtre ne sont observés que sous 3.44.
- defer : un libellé figé dans l'extension (« Altitude (m) », « Ombrage ») mériterait d'être dicté par le noyau dans le contrat.
- low, rejeté : une légende repliée peut lister des couches sans entité dans le cadre. C'est accepté et écrit.
