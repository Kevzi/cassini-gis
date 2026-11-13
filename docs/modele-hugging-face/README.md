---
license: apache-2.0
base_model: Qwen/Qwen3.5-4B
language:
  - fr
  - en
library_name: gguf
pipeline_tag: text-generation
tags:
  - gguf
  - gis
  - qgis
  - cassini
  - llama.cpp
---

# cassini-composition-4b (GGUF, Q4_K_M)

Modèle local de **Cassini**, l'interface en langage naturel d'un SIG qui fonctionne 100 % hors
ligne (<https://kevzi.github.io/cassini-gis>). Il transforme une demande en français en une
suite de commandes typées du registre de Cassini, que l'utilisateur relit, valide et exécute
dans QGIS.

*English: the local model of Cassini, an offline natural-language interface for GIS. It turns a
French request into a sequence of typed commands that the user reviews before anything runs.*

## Usage prévu

- **Civil et généraliste** : analyses cartographiques d'un bureau d'études (urbanisme,
  environnement), d'une collectivité, d'un étudiant — zone tampon, intersection, sélection,
  pente, champ de vision, export de carte en PDF.
- Il ne produit **aucune réponse libre** : seulement des plans de commandes, vérifiés par le
  noyau de Cassini avant exécution ; l'utilisateur valide chaque plan.
- Il ne se connecte à aucun réseau : chargé dans le processus du noyau par llama.cpp.
- Hors de ce cadre (conversation générale, génération de code, tout autre usage), il n'a pas été
  évalué et n'est pas conçu pour cela.

## Fichier

| Fichier | Contenu |
|---|---|
| `cassini-composition-4b-0.1.0-Q4_K_M.gguf` | modèle quantifié Q4_K_M, 2 783 446 944 octets |

SHA-256 : `b687313f4a49c23e13673d14d185f3ee56d3e710e8d01c08595339236b21a865`

Posé dans le dossier `models\` à côté de `cassini.exe`, le noyau le trouve seul. Les installeurs
de Cassini le téléchargent depuis cette adresse et vérifient cette empreinte.

## Origine et modifications

- **Base** : [`Qwen/Qwen3.5-4B`](https://huggingface.co/Qwen/Qwen3.5-4B), licence Apache-2.0,
  © Alibaba Cloud (voir `NOTICE`).
- **Modifications** (ce fichier n'est pas le modèle de Qwen) : ajustement par adaptateur LoRA sur
  un jeu d'exemples **synthétiques** — des demandes en français et le plan de commandes attendu —
  engendré par le générateur du dépôt de Cassini (`training/`), sans donnée de tiers ; fusion de
  l'adaptateur dans les poids de la base ; conversion en GGUF et quantification Q4_K_M avec
  llama.cpp.

## Licence

Apache-2.0, comme la base : voir `LICENSE`. Cassini (noyau) est aussi sous Apache-2.0 ; son
extension QGIS est sous GPL, comme QGIS.

## Limites

Le modèle peut proposer un plan faux ou refuser une demande qu'il juge hors de son registre :
c'est pourquoi Cassini affiche le plan et attend la validation de l'utilisateur. Il est ajusté
pour le français ; les autres langues ne sont pas évaluées.

## Contact

cassini-sig@proton.me
