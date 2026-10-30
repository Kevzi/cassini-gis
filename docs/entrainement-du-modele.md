# Entraînement du modèle de composition

Le ticket 5.10 ajuste un petit modèle pour qu'il transforme une phrase française en plan
de commandes du registre. Ce document décrit ses quatre tâches : le jeu d'exemples engendré,
le jeu tenu à l'écart qui le mesure et le score de référence (tâche 1), l'entraînement
reproductible et la fusion en GGUF (tâche 2), le branchement du modèle ajusté dans `ai/`
(tâche 3), puis l'inventaire vu par le modèle dans QGIS et les mesures (tâche 4, en fin de
document).

## Régénérer le jeu d'exemples

Une seule commande, depuis la racine du dépôt :

```
cargo run -p cassini-training --release -- --seed 1 --count 8000 --out exemples.jsonl
```

- `--seed` (par défaut 1) : la graine. Même graine, même fichier, à l'octet près, sous
  Windows comme sous Linux.
- `--count` (par défaut 8000 depuis 5.15, 5000 avant) : le nombre d'exemples.
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
- `retry` (exemples de redemande seulement, depuis 5.15) : `{"rejected": "...", "errors": [...]}`,
  la sortie rejetée et ses écarts (voir « La redemande » plus bas).

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
   une commune citée comme simple contexte (« à Orly », seule forme depuis 5.15).
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
| `conditions_multiples` | zone et distance, cible filtrée et distance, deux distances, inclusion dans une couche entière : intersections successives |
| `constructible` | vocabulaire d'urbanisme : « parcelles constructibles », avec ou sans distance et PDF |
| `export_pdf` | « sors-moi la carte en PDF », titre donné par l'utilisateur ou déduit de la demande |
| `isochrone` | « zone atteinte en 10 minutes à pied depuis le point ... (Lambert-93) » |
| `isochrone_vers_cible` | « les écoles à 10 min à pied du point ... (Lambert-93) » → isochrone, puis intersection de la cible avec l'étape 1 ; la couche du lieu (gares, écoles...) est souvent dans l'inventaire, seules les coordonnées donnent l'origine ; une fois sur trois, le lieu est nommé avec ses coordonnées (« de la gare de X (652 000 / 6 862 000, Lambert-93) ») et reste un plan |
| `decompte_par_zone` | « nombre d'écoles par commune », nom des zones sur demande seulement |
| `ouverture` | ouvrir un fichier non chargé, CSV à coordonnées compris |
| `description` | décrire une couche, inventaire du projet, version, modèles installés |
| `question_couche_absente` | une couche dont rien ne parle dans l'inventaire → question |
| `question_champ_absent` | un critère qu'aucun champ ne porte → question, jamais un champ voisin deviné |
| `refus` | courriel, suppression, reprojection, export de couche, style, itinéraire, téléchargement, 3D → refus affirmatif |
| `refus_hors_registre` | champ de vision, ligne de vue, pente, cadrage ou zoom de la carte, PDF en A3, A0 ou portrait, altitude moyenne par zone (statistiques sur un raster), MNT présent pour le terrain, trajet depuis plusieurs lieux (« de chaque gare », « de toutes les gares », « des gares ») → refus affirmatif, jamais un plan de substitution |
| `appartenance_commune` | « qui appartiennent à la commune de X », « sur le territoire de X » → sélection de la commune par son nom, puis intersection ; sans couche de communes, ou sans nom de commune, une question. Une commune citée seule (« à Orly ») reste un contexte sans effet |
| `question_isochrone` | isochrone ou cible à une durée de trajet d'un lieu nommé (« depuis la mairie de X », « les parcelles à 10 min à pied de la gare de X »), sans coordonnées → question qui demande le point en X et Y avec son système ; « d'une gare » → question qui demande laquelle |
| `etape_unique` | une seule opération demandée, une autre couche citée pour le contexte (« je m'occuperai des écoles plus tard ») → une étape |
| `redemande` | environ 5 % du jeu : un exemple d'une autre famille vu au tour de redemande (voir plus bas) |

Depuis 5.15, `conditions_multiples` porte aussi les plans à **trois conditions** (cible
filtrée, zone et distance ; zone et deux distances), quatre à six étapes, jamais plus de six.
L'export PDF s'y ajoute une fois sur quatre, et seulement aux plans de cinq étapes ou moins
(jamais à « zone et deux distances », qui en a déjà six).

### Nouvelle passe de 5.17 (tâche A)

Les constats de 5.15 sur le jeu tenu à l'écart et sur la phrase E de `duree_reel` deviennent
des familles, jamais des règles de prompt :

- **Trajet depuis un lieu de l'inventaire.** La même tournure de trajet (« à N minutes à pied
  de ... », « situées à », « à moins de », « accessibles en ... depuis », « qu'on rejoint
  en ») rend une question quand l'origine est un lieu sans coordonnées (`question_isochrone`,
  couche du lieu présente le plus souvent, cible souvent citée, la question du
  garde-fou de 5.16 parmi les réponses), et un plan quand l'origine est un point en
  coordonnées (`isochrone_vers_cible` : `analysis.isochrone`, puis `analysis.intersect` de la
  cible avec `step:1`). Une distance en mètres reste un plan (`distance`). « À N minutes à
  pied de » nu passe par « situées à » : c'est la tournure de la phrase E.
- **Origines multiples** (décision du propriétaire, 2026-10-23, option a) : « de chaque
  gare », « de toutes les gares », « des gares » → refus affirmatif (`refus_hors_registre`) :
  l'isochrone du catalogue part d'un seul point. « D'une gare » → question (laquelle, en
  coordonnées) ; « de la gare de X » → question.
- **Plans à plusieurs conditions.** La cible filtrée l'est aussi par un nombre (`gt`, `lt`,
  `gte`, `lte`), une différence (`ne`) ou un nom (`contains`, `begins_with`), plus seulement
  par égalité. Deux formes s'ajoutent : cible filtrée et deux distances (cinq étapes :
  sélection, deux tampons, deux intersections), et distance avec inclusion dans une couche
  entière (« situées dans une zone humide », la couche prise telle quelle, intersections dans
  l'ordre de la phrase). Le poids de `conditions_multiples` passe de 9 à 13. Un test vérifie
  qu'aucune étape n'est perdue : chacune sert à une étape suivante.

**Limite de la mesure.** Ces familles visent des écarts nommés par le jeu tenu à l'écart
(H08, H17 à H23, H40), sans en lire les phrases. Viser des écarts nommés affaiblit
pourtant `ecart_reel` comme mesure aveugle : le score qui suivra mesure en partie ce qu'on a
cherché à corriger. Il faudra le lire avec cette réserve, et un jeu neuf, écrit par un autre
auteur, redonnera une mesure aveugle.

### Les inventaires de QGIS (5.15)

Les valeurs d'un champ texte sont triées, comme le produit les rend. `unloaded_files` est
vide sauf quand la famille ouvre un fichier (`ouverture`, avec parfois un second fichier à ne
pas ouvrir) : dans QGIS, l'extension ne parcourt pas le disque. Un inventaire sur trois prend
les formes que rend `core/src/agent_inventory.rs` depuis QGIS : raster sans `feature_count`,
couche temporaire sans `source`, SCR qui n'est pas EPSG (couche ou projet sans `epsg`), table
de distraction sans géométrie (`geometry: null` sur un vecteur), projet non enregistré (sans
`name`). `training/tests/forme_inventaire.rs` vérifie que chacune de ces formes, rendue par
le produit, est vue à l'entraînement.

### La redemande (5.15)

Un exemple `redemande` part d'un exemple valide d'une autre famille dont la réponse est un
plan d'appels, et corrompt ce plan comme un modèle le corromprait : identifiant de couche
inconnu, référence `step:k` vers l'étape elle-même ou une suivante, étape retirée qui casse
une référence. La corruption n'est gardée que si `check_plan` la rejette (un identifiant
inconnu ne l'est jamais : `check_plan` ne voit pas l'inventaire, l'exemple est laissé). Le
prompt est alors celui que `cassini-ai` assemble à la redemande (`retry_turns`) : le prompt
du premier essai, la sortie rejetée en tour assistant, puis le tour « Ta réponse n'est pas
conforme : étape k : message. Redonne le plan corrigé, uniquement le JSON. » ; la réponse
attendue est le plan juste. La perte ne porte que sur cette réponse, `data.py` n'a pas changé.

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
  sans accents ni ponctuation), ni à un nombre ou une commune près. S'il échoue, on retire
  la tournure fautive du générateur et on le signale ; on ne retouche jamais une famille
  pour coller au jeu tenu à l'écart. Depuis 5.15, le même test couvre les phrases des
  essais réels `plan_reel`, `question_reel` et `duree_reel` (qui, eux, ne sont pas tenus à
  l'écart). Il a fait retirer trois tournures : « Fais une zone tampon de N m autour des
  ... » devient « Fais-moi une zone tampon ... », « Décris la couche des ... » devient
  « Décris-moi la couche des ... », et « de la section AB » sort des tournures des parcelles
  (« en section AB », « de section AB » restent).
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

Depuis 5.15, les consignes disent que les étapes se comptent à partir de 1, comme le long
prompt (« la couche produite par l'étape k (en comptant à partir de 1) par
"step:k.layer_id" »). Le modèle livré par 5.10 a été entraîné sur l'ancien texte : il faut le
ré-entraîner (tâche B de 5.15) avant de le mesurer avec ce prompt.

Ce texte est écrit **à un seul endroit** : `training/src/prompt.rs` (`render_prompt`,
`completion`). La commande l'écrit dans le jeu :

```
cargo run -p cassini-training --release -- --format chat --seed 1 --count 8000 --out entrainement.jsonl
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
   (8 000 exemples depuis 5.15, 5 000 avant), graine 2 pour la validation (300), dont elle retire les prompts déjà
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

### Un adaptateur de domaine : `--exemples-externes` (5.23)

La méthode garde la base intacte : l'adaptateur GGUF se charge sur la base GGUF. La même
commande entraîne un autre adaptateur, de domaine, sur un fichier d'exemples écrit ailleurs,
hors de ce dépôt : **même base à la même révision, mêmes graines, mêmes hyperparamètres,
même prompt court**.

```
uv run --project training/entrainement python training/entrainement/pipeline.py \
    --sortie <dossier hors dépôt> --exemples-externes <fichier.jsonl> --nom <nom>
```

- Le fichier est au format `chat` de `cassini-training` : une ligne `{"prompt", "completion"}`
  par exemple, le prompt déjà mis en forme (prompt court, chatml), la réponse finie par
  `<|im_end|>`. Il est lu et vérifié **avant** le GPU et le téléchargement : une ligne mal
  formée arrête la commande avec le nom du fichier et le numéro de la ligne.
- `--nom` : minuscules, chiffres, `-` et `_`. Un nom qui reprendrait une sortie du modèle
  civil (tout nom qui commence par `cassini-composition`) est refusé avant tout travail.
- **Mélange** : par défaut, le jeu civil engendré (`--exemples`, `--validation`) plus le fichier
  externe ; `--externe-seul` n'entraîne que sur le fichier externe.
- **Validation** : par défaut, 10 % du fichier externe, tirés par la graine de validation, sont
  mis de côté pour le score ; `--validation-externe <fichier>` donne un second fichier à la place,
  et rien n'est alors retiré du premier. Un prompt de validation vu à l'entraînement en est
  retiré et compté.
- Deux scores, séparés : `validation` (le jeu civil, quand il est mélangé, comme en 5.17) et
  `validation_externe`. Ensemble, ils montrent si l'adaptateur oublie les commandes ordinaires.
- `rapport.json` ne garde du fichier externe que son empreinte SHA-256 et ses comptes
  (`donnees.externe`) ; l'essai `llama-completion` sur un exemple externe n'y écrit ni la sortie
  ni l'attendu. Les sorties de validation externes (`validation-externe-sorties.jsonl`) restent
  dans le dossier de sortie, hors dépôt.
- `--reprendre` refuse un autre fichier externe ou un autre mélange que ceux de l'entraînement
  commencé.

| Fichier de sortie | Rôle |
|---|---|
| `qwen3.5-4b-base-Q4_K_M.gguf` | la base, identique à chaque entraînement, partagée par tous les adaptateurs |
| `<nom>-adaptateur-f16.gguf` | l'adaptateur LoRA de domaine |
| `<nom>.modele.json` | sa déclaration : base + adaptateur, prompt court |

Pas de modèle fusionné. Pour l'installer, copier l'adaptateur et sa déclaration dans le dossier
des modèles (celui que désigne `CASSINI_MODELS`, ou `models/` à côté de l'exécutable), et la
base si elle n'y est pas déjà : `cassini run model.list '{}'` liste alors la déclaration. Le
choix entre adaptateurs reviendra au sélecteur de 5.14.

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

- sans modèle, pour chacun des 8 000 exemples engendrés, les tours que `ai` assemble valent
  `render_prompt`, octet pour octet, encadrement chatml compris ; et pour chaque exemple de
  redemande, la sortie rejetée jugée par `ai` donne les mêmes écarts, et les tours de
  `retry_turns` valent le prompt de redemande de l'entraînement (`render_retry_prompt`) ;
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

### Budget d'inventaire (5.18)

Chaque plan exécuté ajoute ses couches de résultat au projet, donc à l'inventaire : à la recette
enchaînée de 5.17, la 4ᵉ phrase ne tenait plus dans le contexte (8 192 jetons). Le noyau borne
donc l'inventaire vu par le modèle (`core/src/agent_inventory.rs`, `within_budget_by_batches` depuis 5.19) : sous le
budget, il est rendu tel quel ; au-delà, toutes les couches de l'utilisateur restent, avec les
couches de résultat les plus récentes qui tiennent, dans l'ordre de l'inventaire. Une couche de
résultat est une couche dont le fichier source est dans le dossier de travail `<projet>.cassini/`
ou dans son dossier de rejeu `replay/` ; la plus récente est celle dont le fichier est le plus
récent (à date égale, le plus grand numéro d'étape). Rien n'est retiré de QGIS et une couche
gardée n'est pas modifiée. Une couche retirée reste donc dans le projet : un plan qui la cite
s'exécute, seul le modèle ne la voit plus.

Le budget est en **caractères** du texte JSON compact de l'inventaire, parce que le noyau ne
tokenise pas. Mesure du 2026-10-24, tokeniseur du modèle livré
(`cassini-composition-4b-Q4_K_M.gguf`), test ignoré
`the_inventory_budget_fits_the_context_with_the_real_tokenizer` de `ai/src/plan.rs` :

| Mesure | Caractères | Jetons | Jeton par caractère |
|---|---|---|---|
| inventaire de Villeneuve (6 couches) | 4 007 | 1 458 | 0,364 |
| couche d'intersection à 34 champs (parcelles × zones du PLU) | 1 999 | 724 | 0,362 |
| tout le reste au premier essai (consignes, armature, longue phrase) | — | 359 | — |
| ajout de la redemande au pire (sortie recopiée à 1 500 caractères, 8 écarts) | — | 1 106 | — |

Il reste donc 8 192 − 1 024 (réponse) − 359 − 1 106 = 5 703 jetons pour l'inventaire, soit
15 600 caractères au rapport mesuré. Le budget est fixé à **14 000 caractères** : Villeneuve
plus quatre intersections et une cinquième coupée (13 972 caractères) donnent une redemande de
6 501 jetons, plus 1 024 de réponse, soit 7 525 sur 8 192. Le budget tient tant que
l'inventaire ne dépasse pas 0,41 jeton par caractère. Le contrôle exact de `cassini-ai`
(`ai.prompt_too_long`) reste le dernier filet ; son message nomme désormais l'inventaire du
projet et conseille d'en retirer des couches. Le budget vise le prompt court du modèle livré :
le long prompt, plus lourd de son catalogue, garde son erreur typée.

**Recette enchaînée** (2026-10-24, QGIS 3.44.15, `projet-test-villeneuve.qgz`, noyau du
produit sur processeur, modèle livré seul dans `CASSINI_MODELS`, sonde jetable hors dépôt sur le
modèle de `qgis-plugin/tests/probe_demo.py`) : les dix phrases de la recette de 5.17, dans une
même session, chaque plan exécuté. Aucune `ai.prompt_too_long` (en 5.17 : phrases 4 à 10) ; le
projet passe de 6 à 18 couches. Les phrases 1 à 3 rendent les mêmes plans et les mêmes nombres
qu'en 5.17 (5 étapes, 2 429 parcelles ; une sélection, 137 zones ; 3 étapes, 2 513 parcelles,
« ces zones » toujours mal reprises, hors ticket). Les phrases 4 à 10 rendent ce qu'elles
rendaient une à une sur projet vierge en 5.17 : PDF, tampon puis intersection (40 576), question
du garde-fou, isochrone puis intersection, refus, question du modèle, refus.

Seconde recette enchaînée, après la relecture (même poste, même modèle, dix autres phrases :
démo, zones U, parcelles dans ces zones près d'une gare, tampon de 200 m, routes à 30 km/h,
bâtiments près de ces routes, altitude moyenne du MNT par zone, zones N, démo de nouveau, PDF) :
aucune `ai.prompt_too_long`, le projet passe de 6 à 19 couches (39 289 caractères bruts).
Huit plans exécutés, un refus juste (statistiques sur un raster). Constat : la phrase de démo,
juste en 1ʳᵉ position, rend en 9ᵉ position une question (« Je ne vois pas de champ de type de
zone dans la couche plu_zones »), alors que `plu_zones` et `typezone` sont dans l'inventaire
vu. L'inventaire au budget (14 000 caractères) est bien plus gros que ceux de l'entraînement
(couches citées plus une à quatre de distraction) : la justesse du modèle sur de longs
inventaires de session reste à apprendre (`deferred-work.md`).

Jeu de mesures sur GPU après le changement, modèle livré : `plan_reel` 25 / 25 (0 question),
`question_reel` 12 / 12, `ecart_reel` 33 / 40 (plans 19 / 26, questions 8 / 8, refus 6 / 6),
les mêmes qu'en 5.17 : ces essais passent par `cassini-ai` et des inventaires sous le budget.

### Ordre stable et identifiants courts (5.19)

Pour que la lecture d'une phrase reprenne l'état du moteur au plus près, l'inventaire vu par le
modèle est rangé dans un ordre stable (`stable_order`) : les couches de l'utilisateur d'abord,
dans l'ordre de QGIS, puis les couches de résultat de la plus ancienne à la plus récente (l'ordre
inverse du budget). Un plan exécuté n'ajoute donc ses couches qu'à la fin. L'entraînement mélange
l'ordre des couches : cet ordre est un ordre déjà vu.

Le modèle voit aussi un identifiant court pour chaque identifiant QGIS `nom_<uuid>` : `nom_` suivi
des 4 premiers chiffres hexadécimaux de l'uuid (`parcelles_e9ba` pour
`parcelles_e9ba733c_8a43_4717_ac73_04344c1b6bb5`), la forme engendrée pour un tiers des couches
de l'entraînement. Deux courts qui se confondent reçoivent un chiffre de plus, ensemble, jusqu'à
être uniques ; tout autre identifiant est montré tel quel. La table est faite sur toutes les
couches avant le budget, pour qu'un court ne change pas quand une couche sort de la vue. Le noyau
retraduit le plan rendu vers les identifiants complets avant de le vérifier (`plan.propose`) : le
plan gardé, montré, exécuté et journalisé ne porte que les vrais identifiants ; un court inconnu
reste tel quel et le plan est refusé comme avant. Les essais réels `plan_reel`, `question_reel`
et `ecart_reel` passent par la même table (`CASSINI_TEST_FULL_IDS=1` pour les identifiants
complets). Mesures : `mesure-duree-par-phrase.md`, « Session enchaînée dans QGIS ».

Le budget de 5.18 retire désormais **par paquets** (décision du 2026-10-29) : quand la vue dépasse
14 000 caractères, les résultats les plus anciens sortent jusqu'à descendre sous 7 000
(`within_budget_by_batches`). La décision est rejouée à chaque phrase sur les résultats présents,
du plus ancien au plus récent : les résultats gardés restent les mêmes jusqu'au dépassement
suivant, et l'invite d'avant reste un préfixe de la nouvelle.

L'invite reste celle de l'entraînement à l'octet près : `cassini-ai` coupe seulement sa lecture
juste avant la dernière clé de chaque couche, là où le découpeur de Qwen ne change aucun jeton
(prouvé au vrai tokeniseur sur 300 inventaires engendrés, 1 357 coupes, et sur les 339 redemandes
du jeu entier).

## Tâche 4 : mesures

Tâche 4b du ticket 5.10, le 2026-10-23, au commit `540f59c` plus les changements de la tâche :
les deux déclarations de la tâche 2 (`C:\Users\kevin\cassini-modeles\ajuste-5-10-t2\`, prompt
court) et, pour comparaison, Qwen3.5-9B Q4_K_M seul (long prompt). Décision du propriétaire
(2026-10-23) : un chiffre sous la barre est consigné tel quel, la tâche se ferme comme
« mesurée » et un ré-entraînement est proposé ; le ticket 5.10 reste ouvert.

### Ce qui a changé pour mesurer

- `cassini_ai::plan::resolve_model` lit un modèle d'une seule façon (GGUF seul → long prompt ;
  déclaration → base, adaptateur et prompt qu'elle nomme), pour la CLI et pour les essais. Une
  déclaration absente rend `ai.model_not_found`, une déclaration illisible
  `models.bad_declaration`, sans recopier son dossier.
- Les essais réels `ecart_reel`, `question_reel`, `plan_reel` et `duree_reel` prennent dans
  `CASSINI_TEST_MODEL` un GGUF ou une déclaration (`ai/tests/common/mod.rs`).
- `plan_reel` : au plus 2 fausses questions sur 25 pour le long prompt (concession du
  2026-10-17), au plus 1 pour le prompt court (plafond strict du ticket).
- `ecart_reel` : le notateur est une fonction (`grade`) testée sans modèle. Pour chaque phrase
  manquée, il imprime la forme de l'écart (sorte, nombre d'étapes, ordre, commande ou nom du
  paramètre à l'étape k), jamais une valeur ni le texte ; la ligne `REDEMANDES` compte les
  phrases passées par la redemande et celles qu'elle a sauvées. Les étapes « dans un autre
  ordre » se reconnaissent en remplaçant chaque `step:k` par l'étape k elle-même.
- `duree_reel` : le scénario commence par `A0`, la phrase de démo exacte de `CLAUDE.md`, et
  passe d'abord une amorce (« Décris le projet. ») qui paie seule le chargement et la lecture
  de l'invite jusqu'à l'instantané : la ligne de `A0` ne mesure que sa demande, comme une
  demande dans une session de `cassini serve` déjà ouverte.

### Parité des découpeurs

`training/entrainement/parite_decoupeurs.py` (lancée à la main, dans l'environnement `uv` de
la chaîne) découpe les prompts d'un jeu `--format chat` par le découpeur de Hugging Face de
la base (révision épinglée, `add_special_tokens=False`, comme l'entraînement) et par
`llama-tokenize` du llama.cpp du cache (`--ids --no-bos`) sur le GGUF de base :

```
UV_PROJECT_ENVIRONMENT=~/.cache/cassini-entrainement/venv \
  uv run --project training/entrainement python training/entrainement/parite_decoupeurs.py \
  --gguf ~/cassini-entrainement/5-10-t2/qwen3.5-4b-base-Q4_K_M.gguf \
  --jeu ~/cassini-entrainement/5-10-t2/travail/donnees/validation.jsonl
```

Résultat : **300 / 300 prompts identiques** (205 803 jetons), aucun écart. Avec le test de la
tâche 3 (jetons de `ai` lus par segments = découpage d'un tenant par llama.cpp), le modèle voit
dans le produit le découpage de son entraînement.

### Jeu tenu à l'écart (GPU)

`cargo test -p cassini-ai --release --features cuda -- --ignored --nocapture ecart_reel`,
RTX 3070 Ti, décodage glouton :

| Modèle | Prompt | Plans | Questions | Refus | **Total** | Redemandes (sauvées) | Durée |
|---|---|---|---|---|---|---|---|
| base + adaptateur | court | 19 / 26 | 8 / 8 | 5 / 6 | **32 / 40** | 1 (0) | 414 s |
| fusionné | court | 20 / 26 | 8 / 8 | 5 / 6 | **33 / 40** | 2 (0) | 595 s |
| Qwen3.5-9B (référence) | long | 19 / 26 | 7 / 8 | 6 / 6 | **32 / 40** | 1 (1) | 389 s |

La référence de la tâche 1 (32/40) se retrouve à l'identique. Le modèle ajusté l'**égale**
(base + adaptateur) ou la **dépasse d'une phrase** (fusionné), avec un modèle deux fois plus
petit et un prompt bien plus court. La phrase de démo (H01) est juste pour les deux chemins ;
le 9B la manque (opérateur de la sélection).

Formes des écarts du modèle ajusté (les mêmes pour les deux chemins, à H11 et H20 près) :

| Phrase | Écart |
|---|---|
| H08 | une étape de trop (2 au lieu de 1) |
| H17, H18 | une étape de moins (3 au lieu de 4) |
| H19 | une étape de moins (5 au lieu de 6) |
| H20 | une étape de moins (adaptateur) ; plan invalide deux fois (fusionné) |
| H22 | une étape de trop (4 au lieu de 3) |
| H11 | paramètre `origin` de l'isochrone (adaptateur seulement) |
| H40 | refus attendu : un plan après redemande (adaptateur), un plan invalide deux fois (fusionné) |

Les plans de plusieurs conditions (intersections successives) sont la faiblesse : quatre
plans sur sept manqués perdent une étape. Les questions sont parfaites (8/8), là où le 9B
transforme H32 en refus.

**Valeur de la redemande** : 1 à 2 phrases sur 40 y passent, aucune n'est sauvée par le
modèle ajusté (le 9B en sauve une). Le modèle ajusté n'a jamais appris ce tour : il est gardé
(AD-23, une seule redemande), mais ne rapporte rien aujourd'hui.

### Seuil de clarification (GPU)

| Essai | base + adaptateur | fusionné | Seuil |
|---|---|---|---|
| `question_reel` (12 phrases ambiguës) | **11 / 12** questions | **11 / 12** | au moins 9 : tenu |
| `plan_reel` : fausses questions sur 25 phrases claires | **0** | **0** | au plus 1 : tenu |
| `plan_reel` : plans conformes | **16 / 25** | **16 / 25** | 80 % (20) : **sous la barre** |

`plan_reel` **échoue** donc son assertion des 80 % avec les deux déclarations (16/25) ; son
plafond de fausses questions (0, au plus 1) est tenu.

Q11 (« qui appartiennent à la commune ») rend un plan d'intersection au lieu d'une question.
Les neuf phrases manquées de `plan_reel` (S10, S15, S18, S20 à S25) attendent toutes un
**refus** : cadrage de la carte, altitude par commune (statistiques sur un raster), PDF en A3,
et six demandes de terrain (champ de vision, ligne de vue, pente) que le registre n'a pas
encore. Le modèle ajusté y rend un plan de substitution (tampon, sélection, statistiques,
isochrone) ou un plan invalide, jamais une question. Le générateur n'a aucune famille de refus
pour ces demandes : c'est à apprendre, pas à corriger dans le prompt.

### Durée sur processeur (4 fils, 4 cœurs physiques)

`duree_reel`, binaire **sans CUDA**, `CASSINI_TEST_THREADS=4`, masque d'affinité `0x55` sur le
Ryzen 7 5800X, inventaire traduit du projet réel
(`../donnees-test-cassini/inventaire-villeneuve-traduit-4b.json`, hors dépôt : 4 007 caractères,
1 458 jetons du GGUF de base, par le noyau et l'extension actuels ; la tâche 4a en comptait
2 439, chiffre pris avant les correctifs de sa relecture, et aucun filtre de longueur n'explique
l'écart). Modèle fusionné :

| id | appels | chargement (s) | lecture du prompt (s) | jetons de l'invite | jetons repris | écriture (s) | jetons écrits | total (s) | issue |
|---|---|---|---|---|---|---|---|---|---|
| amorce | 1 | 2,5 | 28,5 | 1737 | 0 | 2,3 | 17 | 33,4 | plan, 1 étape(s) |
| **A0** | 1 | 0,0 | 1,0 | 1768 | 1719 | 40,4 | 248 | **41,4** | plan, 5 étape(s) |
| A | 1 | 0,0 | 1,1 | 1772 | 1719 | 40,9 | 252 | 42,0 | plan, 5 étape(s) |
| B1 | 1 | 0,0 | 0,5 | 1740 | 1719 | 11,4 | 71 | 11,9 | plan, 1 étape(s) |
| B2 | 1 | 0,0 | 0,7 | 1751 | 1719 | 19,6 | 120 | 20,3 | plan, 2 étape(s) |
| B3 | 1 | 0,0 | 0,5 | 1740 | 1719 | 3,3 | 23 | 3,9 | plan, 1 étape(s) |
| C | 1 | 0,0 | 0,6 | 1746 | 1719 | 20,6 | 124 | 21,3 | plan, 2 étape(s) |
| D | 1 | 0,0 | 0,5 | 1740 | 1719 | 4,3 | 27 | 4,8 | question |
| E | 2 | 0,1 | 4,4 | 3685 | 3438 | 39,4 | 242 | 43,8 | erreur : agent.plan_invalid |

Moyennes hors amorce : 23,7 s. Base + adaptateur : A0 en **43,2 s** (écriture 42,0 s),
moyenne 26,8 s.

**Verdict A0 : 41,4 s, au-dessus des 30 s.** La lecture de l'invite est réglée : l'invite
entière ne fait plus que 1 737 jetons (contre 5 130 au long prompt), lue en 28,5 s une fois par
session, puis 1 s par demande. Il reste l'écriture des 248 jetons du plan à cinq étapes, à
environ 6 jetons/s, bornée par la mémoire (Qwen3.5-9B : environ 4 jetons/s au même réglage,
relevé du 2026-10-18 de `mesure-duree-par-phrase.md`, « Après le cache »). C'est la longueur du plan
écrit qu'il faut réduire (identifiants de couche longs recopiés à chaque étape, titre du PDF),
ou le débit d'écriture qu'il faut augmenter.

### Démo de bout en bout dans QGIS

Le 2026-10-23, QGIS 3.44.15 (Python de QGIS, plateforme offscreen), projet
`projet-test-villeneuve.qgz`, sonde `qgis-plugin/tests/probe_demo.py` (lancée à la main sous
`python-qgis-ltr.bat`, `CASSINI_CORE` et `CASSINI_MODELS` posés, projet en argument ; refaite après
son versionnement : même plan, PDF lisible, inventaire 0,05 s) : vrai `cassini serve` lancé par
`qgis-plugin/cassini_qgis/client.py`, avec les vrais gestes de l'extension. Noyau construit
avec `--features cassini-ai/cuda` ; `CASSINI_MODELS` sur un dossier qui ne contient que le
modèle fusionné et sa déclaration (liens durs), le dossier des modèles du poste n'est pas
touché. Phrase exacte de `CLAUDE.md` :

> Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à Villeneuve-le-Roi
> et sors-moi la carte en PDF.

| Étape | Résultat | Durée |
|---|---|---|
| `project.inventory` (geste QGIS, demandé par `plan.propose`) | 6 couches ; traduit par le noyau, identique octet pour octet à l'inventaire de la mesure CPU | **0,05 s** |
| Composition (`plan.propose`, chargement du modèle compris) | plan de 5 étapes, validé | 24,8 s |
| 1. `analysis.select_by_attribute` `typezone = U` sur `plu_zones` | 137 zones | 0,17 s |
| 2. `analysis.buffer` 500 m autour des gares | 8 objets | 0,03 s |
| 3. `analysis.intersect` parcelles × zones U | 6 792 objets | 3,7 s |
| 4. `analysis.intersect` (étape 3) × tampon (étape 2) | 2 429 objets | 1,8 s |
| 5. `layout.export_pdf`, titre « Parcelles constructibles du PLU à moins de 500 m d'une gare à Villeneuve-le-Roi » | `30-layout.export_pdf.pdf`, 1 page, 3,2 Mo | 1,3 s |

**Réussite** : le plan enchaîne sélection de la zone U, tampon de 500 m sur les gares, deux
intersections et PDF titré, il s'exécute sans erreur, le PDF est écrit à côté du projet (`projet-test-villeneuve.cassini/`) et les cinq
étapes sont au journal. `project.inventory` ne coûte que 0,05 s sur ce projet : la mise en
cache des valeurs par session n'est pas nécessaire. Le chemin complet (`serve`, canal,
`plan.propose`, inventaire traduit, vrai modèle) est vérifié.

### Bilan contre la barre du ticket

| Barre | Mesure | |
|---|---|---|
| ≥ 32/40 sur le jeu tenu à l'écart | 32/40 (adaptateur), 33/40 (fusionné) | tenue |
| ≥ 9/12 questions | 11/12 | tenue |
| ≤ 1 fausse question sur 25 | 0 | tenue |
| `plan_reel` ≥ 80 % | 16/25, l'essai échoue | **sous la barre** (refus de terrain, cadrage, A3, raster jamais appris) |
| < 30 s par longue phrase, CPU 4 fils, à chaud | A0 : 41,4 s | **sous la barre** (écriture du plan) |
| première demande d'une session, CPU 4 fils | chargement 2,5 s + lecture complète de l'invite 28,5 s (amorce), puis A0 41,4 s : environ 72 s (somme des deux lignes, non mesurée d'un seul appel) | **sous la barre** |
| démo QGIS, phrase exacte | plan juste, exécuté, PDF | tenue |

Déclaration retenue : le **modèle fusionné** (`cassini-composition-4b.modele.json`), meilleur
d'une phrase et un peu plus rapide. Un ticket de ré-entraînement est proposé (voir
`deferred-work.md`).

## Tâche B de 5.15 : ré-entraînement et mesures

Le générateur de la tâche A (nouvelles familles, redemande, inventaires de QGIS, prompt court
« en comptant à partir de 1 ») a été entraîné par la commande unique, sans autre changement :
8 000 exemples, même base, même méthode, même révision de llama.cpp.

### Entraînement

| Mesure | Valeur |
|---|---|
| Exemples | 8 000 d'entraînement, 300 de validation, aucun écarté car trop long |
| Jetons par séquence | médiane 735, maximum 1 521 (la limite de 2 048 suffit, redemandes comprises) |
| Validation (graine 2, glouton) | 299 / 300 : plans 231 / 232, questions 37 / 37, refus 31 / 31 |
| Durée totale | 241 min (RTX 3070 Ti, WSL) |
| Essai `llama-completion` | base + adaptateur et fusionné rendent le plan attendu |

### Parité des découpeurs

- `parite_decoupeurs.py` (Hugging Face contre llama.cpp, d'un tenant) : **300 / 300**, 0 écart.
- Test de `ai` sur le vrai modèle (la base `qwen3.5-4b-base-Q4_K_M.gguf` sortie par cet
  entraînement), étendu depuis 5.15 au tour de redemande : 300 premiers essais et
  **349 redemandes** du jeu de 8 000, mêmes écarts que le jugement de `ai`, même point
  d'instantané de la tête fixe, jetons de `ai` (par segments) égaux à ceux du
  prompt d'entraînement lu d'un tenant.

### Essais réels (GPU, modèle fusionné, prompt court)

| Essai | 5.10 (fusionné) | 5.15 |
|---|---|---|
| `plan_reel` | 16 / 25 | **25 / 25**, 0 question |
| `question_reel` | 11 / 12 | **12 / 12** |
| `ecart_reel` | 33 / 40 | **32 / 40** (plans 19 / 26, questions 8 / 8, refus 5 / 6), 316 s |
| redemandes sur `ecart_reel` | 2 (0 sauvée) | 1 (0 sauvée) |

Formes des écarts du jeu tenu à l'écart :

| Phrase | Écart |
|---|---|
| H08, H23 | une question au lieu d'un plan |
| H17, H18, H20 | une étape de moins |
| H19 | paramètre `operator` de l'étape 1 |
| H22 | commande de l'étape 2 |
| H40 | un plan au lieu du refus attendu |

Sur le jeu tenu à l'écart, le modèle de 5.15 rend **une phrase juste de moins** que le
fusionné de 5.10 (32 contre 33). Les plans à plusieurs conditions restent sa faiblesse, alors que les
phrases de l'épreuve sont désormais toutes justes. Ces formes nomment des constats ; elles
ne disent rien du texte des phrases, qui n'est jamais lu.

### Durée sur processeur (4 fils, 4 cœurs physiques)

Même recette qu'en 5.10 (affinité `0x55`, `CASSINI_TEST_THREADS=4`, inventaire de
Villeneuve-le-Roi traduit) :

| id | appels | lecture du prompt (s) | écriture (s) | jetons écrits | total (s) | issue |
|---|---|---|---|---|---|---|
| amorce | 1 | 26,7 (+ chargement 2,5) | 2,0 | 17 | 31,2 | plan, 1 étape |
| A0 | 1 | 1,0 | 36,7 | 248 | 37,7 | plan, 5 étapes |
| A | 1 | 1,1 | 39,4 | 252 | 40,5 | plan, 5 étapes |
| B1 | 1 | 0,5 | 10,3 | 71 | 10,8 | plan, 1 étape |
| B2 | 1 | 0,6 | 26,4 | 180 | 27,1 | plan, 3 étapes |
| B3 | 1 | 0,5 | 2,9 | 23 | 3,5 | plan, 1 étape |
| C | 1 | 0,6 | 18,2 | 124 | 18,8 | plan, 2 étapes |
| D | 1 | 0,5 | 3,9 | 27 | 4,4 | question |
| E | 2 | 4,7 | 44,7 | 304 | 49,5 | plan, 2 étapes |

Moyennes hors amorce : 24,0 s. A0 passe de 41,4 s à 37,7 s ; la durée n'est pas un critère
de 5.15. L'exigence produit (une longue phrase en moins de 30 s sur processeur) reste
manquée par A0, A et E ; elle est suivie dans `deferred-work.md`.

**Phrase E** (« … à 10 minutes à pied de la gare de Villeneuve-le-Roi. ») : un plan valide
après redemande, au lieu de la question attendue. Le modèle **invente l'origine** de
l'isochrone (`origin` en EPSG:4326, x = 2,458, y = 48,658, à peu près Villeneuve-le-Roi) et
croise les parcelles avec la zone obtenue. La couche des gares est dans l'inventaire, mais
rien n'y donne le point en coordonnées. En 5.10, la même phrase finissait en plan invalide.

### Bilan contre la barre du ticket

| Barre | Mesure | |
|---|---|---|
| `plan_reel` ≥ 20 / 25 | 25 / 25 | tenue |
| ≤ 1 fausse question sur 25 | 0 | tenue |
| `ecart_reel` ≥ 32 / 40 | 32 / 40 | tenue, de justesse |
| `question_reel` ≥ 9 / 12 | 12 / 12 | tenue |
| phrase E rendue en question | un plan, origine inventée | **sous la barre** |

Barre manquée sur la phrase E seulement. Décision du propriétaire pour ce cas : consigner et
s'arrêter. La démo QGIS n'est pas rejouée, le modèle n'est pas installé dans `models/` ; les
sorties restent hors du dépôt, dans le dossier des modèles de la machine (`ajuste-5-15`). Une nouvelle passe sur
le générateur (isochrone dont l'origine est une couche de points présente sans le point
choisi, et plans à plusieurs conditions) fera l'objet d'un autre ticket.

## Tâche B de 5.17 : ré-entraînement et mesures

Le générateur de la tâche A de 5.17 (commit `ac11428` : trajet depuis un lieu de
l'inventaire, origines multiples, plans à plusieurs conditions) a été entraîné par la
commande unique, sans autre changement : 8 000 exemples, même base (`Qwen/Qwen3.5-4B` @
`851bf6e`), même méthode, même révision de llama.cpp (`d812350`).

### Entraînement

| Mesure | Valeur |
|---|---|
| Exemples | 8 000 d'entraînement, 300 de validation, aucun écarté car trop long |
| Jetons par séquence | médiane 749, maximum 1 659 (la limite de 2 048 suffit) |
| Perte finale | 0,0044 (500 pas) |
| Validation (graine 2, glouton) | 298 / 300 : plans 236 / 237, questions 35 / 36, refus 27 / 27 |
| Durée totale | 330 min, dont 49 min de validation (RTX 3070 Ti, WSL) |
| Essai `llama-completion` | base + adaptateur et fusionné rendent le plan attendu |
| Modèle fusionné | `cassini-composition-4b-Q4_K_M.gguf`, SHA-256 `6bde9803c06b85d796255c35d533f9f856681a8ee66049416100badf185a8084` |

La durée passe de 241 à 330 min pour le même nombre d'exemples ; les séquences sont un peu
plus longues (médiane 749 contre 735), ce qui n'explique pas tout. La cause n'est pas établie.

### Parité des découpeurs

- `parite_decoupeurs.py` : **300 / 300**, 0 écart.
- Test de `ai` sur la base `qwen3.5-4b-base-Q4_K_M.gguf` sortie par cet entraînement :
  300 premiers essais et **339 redemandes** identiques au jeton.

### Essais réels (GPU, modèle fusionné, prompt court)

Les scores de justesse se mesurent sur le GPU (décision du propriétaire du 2026-10-17) ; ils ne sont pas
refaits sur le processeur.

| Essai | 5.15 | 5.17 |
|---|---|---|
| `plan_reel` | 25 / 25, 0 question | **25 / 25**, 0 question, 0 du garde-fou |
| `question_reel` | 12 / 12 | **12 / 12**, 0 du garde-fou |
| `ecart_reel` | 32 / 40 | **33 / 40** (plans 19 / 26, questions 8 / 8, refus 6 / 6), 331 s |
| redemandes sur `ecart_reel` | 1 (0 sauvée) | 2, sur H17 et H19 (0 sauvée) |

Formes des écarts du jeu tenu à l'écart :

| Phrase | Écart en 5.17 | En 5.15 |
|---|---|---|
| H08 | une question au lieu d'un plan | même écart |
| H13 | paramètre `label_field` de l'étape 1 | juste |
| H17, H18 | une étape de moins | même écart |
| H19 | une étape de moins | paramètre `operator` de l'étape 1 |
| H20 | une étape de plus | une étape de moins |
| H22 | une étape de plus | commande de l'étape 2 |
| H23 | juste | une question au lieu d'un plan |
| H40 | juste | un plan au lieu du refus attendu |

H23 et H40 deviennent justes ; H13 recule. Les plans à plusieurs conditions restent la
faiblesse : le nombre d'étapes y est encore faux, dans les deux sens. Le gain tient à une
phrase ; il se lit avec la réserve de la tâche A : les familles ont visé des écarts nommés,
`ecart_reel` n'est plus tout à fait une mesure aveugle.

### Durée sur processeur (4 fils, 4 cœurs physiques)

Même recette qu'en 5.15 (affinité `0x55`, `CASSINI_TEST_THREADS=4`, inventaire de
Villeneuve-le-Roi traduit) :

| id | appels | lecture du prompt (s) | écriture (s) | jetons écrits | total (s) | 5.15 (s) | issue |
|---|---|---|---|---|---|---|---|
| amorce | 1 | 28,1 (+ chargement 2,5) | 2,2 | 17 | 32,8 | 31,2 | plan, 1 étape |
| A0 | 1 | 1,0 | 39,5 | 248 | 40,5 | 37,7 | plan, 5 étapes |
| A | 1 | 1,1 | 39,7 | 252 | 40,9 | 40,5 | plan, 5 étapes |
| B1 | 1 | 0,5 | 11,1 | 71 | 11,7 | 10,8 | plan, 1 étape |
| B2 | 1 | 0,7 | 28,7 | 180 | 29,4 | 27,1 | plan, 3 étapes |
| B3 | 1 | 0,5 | 3,1 | 23 | 3,7 | 3,5 | plan, 1 étape |
| C | 1 | 0,6 | 19,6 | 124 | 20,3 | 18,8 | plan, 2 étapes |
| D | 1 | 0,5 | 4,2 | 27 | 4,8 | 4,4 | question |
| E | 1 | 0,7 | 5,6 | 32 | 6,4 | 49,5 | question du modèle |

Moyenne hors amorce : 19,7 s (24,0 s en 5.15). Tout le gain vient de E ; les autres phrases
écrivent le même nombre de jetons qu'en 5.15 et prennent de 1 à 8 % de plus, écart de la
machine d'un jour à l'autre. L'exigence produit (une longue phrase en moins de 30 s sur
processeur) reste manquée par A0 et A ; elle est suivie dans `deferred-work.md`.

### La phrase E

« Trouve les parcelles à 10 minutes à pied de la gare de Villeneuve-le-Roi. », sur le même
inventaire traduit, avec la déclaration du modèle de 5.17 (`--model`) :

| Chemin | Réponse |
|---|---|
| `duree_reel` (processeur, 4 fils, tête du prompt reprise d'un instantané) | question du modèle, 32 jetons |
| `llama-completion`, prompt d'entraînement rendu par `cassini-training`, sans grammaire | question du modèle : « Quelles sont les coordonnées du point de départ (X, Y et système, par exemple Lambert-93) ? » |
| `cassini plan`, noyau du produit (processeur) ou noyau CUDA, modèle installé ou copie d'origine | question du **garde-fou** (`"source":"guard"`) |
| panneau QGIS (`cassini serve`, `plan.propose`), noyau installé | question du **garde-fou** |

Le modèle hésite entre la question et un point de départ inventé : la balance penche d'un
côté ou de l'autre selon la façon dont le prompt est calculé (fils, instantané de la tête,
noyau). La cause exacte n'est pas mesurée. Dans le produit, c'est le point inventé qui
l'emporte, et le garde-fou de 5.16 le change en question : l'utilisateur ne reçoit jamais un
plan faux, mais la réponse n'est pas apprise.

Piège de mesure : une phrase passée à `cassini.exe` par un script PowerShell enregistré en
UTF-8 **sans BOM** arrive aux accents abîmés (Windows PowerShell lit le script en ANSI). Une
première mesure de E, faite ainsi, avait donné à tort une question du modèle sur processeur.
Les scripts de mesure s'enregistrent en UTF-8 avec BOM.

Phrases de contraste, même inventaire, noyau et modèle installés (script en UTF-8 avec BOM) :

| Phrase | Réponse |
|---|---|
| « … à 10 minutes à pied du point 655200, 6849800 (Lambert-93). » | `analysis.isochrone` depuis ce point, puis `analysis.intersect` des parcelles |
| « … à 10 minutes à pied de chaque gare. » | refus : l'isochrone part d'un seul point |
| « … à 10 minutes à pied des points 655200, 6849800 et 656100, 6850400 (Lambert-93). » | question du garde-fou : `llama-completion` montre deux origines écrites dans un même objet, sortie rejetée |

### Démo de bout en bout dans QGIS

Le 2026-10-24, même sonde et même projet qu'à la tâche 4 de 5.10. **Noyau du produit**
(processeur, installé dans `%LOCALAPPDATA%\Cassini\bin`), `CASSINI_MODELS` sur un dossier qui
ne contient que le modèle fusionné et sa déclaration. Phrase exacte de `CLAUDE.md`.

| Étape | Résultat | Durée |
|---|---|---|
| `project.inventory` | 6 couches | 0,06 s |
| Composition (`plan.propose`, chargement du modèle et lecture du prompt à froid compris) | plan de 5 étapes, validé | 67,1 s |
| 1. `analysis.select_by_attribute` `typezone = U` sur `plu_zones` | 137 zones | 0,17 s |
| 2. `analysis.buffer` 500 m autour des gares | 8 objets | 0,05 s |
| 3. `analysis.intersect` parcelles × zones U | 6 792 objets | 3,9 s |
| 4. `analysis.intersect` (étape 3) × tampon (étape 2) | 2 429 objets | 1,9 s |
| 5. `layout.export_pdf`, titre « Parcelles constructibles du PLU à moins de 500 m d'une gare à Villeneuve-le-Roi » | 1 page, 3,0 Mo, PDF lisible | 1,2 s |

**Réussite** : même plan et mêmes nombres qu'à la tâche 4 de 5.10, les cinq étapes au
journal. La composition à froid sur processeur (67 s) compte le chargement et la première
lecture du prompt ; la mesure à chaud est celle de `duree_reel`.

### Bilan contre la barre du ticket

| Barre | Mesure | |
|---|---|---|
| `plan_reel` ≥ 20 / 25 | 25 / 25 | tenue |
| ≤ 1 fausse question sur 25 | 0 | tenue |
| `ecart_reel` ≥ 32 / 40 | 33 / 40 | tenue |
| `question_reel` ≥ 9 / 12 | 12 / 12 | tenue |
| phrase E rendue en question par le modèle | question du garde-fou dans le produit ; du modèle seulement dans `duree_reel` | **manquée** |

Barre manquée sur E. Décision du propriétaire (2026-10-24) : garder quand même le modèle de
5.17, meilleur que celui de 5.15 sur tout le reste, E restant rattrapée par le garde-fou. Il est
installé dans le dossier des modèles du poste sous le nom de déclaration que le panneau a
retenu (`cassini-composition-4b.modele.json`) ; celui de 5.15 y reste sous le nom
`cassini-composition-4b-5-15`. La suite de E fera l'objet d'un autre ticket (`deferred-work.md`).
