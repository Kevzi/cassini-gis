---
title: 'Refactor sweep de l epic publication'
type: 'chore'
ticket: '13'
created: '2026-11-23'
status: 'in-review'
baseline_revision: 'f1117af5c0994efa9e466517c8005c24137028eb'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Quatre constats reportés pendant l'epic 7 sont encore dans le code : la release peut partir d'un commit dont la recette n'est pas verte, le téléchargement du modèle (2,8 Go) n'a aucune nouvelle tentative ni délai global et accepte une adresse non https, et la preuve d'altération de l'installeur en ligne ne touche que la dernière entrée du manifeste.

**Approach:** Corriger ces quatre points petits et testables. Les constats moyens ou de faible valeur (message « à jour » avant les pages de l'installeur, champ de schéma du manifeste, lecture du journal `/LOG=`, détection de binaire) sont consignés comme reportés dans `deferred-work.md`, sans correction.

## Boundaries & Constraints

**Always:** les tests existants de l'installeur restent verts ; le téléchargement continue de vérifier taille et SHA-256 ; `file://` reste accepté dans les tests seulement.

**Never:** changer le format du manifeste ou le comportement de l'installeur Windows ; pousser une étiquette de version sans l'accord du propriétaire ; ajouter une dépendance.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Coupure passagère | le serveur coupe puis répond | le téléchargement reprend après une attente croissante, trois essais au plus | échec nommé après le dernier essai |
| Adresse http | `http://…` hors essai | refusée avant toute connexion | `ModeleErreur` explicite |
| Délai global dépassé | téléchargement trop long | arrêt et fichier partiel supprimé | `ModeleErreur` explicite |
| Commit sans recette verte | étiquette sur un commit dont la recette n'a pas réussi | la release s'arrête avant de construire | message qui nomme le commit |
| Altération d'une entrée | chaque entrée du manifeste, une à la fois | l'installeur refuse à chaque fois | échec de la recette si l'une passe |

</frozen-after-approval>

## Code Map

- `installer/telecharger_modele.py` (l.51-63) -- `urlopen(..., timeout=60)` seul ; ajouter essais avec attente, délai global et contrôle du schéma https.
- `installer/tests/test_hors_ligne.py` -- tests du téléchargeur ; y ajouter coupure, http refusé et délai.
- `.github/workflows/release.yml` -- job `linux` (l.31) : première étape à ajouter, qui vérifie la recette verte du commit ; étape d'altération (l.236-250) à étendre à toutes les entrées ; ne lancer la vérification que sur étiquette.
- `.github/workflows/recette.yml` -- n'est pas modifié ; seulement lu par la vérification.
- `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- consigner les constats reportés et clore les trois déjà traités (octet 0x08 du README, SHA des installeurs sur la page, déclencheur à blanc).

## Tasks & Acceptance

**Execution:**
- [ ] `installer/telecharger_modele.py` -- essais (3) avec attente croissante, délai global, refus d'une adresse non https sauf `file://` -- constats 2 et 3.
- [ ] `installer/tests/test_hors_ligne.py` -- tests de la matrice ci-dessus -- preuve hors ligne.
- [ ] `.github/workflows/release.yml` -- étape « recette verte du commit » (`gh run list --workflow recette.yml --commit`) avant toute construction, exemptée en essai à blanc ; altération de chaque entrée du manifeste -- constats 1 et 4.
- [ ] `deferred-work.md` -- consigner 5, 6, 7 et la détection de binaire comme reportés, clore 8, 9, 10.

**Acceptance Criteria:**
- Given une coupure passagère, when le modèle se télécharge, then il aboutit sans intervention et son SHA-256 est vérifié.
- Given une étiquette sur un commit sans recette verte, when la release démarre, then elle échoue avant de construire.
- Given un manifeste dont une entrée est altérée, when l'installeur est prouvé, then il refuse pour chacune des entrées.

## Verification

**Commands:**
- `uv run --no-project python -I -m unittest discover -s installer/tests` -- expected: tout vert
- `gh workflow run release.yml --ref main` -- expected: essai à blanc vert (rien n'est publié)

**Manual checks (if no CLI):**
- La vérification complète du ticket (« recette verte sur une nouvelle étiquette ») exige une étiquette de version : à décider avec le propriétaire, non lancée seule.
