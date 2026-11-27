# Catalogue du programme d'exemple

Les commandes ajoutées par `programme-exemple`, au format du catalogue public : un bloc
`json fiche` par commande, sous un titre de niveau 3. La recette `check_added_catalogue` de
`cassini-core` le confronte au registre du programme.

### `exemple.echo`

Rend le texte reçu et sa longueur.

```json fiche
{"id": "exemple.echo", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"text": {"type": "string", "required": true}},
 "result": ["text", "length"]}
```
