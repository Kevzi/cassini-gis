# Épreuve du modèle local (epic 8)

Code **jetable**, hors des crates et de `qgis-plugin` : rien ici n'entre dans le
produit. Il sert à mesurer si un modèle quantifié, sur le poste et sans réseau,
sait transformer une phrase française en suite de commandes typées correcte.
Aucun fichier de modèle n'est versé au dépôt.

## État

Entrées 8.2 (catalogue, inventaire, 25 phrases, validateur), 8.3 (harnais de
mesure) et 8.5 (formes de sollicitation) **terminées**. Restent la comparaison
des modèles (8.4) et le verdict (8.6).

| Fichier | Rôle |
|---|---|
| `catalog.json` | 16 commandes factices : id, descriptions fr et en, `runs_in`, schéma des paramètres, schéma du résultat |
| `inventory.json` | ce que `project.inventory` rendrait pour un projet factice : 7 couches à identifiants opaques, 3 fichiers pas encore ouverts |
| `sentences.json` | 25 phrases françaises `{id, text, expected, expect_refusal}` ; 5 portent un plan de terrain, une demande un refus |
| `reading_independent.json` | la seconde lecture, faite sans voir les plans attendus : 25 plans sur 25 identiques ; rejouable avec `python check_sentences.py reading_independent.json` |
| `sentences_en.json` | 10 phrases anglaises `{id, text, same_plan_as}` : même plan qu'une phrase française, mesurées à part |
| `schema_validator.py` | valide une valeur contre le sous-ensemble de JSON Schema ci-dessous |
| `plan_validator.py` | `check_plan`, `check_output`, `canonical`, `plans_equal` : ce que 8.3 réutilise |
| `harness.py` | le harnais : pose les phrases à un modèle local, valide, redemande une fois, imprime deux taux, écrit un rapport JSON dans `reports/` (ignoré par git) |
| `examples.json` | trois exemples phrase-plan pour l'invite, hors des phrases mesurées |
| `check_harness.py` | contrôle le harnais sans modèle, par rejeu de sorties enregistrées |
| `run_forms.py` | enchaîne les huit formes de sollicitation et écrit `resultats-8-5.md` (généré) et `.json` ; `--rebuild` refait le tableau sans appeler le modèle |
| `lecture-8-5.md` | la lecture, écrite à la main, des mesures de 8.5 : ce qui joue, ce qui ne joue pas, la forme retenue |
| `check_catalog.py` | cohérence du catalogue et de l'inventaire, plus 9 mutations qui doivent toutes être refusées |
| `check_sentences.py` | cohérence des phrases, 19 plans faux qui doivent tous être refusés, égalité de plans ; avec un fichier `{"S01": plan, …}` en argument, compare une lecture indépendante aux plans attendus |

Les commandes ne font rien : c'est la **forme** (CAP-6) qui est mesurée, pas le
calcul. Les données de l'inventaire sont inventées. Recette :
`python -I check_catalog.py && python -I check_sentences.py`.

## Lancer une mesure

```
python harness.py --llama <llama-completion.exe> --model <fichier.gguf> --quant Q4_K_M
       [--catalog full|filtered] [--examples] [--output free|schema] [--lang fr|en] [--ids S07,S17]
```

Un processus `llama-completion` par appel, aucun serveur, aucun réseau. La sortie
doit être exactement du JSON : seuls le bloc de réflexion et une clôture ``` sont
ignorés, tout autre texte autour la rend non conforme (AD-23). Un échec d'appel
(délai dépassé, code de sortie non nul) est compté à part, jamais comme une erreur
du modèle. Les
paramètres de sollicitation sont : catalogue complet ou filtré (les 8 commandes
les plus proches de la phrase par recouvrement lexical, rappel rapporté),
avec ou sans exemples, sortie libre ou contrainte par un schéma JSON engendré
depuis le catalogue. Le rapport donne par phrase la sortie, les écarts et les
temps, et en tête : taux au premier essai, taux après relance, jetons, temps de
chargement, de lecture et de génération. `--think` laisse le mode réflexion du
modèle (coupé par défaut) ; `--template mistral` change le format d'invite.

Une sortie non conforme est redemandée **une fois**, avec la liste des écarts ;
un plan conforme mais différent de l'attendu est faux et n'est pas redemandé.
Les chemins du poste sont passés en paramètres et n'apparaissent jamais en dur.

## Le plan et sa conformité

Un plan est une liste d'étapes `{"command", "params"}`, rien d'autre. Un refus
est `{"refusal": "<raison>"}`. Un plan est **conforme** quand (AD-23) :

- chaque commande est au catalogue et chaque `params` valide son schéma ;
- chaque couche est un identifiant de l'inventaire, ou `step:k` avec *k*
  strictement inférieur au rang de l'étape et une étape qui produit une couche ;
- chaque `layer.add` ouvre un fichier parmi les fichiers non ouverts ;
- `dem` et `raster` désignent un raster ; sur une couche de l'inventaire, un filtre
  cite un champ qui existe et dont la valeur a le bon type (le champ d'une couche
  produite par une étape n'est pas contrôlé, et les `values` d'exemple n'enferment
  pas la valeur : un filtre peut citer une valeur absente) ;
- un point de terrain a l'EPSG du modèle d'élévation, suivi à travers les étapes
  de terrain et de reprojection, sinon le plan est rejeté (AD-19) : la bonne réponse à une coordonnée WGS84 est un refus.

## Égalité de plans

Deux plans conformes sont **égaux** quand leurs formes canoniques le sont :
chaque `step:k` est remplacé par l'étape qu'il désigne, on compare les arbres
des étapes terminales, sans tenir compte de l'ordre des étapes ni de celui des
listes. L'intersection est symétrique et associative. `title` et
`sources_note` sont ignorés ; une valeur égale à son défaut équivaut à
l'absence. Une étape en trop rend les plans différents. Deux refus sont égaux.

**Limite connue :** les unités ne sont pas normalisées, donc 500 m et 0,5 km ou
2 km et 2 000 m donnent deux plans différents, alors qu'ils sont équivalents.
Les phrases de référence nomment leurs unités pour éviter le cas ; une
normalisation est reportée (voir `deferred-work.md`).

S19 croise des bâtiments en EPSG 4326 avec une zone tampon en 2154 sans les
reprojeter : on suppose que QGIS aligne les systèmes à l'intersection, à
vérifier à l'epic du registre.

Règles de lecture communes aux phrases : une option facultative n'est posée que
si la phrase la demande ; une carte PDF ne montre que la couche résultat, sauf
si la phrase en nomme d'autres.

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
- Les champs de catégorie de l'inventaire portent `values`, des valeurs
  d'exemple (12 au plus). Un premier essai à la main sur la phrase de démo,
  le 2026-10-15, a montré que, sans elles, le modèle invente la valeur du filtre
  (`typezone contains "constructible"` au lieu de `U` ou `AUc`).

## À éprouver : la forme `step:k`

Le même essai a montré une confusion de numérotation : le modèle a compté ses
étapes de calcul sans compter les ouvertures de fichiers qu'il avait ajoutées,
et chaque `step:k` pointait alors vers la mauvaise couche. La forme de
référence entre étapes n'est donc pas acquise ; 8.3 à 8.5 la mesurent, et le
verdict de 8.6 peut la remplacer (par exemple par un nom donné à chaque sortie).

## Sous-ensemble de JSON Schema (draft 2020-12)

`schema_validator.py` ne connaît que ces mots-clés :

`$ref` (uniquement `#/$defs/nom`, résolu contre le `$defs` de `catalog.json`),
`description`, `default`, `type` (un nom ou une liste de noms), `enum`,
`properties`, `required`, `additionalProperties` (toujours `false`), `items`,
`minItems`, `maxItems`, `minimum`, `exclusiveMinimum`, `minLength`,
`maxLength`, `pattern` (syntaxe commune à Python et ECMAScript, **ancré** `^…$` et apparié en
entier : un saut de ligne final est refusé).

Pas de `oneOf`, `anyOf`, `allOf`, `not`, `if`, ni de `$ref` extérieur.
`check_catalog.py` refuse tout mot-clé hors de cette liste.
