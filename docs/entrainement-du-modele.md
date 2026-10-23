# Entraînement du modèle de composition

Le ticket 5.10 ajuste un petit modèle pour qu'il transforme une phrase française en plan
de commandes du registre. Ce document décrit les trois premières des quatre tâches : le jeu
d'exemples engendré, le jeu tenu à l'écart qui le mesure et le score de référence
(tâche 1), l'entraînement reproductible et la fusion en GGUF (tâche 2), puis le
branchement du modèle ajusté dans `ai/` (tâche 3, en fin de document). La mesure du modèle
ajusté est la tâche 4 (voir `deferred-work.md`).

## Régénérer le jeu d'exemples

Une seule commande, depuis la racine du dépôt :

```
cargo run -p cassini-training --release -- --seed 1 --count 5000 --out exemples.jsonl
```

- `--seed` (par défaut 1) : la graine. Même graine, même fichier, à l'octet près, sous
  Windows comme sous Linux.
- `--count` (par défaut 5000) : le nombre d'exemples.
- `--out` : le fichier écrit. Le jeu n'est pas versionné : il se régénère.

Le résumé par famille (exemples gardés, écartés, doublons) s'imprime sur la sortie
d'erreur. La commande échoue si elle n'atteint pas le nombre demandé.

Chaque ligne du fichier est un enregistrement JSON :

```
{"inventory": {...}, "request": "...", "expected": {...}}
```

- `inventory` : l'inventaire d'un projet synthétique (couches, champs et valeurs,
  fichiers non ouverts), de la même forme que celui de l'épreuve du modèle local.
- `request` : la demande en français.
- `expected` : le plan attendu, sous la forme exacte que rend l'agent —
  `{"kind":"calls","calls":[...]}`, `{"kind":"question","question":"..."}` ou
  `{"kind":"refusal","reason":"..."}`.

Ce format structuré n'écrit aucun texte de prompt. Le format `--format chat` de la
tâche 2 (plus bas) écrit, lui, le texte exact vu par le modèle.

## Comment un exemple est fait

Le générateur vit dans le crate `training/` (`cassini-training`, jamais publié). Il
dépend du noyau (`cassini-core`) et pas de la couche IA. Le hasard vient d'un petit
générateur pseudo-aléatoire écrit à la main (`training/src/rng.rs`) : aucune dépendance
n'a été ajoutée.

1. Une famille tire un inventaire : les couches dont la phrase a besoin, puis des couches
   de distraction et des fichiers non ouverts. Deux couches d'un même groupe (par exemple
   « Routes » et « Réseau routier ») ne se retrouvent jamais ensemble : « une route »
   deviendrait ambigu.
2. Elle écrit la demande à partir de gabarits à emplacements : tournure d'ouverture,
   distance, valeur de champ, accord des participes, ponctuation, politesse, et parfois
   une commune citée comme simple contexte.
3. Elle construit le plan attendu en suivant les règles de chaînage du prompt : le filtre
   sur un champ avant le tampon ou l'intersection, le tampon sur la couche de référence,
   puis des intersections successives dont la première porte la couche cherchée.
4. L'exemple n'est écrit que s'il passe `Registry::check_plan` du registre réel, et
   quelques gardes de plus : les couches citées existent dans l'inventaire, le fichier
   ouvert est parmi les fichiers non chargés, une question finit par « ? », un refus n'est
   jamais formulé en question. Un exemple écarté est compté dans le résumé, jamais écrit.

Les thèmes de couches sont tous civils : urbanisme (parcelles, zonage du PLU, bâtiments,
espaces verts), environnement (rivières, zones humides, zones inondables, ZNIEFF,
captages, forêts, arbres), réseaux (gares, arrêts de bus, routes, pistes cyclables,
lignes électriques, canalisations, parkings), équipements et SIG général (écoles,
hôpitaux, pharmacies, commerces, communes, quartiers). Les valeurs imitent les
standards publics et ne sont tirées d'aucun fichier réel.

## Les familles

| Famille | Ce qu'elle enseigne |
|---|---|
| `filtre_egalite` | « les gares de la ligne D », « les routes nationales » → sélection par attribut |
| `garde_seulement` | « garde seulement », « ne garde que », « conserve uniquement » → une sélection, jamais une suppression |
| `filtre_numerique` | « bâtiments de plus de 12 m de haut », « communes de moins de 12 500 habitants » → `gt`, `lt`, `gte`, `lte` |
| `filtre_texte` | « dont le nom contient », « commence par » → `contains`, `begins_with` |
| `filtre_different` | « sauf celles de la section AB », « qui ne sont pas » → `ne` |
| `tampon` | zone tampon en m ou en km, fusionnée sur demande seulement |
| `tampon_filtre` | tampon autour d'une couche filtrée |
| `distance` | « à moins de 500 m d'une gare » → tampon sur la référence, puis intersection |
| `distance_reference_filtree` | distance à une référence filtrée (« des gares de la ligne D ») |
| `dans_une_zone` | « en zone N », « en zone inondable d'aléa fort » → sélection de la zone, puis intersection |
| `intersection` | « situés dans une zone humide », « croise la couche A avec la couche B » |
| `conditions_multiples` | zone et distance, cible filtrée et distance, deux distances : intersections successives |
| `constructible` | vocabulaire d'urbanisme : « parcelles constructibles », avec ou sans distance et PDF |
| `export_pdf` | « sors-moi la carte en PDF », titre donné par l'utilisateur ou déduit de la demande |
| `isochrone` | « zone atteinte en 10 minutes à pied depuis le point ... (Lambert-93) » |
| `decompte_par_zone` | « nombre d'écoles par commune », nom des zones sur demande seulement |
| `ouverture` | ouvrir un fichier non chargé, CSV à coordonnées compris |
| `description` | décrire une couche, inventaire du projet, version, modèles installés |
| `question_couche_absente` | une couche dont rien ne parle dans l'inventaire → question |
| `question_champ_absent` | un critère qu'aucun champ ne porte → question, jamais un champ voisin deviné |
| `refus` | courriel, suppression, reprojection, export de couche, style, itinéraire, téléchargement, A3, 3D → refus affirmatif |

Une phrase mal comprise par le modèle devient une famille ou une tournure de plus ici,
jamais une règle de prompt.

### Limite connue : « constructible »

« Constructible » se traduit par `typezone` égal à `U`, la zone urbaine du standard du
CNIG. Les zones `AUc` (à urbaniser, ouvertes) sont aussi constructibles, mais le registre
n'a ni opérateur `in` ni union de couches : une seule sélection ne peut pas couvrir les
deux. Le jeu s'en tient donc à `U`.

## Le jeu tenu à l'écart

`ai/tests/tenu_a_l_ecart/` porte 40 phrases inédites sur un inventaire de projet
d'urbanisme : 26 plans (dont la phrase de démonstration exacte de `CLAUDE.md`),
8 questions et 6 refus. C'est la barre honnête.

Règle : il a été écrit par un autre auteur que le générateur, qui ne l'a pas lu. Seuls
deux fichiers le lisent :

- `training/tests/recouvrement.rs` engendre le jeu avec la graine par défaut et vérifie
  qu'aucune phrase tenue à l'écart n'y figure telle quelle (texte normalisé : minuscules,
  sans accents ni ponctuation). S'il échoue, on retire la tournure fautive du générateur
  et on le signale ; on ne retouche jamais une famille pour coller au jeu tenu à l'écart.
- `ai/tests/ecart_reel.rs` mesure le vrai modèle sur ce jeu, sans jamais imprimer le texte
  des phrases.

## Score de référence

Commande (Windows, GPU) :

```
$env:CASSINI_TEST_MODEL="C:\Users\kevin\cassini-modeles\modeles\Qwen3.5-9B-Q4_K_M.gguf"
cargo test -p cassini-ai --release --features cuda -- --ignored --nocapture ecart_reel
```

Un plan est compté juste s'il est identique au plan attendu, sauf le titre du PDF, dont
la présence suffit (nombres comparés en valeur : `500.0` vaut `500`). Une question
attendue doit rendre une question, un refus attendu un refus. Un plan rendu qui ne passe
pas `check_plan` compte comme faux, et l'essai échoue après avoir imprimé la ligne `SCORE`.

Mesure du 2026-10-22, Qwen3.5-9B Q4_K_M sur GPU (`--features cuda`), prompt complet de
`ai/src/plan.rs` au commit `300ac5ee346d06b22abcf22b71e4189a61c9fb68` (dernier commit
qui touche ce fichier), décodage glouton (celui de `cassini-ai`), un seul passage,
344 s pour les 40 phrases (re-mesuré après les correctifs de notation : identique, 346 s ; la phrase de démo H01 est manquée) :

| Sorte | Justes |
|---|---|
| Plans | 19 / 26 |
| Questions | 7 / 8 |
| Refus | 6 / 6 |
| **Total** | **32 / 40** |

Les sept plans manqués ont tous rendu un plan (jamais une question ni un refus) : l'écart
porte sur le contenu du plan. La question manquée a rendu un refus, le défaut déjà noté
dans `deferred-work.md` (une couche absente lue comme une opération impossible). C'est
la barre que le modèle ajusté devra atteindre ou dépasser.

## Tâche 2 : entraînement reproductible et fusion en GGUF

### Le prompt court

Le modèle ajusté ne voit plus le long prompt de `ai/src/plan.rs` (catalogue et exemples) :
il voit un message système de quelques phrases (rôle, trois formes de réponse, chaînage,
question plutôt que deviner, refus seulement si aucune commande ne sait faire), puis
l'inventaire du projet et la demande, au format chatml de Qwen, réflexion coupée.

Ce texte est écrit **à un seul endroit** : `training/src/prompt.rs` (`render_prompt`,
`completion`). La commande l'écrit dans le jeu :

```
cargo run -p cassini-training --release -- --format chat --seed 1 --count 5000 --out entrainement.jsonl
```

Chaque ligne est `{"prompt": "...", "completion": "..."}` : `prompt` est le texte exact vu
par le modèle, jusqu'à l'ouverture du tour assistant comprise ; `completion` est le plan en
JSON compact, `kind` en tête, suivi de `<|im_end|>`. La chaîne Python lit ces deux champs
tels quels et ne met rien en forme : un écart d'un espace entre deux mises en forme
apprendrait au modèle un format qu'il ne verrait jamais. La tâche 3 fera lire ce même
module à `ai/`.

### Une commande pour ré-entraîner

Sous WSL, avec le GPU de 8 Go, depuis la racine du dépôt, dans un dossier vide **hors du
dépôt** :

```
UV_PROJECT_ENVIRONMENT=~/.cache/cassini-entrainement/venv \
  uv run --project training/entrainement python training/entrainement/pipeline.py --sortie ~/cassini-entrainement/5-10-t2
```

(`UV_PROJECT_ENVIRONMENT` est facultatif : il place l'environnement Python sur le disque de
WSL plutôt que dans `training/entrainement/.venv`, sur le disque Windows, plus lent.)

La commande, sans intervention :

1. vérifie le GPU **avant tout téléchargement** et avant de toucher au dossier de sortie
   (au moins 7 Gio, sinon arrêt avec message) ;
2. refuse un dossier de sortie déjà rempli (rien n'est écrasé ; `--reprendre` finit un
   entraînement commencé en sautant les étapes déjà faites) ou situé dans le dépôt ;
3. engendre les jeux par `cassini-training --format chat` : graine 1 pour l'entraînement
   (5 000 exemples), graine 2 pour la validation (300), dont elle retire les prompts déjà
   vus à l'entraînement ;
4. prend la base `Qwen/Qwen3.5-4B` (Apache-2.0) à la révision épinglée
   `851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a`, dans le cache de Hugging Face ;
5. écarte (et compte) tout exemple de plus de 2 048 jetons, sans jamais le tronquer ;
6. entraîne un adaptateur QLoRA : base en nf4, gradient checkpointing, graine fixe, perte
   sur la réponse seule, projections du modèle de langue seulement. La graine fixe rend les
   **jeux** reproductibles à l'octet, pas les **poids** : bitsandbytes et l'implémentation de
   référence de l'attention linéaire ne sont pas déterministes sur GPU ;
7. mesure le score exact sur la validation (`scoring.py`), en décodage glouton : règle de
   `ai/tests/ecart_reel.rs`, avec une exigence de plus, une question ou un refus doit avoir
   un texte non vide ;
8. clone llama.cpp au commit `d81235049384534c167caea52b85a694f6103d14` (celui que lie
   `cassini-ai` par llama-cpp-sys-2 0.1.160) dans `~/.cache/cassini-entrainement/`, construit
   ses outils sur CPU, convertit la base et l'adaptateur en GGUF, fusionne, puis quantifie ;
   chaque fichier s'écrit sous un nom temporaire et ne prend son nom final qu'une fois
   complet. Un test vérifie que `Cargo.lock` tire toujours llama-cpp-sys-2 0.1.160 ;
9. essaie, avec `llama-completion` (l'outil de llama.cpp qui prend un prompt brut ;
   `llama-cli` applique son propre gabarit de conversation), la première demande de
   validation qui attend un plan d'appels, par les deux chemins : la base GGUF avec
   l'adaptateur GGUF (`--lora`), et le modèle fusionné. Les deux sorties doivent se lire
   comme un plan JSON, sinon la commande s'arrête ;
10. écrit `rapport.json`, avec le commit du dépôt, l'état de l'arbre de travail (modifié ou
   non, chemins modifiés) et l'empreinte de `uv.lock`.

Dépendances épinglées par `training/entrainement/uv.lock` (Python 3.12) : torch (BSD-3),
transformers, peft, accelerate, huggingface-hub (Apache-2.0), bitsandbytes (MIT). Rien de cela n'entre dans
la recette : les tests `python3 -I -m unittest discover -s training/entrainement/tests`
(lecture du jeu, notation, gardes de la chaîne) n'importent pas torch.

### Ce qui sort

| Fichier | Rôle |
|---|---|
| `qwen3.5-4b-base-Q4_K_M.gguf` | la base, quantifiée, sur laquelle se charge un adaptateur |
| `cassini-composition-adaptateur-f16.gguf` | l'adaptateur LoRA de composition (environ 50 Mo) |
| `cassini-composition-4b-Q4_K_M.gguf` | base et adaptateur fusionnés, quantifiés Q4_K_M |
| `rapport.json` | révision de la base, commit de llama.cpp, graines, hyperparamètres, versions, perte, score de validation, empreintes SHA-256 |
| `validation-sorties.jsonl` | chaque sortie de validation, attendue et rendue |

Le dossier `travail/` garde les jeux engendrés et l'adaptateur au format de Hugging Face ;
les GGUF intermédiaires en f16 (près de 9 Go chacun) sont effacés à la fin.

### Un adaptateur de domaine, plus tard

La méthode garde la base intacte : l'adaptateur GGUF se charge sur la base GGUF. Un autre
adaptateur de domaine, entraîné ailleurs sur **la même base à la même révision**, avec la
même commande et le même prompt court, se chargera de la même façon, sans ré-entraîner
celui-ci. Le choix entre adaptateurs reviendra au sélecteur de 5.14.

### Mesure

Premier entraînement complet, le 2026-10-22, sous WSL sur la RTX 3070 Ti (8 Go), au commit
`4019c0c` plus une version antérieure des changements de la tâche 2 : avant le renommage en
anglais, avant l'essai par les deux chemins, avant l'écriture des fichiers sous nom temporaire
et avant l'arrêt des restes sans pas d'optimiseur, ce qui fait 8 exemples d'écart
(`rapport.json` de
`~/cassini-entrainement/5-10-t2/`, hors dépôt) :

| Mesure | Valeur |
|---|---|
| Exemples d'entraînement | 5 000 (graine 1), aucun écarté car trop long |
| Longueur des séquences | 742 jetons en médiane, 1 326 au plus |
| Paramètres entraînés | 26,3 millions sur 4,57 milliards (rang 16, alpha 32) |
| Pas d'optimiseur | 312 (lot 1, accumulation 16, une époque, taux 2e-4 en cosinus) |
| Perte | 0,436 au pas 10, 0,005 en fin d'entraînement |
| Pic de mémoire vidéo | 6,5 Gio (réservée par torch), sous les 8 Go |
| Durée de l'entraînement | 2 h 40 |
| Durée de la validation | 47 min (300 demandes, décodage glouton sur le GPU) |
| Durée totale de la commande | 3 h 31, conversion et quantification comprises |
| **Score de validation** | **300 / 300** : plans 250 / 250, questions 37 / 37, refus 13 / 13 |
| Essai `llama-completion` du modèle fusionné | plan JSON lisible et juste (une question) |
| Essai des deux chemins (refait à la main avec `try_model`, sur la première demande qui attend un plan d'appels) | base + adaptateur : plan juste ; modèle fusionné : plan juste |

Les noyaux rapides de l'attention linéaire de Qwen3.5 (`flash-linear-attention`,
`causal_conv1d`) ne sont pas installés : transformers prend son implémentation de
référence en PyTorch, correcte mais plus lente. C'est elle qui fixe la durée.

Le score de validation mesure ce que le modèle a appris des **familles** : la validation
est engendrée par le même générateur, avec une autre graine. Un score parfait y est donc
attendu et ne dit rien de la généralisation à des tournures inédites. La barre honnête
reste le jeu tenu à l'écart, mesuré à la tâche 4 avec le prompt court branché dans `ai/`.

## Tâche 3 : le modèle ajusté branché dans `ai/`

### La déclaration d'un modèle

Un GGUF seul est un modèle ordinaire : il reçoit le long prompt de `ai/src/plan.rs`, comme
avant (c'est le cas de Qwen3.5-9B). Un modèle qui attend autre chose se **déclare** par un
petit fichier `*.modele.json` posé à côté de ses GGUF :

```
{"model": "qwen3.5-4b-base-Q4_K_M.gguf", "adapter": "cassini-composition-adaptateur-f16.gguf", "prompt": "short"}
```

- `model` : le GGUF du modèle (la base, quand un adaptateur est nommé) ;
- `adapter` (facultatif) : l'adaptateur LoRA posé sur cette base, un seul ;
- `prompt` : `short` pour le prompt court de l'entraînement, `full` (ou absent) pour le long.

Des noms de fichiers seuls, du même dossier que la déclaration : une déclaration qui nomme
un dossier, un séparateur ou `..`, qui n'est pas du JSON lisible ou qui porte un champ
inconnu est ignorée par la liste. Une déclaration devient **une** entrée de `model.list`,
et les GGUF qu'elle nomme n'y figurent plus seuls. Une déclaration qui nomme un GGUF absent
reste listée : son chargement rend `ai.model_not_found`, avec le seul nom du fichier. Un
fichier qui n'est pas un adaptateur de cette base rend `ai.adapter`, et rien n'est gardé en
mémoire.

La chaîne d'entraînement écrit les deux déclarations dans son dossier de sortie, aussi à la
reprise (`--reprendre`) :

| Déclaration | Modèle |
|---|---|
| `cassini-composition-4b-adaptateur.modele.json` | base Q4_K_M + adaptateur, prompt court |
| `cassini-composition-4b.modele.json` | modèle fusionné, prompt court |

### Le prompt court dans `ai/`

`ai/src/plan.rs` porte une copie du texte des consignes de `training/src/prompt.rs`
(`SHORT_SYSTEM`) : le produit n'embarque pas la chaîne d'entraînement, qui n'est qu'une
dépendance de développement de `cassini-ai`. Deux tests tiennent les deux textes ensemble :

- sans modèle, pour chacun des 5 000 exemples engendrés, les tours que `ai` assemble valent
  `render_prompt`, octet pour octet, encadrement chatml compris ;
- avec le vrai modèle (`CASSINI_TEST_MODEL`, la base Q4_K_M), pour 300 exemples, les jetons
  que `ai` lit par segments sont ceux du prompt d'entraînement découpé d'un tenant par le
  découpeur de llama.cpp du même GGUF (l'égalité avec celui de Hugging Face, qui a servi à
  l'entraînement, n'est pas vérifiée par ce test).

Le prompt court se coupe au même endroit que le long : la fin des consignes est le point
d'instantané, l'inventaire vient ensuite, puis la demande. Même grammaire, même jugement
par `check_plan`, même redemande unique. L'adaptateur est initialisé une fois au chargement
et posé sur chaque contexte neuf, avant la reprise d'un instantané ; base et adaptateur sont
gardés ensemble, sous une seule clé : un autre adaptateur sur la même base fait tout
recharger et oublier les instantanés.

`cassini plan --model` prend un GGUF ou une déclaration ; sans `--model`, et dans le canal
de l'extension, le noyau prend la première entrée de `model.list`, par ordre de nom. Le
choix entre modèles reviendra au sélecteur de 5.14.

### Installer le modèle ajusté

Copier le contenu du dossier de sortie (les trois GGUF et les deux déclarations) dans un
dossier de modèles : celui que désigne `CASSINI_MODELS`, ou `models/` à côté de
l'exécutable. Vérifier :

```
cassini run model.list '{}'
```

La liste rend les deux déclarations et aucun des trois GGUF seul. Attention : sans
`--model`, la première entrée par ordre de nom l'emporte. Dans un dossier où se trouve
aussi `Qwen3.5-9B-Q4_K_M.gguf`, c'est lui (la majuscule passe avant les minuscules).

### Démonstration

Le 2026-10-22, binaire `cassini` construit sous Windows avec `--features cassini-ai/cuda`
(RTX 3070 Ti), inventaire `tests/epreuve-modele-local/inventory.json`, phrase « Trouve les
parcelles de la zone U à moins de 500 m d'une gare » :

```
cassini plan "Trouve les parcelles de la zone U à moins de 500 m d'une gare" --inventory tests/epreuve-modele-local/inventory.json --model <dossier>/cassini-composition-4b-adaptateur.modele.json
```

| Modèle | Prompt | Résultat | Durée (chargement compris) |
|---|---|---|---|
| base + adaptateur (déclaration) | court | plan valide : sélection `typezone = U`, tampon de 500 m sur les gares, deux intersections | 32 s |
| fusionné (déclaration) | court | même plan | 24 s |
| Qwen3.5-9B (GGUF seul) | long | plan valide, mêmes étapes, intersections dans l'autre ordre | 33 s |

Le dossier de sortie de la tâche 2 a été repris par la chaîne (`--reprendre`, une minute :
les deux déclarations écrites, l'essai par les deux chemins refait) ; `model.list` sur sa
copie rend bien deux entrées. Ces durées ne sont pas une mesure : la mesure du jeu tenu à
l'écart et de la durée sur CPU est la tâche 4.

## L'inventaire vu par le modèle dans QGIS

Tâche 4a du ticket 5.10. Dans QGIS, le modèle ne reçoit pas la sortie de
`project.inventory` telle quelle : le noyau la traduit d'abord dans la forme exacte des
inventaires de l'entraînement (`core/src/agent_inventory.rs`, appelée par `plan.propose`
avant la composition).

**Ce que rapporte l'extension** (faits bruts de QGIS, tous facultatifs : une extension plus
ancienne ne les rend pas, et les clés d'avant gardent leur sens) :

| Clé | Contenu |
|---|---|
| `project_name` | nom du fichier du projet, sans extension ; `null` si le projet n'est pas enregistré |
| `layers[].kind` | `vector`, `raster` ou `other` |
| `layers[].geometry` | nom WKB de QGIS (`MultiPolygon`, `PointZ`, `NoGeometry`…) ; `null` hors vecteur |
| `layers[].feature_count` | nombre d'objets d'une couche vecteur |
| `layers[].fields` | `{name, type, values?}` : type tel que QGIS le nomme ; pour un champ ni numérique ni date, au plus 11 valeurs distinctes non nulles, triées |

L'extension ne décide rien : elle lit au plus 11 valeurs par champ (une de plus que le
plafond du noyau, pour qu'il sache qu'il y en a trop), ne parcourt pas le disque et ne modifie
aucun fichier.

**Ce que décide le noyau** :

- forme `{"project": {name, epsg}, "layers": [...], "unloaded_files": []}` ; par couche,
  `layer_id`, `name`, `kind`, `geometry`, `epsg`, `feature_count`, `source`, `fields` ;
- SCR : `EPSG:2154` devient `2154` ; un SCR qui n'est pas EPSG (`IGNF:LAMB93`) n'a pas de clé
  `epsg` ;
- types : `Integer`, `Integer64` et leurs voisins deviennent `integer` ; `Real`, `Double` et
  leurs voisins `number` ; tout le reste (`String`, `Date`, type inconnu) `string` ;
- valeurs : seulement sur un champ texte, 10 au plus (le jeu tenu à l'écart en montre 10 au
  plus, l'entraînement 9) ; au-delà, ou sans valeur, le champ n'a pas de clé `values` ;
- géométrie : ramenée aux six de l'entraînement (`PointZ` devient `Point`, `MultiSurface`
  `MultiPolygon`) ; sans géométrie ou inconnue, `null` ;
- raster : `geometry: null`, `fields: []`, et pas de `feature_count`, que QGIS n'a pas pour un
  raster (c'est le seul écart de forme avec le générateur, qui en tire un au hasard) ;
- `unloaded_files` vaut toujours `[]`.

`training/tests/forme_inventaire.rs` traduit une sortie représentative de `project.inventory`
et la confronte au jeu engendré (graine 1) : mêmes clés, mêmes sortes de valeurs, types et
géométries déjà vus. Si le générateur change de forme, ce test échoue et la traduction se met
à jour avec le ré-entraînement. `cassini plan --inventory` prend toujours un inventaire déjà
dans la forme apprise : il n'est pas traduit.

**Mesure sur le projet réel** (2026-10-23, QGIS 3.44.15, `projet-test-villeneuve.qgz`, sonde
jetable hors dépôt) : six couches ; la couche `plu_zones` porte `typezone` avec ses valeurs
`["N", "Nh", "U"]`. L'inventaire traduit fait 5 289 caractères, soit **2 439 jetons** du GGUF
de base (`qwen3.5-4b-base-Q4_K_M.gguf`, découpeur de llama.cpp), contre 3 196 pour la sortie
brute. C'est plus que la plus longue séquence entière vue à l'entraînement (1 326 jetons) : le
PLU réel porte 24 champs, dont des empreintes et des libellés longs. L'effet sur le modèle
ajusté se mesure à la tâche 4b.
