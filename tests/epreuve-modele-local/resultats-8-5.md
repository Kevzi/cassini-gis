# Formes de sollicitation — entrée 8.5

Modèle : Qwen3.5-9B-Q4_K_M, quantification Q4_K_M. 25 phrases françaises, température 0, graine fixe : les huit mesures ne diffèrent que par la forme. Aucun accès réseau.

| Forme | Juste d'emblée | Juste après relance | Conformes d'emblée | Relancées | Erreurs d'appel | Rappel du filtre | Jetons d'invite | Génération moyenne | Temps total |
|---|---|---|---|---|---|---|---|---|---|
| libre · catalogue complet · sans exemples | 16/25 (64%) | 16/25 (64%) | 22/25 | 3 | 0 | — | 4467 | 1319 ms | 184 s |
| libre · catalogue complet · avec exemples | 20/25 (80%) | 20/25 (80%) | 24/25 | 1 | 0 | — | 4871 | 1207 ms | 171 s |
| libre · catalogue filtré · sans exemples | 16/25 (64%) | 16/25 (64%) | 20/25 | 5 | 0 | 20/25 | 3450 | 1681 ms | 199 s |
| libre · catalogue filtré · avec exemples | 20/25 (80%) | 20/25 (80%) | 23/25 | 2 | 0 | 20/25 | 3843 | 1500 ms | 180 s |
| schéma · catalogue complet · sans exemples | 17/25 (68%) | 17/25 (68%) | 23/25 | 2 | 0 | — | 4462 | 1302 ms | 182 s |
| schéma · catalogue complet · avec exemples **(meilleure)** | 20/25 (80%) | 20/25 (80%) | 24/25 | 1 | 0 | — | 4874 | 1287 ms | 180 s |
| schéma · catalogue filtré · sans exemples | 16/25 (64%) | 16/25 (64%) | 20/25 | 5 | 0 | 20/25 | 3434 | 1586 ms | 201 s |
| schéma · catalogue filtré · avec exemples | 19/25 (76%) | 19/25 (76%) | 23/25 | 2 | 0 | 20/25 | 3850 | 1827 ms | 194 s |

**Meilleure forme contrainte : schéma · catalogue complet · avec exemples** (20/25 d'emblée, 20/25 après relance).
Aucune égalité à départager.
Formes à moins de deux phrases de la meilleure, donc statistiquement indiscernables d'elle : libre · catalogue complet · avec exemples ; libre · catalogue filtré · avec exemples ; schéma · catalogue filtré · avec exemples.
Aucune erreur d'appel.
La sortie libre n'est qu'un témoin : le produit contraindra la sortie (AD-23).

## État de chaque phrase

✓ juste d'emblée · ↻ juste après relance · ✗ conforme mais différent de l'attendu · ! non conforme · E erreur d'appel

| Phrase | F1 | F2 | F3 | F4 | F5 | F6 | F7 | F8 |
|---|---|---|---|---|---|---|---|---|
| S01 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| S02 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| S03 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| S04 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| S05 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| S06 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| S07 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| S08 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| S09 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| S10 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| S11 | ✗ | ✓ | ✗ | ✓ | ✗ | ✓ | ✗ | ✓ |
| S12 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| S13 | ✓ | ✓ | ! | ✗ | ✓ | ✓ | ! | ✗ |
| S14 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| S15 | ! | ! | ! | ! | ! | ✗ | ! | ! |
| S16 | ✗ | ✓ | ✗ | ✓ | ✗ | ✓ | ✗ | ✓ |
| S17 | ✗ | ✗ | ! | ! | ✗ | ✗ | ! | ! |
| S18 | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ | ✓ | ✗ |
| S19 | ! | ✗ | ! | ✗ | ! | ✗ | ! | ✗ |
| S20 | ✓ | ✓ | ✗ | ✓ | ✓ | ✓ | ! | ✗ |
| S21 | ! | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| S22 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| S23 | ✗ | ✓ | ✗ | ✓ | ✗ | ✓ | ✗ | ✓ |
| S24 | ✗ | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ | ✓ |
| S25 | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

Légende des colonnes : F1 = libre · catalogue complet · sans exemples ; F2 = libre · catalogue complet · avec exemples ; F3 = libre · catalogue filtré · sans exemples ; F4 = libre · catalogue filtré · avec exemples ; F5 = schéma · catalogue complet · sans exemples ; F6 = schéma · catalogue complet · avec exemples ; F7 = schéma · catalogue filtré · sans exemples ; F8 = schéma · catalogue filtré · avec exemples.

## Limites

Un modèle, une quantification, 25 phrases, une mesure par forme (déterministe à température 0) : un écart de moins de deux phrases (8 points) n'est pas un écart. Le filtre lexical à 8 commandes ne retient les bonnes commandes que pour une partie des phrases (colonne « Rappel du filtre »), ce qui plafonne les formes filtrées.
