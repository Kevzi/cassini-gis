# Changement de cap : `layer.remove` refuse une couche non enregistrée (2026-11-10)

Statut : **approuvé** par le propriétaire le 2026-11-10 (« on suit toutes les
recommandations »). Portée mineure, ajustement direct.

## 1. Le problème

- **Déclencheur :** relecture `thorough` de l'entrée 27 de l'epic 10 (`layer.remove`), deux
  constats `medium` reportés à la décision du propriétaire (`deferred-work.md`).
- **Nature :** risque de perte de données. `QgsProject.removeMapLayer` détruit la couche : une
  couche en mémoire (temporaire, sans fichier) perd tout son contenu, une couche en cours
  d'édition perd ses modifications non enregistrées. Aucun recours : le fichier, s'il existe,
  n'a pas reçu ces modifications.
- **Pourquoi un changement de cap :** la fiche de `layer.remove` (changement de cap du
  2026-11-10, `change-commande-layer-remove`) dit « Erreurs : aucune propre » ; le catalogue
  est figé (§1, changement de cap du 2026-11-05) et un code d'erreur ne s'ajoute à une fiche
  que par un changement de cap.

## 2. Analyse d'impact

| Endroit | Effet |
|---|---|
| Principe 6 (CLAUDE.md) | Renforcé : Cassini ne perd plus des données que l'utilisateur n'a pas encore écrites dans un fichier. |
| Catalogue | Fiche `layer.remove` : un code d'erreur propre, `layer.unsaved`. Paramètres, résultat et statut inchangés. |
| Nom du code | Forme `<domaine>.<raison>` des codes des gestes (`layer.not_found`, `layer.not_vector`, `layer.crs_conflict`) ; un seul code pour les deux cas, le message dit lequel. |
| Architecture | Aucune règle nouvelle. Le code naît dans l'extension, comme tous les codes des gestes ; le noyau le journalise et le rejoue sans le connaître. |
| Rejeu (AD-11) | Chemin générique d'une étape QGIS refusée (correctif `plan-bug-rejeu-etape-qgis-echouee`) : refusée de nouveau, le rejeu reste identique ; retirée sans refus au rejeu (couche enregistrée entre-temps, ou couche en mémoire absente d'une session neuve), le rejeu diverge à cette étape, en la nommant. Comportement honnête, rien n'est sauté. |
| Journal | Une ligne d'erreur comme les autres ; rien de nouveau. |
| Extension | Le geste regarde la couche avant de la retirer : fournisseur `memory`, ou couche en édition avec des modifications non enregistrées → refus, la couche reste au projet, intacte. |
| Panneau, MCP | Le message du refus est montré tel quel, en français : il dit ce que l'on perdrait et quoi faire (enregistrer la couche, ou enregistrer ou annuler ses modifications). |
| Générateur d'entraînement | Aucun changement : le refus se décide à l'exécution, pas au plan. |
| Epics | Epic 10 : complément de l'entrée 27, sur sa branche, sans entrée nouvelle. |
| PRD, UX | Aucun changement. |

## 3. Voie recommandée

**Ajustement direct** : un refus de plus dans le geste de l'entrée 27, sans retour en arrière
ni réduction de périmètre. Effort : petit (extension, fiche, tests, sonde QGIS). Risque :
élevé, comme l'entrée 27, parce qu'il touche au rejeu (CLAUDE.md §8 : relecture `thorough`).

## 4. Changements proposés

### Catalogue — `spec-cassini-mvp/catalogue-des-commandes.md`

Fiche `layer.remove`, phrase des erreurs :

> Résultat : `layer_id`. Erreurs : `layer.unsaved` — la couche est en mémoire (sans fichier) ou
> a des modifications non enregistrées : le retrait est refusé, la couche reste au projet,
> intacte (changement de cap du 2026-11-10, `change-refus-retrait-couche-non-enregistree`) ;
> `registry.invalid_params` (paramètre absent, champ en trop). Rejoué, un retrait refusé
> redonne le même refus tant que la couche n'est pas enregistrée ; retiré au rejeu (couche
> enregistrée entre-temps, ou couche en mémoire absente d'une session neuve), il diverge à
> cette étape. Une étape suivante qui cite la couche retirée échoue en la nommant, par le code de son propre geste (`layer.not_found` pour
> une analyse, `qgis.layer_missing` pour un geste de style), au plan comme au rejeu.

Le bloc `json fiche` ne change pas (il ne porte pas les codes d'erreur).

### Extension — `qgis-plugin/cassini_qgis/layer_remove.py`

Avant `removeMapLayer`, refus `layer.unsaved` pour une couche en mémoire ou en édition avec
des modifications non enregistrées ; la couche n'est pas touchée.

## 5. Passage à la réalisation

- **Portée : mineure** (ajustement direct, par l'agent de développement).
- **Puis :** `bmad-build` sur la branche `story/10-27-layer-remove`, relecture `thorough`.
- **Réussite :** tests de l'extension et du noyau verts ; dans QGIS, une couche en mémoire et
  une couche en édition modifiée sont refusées par `layer.unsaved`, restent au projet avec
  leur contenu, et le rejeu en même session est identique.
