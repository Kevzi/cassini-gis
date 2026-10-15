---
title: 'Verdict écrit et recommandation'
type: 'chore'
ticket: '6'
created: '2026-10-15'
status: 'ready-for-dev'
baseline_revision: ''
route: 'oneshot'
route_source: 'auto'
risk: 'medium'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** l'epic 8 a mesuré trois combinaisons et huit formes, mais rien n'est tranché : le propriétaire ne peut pas encore décider seul si les epics 2 (registre) et 5 (agent) partent, et l'epic 2 ne sait pas encore comment rédiger ses descriptions de commandes ni quel format de plan l'agent rendra.

**Approach:** écrire un verdict en français, oui, non ou oui sous conditions, à partir des chiffres de 8.4 et 8.5 seuls, dans `epic-epreuve-du-modele-local/verdict-epreuve-du-modele-local.md`. Il nomme le modèle avec sa licence, la quantification, la configuration minimale, la forme de sollicitation et le format du plan rendu ; il donne aussi ce qui a été essayé, ce qui ne l'a pas été, et ce que le verdict change pour les epics 2 et 5. L'entrée est à décision humaine : le texte présente une recommandation, et le propriétaire la signe ou la change.

**Décisions du propriétaire (2026-10-15) :** pas de seuil chiffré du « oui » : le verdict lit la nature des erreurs, en tenant compte de ce que l'utilisateur valide chaque plan avant exécution (CLAUDE.md §6). Une mesure sur processeur seul (`--ngl 0`, Qwen3.5-9B Q4_K_M, forme retenue) est faite avant d'écrire le verdict, pour établir le plancher processeur.

## Boundaries & Constraints

**Always:**
- Chaque affirmation du verdict renvoie à un chiffre de `resultats-8-4.md`, `resultats-8-5.md` ou aux lectures ; ce qui n'a pas été mesuré est écrit comme tel.
- Le format du plan rendu est celui du harnais : suite d'objets `{"command", "params"}`, ou `{"refusal"}`, validée contre les schémas avant tout affichage, jamais réparée (AD-23).
- Un écart de moins de deux phrases sur 25 n'est pas un écart.
- La mesure sur processeur seul est faite avec le harnais tel quel, sans réseau, et rapportée avec ses temps de chargement, de lecture et de génération séparés ; si elle dépasse le délai d'un appel, le dire et relever le délai.

**Never:**
- Écrire le verdict comme acquis : il reste une recommandation tant que le propriétaire ne l'a pas signée.
- Changer le comparateur, le catalogue ou les phrases pour améliorer un score.
- Modifier le plan du projet, `CLAUDE.md` ou les epics : si le verdict est non, ce sera une décision séparée.

</frozen-after-approval>

## Tasks & Acceptance

**Execution:**
- [ ] `tests/epreuve-modele-local/resultats-8-6-cpu.json` -- créer : la mesure `--ngl 0`, Qwen3.5-9B Q4_K_M, schéma, catalogue complet, avec exemples, 25 phrases, lancée en arrière-plan ; `harness.py` est utilisé sans modification, `--timeout` relevé si besoin.
- [ ] `_bmad-output/initiative-cassini-mvp/epic-epreuve-du-modele-local/verdict-epreuve-du-modele-local.md` -- créer : le verdict, sa justification chiffrée, la configuration minimale, la forme de sollicitation, le format du plan, ce que l'epic 2 en retient pour rédiger ses descriptions, les risques et ce qui n'a pas été essayé.
- [ ] `tests/epreuve-modele-local/README.md` -- mettre l'état à jour : toutes les entrées terminées, lien vers le verdict.

**Acceptance Criteria:**
- Given le verdict, when le propriétaire le lit, then il y trouve oui, non ou oui sous conditions, le modèle et sa licence, la quantification, la configuration minimale, la forme et le format du plan, sans ouvrir un autre fichier.
- Given le verdict, when l'epic 2 le lit, then il sait quoi mettre dans une description de commande et dans un schéma de paramètres, d'après les exemples et les erreurs observés.

## Implementation Notes

Route oneshot : un document, une ligne de README et une mesure lancée avec le harnais existant, sans code nouveau.

## Verification

**Manual checks:**
- Relire chaque chiffre du verdict contre `resultats-8-4.md` et `resultats-8-5.md`.
