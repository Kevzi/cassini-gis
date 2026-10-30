---
title: "La chaîne d'entraînement accepte un fichier d'exemples externe"
type: 'feature'
ticket: '23'
created: '2026-10-30'
status: 'done'
baseline_revision: '43a0e8f0f9371ab6aea6feac728eaea8fe29398b'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** La commande d'entraînement n'apprend que le jeu engendré depuis le registre. Un adaptateur de domaine, entraîné hors de ce dépôt sur ses propres exemples, ne peut pas la réutiliser.

**Approach:** Une option donne à la commande un fichier d'exemples externe au format `chat` (`{prompt, completion}`). Elle entraîne alors un adaptateur sur la même base et avec le même prompt court, sous un nom choisi qui n'écrase pas le modèle civil, et écrit sa déclaration `*.modele.json`. Le rapport consigne l'empreinte et le nombre des exemples externes, jamais leur contenu.

**Décisions (2026-10-30, propriétaire) :**
- **Mélange** : par défaut, le jeu civil engendré plus le fichier externe ; une option prend le fichier externe seul.
- **Validation** : par défaut, une part du fichier externe est mise de côté pour le score ; une option accepte un second fichier de validation externe.
- **Sorties** (2026-10-30, réponse 1a) : avec un fichier externe, la sortie contient l'adaptateur, la base GGUF et la déclaration (base + adaptateur), sans fusion. La base n'est pas dans le dossier des modèles du poste ; identique à chaque entraînement, elle servira à tous les adaptateurs. Taille du plan (≈ 2 000 jetons) gardée par le propriétaire.

## Boundaries & Constraints

**Always:** sans l'option, la commande et ses sorties restent exactement celles de 5.17 (mêmes noms, fusion comprise) ; mêmes base, révision, graines et hyperparamètres ; le dossier de sortie reste hors du dépôt ; le score civil de validation reste mesuré quand le jeu civil est mélangé.

**Never:** aucun vocabulaire de domaine dans ce dépôt, tests compris (fichier synthétique seulement) ; aucun contenu externe dans `rapport.json` ni dans le dépôt ; aucun changement du prompt, de la grammaire ni du noyau.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Mélange par défaut | fichier externe valide, un nom | entraînement sur jeu civil + externe ; adaptateur et déclaration sous le nom donné | — |
| Externe seul | option « externe seul » | entraînement sur le fichier externe seul | — |
| Validation externe donnée | second fichier | il sert au score externe ; aucune part n'est retirée du premier | — |
| Ligne mal formée | une ligne sans `completion` | arrêt avant le GPU et le téléchargement | message qui nomme le fichier et la ligne |
| Nom qui écrase le civil | nom = noms de sortie civils | refus avant tout travail | message clair |
| Validation recouvrant l'entraînement | prompt identique des deux côtés | retiré de la validation et compté | — |
| Sans option | commande de 5.17 | sorties et rapport identiques à 5.17 | — |

</frozen-after-approval>

## Code Map

- `training/entrainement/pipeline.py` -- noms de sortie `BASE_NAME`, `ADAPTER_NAME`, `MERGED_NAME`, `DECLARATIONS` (l.91-102), `write_declarations` ; `generate` (l.200) engendre `entrainement.jsonl` et `validation.jsonl` ; `launch` (l.503) : GPU, dossier, jeux (l.528-536), rapport `donnees` avec empreintes (l.547-560), `convert` (l.388) qui fait base, adaptateur et fusion, `try_model` sur les deux chemins (l.596-602) ; `main` (l.623), options en français (`--sortie`, `--reprendre`, `--pas-max`). Lire et vérifier le fichier externe avant `gpu_memory()` : l'ordre « refus avant tout coût » est déjà la règle de `launch`.
- `training/entrainement/data.py` -- `read_jsonl` / `read_line` (refus ligne par ligne, message `ligne N`), `drop_overlap`, `drop_too_long` : à réutiliser tels quels pour le fichier externe ; ajouter seulement le découpage d'une part de validation, déterministe par graine.
- `training/entrainement/tests/test_data_scoring.py` -- style des tests (sans torch) ; `test_the_two_declarations_name_the_outputs_by_file_name_with_the_short_prompt` (l.195) à garder vert.
- `docs/entrainement-du-modele.md` -- section « Un adaptateur de domaine, plus tard » (l.318) : la remplacer par l'usage de l'option.
- Lancement réel (5.17) : sous WSL, `UV_PROJECT_ENVIRONMENT=~/.cache/cassini-entrainement/venv uv run --project training/entrainement python training/entrainement/pipeline.py --sortie <dossier>`, journal à côté ; cache de llama.cpp et de la base déjà rempli.
- Ne pas toucher : `training/src/` (générateur et prompt), `ai/`, `core/`.

## Tasks & Acceptance

**Execution:**
- [x] `training/entrainement/data.py` -- part de validation tirée d'un jeu, déterministe par graine -- seul ajout de lecture nécessaire.
- [x] `training/entrainement/pipeline.py` -- options du fichier externe, du nom, du mode « externe seul » et du fichier de validation externe ; garde du nom ; jeux mélangés ; score civil et score externe séparés dans le rapport ; sorties : adaptateur, base GGUF et déclaration, sans fusion ; chemin sans option inchangé.
- [x] `training/entrainement/tests/` -- un test par ligne de la matrice, avec un fichier synthétique écrit par le test.
- [x] `docs/entrainement-du-modele.md` -- usage de l'option, sorties, copie dans le dossier des modèles.
- [x] essai réel court (`--pas-max`) sur GPU avec un fichier externe synthétique, hors dépôt ; résultat dans les Implementation Notes.

**Acceptance Criteria:**
- Given un fichier externe synthétique et un nom, when la commande tourne avec `--pas-max`, then le dossier de sortie contient l'adaptateur et sa déclaration sous ce nom, et `cassini run model.list` les liste quand le dossier est pris comme dossier des modèles.
- Given le même essai, when on lit `rapport.json`, then il donne l'empreinte et le nombre des exemples externes, deux scores de validation, et aucun prompt externe.
- Given les tests de `training/entrainement/tests` et la recette, when ils tournent, then ils sont verts, CI comprise.

## Implementation Notes

- Options : `--exemples-externes`, `--nom`, `--externe-seul`, `--validation-externe`. `check_options` et `read_external` passent avant `gpu_memory()` ; un nom qui commence par `cassini-composition` ou qui retombe sur une sortie civile est refusé. Part de validation : 10 % (`EXTERNAL_VALIDATION_SHARE`), tirée par `VALIDATION_SEED` (`data.split_validation`).
- Sorties externes : `qwen3.5-4b-base-Q4_K_M.gguf`, `<nom>-adaptateur-f16.gguf`, `<nom>.modele.json` ; `convert` saute la fusion (`Outputs.merged = None`). Sans option, `CIVIL_OUTPUTS` reprend les noms et déclarations de 5.17.
- Rapport : `donnees.externe` (empreintes et comptes), `validation` (civil, si mélangé) et `validation_externe`. L'essai `llama-completion` sur un exemple externe ne consigne ni sortie ni attendu (`private`). Ajout hors plan : `check_resume` refuse `--reprendre` avec un autre fichier ou un autre mélange.
- Essai réel le 2026-10-30, RTX 3070 Ti sous WSL : `--exemples 32 --validation 4 --pas-max 2`, fichier externe synthétique de 40 exemples (`cassini-training --format chat --seed 77`, hors dépôt dans `~/cassini-entrainement/5-23-essai-donnees/`), `--nom domaine-essai`, sortie `~/cassini-entrainement/5-23-essai/`. 5 min, code de sortie 0. Rapport : 68 exemples d'entraînement dont 36 externes, 4 validations externes ; scores civil 0/4 et externe 1/4 (attendus après 2 pas) ; essai base + adaptateur : plan lisible. Aucun prompt externe dans `rapport.json` (vérifié par recherche). `cassini.exe run model.list` (installé), `CASSINI_MODELS` sur ce dossier : une entrée, `domaine-essai.modele.json`.
- Vérification : `python -I -m unittest discover -s training/entrainement/tests` 38 tests verts (11 nouveaux dans `test_exemples_externes.py`) ; `cargo test -p cassini-training --locked` (PowerShell) vert, inchangé.

## Plan Change Log

## Review Triage Log

Relecture `quick`, itération 0.

| # | Constat | Verdict | Preuve | Suite |
|---|---|---|---|---|
| 1 | Une réponse externe qui n'est pas un plan (`bonjour<|im_end|>`, `kind` inconnu) passe la lecture puis fait planter `Score.record` après l'entraînement | medium | `data.read_line` ne vérifie que la marque de fin ; `scoring.Score.record` indexe `read_plan(expected)["kind"]` (scoring.py l.118) | patch : `read_dataset` refuse la ligne avant le GPU, avec le fichier et le numéro ; test ajouté |
| 2 | Une validation vidée par le recouvrement ou la longueur n'est vue qu'après l'entraînement | medium | le seul contrôle était dans la boucle des scores, après `train` | patch : arrêt juste après `drop_too_long`, avant l'entraînement |
| 3 | `check_resume` ne compare pas `--nom` | low | réel, mais une reprise sous un autre nom est improbable et la correction ajoute un paramètre | rejeté |
| 4 | Le faux `snapshot_download` du test de ligne mal formée ne vérifie rien (import local dans `launch`) | low | `launch` importe `snapshot_download` dans la fonction | patch : faux et assertion retirés |
| 5 | La ligne « Sans option » n'est testée qu'en partie (rapport de `launch` non figé) | low | à la lecture, les clés de 5.17 et leur ordre sont gardés ; figer le rapport exigerait de simuler tout `launch` | rejeté |

Après les correctifs : 39 tests Python verts ; `cargo test -p cassini-training --locked` vert.

## Design Notes

Le mélange par défaut garde les phrases civiles : choisi dans les Réglages, un adaptateur entraîné sur le domaine seul oublierait les commandes ordinaires. Les deux scores séparés montrent s'il les oublie quand même.

## Verification

**Commands:**
- `python -I -m unittest discover -s training/entrainement/tests` -- expected: vert.
- `cargo test -p cassini-training --locked` (PowerShell) -- expected: vert, inchangé.
