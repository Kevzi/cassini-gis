---
title: 'Accélérer l''entraînement du modèle sur la carte de 8 Go'
type: 'chore'
ticket: ''
created: '2026-11-09'
status: 'built'
baseline_revision: '4e10efe1392eecaf4fa098b35fe54cbb2049962a'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** un entraînement complet prend environ 6 h sur la RTX 3070 Ti de 8 Go du propriétaire (≈ 25 s par pas, 500 pas, pic 6,55 Gio, puis ≈ 50 min de validation par PyTorch) ; le journal signale que `causal_conv1d` et `flash-linear-attention` manquent et que les couches à attention linéaire de Qwen3.5 tournent par leur implémentation de secours, « beaucoup plus lente ».

**Approach:** essayer quatre pistes dans l'ordre voulu par le propriétaire (2026-11-09) — 1 les deux accélérateurs, 4 Unsloth, 2 le regroupement des exemples courts (packing), 3 la validation par llama.cpp sur le GGUF — en mesurant chacune sur un entraînement court, et garder seulement celles qui accélèrent sans changer la qualité ; puis relancer le second essai de 11.23 avec la configuration retenue.

## Boundaries & Constraints

**Always:** mesure de référence : 10 pas ≈ 4,1 min, perte 0,7766 au pas 10 et 0,3345 au pas 20, pic 6,55 Gio (journal `~/cassini-entrainement/regroupe-11-23b.log`) ; chaque piste mesurée par `--pas-max 20` (ou 30) sur le même jeu (graine 1), en secondes par pas, pic mémoire et perte aux pas 10 et 20 ; une piste n'est gardée que si la perte reste à ±10 % de la référence aux mêmes pas et que la mémoire tient dans 8 Go ; versions épinglées dans `training/entrainement/pyproject.toml` et `uv.lock`, licence de chaque ajout dans l'en-tête (`causal-conv1d` BSD-3, `flash-linear-attention` MIT, `unsloth` Apache-2.0) ; même base, même méthode QLoRA, mêmes hyperparamètres, `DEFAULT_COUNT` et `DEFAULT_SEED` inchangés ; le packing ne mélange jamais l'attention de deux exemples ; la piste 3 garde le score de validation comparable : sur le modèle déjà entraîné `~/cassini-entrainement/regroupe-11-23/` (297/300 par PyTorch), la validation par llama.cpp doit donner le même score à ±2 près, sinon elle n'est pas gardée ; rien dans le noyau, l'extension ou `ai/`.

**Never:** installer quoi que ce soit avec `sudo` ou toucher au système (pas de toolkit CUDA installé au niveau du système) : si une piste demande une compilation impossible sans cela, la noter et passer à la suivante ; s'arrêter après deux essais ratés par piste ; lancer l'entraînement complet (c'est l'étape suivante, faite par Claude sur décision) ; changer de branche dans le dépôt principal `/mnt/c/Users/kevin/Downloads/Projet code/CASSINI` (un autre agent y travaille) — tout se fait dans le worktree `~/cassini-worktrees/11-23`, branche `story/acceleration-entrainement`.

</frozen-after-approval>

## Code Map

- `training/entrainement/pipeline.py` -- `train` l. 436 (QLoRA, `use_gradient_checkpointing=True`, lot 1, accumulation 16), `validate` l. 515 (`model.generate`, un exemple à la fois), options l. 869-885 (`--pas-max`, `--sortie`, `--exemples`…), étapes GGUF et `llama-completion` (essai final, binaire `~/.cache/cassini-entrainement/llama.cpp/build/bin/`).
- `training/entrainement/pyproject.toml`, `uv.lock` -- dépendances épinglées (torch 2.14.1+cu130, transformers 5.20.2, peft 0.21.2, bitsandbytes 0.50.2) ; pas de `nvcc` dans WSL.
- `training/entrainement/scoring.py`, `data.py` -- notation de la validation, à réutiliser pour la piste 3.
- `training/entrainement/tests/` -- tests Python de la chaîne.
- `docs/entrainement-du-modele.md` -- commande et durées.

## Tasks & Acceptance

**Execution:**
- [x] Piste 1 : `flash-linear-attention` (Triton, sans compilation) et `causal-conv1d` (roue précompilée compatible, sinon noter) ; mesure.
- [x] Piste 4 : Unsloth, s'il prend en charge Qwen3.5-4B avec ces versions ; mesure ; sinon noter pourquoi.
- [x] Piste 2 : packing des exemples sans contamination entre exemples ; mesure.
- [x] Piste 3 : validation par `llama-completion` (ou `llama-server` local sans réseau, ou la crate du noyau) sur le GGUF fusionné, notée par `scoring.py` ; comparaison avec 297/300 et durée.
- [x] Tableau des mesures dans Implementation Notes ; configuration retenue ; doc à jour ; tests de la chaîne verts.

**Acceptance Criteria:**
- Given la configuration retenue, when on lance `--pas-max 20`, then la durée par pas baisse nettement par rapport à 25 s, la perte reste dans ±10 % et le pic mémoire sous 8 Go.
- Given le modèle `regroupe-11-23`, when on le valide par la piste 3, then le score est à ±2 de 297/300 et la durée nettement sous 50 min.

## Implementation Notes

Mesures du 2026-11-09, worktree au commit `4e10efe`, `--pas-max 20`, même jeu (graine 1,
empreinte `1bc2120d…`, identique à `regroupe-11-23b`). Journaux :
`~/cassini-entrainement/essai-vitesse-*.log`.

**Référence refaite.** La référence du plan (0,7766 / 0,7424 au pas 10) vient d'entraînements
de 500 pas ; avec `--pas-max 20`, le planning cosinus tient en 20 pas sans échauffement, et la
perte au pas 10 tombe à 0,47 quelle que soit l'implémentation. La référence comparable a donc
été mesurée dans les mêmes conditions, avec `--noyaux-reference` (implémentations PyTorch).

| Configuration | s / pas (pas 11-20) | Perte 10 | Perte 20 | Pic | Écart perte | Gardée |
|---|---|---|---|---|---|---|
| référence PyTorch | 25,5 | 0,4723 | 0,2754 | 6,29 Gio | — | — |
| 1a fla (règle delta), conv PyTorch | 23,1 | 0,4726 | 0,2767 | 6,50 Gio | +0,1 % / +0,5 % | non |
| 1b fla règle delta + conv fla (Triton) | 17,8 | 0,4740 | 0,2793 | 6,24 Gio | +0,4 % / +1,4 % | **oui** |
| 4 Unsloth, env. à part, transformers 5.17.0 (essai 1, 20 pas) | 10,7 | 0,4778 | 0,2747 | 6,21 Gio | +1,2 % / −0,3 % | essai seulement |
| 4 Unsloth, essai 2 (30 pas, régime) | 10,7 (pas 11-30) | — | — | 6,22 Gio | planning de 30 pas, non comparable | — |
| 2 1b + packing 2 048 | > 33 (pas 10 non atteint en 10 min) | — | — | 8 Go saturés | — | non |
| 2 1b + packing 1 600 | 17,7 | 0,4735 | 0,2760 | 6,92 Gio | +0,3 % / +0,2 % | non (aucun gain) |
| chaîne retenue de bout en bout (venv principal) | 17,8 | 0,4741 | 0,2777 | 6,50 Gio | +0,4 % / +0,8 % | — |

Piste 3, sur `regroupe-11-23` (modèle fusionné Q4_K_M) : 296/300 par `llama-server` CUDA
contre 297/300 par PyTorch (désaccords aux rangs 4, 155, 300 ; 277 sorties identiques), 322 s
contre 3 118 s. Gardée.

**Configuration retenue** : `flash-linear-attention==0.6.0` (règle delta et convolution
causale en Triton) + validation par `llama-server` CUDA sur le GGUF. Options de repli :
`--noyaux-reference`, `--validation-pytorch`.

Notes :
- `causal-conv1d` : aucune roue pour torch 2.14 ; la roue torch 2.10/cu13 échoue au chargement
  (`undefined symbol _ZN3c104cuda29c10_cuda_check_implementation…`). Une compilation aurait
  été possible avec le nvcc des roues (voir piste 3), mais la convolution Triton de fla rend
  ce paquet inutile ; non ajouté.
- Unsloth 2026.11.3 : `transformers<=5.17.0` contre 5.20.2 épinglé (résolution uv refusée) ;
  `unsloth_zoo` est en LGPL-3.0-or-later (le plan supposait Apache-2.0 seul).
- **Décisions du propriétaire (2026-11-09)** : (1) les roues nvcc (licence NVIDIA) restent,
  outillage d'entraînement seulement ; (2) Unsloth essayé quand même, transformers 5.17.x,
  `unsloth_zoo` LGPL-3.0 accepté pour cet essai seulement (outillage d'entraînement, jamais
  dans le noyau).
- Essai Unsloth, hors dépôt : `~/cassini-entrainement/unsloth-essai/` (`pyproject.toml`,
  `uv.lock`, `essai_unsloth.py`), venv `~/.cache/cassini-entrainement/venv-unsloth`. Résolu :
  transformers 5.17.0, huggingface-hub 1.33.0 (5.17 exige < 2.0), le reste aux versions de la
  chaîne (torch 2.14.1, peft 0.21.2, bitsandbytes 0.50.2, accelerate 1.15.0), plus trl 1.15.0,
  xformers 0.0.35 (inutilisable : construit pour torch 2.10, Unsloth prend l'attention PyTorch),
  datasets, diffusers, torchao… transformers 5.17 charge Qwen3.5 (« Fast Qwen3_5 patching »).
  Méthode inchangée : nf4, double quantification, calcul bf16, 26 345 472 paramètres, adaptateur
  aux mêmes 352 tenseurs que la chaîne retenue. `flash-linear-attention` toujours présent
  (Unsloth le range parmi ses `FLA_MODEL_TYPE_PREFIXES` pour qwen3_5 ; `install_fast_kernels`
  a posé la convolution Triton) ; Unsloth compile en plus chaque couche (`torch.compile`) et
  prend son checkpointing « unsloth » (déchargement des activations en mémoire centrale).
  Journaux : `~/cassini-entrainement/essai-vitesse-unsloth.log`, `…-unsloth-30.log`.

- Packing : implémenté (positions remises à 0, `cu_seq_lens_q` vers fla), contrôlé sans
  contamination sur GPU, puis retiré faute de gain.
- nvcc : roues `nvidia-cuda-nvcc/nvvm/crt==13.0.88`, `nvidia-cuda-cccl==13.0.85` dans
  `pyproject.toml` (licence NVIDIA) ; `cuda_toolkit()` pose `lib64 -> lib` et les liens
  `libcudart.so`, `libcublas.so`, `libcublasLt.so` dans `nvidia/cu13` du venv.

- Chaîne retenue de bout en bout (`essai-vitesse-retenue`, `--pas-max 20` puis `--reprendre`) :
  entraînement, conversion, `llama-server` CUDA reconstruit de zéro pour le venv principal
  (≈ 6 min), validation GGUF en 309 s (119/300, attendu après 20 pas), essai par les deux
  chemins, `rapport.json`. Deux défauts trouvés et corrigés en route : CMake effaçait sa
  configuration (CUDA désactivé, validation silencieusement sur CPU) quand le chemin de nvcc
  changeait d'environnement → reconstruction de zéro + arrêt si `--list-devices` ne voit pas
  CUDA ; un `llama-server` orphelin gardait le GPU si la chaîne était tuée → `PR_SET_PDEATHSIG`.
- Recommandation : garder la configuration retenue (fla + validation GGUF) comme base commitée ;
  Unsloth (−40 % de plus par pas) ne se décide qu'après un entraînement complet avec lui et
  un score GGUF à ±2 de la référence, puisqu'il change transformers et huggingface-hub de
  la chaîne et tire du LGPL.
- **Décision du propriétaire (2026-11-09) : « garder tout ».** Unsloth intégré à la chaîne, par
  défaut, au-dessus de fla et de la validation GGUF ; `--sans-unsloth` reprend transformers +
  peft. `pyproject.toml`/`uv.lock` : transformers 5.17.0, huggingface-hub 1.33.0, unsloth et
  unsloth-zoo 2026.11.3 (LGPL-3.0, outillage d'entraînement seulement, dit dans l'en-tête),
  fla 0.6.0 et roues nvcc gardés. `rapport.json` : `chemin_entrainement` et `versions`
  (unsloth, unsloth-zoo, fla, fla-core, triton, nvcc compris).
- Chaîne intégrée de bout en bout (`essai-vitesse-unsloth-chaine`, `--pas-max 20`) : 10,7 s/pas
  (pas 11-20), perte 0,4818 (+2,0 %) / 0,2765 (+0,4 %) contre 0,4723 / 0,2754, pic 6,37 Gio ;
  adaptateur converti en GGUF, fusion et quantification, validation `llama-server` 319 s
  (112/300 après 20 pas ; 119/300 pour le chemin fla seul, écart attendu à ce stade), essai des
  deux chemins lisible comme un plan (comme le chemin fla seul) ; commande entière 15 min.
- `--sans-unsloth` sous transformers 5.17.0 (`essai-vitesse-sans-unsloth`) : 17,9 s/pas,
  0,4742 / 0,2792, pic 6,50 Gio : le repli fonctionne.

## Plan Change Log

## Review Triage Log

## Verification

**Commands:**
- `uv run --project training/entrainement python training/entrainement/pipeline.py --sortie ~/cassini-entrainement/essai-vitesse-<piste> --pas-max 20` (depuis le worktree) -- expected: durée par pas, pic, perte relevés dans le journal
- `uv run --project training/entrainement python -m pytest training/entrainement/tests` (ou la commande des tests existante) -- expected: vert
