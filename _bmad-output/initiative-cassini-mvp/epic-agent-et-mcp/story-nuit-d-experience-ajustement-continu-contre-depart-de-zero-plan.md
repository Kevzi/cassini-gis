---
title: 'Nuit d''experience : ajustement continu contre depart de zero'
type: 'chore'
ticket: '55'
created: '2026-11-16'
status: 'done'
baseline_revision: '31d13200b27f6a6b0e52e98feab94c7b389e719b'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/docs/entrainement-du-modele.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** On ne sait pas si les nuits d'entrainement suivantes peuvent se faire en continu (depuis l'adaptateur installe, ticket 54) sans perdre en qualite face a un depart de la base (jeu complet de 8 600 exemples).

**Approach:** Entrainer deux modeles sur le meme jeu, l'un depuis l'adaptateur installe, l'autre depuis la base ; les passer par les memes barres (par commande, non-oubli du 53, recette d'Annecy) ; produire un tableau comparatif avec les durees ; le proprietaire decide.

## Boundaries & Constraints

**Always:** meme jeu d'exemples et meme juge pour les deux bras ; l'experience decide, pas l'opinion (decision du 2026-11-11) ; l'installation suit les regles habituelles (juge « installable » et Annecy >= 16/20, sur decision du proprietaire).

**Never:** rien n'est installe ni pousse sans decision du proprietaire ; aucun changement de barre ou de juge pendant l'experience.

**Decisions (2026-11-16, proprietaire) :** (1) on lance la vraie nuit, pas de cloture sur les preuves existantes ; (2) jeu complet : le bras « base » entraine les 8 600 exemples par defaut (`DEFAULT_COUNT`, graine torch 1), le bras « continu » repart de l'adaptateur installe avec une seance sur les familles donnees, au meme commit du generateur ; (3) le script de nuit est ecrit dans le depot, `training/entrainement/nuit-5-55.sh`, et chaque bras y est mesure par le meme juge et les memes relevés de reference.

</frozen-after-approval>

## Code Map

- `docs/entrainement-du-modele.md` -- procedure d'ajustement continu (~l.2840), mesures 5.59 (~l.3258), nuit complete (~l.1281).
- `training/entrainement/pipeline.py` -- `--depuis`, `--familles`, `--part-rappel`, `--graine-torch`, `DEFAULT_COUNT`.
- `training/entrainement/non_oubli.py` -- juge « installable / refuse », reference hors depot `~/cassini-entrainement/reference-non-oubli/`.
- `~/cassini-entrainement/nuit-5-54.sh`, `nuit-5-59.sh` (WSL, hors depot) -- enchainement GPU, essais, Annecy, juge ; ne rien installer.
- `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/tickets.toml` -- entree 55 (note du 59 : lien 5.55 « abandonne », a verifier).

## Tasks & Acceptance

**Execution:**
- [ ] `training/entrainement/nuit-5-55.sh` -- script `base | continu <familles> | deux <familles>` derive de `nuit-5-54.sh` (gardes, entrainement, copie sans installation, essais GPU, Annecy et gros projet, juge), avec la duree de chaque entrainement ecrite dans l'etat ; verification `bash -n` et refus propre sans GPU libre
- [ ] lancer la nuit (WSL, GPU libre, CASSINI-nuit au meme commit que le worktree), relever les durees
- [ ] `docs/entrainement-du-modele.md` -- ajouter le tableau barre par barre et par commande, avec durees -- verifie le ticket
- [ ] ce plan -- consigner la decision du proprietaire

**Acceptance Criteria:**
- Given les deux modeles entraines sur le meme jeu, when le juge et la recette d'Annecy tournent, then un tableau compare les deux barre par barre et par commande, avec la duree de chacun.
- Given le tableau, when le proprietaire tranche, then sa decision est ecrite dans ce plan et aucune installation n'a lieu sans elle.

## Implementation Notes

- Script `training/entrainement/nuit-5-55.sh` (commit `262fd8a`), `.gitattributes` : `*.sh` en LF.
- Nuit du 2026-11-16 : bras « base » mesure (`base-5-55`, 2 h 17 d'entrainement, refuse). Bras « continu » non refait : `ajuste-continu-5-59-1` (memes entrees, meme code d'entrainement, refuse) en tient lieu, sur decision du proprietaire pour economiser ~3 h de GPU. Ecart au plan « deux modeles sur le meme jeu », accepte.
- Tableau : `docs/entrainement-du-modele.md`, section « Nuit d'experience 5.55 ».
- **Decision du proprietaire (2026-11-16) :** on suit la recommandation. Les nuits suivantes se font en continu par defaut (3,6 fois moins d'entrainement, au plus proche de l'installe) ; le depart de zero reste reserve au recalage (au-dela de 3 maillons). Aucun des deux modeles n'est installe ; l'installe reste `ajuste-continu-5-58-1`. Defaut commun aux deux : `ecart_reel` baisse.

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 0 high, 0 medium, 1 low, 3 hors defaut.

| Verdict | Route | Constat et evidence |
|---|---|---|
| low | rejete | Le resume du rapport affiche « exemples ? » : les cles `exemples`/`count` ne sont pas dans `donnees` (cle non verifiee). Cosmetique : le nombre d'exemples reste dans `rapport.json` et le journal d'entrainement ; corriger demande de verifier le schema du rapport. |
| false | rejete | « Le script ne livre pas le tableau » : le plan prevoit un tableau compose a la main dans le doc apres la nuit, pas un releve du script. |
| false | rejete | « Taches non couvertes par le diff » : nuit, tableau, decision du proprietaire et note du ticket 59 sont des taches encore ouvertes du plan, pas des defauts. |
| low | rejete | En mode `deux`, un echec du bras base n'arrete pas le bras continu : voulu, chaque bras a son etat et le code de sortie est non nul. |

## Verification

**Manual checks (if no CLI):**
- Le tableau du doc contient les deux modeles, toutes les barres et les durees ; `nuit-*.etat` montre le verdict des deux.
