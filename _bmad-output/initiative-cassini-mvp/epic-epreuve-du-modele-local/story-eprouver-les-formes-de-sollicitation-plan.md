---
title: 'Eprouver les formes de sollicitation'
type: 'chore'
ticket: '5'
created: '2026-10-15'
status: 'built'
baseline_revision: 'e411baf'
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

**Problem:** on ne sait pas quelle forme d'invite fait composer au modèle local les plans les plus justes. La rédaction des descriptions de commandes du vrai registre (epic 2) et le choix du format de plan en dépendent, et la comparaison des modèles de 8.4 doit se faire sur la meilleure forme, pas sur une invite au hasard.

**Approach:** lancer `harness.py` sur les huit formes que le harnais accepte (catalogue complet ou filtré, avec ou sans exemples, sortie libre ou contrainte par schéma), sur le même modèle (Qwen3.5-9B Q4_K_M) et les 25 phrases françaises, à froid et à graine fixe. Un petit programme `run_forms.py` enchaîne les huit mesures et écrit un tableau comparatif `resultats-8-5.md`, qui nomme la meilleure forme.

## Boundaries & Constraints

**Always:**
- Même modèle, même quantification, mêmes 25 phrases, `--temp 0` et graine fixe : les huit mesures ne diffèrent que par la forme.
- La sortie libre n'est qu'un témoin : le produit contraindra la sortie (AD-23). La meilleure forme se cherche donc **parmi les formes contraintes** : taux au premier essai le plus haut, puis taux après relance, puis temps total le plus court.
- Le tableau donne par forme : taux au premier essai, taux après relance, plans conformes d'emblée, relances, erreurs d'appel, rappel du catalogue filtré, jetons d'invite, temps de génération.
- Dire les limites : un modèle, une quantification, 25 phrases ; un écart de moins de deux phrases (8 points) n'est pas un écart.
- Aucun accès réseau : un processus `llama-completion` par appel.

**Never:**
- Comparer d'autres modèles ou quantifications (8.4).
- Modifier `catalog.json`, `inventory.json`, `sentences.json` ou le harnais pour améliorer un taux : on mesure ce qui existe.
- Verser un fichier de modèle au dépôt.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Huit formes | modèle local présent | huit rapports et un tableau, la meilleure forme nommée | — |
| Échec d'appel | délai dépassé sur une forme | compté à part, signalé, la forme reste dans le tableau | le tableau l'indique |
| Égalité | deux formes contraintes au même taux | départage par taux après relance, puis par temps | le départage est écrit |
| Rejeu sans modèle | sorties enregistrées | huit lignes à 100 % | contrôle de `check_harness.py` |

</frozen-after-approval>

## Code Map

- `tests/epreuve-modele-local/run_forms.py` -- à créer : enchaîne les huit formes via `harness.measure`, écrit le tableau Markdown et le JSON de synthèse.
- `tests/epreuve-modele-local/resultats-8-5.md` -- à créer par le programme : le tableau et la meilleure forme, avec les limites.
- `tests/epreuve-modele-local/check_harness.py` -- ajouter un cas de rejeu de `run_forms`.
- `tests/epreuve-modele-local/harness.py` -- à réutiliser sans le modifier (`load_json`, `load_sentences`, `measure`, `summarize`).
- Chemins du poste passés en paramètres, jamais écrits en dur.

## Tasks & Acceptance

**Execution:**
- [x] `run_forms.py` -- huit formes, tableau, choix de la meilleure parmi les formes contraintes.
- [x] `check_harness.py` -- rejeu de `run_forms`, huit lignes à 100 %.
- [x] Lancer les huit mesures sur Qwen3.5-9B Q4_K_M et commiter `resultats-8-5.md`.

**Acceptance Criteria:**
- Given `resultats-8-5.md`, when on le lit, then il donne un taux par forme sur le même modèle et la même quantification, et nomme la meilleure forme contrainte avec son départage.
- Given `python check_harness.py`, when il s'exécute, then il sort 0 et le rejeu de `run_forms` donne huit lignes.

## Implementation Notes

Route oneshot épinglée : peu de code, aucun sous-agent de codage autorisé. La mesure elle-même se lance en arrière-plan et dure plusieurs minutes par forme.

Fait : `run_forms.py`, rejeu contrôlé par `check_harness.py`, huit mesures lancées en arrière-plan sur Qwen3.5-9B Q4_K_M, résultats dans `resultats-8-5.md` et `.json`. Résultat : de 64 % à 80 % de plans justes, meilleure forme contrainte = schéma, catalogue complet, avec exemples (20/25), indiscernable de trois autres formes (moins de deux phrases d'écart). Les exemples sont le seul levier net (+4 phrases en libre, +3 sous schéma) ; le schéma, le filtre et la relance ne changent pas le total, mais le schéma et les exemples font gagner ou perdre des phrases une à une. La lecture écrite à la main est dans `lecture-8-5.md`, le tableau généré dans `resultats-8-5.md`. Le critère « réseau coupé » est tenu par construction (un processus local par appel, aucun serveur) ; le câble n'a pas été physiquement débranché.

## Verification

**Commands:**
- `python -I tests/epreuve-modele-local/check_harness.py` -- expected: sortie 0
- `python -I tests/epreuve-modele-local/run_forms.py --llama <exe> --model <gguf> --quant Q4_K_M` -- expected: tableau écrit

## Review Triage Log

Relecture quick, 14 constats ; les chiffres de ma première lecture ont été recalculés par le relecteur et par moi.
- medium, patch : « +4 phrases, 16 points » ne valait que pour la sortie libre ; sous schéma c'est +3 (12 points), et avec filtre les exemples font perdre S18. Réécrit.
- medium, patch : « trois formes à égalité » en nommait deux ; quatre formes sont à moins de deux phrases de la meilleure (F2, F4, F6, F8). Corrigé et généré par le programme.
- medium, patch : « les erreurs ne sont pas de syntaxe » omettait 19 réponses non conformes au catalogue ; réécrit, avec les descriptions d'erreurs rattachées à la forme où elles ont été lues (F6 pour la plupart).
- medium, patch : le critère « temps de génération » du plan n'était pas dans le tableau ; colonne ajoutée.
- medium, patch : les sections écrites à la main étaient dans un fichier que le programme régénère et écrase ; la lecture vit dans `lecture-8-5.md`, et `--rebuild` refait le tableau depuis le JSON.
- low, patch : réduction de l'invite 21 à 23 % (écrit « environ 20 % ») ; temps total plus long en filtré à cause d'une génération plus longue, dit.
- low, patch : `--examples`, `--catalog` ou `--output` passés en ligne de commande étaient en conflit silencieux avec les formes ; refusés.
- low, patch : « 0 » affiché pour une invite non mesurée, départage sans le critère, erreur d'appel non signalée dans le tableau ; corrigés et couverts par `check_harness.py`.
- low, patch : le dossier de recherche cité n'était pas versé ; il l'est, dans un commit à part.
- low, patch : « RTX 3070 Ti » sans trace ; source indiquée (relevé de 8.1).
