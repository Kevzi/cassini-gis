# Entraînement du modèle de composition

Le ticket 5.10 ajuste un petit modèle pour qu'il transforme une phrase française en plan
de commandes du registre. Ce document décrit les deux premières des quatre tâches : le jeu
d'exemples engendré, le jeu tenu à l'écart qui le mesure et le score de référence
(tâche 1), puis l'entraînement reproductible et la fusion en GGUF (tâche 2, en fin de
document). Le branchement dans `ai/` et la mesure du modèle ajusté sont les tâches 3 et 4
(voir `deferred-work.md`).

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
