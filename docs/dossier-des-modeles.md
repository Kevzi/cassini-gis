# Dossier des modèles

La commande `model.list` donne la liste des modèles d'IA locale (fichiers `.gguf`)
disponibles sur le poste. Elle ne charge ni n'ouvre aucun modèle : elle lit les noms
et les tailles, sans réseau et sans téléchargement.

## Ordre de recherche (fixe)

1. La variable d'environnement `CASSINI_MODELS`, qui désigne un dossier.
2. Le dossier `models/` placé à côté de l'exécutable du noyau (`cassini`).

Si `CASSINI_MODELS` est posée mais que son dossier n'existe pas, c'est une erreur
(`models.no_folder`) : Cassini ne se rabat jamais sur `models/`, pour qu'un réglage
faux ne soit pas masqué. Sans variable ni `models/`, la même erreur est rendue.

Un dossier trouvé sans aucun `.gguf` n'est pas une erreur : la liste est vide.
Seuls les fichiers réguliers dont l'extension est `gguf` (majuscules admises) comptent,
dans le dossier lui-même (pas de sous-dossiers), triés par nom.

## Recette

1. Poser `CASSINI_MODELS` vers un dossier contenant deux fichiers `.gguf`.
2. Lancer la commande `model.list` (CLI ou registre) : les deux fichiers sont listés
   avec leur taille en octets.
3. Pointer `CASSINI_MODELS` vers un dossier absent : l'erreur `models.no_folder` apparaît.

Codes d'erreur : `models.no_folder`, `models.unreadable` (dossier illisible).

## Commandes ajoutées connues du modèle (ticket 5.52)

Un programme noyau dérivé (entrée 2.14) peut ajouter des commandes que le modèle local n'a
jamais apprises. La déclaration `*.modele.json` nomme celles qu'il connaît par un champ
facultatif `commands` :

```json
{"model": "base.gguf", "adapter": "lora.gguf", "prompt": "short", "commands": ["exemple.echo"]}
```

- Seules les commandes ajoutées nommées là entrent dans le prompt, la grammaire et le
  préchauffage du modèle ; les autres en sont exclues, et un plan qui les nomme est rejeté.
- Une déclaration sans le champ reste valide : elle ne connaît aucune commande ajoutée. Un
  identifiant que le registre ne connaît pas est ignoré, sans erreur.
- Un GGUF seul, sans déclaration (long prompt, essais, sans promesse de justesse), n'a rien qui
  limite le catalogue : il voit toutes les commandes ajoutées.
- Le registre officiel n'a aucune commande ajoutée : rien ne change pour lui.
- Le catalogue de session, le formulaire du panneau et les outils MCP listent toujours les
  commandes ajoutées, connues du modèle ou non.
