# Entraînement du modèle de composition

Le ticket 5.10 ajuste un petit modèle pour qu'il transforme une phrase française en plan
de commandes du registre. Ce document décrit la première des quatre tâches : le jeu
d'exemples engendré, le jeu tenu à l'écart qui le mesure, et le score de référence.
L'entraînement lui-même, la fusion en GGUF et la mesure du modèle ajusté sont les
tâches 2 à 4 (voir `deferred-work.md`).

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

Aucun texte de prompt n'est écrit : la mise en forme appartient aux tâches suivantes.

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
