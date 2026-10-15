# Épreuve du modèle local (epic 8)

Code **jetable**, hors des crates et de `qgis-plugin` : rien ici n'entre dans le
produit. Il sert à mesurer si un modèle quantifié, sur le poste et sans réseau,
sait transformer une phrase française en suite de commandes typées correcte.
Aucun fichier de modèle n'est versé au dépôt.

## État

Entrée 8.2, **première moitié** : le catalogue et l'inventaire. Les 25 phrases
de référence et le validateur viennent dans une seconde session.

| Fichier | Rôle |
|---|---|
| `catalog.json` | 16 commandes factices : id, descriptions fr et en, `runs_in`, schéma des paramètres, schéma du résultat |
| `inventory.json` | ce que `project.inventory` rendrait pour un projet factice : 7 couches à identifiants opaques, 3 fichiers pas encore ouverts |
| `check_catalog.py` | contrôle de cohérence ; `python check_catalog.py` sort 0 si tout tient, puis rejoue 9 mutations qui doivent toutes être refusées |

Les commandes ne font rien : c'est la **forme** (CAP-6) qui est mesurée, pas le
calcul. Les données de l'inventaire sont inventées.

## Conventions suivies (contrat d'architecture)

- Id en `domaine.verbe`, anglais, snake_case (AD-6).
- Paramètres neutres : un filtre est `{field, operator, value}`, jamais une
  expression ; aucun nom d'algorithme QGIS (AD-22).
- **Aucun chemin ni nom de sortie** dans les paramètres : le noyau les décide
  (AD-10). Seul `layer.add` reçoit un chemin, d'entrée, relatif au projet (AD-9).
- Une couche est son identifiant d'inventaire, copié tel quel, ou `step:k` pour
  la couche produite par l'étape *k* (AD-9, AD-11). Cette forme de référence est
  inventée pour la mesure ; le verdict de l'entrée 8.6 tranchera celle du plan.
- Toute distance, hauteur ou rayon est `{value, unit}` ; toute coordonnée est
  `{x, y, epsg}` (AD-19). Aucune mesure nue dans les paramètres : le contrôle
  refuse tout nombre en tête d'un paramètre. La valeur d'un filtre, elle, est une
  donnée comparée à un champ, pas une mesure, et reste libre.
- Les tableaux sont bornés par `maxItems` (AD-8).

## Sous-ensemble de JSON Schema (draft 2020-12)

Le validateur de la seconde session ne connaîtra que ces mots-clés :

`$ref` (uniquement `#/$defs/nom`, résolu contre le `$defs` de `catalog.json`),
`description`, `default`, `type` (un nom ou une liste de noms), `enum`,
`properties`, `required`, `additionalProperties` (toujours `false`), `items`,
`minItems`, `maxItems`, `minimum`, `exclusiveMinimum`, `minLength`,
`maxLength`, `pattern` (syntaxe commune à Python et ECMAScript).

Pas de `oneOf`, `anyOf`, `allOf`, `not`, `if`, ni de `$ref` extérieur.
`check_catalog.py` refuse tout mot-clé hors de cette liste.
