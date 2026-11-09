---
title: 'Tenir la mémoire de la carte de 8 Go pendant tout l''entraînement'
type: 'chore'
ticket: ''
created: '2026-11-09'
status: 'done'
baseline_revision: '7b382e3'
route: 'full'
risk: 'medium'
context:
  - '{project-root}/docs/entrainement-du-modele.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/plan-acceleration-entrainement.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

Priorité du propriétaire (2026-11-09). L'entraînement complet `~/cassini-entrainement/regroupe-11-23c`
(chaîne intégrée, commit 7b382e3) a tourné à environ 10,5 s par pas jusqu'au pas 180 environ,
puis à environ 25 s par pas jusqu'à la fin ; le pic de torch est passé de 6,37 à 7,49 Gio et la
carte affichait 7 999 / 8 192 Mio à 100 % : WSL a très probablement débordé en mémoire
centrale. Les essais de 20 pas ne pouvaient pas le voir. Dans le worktree
`/home/kevin/cassini-worktrees/11-23`, sur une NOUVELLE branche `story/memoire-entrainement`
partie de 7b382e3, rien d'autre sur le GPU :

1. Construire une mesure courte qui reproduit le problème : un `--pas-max` (ou une petite
   option) qui passe d'abord les exemples les PLUS LONGS du jeu par défaut (en jetons), pour
   que le pic se voie en 20 à 30 pas ; confirmer le ralentissement sur la chaîne actuelle.
2. Essayer, dans cet ordre, en ne gardant que ce qui aide : `PYTORCH_CUDA_ALLOC_CONF=
   expandable_segments:True` posé par la chaîne avant le chargement de torch ; libérer la
   mémoire en cache entre les pas quand elle approche de la limite (`torch.cuda.empty_cache()`
   après un exemple long, coût mesuré) ; et si besoin encore une borne dure sur la longueur
   des séquences que le générateur respecte déjà (vérifier jetons_max = 1656 dans
   rapport.json) ou les options de mémoire d'Unsloth. Mesurer s/pas sur l'échantillon « longs
   d'abord » ET sur un échantillon normal de 20 pas, pic mémoire, perte comparée aux essais
   précédents.
3. Ensuite seulement, les deux autres pistes approuvées : (a) lot 2 avec accumulation 8 (même
   lot effectif de 16), gardé seulement s'il reste sous 8 Go sur l'échantillon « longs
   d'abord » et s'il est plus rapide ; (b) couper le coût fixe d'environ 10 min autour de
   l'entraînement (garder la base GGUF convertie, llama.cpp construit, le cache de compilation
   d'Unsloth d'un lancement à l'autre quand les entrées n'ont pas changé).

Garder QLoRA nf4, les hyperparamètres, DEFAULT_COUNT et DEFAULT_SEED ; les tests de la chaîne
passent ; doc à jour (Accélération) ; mesures dans les Implementation Notes. Ne pas commiter, ne
pas lancer d'entraînement complet de 500 pas.

</frozen-after-approval>

## Implementation Notes

Mesures du 2026-11-09, branche `story/memoire-entrainement` partie de `7b382e3`, même jeu
(graine 1), venv principal. Journaux : `~/cassini-entrainement/essai-vitesse-*.log`.

| Échantillon | Configuration | s / pas | Perte 10 | Perte 20 | Pic | Gardée |
|---|---|---|---|---|---|---|
| longs d'abord, `--pas-max 30` | chaîne `7b382e3` | 57 (pas 1-10), 41 (11-20) | 0,0631 | 0,0365 | 7,58 Gio, carte 8 006 Mio | référence |
| longs d'abord, `--pas-max 30` | `expandable_segments:True` | 14,3 (11-20), 13,9 (21-30) | 0,0665 (+5,4 %) | 0,0351 (−3,8 %) | 5,85 Gio | **oui** |
| longs d'abord, `--pas-max 20` | `expandable_segments:True` | 14,3 | 0,0641 | 0,0411 | 5,56 Gio | — |
| longs d'abord, `--pas-max 20` | idem + lot 2, accumulation 8 | 28 (pas 1-10) | 0,0653 | arrêté | 5,89 Gio | non (reporté) |
| normal, `--pas-max 20` | chaîne `7b382e3` | 10,7 | 0,4818 | 0,2765 | 6,37 Gio | — |
| normal, `--pas-max 20` | `expandable_segments:True`, code final | 10,7 | 0,4782 (+1,2 %) | 0,2747 (−0,3 %) | 5,67 Gio | **oui** |

Écarts de perte donnés contre la même configuration sans segments extensibles (longs d'abord)
ou contre la référence PyTorch 0,4723 / 0,2754 (normal).

- Cause : le pas du ralentissement de `regroupe-11-23c` (171-180) n'a pas l'exemple le plus long
  (1 448 jetons ; un de 1 560 était passé au pas 100 sans hausse) : c'est la fragmentation du
  cache de torch, qui finit par dépasser la carte. Unsloth retire `expandable_segments` sous WSL
  (`unsloth_zoo/__init__.py`, problème 3511 d'Unsloth) ; posé après son import par
  `torch.cuda.memory._set_allocator_settings`, il tourne sans erreur sur ce poste.
- Retenu : `expandable_segments:True` par défaut (`CASSINI_SEGMENTS_EXTENSIBLES=0` pour
  l'enlever), `--longs-d-abord` pour la mesure. `rapport.json` : `entrainement.allocateur`,
  `hyperparametres.longs_d_abord`.
- Non essayés, inutiles après la première piste (décision du propriétaire) : `empty_cache`
  entre les pas, borne de longueur, options de mémoire d'Unsloth.
- Reportés par le propriétaire (2026-11-09) : lot 2 / accumulation 8 et coût fixe ; entrée
  ajoutée à `deferred-work.md`, ébauche de code hors dépôt
  (`~/cassini-entrainement/pipeline-differe-memoire.py`).
- Pas de nouvel entraînement complet : le ralentissement au-delà de 180 pas n'est donc corrigé
  que sur la mesure courte.

## Plan Change Log

- 2026-11-09, décision du propriétaire : garder `expandable_segments`, ne pas essayer `empty_cache` ni la borne de longueur, reporter le lot 2 et le coût fixe (`deferred-work.md`), et commiter sur `story/memoire-entrainement` (le texte figé disait de ne pas commiter).
