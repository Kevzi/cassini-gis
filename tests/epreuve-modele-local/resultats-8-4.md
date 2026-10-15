# Comparaison de modèles et de quantifications — entrée 8.4

Forme : schéma · catalogue complet · avec exemples (la meilleure de 8.5). 25 phrases françaises, température 0, graine fixe, mêmes phrases pour toutes les combinaisons. Aucun accès réseau pendant la mesure. Processeur : AMD Ryzen 7 5800X 8-Core Processor (16 fils logiques).

| Combinaison | Licence | Juste d'emblée | Juste après relance | RAM au pic | VRAM au pic | Chargement | Lecture de l'invite | Génération | Fils | Contexte |
|---|---|---|---|---|---|---|---|---|---|---|
| Qwen3.5-9B Q4_K_M | Apache-2.0 | 20/25 (80%) | 20/25 (80%) | 5.3 Go | 5.8 Go | 2682 ms | 1558 ms (4874 jetons) | 1273 ms (109 jetons) | 8 | 8192 |
| Ministral 3 8B Q4_K_M | Apache-2.0 | 17/25 (68%) | 17/25 (68%) | 5.0 Go | 6.1 Go | 2598 ms | 1574 ms (4938 jetons) | 1196 ms (99 jetons) | 8 | 8192 |
| Ministral 3 8B Q6_K | Apache-2.0 | 20/25 (80%) | 20/25 (80%) | 7.0 Go | 7.4 Go | 3452 ms | 11359 ms (4938 jetons) | 2272 ms (112 jetons) | 8 | 8192 |

Les temps sont des moyennes par appel ; chargement, lecture de l'invite et génération sont séparés. La RAM est le pic du jeu de travail du processus, la VRAM l'excédent d'occupation de la carte pendant l'appel.
Écart de moins de deux phrases d'emblée avec Qwen3.5-9B Q4_K_M (donc pas un écart) : Ministral 3 8B Q6_K.
**MISE EN GARDE : Ministral 3 8B Q6_K lit l'invite plus de trois fois plus lentement que la plus rapide. Cela signale en général une mémoire vidéo saturée (voir « VRAM au pic ») : les temps de cette ligne ne sont pas comparables à ceux des autres, et le taux, lui, reste valable.**
Aucune erreur d'appel.

## État de chaque phrase

✓ juste d'emblée · ↻ juste après relance · ✗ conforme mais différent de l'attendu · ! non conforme · E erreur d'appel

| Phrase | Qwen3.5-9B Q4_K_M | Ministral 3 8B Q4_K_M | Ministral 3 8B Q6_K |
|---|---|---|---|
| S01 | ✓ | ✓ | ✓ |
| S02 | ✓ | ✓ | ✓ |
| S03 | ✓ | ✓ | ✓ |
| S04 | ✓ | ✓ | ✓ |
| S05 | ✓ | ✓ | ✓ |
| S06 | ✓ | ✓ | ✓ |
| S07 | ✓ | ✓ | ✓ |
| S08 | ✓ | ✓ | ✓ |
| S09 | ✓ | ✓ | ✓ |
| S10 | ✓ | ✓ | ✓ |
| S11 | ✓ | ✗ | ✗ |
| S12 | ✓ | ✗ | ✓ |
| S13 | ✓ | ✓ | ✓ |
| S14 | ✓ | ✗ | ✗ |
| S15 | ✗ | ✓ | ✓ |
| S16 | ✓ | ✓ | ✓ |
| S17 | ✗ | ✗ | ✗ |
| S18 | ✗ | ✓ | ✓ |
| S19 | ✗ | ✗ | ✗ |
| S20 | ✓ | ✗ | ✓ |
| S21 | ✓ | ✓ | ✓ |
| S22 | ✓ | ✓ | ✓ |
| S23 | ✓ | ✗ | ✓ |
| S24 | ✗ | ✗ | ✗ |
| S25 | ✓ | ✓ | ✓ |

## Limites

Trois combinaisons, 25 phrases, une mesure par combinaison (déterministe à température 0) : un écart de moins de deux phrases (8 points) n'est pas un écart. Une seule machine : les temps et les mémoires ne valent que pour elle. La VRAM est un maximum échantillonné (`nvidia-smi` toutes les 0,2 s environ), au-dessus de l'état de la carte avant l'appel : elle peut sous-estimer un pic très bref. Le nombre de fils est le choix par défaut de llama.cpp, pas celui de la machine.
