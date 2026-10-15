---
epic: epic-epreuve-du-modele-local
date: 2026-10-15
verdict: accepted-with-open-items
criteria: declared
headless: false
---

# Rétrospective — Épreuve du modèle local (epic 8)

**Verdict : accepted-with-open-items.** Après le retro, les plans 8.3 à 8.6 ont été rangés dans le dossier de l'epic : `tickets.py status` voit les six tickets à `built`, sans ticket non terminé, et le verdict machine n'est plus forcé à *rejected*. Le propriétaire a écarté la mesure débranchée (« je sais que c'est bon ») : la réserve du critère 4 est acceptée par décision humaine. Le premier état du fichier (*rejected*, `pending_tickets` = 8.3 à 8.6) est décrit plus bas pour mémoire.

## Epic summary

Six tickets, tous livrés et revus (plan à `status: built`), mais `tickets.py status` n'en voit que deux comme terminés.

| Ref | Ticket | Plan | `status` vu par l'outil | Baseline du plan |
|---|---|---|---|---|
| 8.1 | Modèle quantifié hors ligne (spike) | `epic-…/spike-…-plan.md` | built | `10a094b` |
| 8.2 | Registre factice et phrases | `epic-…/story-…-plan.md` (partie 1) et `plan-8-2-phrases-et-validateur.md` (partie 2) | built | `f729b1b`, `2ebbb0f` |
| 8.3 | Harnais de mesure | `plan-8-3-harnais-de-mesure.md` | *(vide, planned)* — plan `built` | `e777918` |
| 8.4 | Comparer modèles | `plan-8-4-comparer-modeles.md` | *(vide, planned)* — plan `built` | `3031462` |
| 8.5 | Formes de sollicitation | `plan-8-5-formes-de-sollicitation.md` | *(vide, planned)* — plan `built` | `e411baf` |
| 8.6 | Verdict écrit | `plan-8-6-verdict.md` | *(vide, planned)* — plan `built` | `f2f60a91` |

**`pending_tickets` : 8.3, 8.4, 8.5, 8.6.** Cause constatée : les plans de ces quatre tickets (et la partie 2 de 8.2) ont été écrits à la racine de l'initiative (`_bmad-output/initiative-cassini-mvp/plan-8-*.md`), pas dans le dossier de l'epic où `tickets.py find` les cherche (`story-…-plan.md`), et `tickets.toml` ne porte pas leur statut. Le propriétaire a accepté de faire le retro malgré cela. Les plans eux-mêmes sont à `built` ; aucun ticket n'est à `built` seulement dans l'outil.

Plages de commits (une plage par plan, baselines ordonnées par l'historique ; la dernière plage va à `HEAD`, **inférée**) : 8.1 `5528df7` ; 8.2 `d73b391`…`4c77420` ; 8.3 `e777918`→`e411baf` ; 8.5 `e411baf`→`3031462` ; 8.4 `3031462`→`9a1ad34` ; 8.6 `f2f60a9`→`1dcf6d4`. Volume : 21 fichiers, 3 543 lignes ajoutées sous `tests/` depuis `2ebbb0f`. `git_evidence.py` n'a **pas** été lancé : les plages ont été relevées à la main d'après `git log` (périmètre réduit, noté).

**Inventaire.** Disponible : fichier de l'epic, plans et leurs Review Triage Log, résultats chiffrés, lectures, recette Python, `git log`. Absent : fichiers story (aucun ticket tiré), PRD et architecture de l'initiative au format `prd-*` (l'architecture existe sous un autre nom et n'a pas été relue ici), **journaux de session** (l'analyse des leçons de processus est donc limitée à ce que les plans consignent), rétrospective précédente.

## Findings

Provenance entre crochets. Disposition : *corriger* / *différer* / *accepter*.

**F1 — Critère 4 « aucun accès réseau » : pas de preuve d'exécution débranchée.** `resultats-8-4.md:3` et `resultats-8-5.md:3` écrivent « Aucun accès réseau » ; `harness.py:11` et `plan-8-3:29` le posent comme règle. Ce qui est établi : le code Python n'importe ni socket ni urllib (recherche dans `tests/epreuve-modele-local/*.py`), et `llama-completion` n'ouvre aucun port TCP ni UDP, relevé huit fois en 8.1 (`spike-…-plan.md`, Review Triage Log). Ce qui n'est **pas** établi : qu'une mesure de 8.3 à 8.6 ait tourné câble débranché. Le propriétaire a renoncé à l'épreuve débranchée en 8.1 (verdict du verify du 2026-10-14) en disant qu'elle se ferait en 8.4 et 8.5 ; aucun plan 8.4 ou 8.5 ne l'enregistre, et la mesure sur processeur de 8.6 a été lancée sans vérifier l'état du réseau. *Disposition : différer.* Une mesure courte câble débranché, consignée, ferme la réserve.

**F2 — Plans hors du dossier de l'epic.** [`tickets.py status` ; emplacement des `plan-8-*.md`] Quatre tickets terminés sont comptés non terminés ; le verdict machine de l'epic est faussé. *Corriger* : déplacer les plans dans le dossier de l'epic selon la convention de l'outil et marquer les tickets, par `bmad-ticket`.

**F3 — README de l'épreuve périmé.** [`tests/epreuve-modele-local/README.md:15`] Après la signature, il dit encore « recommandation, à signer par le propriétaire ». *Corriger* : une ligne.

**F4 — Défaut de garde-fou persistant dans `run_forms.py`.** [`run_forms.py:55-60` ; `run_models.py:46`, corrigé ; Review Triage Log de 8.4] `--catalog=x` n'est pas refusé dans `run_forms.py` (la comparaison est faite sur l'argument entier, sans le `split("=")` de `run_models.py`) : une forme pourrait être faussée en silence. Signalé en 8.4 comme « non touché » et jamais repris dans `deferred-work.md`. Le code étant jetable et les huit mesures de 8.5 ayant été lancées sans cet argument, l'effet sur les chiffres publiés est nul. *Accepter* ou *corriger* en une ligne.

**F5 — Les revues de chaque ticket ont corrigé des chiffres, pas seulement du code.** [Review Triage Log de 8.5 (cinq constats medium sur la lecture des chiffres) et de 8.6 (trois medium : S24/S15 « partout », 8 Go non mesuré, délai relevé non dit)] Les écarts entre prose et tableaux sont revenus à chaque entrée de mesure. Ils ont tous été rattrapés par la revue `quick`, mais c'est la revue qui porte la fiabilité, pas l'écriture. *Leçon de processus* (ci-dessous).

**F6 — Limites de la mesure, déjà écrites, à ne pas perdre.** [`resultats-8-4.md`, `resultats-8-5.md`, `verdict-…md` section Risques] 25 phrases, un modèle principal, une machine, une mesure par forme (température 0, graine fixe) ; catalogue de 16 commandes ; `step:k` jamais comparé à une autre référence ; S17 (phrase de démo) fausse dans la configuration retenue. Le verdict les énonce ; la mesure sur machine plus modeste manque pour promettre un temps. *Accepter* : c'est le contenu du « oui sous conditions ».

**F7 — Écart assumé avec AD-15.** [`epic-…md`, Notes] Harnais en Python appelant `llama-completion`, au lieu de charger le modèle dans le processus du noyau. Décision du propriétaire du 2026-10-14, limitée au code jetable ; le risque de compilation de `llama-cpp-2` sous Windows reste entier pour l'epic 5. *Accepter*, à rappeler à l'epic 5.

**Frontières entre tickets** (lues sur les plans et le code, pas par une passe `bmad-review` : voir Assumptions) : la normalisation d'unités ajoutée en 8.3 (`canonical`) est contredite par le README de 8.2, qui la dit non faite (« 500 m et 0,5 km donnent deux plans différents »). Preuve : `plan-8-3:84` (« normalisation d'unités dans `canonical` ») contre `README.md`, section « Limite connue ». *Corriger* : mettre le README d'accord avec le code, ou l'inverse.

## Behavior verification

Exercé : `check_catalog.py` (16 commandes, 7 couches, 9 mutations refusées), `check_sentences.py` (25 phrases, 19 cas négatifs refusés), `check_harness.py` (25 phrases rejouées à 100 %, plan faux compté faux, 4 types de sortie non conforme redemandés, refus WGS84) : tous verts, 2026-10-15. Le modèle lui-même a été exécuté au cours de cette session pour 8.6 (mesure sur processeur : 20/25, 26,5 s par appel). Les mesures 8.4 et 8.5 n'ont **pas** été refaites dans ce retro ; leurs chiffres sont lus dans `resultats-8-4.md` et `resultats-8-5.md`.

## Previous-retro follow-through

Aucune rétrospective précédente trouvée (aucun fichier `*-retrospective.md` dans l'arbre) : rien à suivre.

## Action items

**Suite donnée le 2026-10-15 (propriétaire : « fix les problèmes trouvés »)** : A1 écarté par le propriétaire ; A2, A3 et A4 appliqués (plans 8.3 à 8.6 déplacés dans le dossier de l'epic, `deferred-work.md` mis à jour ; README corrigé ; `run_forms.py` aligné sur `run_models.py`) ; la partie 2 de 8.2 reste à la racine (`plan-8-2-phrases-et-validateur.md`) car l'outil refuse deux plans pour un même ticket ; A5 et A6 restent ouverts.

| # | Action | Propriétaire |
|---|---|---|
| A1 | Faire tourner une mesure courte (par exemple 5 phrases) câble débranché, consigner l'état du réseau avant et après, pour fermer F1 | Propriétaire |
| A2 | Ranger les plans 8.3 à 8.6 et la partie 2 de 8.2 dans le dossier de l'epic et marquer les tickets (via `bmad-ticket`), pour que l'outil voie l'epic comme fini ; modification de l'arbre, *proposée, non appliquée* | Propriétaire, avec `bmad-ticket` |
| A3 | Corriger la ligne 15 du README de l'épreuve (F3) et la « Limite connue » sur les unités (frontière 8.2/8.3) | Dev |
| A4 | Aligner `run_forms.py` sur `run_models.py` pour `--catalog=x` (F4), ou l'écrire dans `deferred-work.md` | Dev |
| A5 | Rappeler à l'epic 5 l'écart AD-15 (F7) et la mesure sur poste plus modeste (F6) | Propriétaire |
| A6 | Leçon de processus : pour toute entrée qui produit des chiffres, faire recalculer les chiffres de la prose par un script contre le JSON avant la revue (F5) ; les plans à écrire dans le dossier de l'epic dès la création (F2) | Propriétaire |

Aucun de ces points n'est un changement de contrat ni de spec.

## Acceptance verdict

**Final : accepted-with-open-items** (A5, A6 ouverts ; critère 4 accepté par le propriétaire, sans mesure débranchée ; tickets à `built`, non clos dans l'outil).

**Machine au premier passage : rejected.** Preuve : `pending_tickets` = 8.3, 8.4, 8.5, 8.6 (`tickets.py status`, 2026-10-15), règle dure du workflow, y compris hors mode headless. Critères **déclarés** (le fichier de l'epic a un « Done when »).

Lecture du « Done when » sur les preuves, pour la décision humaine :

1. **Tenu.** 25 phrases (≥ 20) et trois combinaisons (≥ 2) mesurées : `resultats-8-4.md`.
2. **Tenu.** Verdict écrit, signé le 2026-10-15 : oui sous conditions, Qwen3.5-9B Q4_K_M, 5,7 Go de RAM mesurés, Ryzen 7 5800X ; échecs et non-essais documentés : `verdict-epreuve-du-modele-local.md`. Réserve : la configuration « minimale » est la configuration mesurée, pas un plancher (écrit dans le verdict).
3. **Tenu.** La forme et le format du plan sont écrits, avec huit règles pour l'epic 2 (`verdict-…md`, « Ce que l'epic 2 en retient »).
4. **Réserve (F1).** Aucune trace d'exécution débranchée ; code et ports vérifiés seulement.
5. **Tenu.** `tests/` est hors de `members = ["core", "app/src-tauri"]` du `Cargo.toml` ; rien d'importé par le produit.

**Recommandation : accepted-with-open-items**, les items ouverts étant A1 et A2. C'est une décision du propriétaire ; ce fichier reste à *rejected* tant qu'elle n'est pas prise.

## Open questions

- Une mesure débranchée sera-t-elle faite (A1), ou le propriétaire maintient-il sa renonciation de 8.1 pour toute l'épreuve ? Cela décide si le critère 4 est tenu ou accepté tel quel.
- Les plans de 8.3 à 8.6 doivent-ils rejoindre le dossier de l'epic, ou la convention de l'outil est-elle à changer ? (A2)

## Assumptions

Run interactif : les confirmations (retro malgré `pending_tickets`, pas de priorité particulière) sont données par le propriétaire. Deux périmètres réduits : (a) `git_evidence.py` non lancé, plages relevées à la main ; (b) `bmad-review` non invoqué sur la diff de l'epic, la passe de frontières a été faite à la lecture, sur la base de revues `quick` déjà passées pour chaque ticket. Le critère 4 n'a pas été éprouvé câble débranché dans ce retro.
