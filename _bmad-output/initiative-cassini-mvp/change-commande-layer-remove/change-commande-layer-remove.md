# Changement de cap : commande `layer.remove` (2026-11-10)

Statut : **approuvé** par le propriétaire le 2026-11-10. Portée mineure, ajustement direct.

## 1. Le problème

- **Déclencheur :** demande du propriétaire du 2026-11-09, après `data.convert` (10.9) :
  pouvoir ranger un projet après une analyse en retirant une couche dont on n'a plus besoin.
- **Nature :** nouvelle exigence. Aujourd'hui le catalogue n'a aucune commande qui retire une
  couche ; le modèle refuse (« Aucune commande du catalogue ne retire de couche du projet »,
  `training/src/families.rs`, famille `refus`), et le jeu tenu à l'écart l'attend (H35,
  famille « retrait de couche », `ai/tests/tenu_a_l_ecart/sentences.json`).
- **Pourquoi un changement de cap :** le catalogue est figé (`catalogue-des-commandes.md` §1,
  changement de cap du 2026-11-05) ; un identifiant ne s'y ajoute que par un changement de cap.

## 2. Analyse d'impact

| Endroit | Effet |
|---|---|
| Principe 6 (CLAUDE.md) | Respecté : la couche quitte le projet QGIS, son fichier n'est ni effacé ni modifié. |
| Catalogue | Une ligne au §4, une fiche `planifiee` puis `faite` (§7, epic 10). |
| Architecture | Aucune règle nouvelle : la commande entre dans l'amendement du 2026-11-08 d'AD-24 (entrée `project`, sortie `none`, geste fixe de l'API de projet), comme `layer.set_style` et `layer.arrange`. Désignation par `layer_id` (AD-9). |
| Rejeu (AD-11) | Rejouée comme toute étape, jamais sautée. Une couche déjà absente n'est pas une erreur (même règle que `layer.arrange`) : le rejeu dans la même session, où la couche a déjà été retirée, reste identique. Une étape suivante qui cite la couche retirée échoue en la nommant (`qgis.layer_missing`), au plan comme au rejeu. |
| Journal | Une ligne comme les autres ; rien de nouveau. |
| Extension | Une ligne de la table fermée : retrait de la couche du projet (`QgsProject.removeMapLayer`), sans toucher au fichier ; version de chaîne 1. |
| Panneau, MCP | Commande offerte au modèle, au client MCP et au formulaire (`dictated_by_core: false`). Le plan est montré et validé avant exécution, comme toujours. |
| Générateur d'entraînement | Nouvelle famille (`retrait_couche`) ; les deux branches du refus « retire une couche » deviennent des plans. Le modèle l'apprend au prochain entraînement groupé. |
| Jeu tenu à l'écart | H35 passe de `refusal` à un plan d'une étape `layer.remove` sur la couche des routes ; barre d'`ecart_reel` inchangée (32/40). |
| Epics | Epic 10 (`epic-catalogue-etendu-qgis`) : une entrée nouvelle, construite tout de suite par dérogation à l'ordre de l'epic (comme `data.reproject` le 2026-11-08). |
| PRD, UX | Aucun changement. |

## 3. Voie recommandée

**Ajustement direct** : une entrée de plus dans l'epic 10, sans retour en arrière ni
réduction de périmètre. Effort : une entrée moyenne (noyau, extension, catalogue, générateur,
tests), puis l'apprentissage groupé au prochain entraînement. Risque : élevé (décision du propriétaire du 2026-11-10), parce que la commande touche au rejeu
(CLAUDE.md §8 : relecture `thorough`) ; elle ne touche aucun fichier de l'utilisateur.

## 4. Changements proposés

### Catalogue — `spec-cassini-mvp/catalogue-des-commandes.md`

§4, nouvelle ligne après `data.convert` :

```
| `layer.remove` | planifiée | qgis | epic 10, entrée 27 |
```

§7, nouvelle fiche :

> ### `layer.remove`
>
> Retire une couche du projet QGIS ouvert (changement de cap du 2026-11-10). Le fichier de la
> couche n'est ni effacé ni modifié. Une couche déjà absente n'est pas une erreur : le résultat
> est le même. Entrée `project`, sortie `none` (amendement du 2026-11-08 d'AD-24).
>
> | Paramètre | Type | Oblig. | Unité / valeurs | Sens |
> |---|---|---|---|---|
> | `layer_id` | chaîne | oui | — | Couche à retirer du projet. |
>
> Résultat : `layer_id`. Erreurs : aucune propre (une étape suivante qui cite la couche
> retirée échoue par `qgis.layer_missing`).
>
> ```json fiche
> {"id": "layer.remove", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
>  "params": {"layer_id": {"type": "string", "required": true}},
>  "result": ["layer_id"]}
> ```

### Epic 10 — `epic-catalogue-etendu-qgis/tickets.toml`

Nouvelle entrée 27 « Retirer une couche du projet (`layer.remove`) » : commande au registre,
ligne de la table fermée de l'extension, fiche passée en `faite`, famille du générateur et
refus retiré, H35 rescoré ; vérification : test de l'extension (couche retirée, fichier
intact, couche absente sans erreur), test du rejeu (même session, session neuve), recette
QGIS réelle. Construite tout de suite, avant les entrées 1 à 6 de l'epic 10.

### Décision dans l'epic 10 — `epic-catalogue-etendu-qgis.md`

> - Decision: 2026-11-10 — `layer.remove` entre au catalogue (changement de cap
>   `change-commande-layer-remove`) : retrait du projet, jamais du fichier ; idempotente.

## 5. Passage à la réalisation

- **Portée : mineure** (ajustement direct, par l'agent de développement).
- **À appliquer avec `bmad-ticket` :** l'entrée 27 de l'epic 10 ci-dessus.
- **Puis :** `bmad-build` sur cette entrée, une fois l'entraînement en cours et ses essais
  terminés (même dossier de compilation).
- **Réussite :** la recette `core/tests/catalogue.rs` verte avec la fiche `faite` ; dans
  QGIS, « retire la couche … du projet » retire la couche, son fichier reste, le rejeu est
  identique ; le modèle l'apprend au prochain entraînement groupé.
