# Lecture des chiffres — entrée 8.4

Écrite à la main, à partir de `resultats-8-4.md`. Elle ne choisit aucun modèle : c'est le verdict de 8.6.

**Mesure.** Forme « schéma · catalogue complet · avec exemples », 25 phrases, température 0, graine fixe, carte de 8 Go (RTX 3070 Ti), Ryzen 7 5800X, 8 fils. Les trois fichiers GGUF ont été comparés le 2026-10-15 à l'empreinte SHA-256 publiée sur Hugging Face (champ `lfs.oid` de l'API du dépôt), toutes égales :

| Fichier | Dépôt | SHA-256 |
|---|---|---|
| `Qwen3.5-9B-Q4_K_M.gguf` | unsloth/Qwen3.5-9B-GGUF | `03b74727a860a56338e042c4420bb3f04b2fec5734175f4cb9fa853daf52b7e8` |
| `Ministral-3-8B-Instruct-2512-Q4_K_M.gguf` | unsloth/Ministral-3-8B-Instruct-2512-GGUF | `5dbc3647eb563b9f8d3c70ec3d906cce84b86bb35c5e0b8a36e7df3937ab7174` |
| `Ministral-3-8B-Instruct-2512-Q6_K.gguf` | unsloth/Ministral-3-8B-Instruct-2512-GGUF | `c2d0939ba8c980b445097ed00bd3029cfbb937185f3c9a31e26fb62f332c134b` |

(Le dépôt officiel mistralai ne publie que le Q4_K_M, avec une empreinte différente : les fichiers mesurés viennent d'unsloth.) Licence lue sur la fiche : Apache-2.0 pour Qwen3.5-9B et pour Ministral 3 8B.

**Contrôle du gabarit.** L'invite de Ministral suit sa fiche (`[SYSTEM_PROMPT]…[/SYSTEM_PROMPT][INST]…[/INST]`). `llama-tokenize` montre que le jeton de début est ajouté seul : un `<s>` écrit à la main serait doublé, le gabarit n'en écrit donc aucun. Essai sur S07 : plan juste, lisible, 4933 jetons d'invite.

## Ce que disent les chiffres

- **Qwen3.5-9B Q4_K_M : 20/25, identique à 8.5.** La mesure se reproduit à l'identique avec la même version du harnais.
- **Ministral Q6_K : 20/25, comme Qwen.** Égalité. Les phrases fausses diffèrent pourtant : Qwen échoue sur S15, S17, S18, S19, S24 ; Ministral Q6_K sur S11, S14, S17, S19, S24. Trois phrases (S17, S19, S24) sont fausses pour les trois combinaisons : elles tiennent sans doute au catalogue ou au jeu de phrases, pas au modèle (à examiner en 8.6).
- **Ministral Q4_K_M : 17/25.** Trois phrases de moins que Qwen et que le Q6_K. Un écart de moins de deux phrases n'est pas un écart ; trois phrases en est un, mais mince sur 25 phrases et une seule mesure.
- **Pour Ministral, le Q6_K compte trois phrases de plus que le Q4_K_M** (20 contre 17) : au-delà du seuil de deux phrases, mais sur 25 phrases et une seule mesure, donc à confirmer.

## Mémoire et temps

| | RAM au pic | VRAM au pic | Lecture de l'invite | Génération |
|---|---|---|---|---|
| Qwen Q4_K_M | 5,3 Go | 5,8 Go | 1558 ms | 1273 ms |
| Ministral Q4_K_M | 5,0 Go | 6,1 Go | 1574 ms | 1196 ms |
| Ministral Q6_K | 7,0 Go | 7,4 Go | 11359 ms | 2272 ms |

Le Q6_K lit l'invite sept fois plus lentement et génère deux fois plus lentement que le Q4_K_M, alors que le fichier n'est qu'un tiers plus gros. Le journal de llama.cpp dit que ce modèle ne tient pas dans la mémoire vidéo libre (« failed to fit params to free device memory »). Mon hypothèse, non vérifiée : au-delà de 8 Go le pilote Windows déborde sur la mémoire partagée, ce qui ralentit tout. Ce qui la prouverait : refaire la mesure avec moins de couches sur la carte (`--ngl`) ou une carte de 12 Go. Le chiffre de 7,4 Go de VRAM est un excédent d'occupation de la carte, pas la taille du modèle seul.

À retenir pour 8.6, sans conclure : sur cette carte de 8 Go, le Q6_K est mesuré saturé, le Q4_K_M ne l'est pas (5,8 à 6,1 Go de VRAM, 5 Go de RAM). Le journal de llama.cpp n'est pas conservé dans le dépôt.

## Limites

Trois combinaisons, une seule machine, 25 phrases, une mesure chacune. Aucune mesure sur processeur seul, qui est pourtant le cas d'un poste sans carte graphique. Les temps de génération et de lecture sont des moyennes par appel, avec un contexte de 8192 jetons (aucune réduction n'a été nécessaire). Les 8 fils sont le choix par défaut de llama.cpp sur un processeur à 16 fils logiques. La VRAM est un maximum échantillonné toutes les 0,2 s environ, pas une lecture continue.
