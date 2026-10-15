---
title: 'Harnais de mesure du taux de plans corrects'
type: 'chore'
ticket: '3'
created: '2026-10-15'
status: 'built'
baseline_revision: 'e777918'
route: 'oneshot'
route_source: 'pinned'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** l'épreuve du modèle local ne produit aucun chiffre tant qu'aucun programme ne pose les 25 phrases à un modèle, ne valide ses plans et ne les compare aux plans attendus. Les entrées 8.4 et 8.5 ne doivent ensuite que lancer des mesures.

**Approach:** un harnais Python, bibliothèque standard seule, dans `tests/epreuve-modele-local/`, qui appelle `llama-completion` en ligne de commande, valide chaque sortie avec `plan_validator.py`, redemande une fois une sortie non conforme, compare à l'attendu et imprime un taux au premier essai et un taux après relance. Il prend en paramètres le modèle, la quantification et la forme de sollicitation : catalogue complet ou filtré, avec ou sans exemples, sortie contrainte par schéma JSON ou libre. Le report d'unités de `deferred-work.md` est traité d'abord : `canonical` normalise m et km, s et min.

## Boundaries & Constraints

**Always:**
- Code jetable hors du produit ; bibliothèque standard seule ; aucun accès réseau, aucun serveur : un processus `llama-completion` par appel (AD-15 écarté pour ce code, comme l'epic le décide).
- Une sortie non conforme est rejetée et redemandée, jamais réparée à la main (AD-23). La redemande reçoit la liste des écarts ; elle n'a lieu qu'une fois, et seulement si la première sortie est non conforme.
- Un plan conforme mais différent de l'attendu est faux et n'est pas redemandé.
- Taux au premier essai : plans corrects dès la première sortie. Taux après relance : ceux-là plus les non conformes devenus corrects à la seconde.
- Les phrases mesurées (`sentences.json`, `sentences_en.json` avec `--lang en`) ne figurent jamais dans les exemples de l'invite ; les exemples vivent dans `examples.json`, sur d'autres phrases.
- Le catalogue filtré garde les commandes les plus proches de la phrase par recouvrement lexical, déterministe ; le rappel du filtre (commandes attendues retenues) est rapporté.
- La sortie contrainte est un schéma JSON engendré depuis `catalog.json` (suite d'étapes ou refus), débarrassé des motifs que llama.cpp ne sait pas lire.
- Chaque mesure écrit un rapport JSON : paramètres, résultat par phrase, taux, jetons, temps de chargement, de lecture et de génération.

**Never:**
- Lancer les mesures comparatives ou choisir le modèle : c'est 8.4 et 8.5.
- Verser un fichier GGUF ou tout fichier de modèle au dépôt.
- Modifier `catalog.json`, `inventory.json` ou les 25 plans attendus.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 25 plans attendus rejoués | réponses de rejeu = plans attendus | taux 100 % aux deux essais | — |
| Plan volontairement faux | plan conforme mais différent | compté faux, non redemandé | — |
| Commande absente du catalogue | sortie nommant `layer.delete` | non conforme au premier essai, redemandé | l'écart nomme la commande |
| Couche ou chemin absent de l'inventaire | identifiant ou fichier inventé | non conforme, redemandé | l'écart nomme l'intrus |
| Sortie non JSON | texte libre | non conforme, redemandé | l'écart dit que la sortie n'est pas du JSON |
| Redemande réussie | première sortie non conforme, seconde correcte | compté au taux après relance seulement | — |
| Refus attendu | WGS84 | correct si la sortie est un refus | un plan de terrain en 4326 est non conforme |

</frozen-after-approval>

## Code Map

- `tests/epreuve-modele-local/plan_validator.py` -- modifier `canonical` : normaliser m et km en mètres, s et min en secondes.
- `tests/epreuve-modele-local/harness.py` -- à créer : invite, appel de llama.cpp, extraction du JSON, double essai, taux, rapport ; option `--replay` pour rejouer des sorties enregistrées sans modèle.
- `tests/epreuve-modele-local/examples.json` -- à créer : trois exemples phrase-plan hors du jeu mesuré.
- `tests/epreuve-modele-local/check_harness.py` -- à créer : rejoue des sorties enregistrées et vérifie la matrice ci-dessus, sans modèle.
- `tests/epreuve-modele-local/README.md` -- compléter : usage du harnais.
- `sentences.json`, `catalog.json`, `inventory.json`, `schema_validator.py` -- à réutiliser sans les modifier.
- Chemins sur le poste : `C:/Users/kevin/cassini-modeles/llama.cpp/llama-completion.exe` et `C:/Users/kevin/cassini-modeles/modeles/Qwen3.5-9B-Q4_K_M.gguf`, passés en paramètres, jamais écrits en dur.

## Tasks & Acceptance

**Execution:**
- [x] `plan_validator.py` -- normalisation d'unités dans `canonical`, avec un cas dans `check_sentences.py`.
- [x] `harness.py` -- construire l'invite (système, catalogue complet ou filtré, inventaire, exemples éventuels), appeler `llama-completion`, extraire le JSON, valider, redemander une fois, comparer, imprimer et écrire le rapport.
- [x] `examples.json` -- trois exemples distincts des 25 phrases et des 10 anglaises.
- [x] `check_harness.py` -- rejeu de la matrice, sans modèle.
- [x] `README.md` -- usage et paramètres.

**Acceptance Criteria:**
- Given `python check_harness.py`, when il s'exécute, then il sort 0 après avoir rejoué les 25 phrases d'un coup avec deux taux de 100 %, compté faux un plan volontairement faux et rejeté comme non conformes une commande, une couche et un chemin absents.
- Given le modèle local et `harness.py` sur une phrase, when la sortie contrainte est demandée, then le harnais imprime les deux taux et écrit un rapport, sans réseau.

## Implementation Notes

Route oneshot épinglée : l'estimation dépasse 100 lignes mais aucun sous-agent de codage n'est autorisé ; l'implémentation se fait en direct.

Fait : normalisation d'unités dans `canonical` (m et km, s et min), `harness.py`, `examples.json`, `check_harness.py`, README. Preuve du critère 2 : `harness.py` lancé sur Qwen3.5-9B Q4_K_M, sortie contrainte par schéma, phrases S07, S17 et S25 : 3 sur 3 conformes d'emblée, 2 sur 3 correctes (S07 et le refus WGS84 de S25 ; la chaîne de démo S17 est conforme mais fausse), aucune relance, charge 2,6 s, lecture de 4 455 jetons en 1,5 s, génération de 128 jetons en 1,5 s ; rapport écrit, aucun réseau. Ce n'est qu'un essai de fonctionnement sur trois phrases, pas une mesure : les mesures sont l'affaire de 8.4 et 8.5.

## Design Notes

Essai préalable : llama.cpp accepte `-jf <schéma>`, une sortie contrainte donne du JSON valide du catalogue ; seul le motif de `layer.add` (assertion négative) n'est pas géré et doit être simplifié avant. Sur Windows, le fichier de schéma s'écrit en UTF-8 (la console, non).

## Verification

**Commands:**
- `python -I tests/epreuve-modele-local/check_harness.py` -- expected: sortie 0
- `python -I tests/epreuve-modele-local/check_sentences.py` -- expected: sortie 0

## Review Triage Log

Relecture quick, 8 constats, tous réels, tous corrigés.
- medium : un texte avant ou après le JSON était écarté en silence, donc compté correct au premier essai, ce qui gonflait le taux que 8.4 et 8.5 comparent (AD-23 : rejeté, jamais réparé). L'extraction exige maintenant du JSON seul ; seuls le bloc de réflexion et une clôture ``` sont tolérés. Cas négatifs ajoutés.
- medium : un délai dépassé ou un code de sortie non nul étaient comptés comme une sortie non conforme du modèle, et un chemin faux plantait tout. Désormais un statut d'erreur d'appel, sans relance, compté à part et signalé à l'écran ; les chemins sont vérifiés avant de commencer.
- low : avec `--think`, le raisonnement du modèle était lu comme la réponse ; le bloc `<think>` est retiré. Cas ajouté.
- low : `--ids` ne correspondant à rien faisait planter l'affichage après l'écriture du rapport ; message clair et code 2, avec les identifiants connus.
- low : le filtre lexical comptait les mots vides ; liste de mots vides ajoutée, rappel assertif plancher (19 sur 24, valeur mesurée, inchangée par les mots vides).
- low : trois assertions de `check_harness.py` ne vérifiaient pas ce que leur message disait (rappel de la réponse précédente, nom de la couche et du chemin intrus) ; resserrées.
- low : critère 2 sans preuve dans le diff ; consignée ci-dessus.
- Note : le rappel du catalogue filtré à 8 commandes est de 19 sur 24 plans, soit un plafond de 79 % pour cette forme tant que le filtre ne s'améliore pas : à garder en tête pour 8.5.
