---
title: 'Modele ajuste, tache 2 : entrainement reproductible et fusion en GGUF'
type: 'feature'
ticket: '10'
created: '2026-10-22'
status: 'built'
baseline_revision: '4019c0c74853488e13adaf2e55a52c8cf48abf7b'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['quick', 'blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: ['{project-root}/docs/entrainement-du-modele.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le jeu d'exemples de la tâche 1 existe, mais rien ne transforme ces exemples en un petit modèle ajusté que llama.cpp sait charger. Sans une commande unique et reproductible, chaque commande ajoutée au registre coûterait un projet au lieu d'un ré-entraînement.

**Approach:** Tâche 2 sur 4 du ticket 5.10. Le crate `training` met chaque exemple en forme de prompt court : il écrit le texte exact vu par le modèle et la réponse attendue, et il est seul à le faire. Une chaîne Python lance la génération, puis un QLoRA de Qwen3.5-4B, une mesure sur un jeu de validation engendré, et sort trois GGUF : la base, l'adaptateur et le modèle fusionné quantifié.

## Boundaries & Constraints

**Always:** Décisions du propriétaire (2026-10-22) : base `Qwen/Qwen3.5-4B` (Apache-2.0), épinglée à une révision précise ; outils Hugging Face seulement (torch BSD-3, transformers, peft, trl, datasets, accelerate Apache-2.0, bitsandbytes MIT) et les scripts de conversion de llama.cpp (MIT), versions épinglées par un `uv.lock`. Les valeurs de champs restent dans les exemples : l'extension les enverra (ticket à part). Entraînement sous WSL, sur le GPU de 8 Go (RTX 3070 Ti). Réflexion coupée, au même format chatml que `ai/src/lib.rs` (`ASSISTANT_OPEN`). La perte ne porte que sur la réponse. La méthode garde la base intacte : l'adaptateur GGUF se charge sur la base GGUF, ce qui laisse la place à un adaptateur de domaine entraîné ailleurs sur la même base. Graine fixe partout. Jeu de validation engendré avec une autre graine. Les modèles et poids sortent hors du dépôt (`*.gguf`, `*.safetensors` déjà ignorés). Vocabulaire civil seulement.

**Never:** Lire ou entraîner sur `ai/tests/tenu_a_l_ecart/`. Toucher `ai/` (prompt, grammaire, chargement : tâche 3) ou `tests/epreuve-modele-local/`. Ajouter une commande au registre ou une famille d'exemples (les couches quasi synonymes restent dans `deferred-work.md`). Mettre torch ou une dépendance lourde dans la recette CI. Unsloth.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Mise en forme | `--format chat` | JSONL `{prompt, completion}` ; `completion` = plan JSON compact, `kind` en tête, puis `<\|im_end\|>` | — |
| Chaîne complète | une commande, dossier de sortie | 3 GGUF, `rapport.json` (révision de base, graines, hyperparamètres, perte, score de validation) | — |
| Reprise | dossier de sortie déjà rempli | refus sans rien écraser, sauf `--reprendre` | message, code non nul |
| GPU absent ou trop petit | — | arrêt avant le téléchargement de la base | message clair |
| Exemple trop long | jeton > longueur max | exemple écarté et compté, jamais tronqué | compte dans le rapport |

</frozen-after-approval>

## Code Map

- `training/src/lib.rs` -- `generate`, `to_jsonl`, `Example { inventory, request, expected }`. `Plan` de `cassini_core::plan` sérialise `kind` en tête (`#[serde(tag = "kind")]`), comme l'impose `plan_grammar` (`ai/src/plan.rs` l.92).
- `training/src/main.rs` -- analyse d'arguments faite à la main ; y ajouter `--format structured|chat`.
- `ai/src/plan.rs` l.117 `SYSTEM`, l.245 `inventory_text` -- règles de chaînage à condenser en quelques phrases, sans catalogue ni exemples. Lecture seule.
- `ai/src/lib.rs` l.222-236 -- chatml : `<|im_start|>{role}\n…<|im_end|>\n`, ouverture `<|im_start|>assistant\n<think>\n\n</think>\n\n`. À reproduire à l'identique, lecture seule.
- `~/.cargo/registry/src/*/llama-cpp-sys-2-0.1.160/llama.cpp` -- copie incomplète : `convert_hf_to_gguf.py` y est, mais ni `gguf-py` ni `convert_lora_to_gguf.py`. La chaîne clone donc llama.cpp au commit du sous-module de llama-cpp-sys-2 0.1.160, dans un cache hors dépôt, puis construit `llama-quantize` et `llama-cli` sur CPU.
- Qwen3.5-4B : `Qwen3_5ForConditionalGeneration` (vision + texte), couches hybrides `linear_attention`/`full_attention`, le `transformers` récent est exigé. Le LoRA porte sur les projections du modèle de langue seulement.
- Tailles mesurées (graine 1) : inventaire de 1 160 caractères en médiane et 2 330 au plus ; plan de 180 caractères en médiane et 840 au plus. Avec un système court, une séquence fait moins de 1 400 jetons.
- `.github/workflows/recette.yml` l.134-142 -- Python 3.12, `unittest` sans dépendances : les tests Python de la chaîne n'importent pas torch.
- `deferred-work.md` -- l.241 : test d'intégration de la commande `cassini-training`, à faire avant que la tâche 2 dépende du fichier.

## Tasks & Acceptance

**Execution:**
- [x] `training/src/prompt.rs` -- `system_text(inventory)` court (rôle, formes de réponse, chaînage, question plutôt que deviner, refus seulement si aucune commande ne sait faire), `render_prompt` en chatml, `completion(plan)` ; tests : longueur de la partie fixe bornée, sortie identique d'un appel à l'autre, `kind` en tête, chaque `expected` du jeu se relit en `Plan` -- un seul endroit décide du texte vu par le modèle
- [x] `training/src/main.rs`, `training/tests/commande.rs` -- `--format chat` ; test d'intégration : deux exécutions identiques, autre graine différente, code non nul sans `--out` -- ferme l'entrée différée
- [x] `training/entrainement/{pyproject.toml,uv.lock,.python-version}` -- dépendances épinglées, Python 3.12 -- reproductible
- [x] `training/entrainement/chaine.py` -- une commande : génération (`cargo run`, graine 1 pour l'entraînement et graine 2 pour la validation), contrôle du GPU, QLoRA (nf4, gradient checkpointing, graine fixe), score de validation, sauvegarde de l'adaptateur, conversion de l'adaptateur et de la base en GGUF, fusion, quantification Q4_K_M, essai `llama-cli` sur une demande, `rapport.json` -- « une commande pour re-entraîner »
- [x] `training/entrainement/{donnees.py,notation.py}` + `tests/` -- lecture du JSONL, écart des exemples trop longs, notation exacte des plans (le titre du PDF n'a qu'à être présent), sans torch, tests `unittest` -- vérifiable en CI
- [x] `.github/workflows/recette.yml` -- lancer ces tests `unittest` -- garde-fou
- [x] `docs/entrainement-du-modele.md` -- section tâche 2 : commande, sorties, durée, mémoire, score de validation, ajout futur d'un adaptateur de domaine -- consigné

**Acceptance Criteria:**
- Given un poste WSL avec le GPU de 8 Go, when on lance la commande unique dans un dossier vide, then elle finit sans intervention, sous 8 Go de mémoire vidéo, et sort la base GGUF, l'adaptateur GGUF, le modèle fusionné Q4_K_M et `rapport.json`.
- Given le modèle fusionné, when `llama-cli` répond à une demande de validation, then la sortie se lit comme un plan JSON.
- Given deux lancements de `cassini-training --format chat` avec la même graine, when on compare, then les fichiers sont identiques à l'octet.
- Given le rapport, when on le lit, then il donne le score exact sur la validation, la révision de la base et le commit de llama.cpp ; le score est recopié dans la doc et dans ce plan.

## Review Triage Log

Passe 1 (thorough, risque high : quick, blind-hunter, edge-case-hunter, verification-gap, intent-alignment). 0 high, 7 medium, 12 low, 4 false ; aucun intent_gap ni bad_plan. Vérification faite à la main sur les sorties du lancement : base Q4_K_M + adaptateur GGUF (`llama-completion --lora`) rend le plan attendu à l'identique sur un exemple de validation à 4 étapes ; la base seule rend un plan faux.

| # | Verdict | Route | Evidence |
|---|---------|-------|----------|
| 1 | medium | patch | CLAUDE.md §9 : noms de code en anglais ; le Python de la chaîne (modules, fonctions, classes, variables) est en français, contrairement à `qgis-plugin/` et `tests/epreuve-modele-local/`. Renommage mécanique, messages restés en français. |
| 2 | medium | patch | La voie base GGUF + adaptateur GGUF (Always) n'est jamais essayée par la chaîne ; l'essai ne porte que sur le fusionné et sur `validation[0]` (une question). Essai des deux voies sur un exemple `calls`. |
| 3 | medium | patch | `convertir` saute tout fichier existant à la reprise ; les outils écrivent directement au nom final, un fichier coupé est réutilisé et publié. Écriture sous un nom temporaire puis renommage. |
| 4 | low | patch | `cassini-training` écrit `--out` par `std::fs::write` direct ; la reprise réutilise `validation.jsonl` s'il existe. Écriture dans un `.tmp` puis renommage. |
| 5 | medium | patch | `commit_cassini` = `HEAD` seul ; la mesure a tourné sur un arbre modifié. Le rapport note l'arbre modifié (`git status --porcelain`) et l'empreinte de `uv.lock`. |
| 6 | medium | patch | Le commit de llama.cpp n'est lié par rien à `Cargo.lock` (`llama-cpp-sys-2` 0.1.160) ; un `cargo update` le décalerait sans échec. Constante de version et test qui lit `Cargo.lock`. |
| 7 | low | patch | `huggingface_hub` importé directement mais non déclaré, absent de `versions()`. Déclaré à la version du verrou (2.2.0), version notée au rapport. |
| 8 | low | patch | Le contrôle du GPU vient après la création de `travail/` : un premier refus laisse un dossier « non vide » qu'un lancement normal refuse ensuite. Contrôle avant toute écriture. |
| 9 | low | patch | `executer` perd la sortie d'erreur des commandes lancées avec `capture_output` : la cause d'un échec de llama.cpp est invisible. Fin de stderr dans le message. |
| 10 | low | patch | `llama-cli` construit sans être utilisé. Retiré de la liste des outils. |
| 11 | low | patch | `notation.py` et la doc disent « même règle que `ecart_reel.rs` » ; la notation exige en plus un texte non vide. Docstring et doc corrigées. |
| 12 | low | patch | La doc dit « graine fixe partout » sans dire que seuls les jeux sont reproductibles à l'octet, pas les poids (bitsandbytes, noyaux non déterministes). Phrase ajoutée. |
| 13 | medium | defer | Parité au jeton avec `ai/` : `ai` découpe par segment avec `special=false` pour le texte des tours, l'entraînement découpe le prompt d'un tenant ; et aucun test ne compare l'encadrement chatml aux constantes de `ai/`. Tâche 3 (Design Notes). |
| 14 | low | defer | `SYSTEM` du prompt court omet « (en comptant à partir de 1) » pour `step:k`. Le corriger maintenant décalerait le prompt du modèle entraîné : au prochain ré-entraînement. |
| 15 | medium | defer | Le score de validation est mesuré sur l'adaptateur en transformers, pas sur les GGUF livrés ; la mesure sur GGUF par `ai/` est la tâche 4. |
| 16 | low | rejeté | Jeux vides, `--pas-max 0` ou négatif, moins de 16 exemples : impossibles aux valeurs par défaut, outil de développeur, garde à ajouter. |
| 17 | low | rejeté | Reprise après ré-entraînement qui garderait validation et GGUF anciens : n'arrive que si l'adaptateur HF est effacé à la main (la sauvegarde précède la validation). |
| 18 | low | rejeté | Reprise avec d'autres `--exemples`/`--validation` que le premier lancement : usage improbable, garde à ajouter. |
| 19 | low | rejeté | `preparer_llama_cpp` bloqué si `git remote add` a échoué une fois : improbable, garde à ajouter. |
| 20 | low | rejeté | Pic de mémoire vidéo lu par `max_memory_reserved` (entraînement seul) : la doc dit déjà « réservée par torch », le pic réel reste sous 8 Go (le lancement a fini). |
| 21 | low | rejeté | Validation retirée sur prompt identique seulement : la doc dit déjà que la validation mesure les familles, pas la généralisation. |
| 22 | low | rejeté | `Score.noter` lève `TypeError` sur un attendu illisible : l'attendu vient du générateur, toujours lisible. |
| 23 | false | rejeté | `uv.lock` absent du diff : exclu volontairement du diff de relecture (83 Ko) ; présent, non ignoré, il sera commité. |
| 24 | false | rejeté | Dépendances `numpy`, `pyyaml`, `sentencepiece`, `protobuf` : exigées par la pile Hugging Face (lecture du tokenizer, conversion), licences BSD/MIT/Apache, signalées au propriétaire. |
| 25 | false | rejeté | AC 2 nomme `llama-cli` : l'écart vers `llama-completion` est consigné ; corriger l'AC reviendrait à éditer le plan. |
| 26 | false | rejeté | Ajouter une commande demande encore une famille écrite à la main : hors de l'intention de cette tâche (Never : aucune famille). |

## Design Notes

Le texte du prompt est écrit en Rust dans `training`, et Python ne met rien en forme : il lit `prompt` et `completion`. Si les deux langages formataient chacun de leur côté, un écart d'un espace suffirait à apprendre au modèle un format qu'il ne verra jamais. La tâche 3 fera lire ce même module à `ai` (dépendance `cassini-ai → cassini-training`, sans cycle) ou le comparera octet par octet.

Le score de validation mesure ce que le modèle a appris des familles. Il ne remplace pas le jeu tenu à l'écart, qu'on mesure à la tâche 4 avec le prompt court branché.

## Mesure (2026-10-22)

Premier entraînement complet, sous WSL sur la RTX 3070 Ti : **score de validation 300 / 300**
(plans 250 / 250, questions 37 / 37, refus 13 / 13), base `Qwen/Qwen3.5-4B` à la révision
`851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a`, llama.cpp au commit
`d81235049384534c167caea52b85a694f6103d14`. Perte finale 0,005, pic de mémoire vidéo
6,5 Gio, 3 h 31 au total. Essai du modèle fusionné : plan JSON lisible et juste. Détail dans
`docs/entrainement-du-modele.md`. Écart à la lettre du plan : l'essai passe par
`llama-completion`, l'outil de llama.cpp qui prend un prompt brut (à ce commit, `llama-cli`
applique son propre gabarit de conversation).

## Verification

**Commands:**
- `cargo test -p cassini-training`, `cargo clippy -p cassini-training --all-targets -- -D warnings`, `cargo fmt --check` (WSL) -- expected: succès
- `python3 -I -m unittest discover -s training/entrainement/tests` -- expected: succès sans torch
- `uv run --project training/entrainement python training/entrainement/chaine.py --sortie <hors dépôt>` -- expected: les trois GGUF et `rapport.json`
- PowerShell `cargo deny check` -- expected: succès (aucune dépendance Rust ajoutée)
