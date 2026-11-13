---
title: "Ajustement continu : l'entraînement repart de l'adaptateur du modèle installé"
type: 'feature'
ticket: '54'
created: '2026-11-11'
status: 'built'
baseline_revision: '8b94aaa1e7ee70b55a544bd85574ad47361535d7'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Chaque correction du modèle impose un entraînement de zéro de plusieurs heures sur 8 600 exemples, deux graines par nuit, alors que le modèle installé (`ajuste-apres-5-49-b`) ne rate que quelques familles (remesure du 2026-11-11).

**Approach:** `pipeline.py` peut repartir de l'adaptateur LoRA d'un entraînement précédent (`--depuis <dossier>`) : jeu de séance = familles choisies + rappel ≥ 40 % tiré de toutes les familles, taux 5e-5, une seule graine ; le rapport porte la lignée ; au plus 3 maillons ; parent refusé si base, révision ou modules diffèrent. Ce plan est le lot 1 (mécanique de reprise et script de nuit écrit, non lancé) ; le contenu du jeu (familles neuves ou renforcées) est le lot 2.

## Décisions (propriétaire, 2026-11-11)

- D1 : découpage en deux lots ; ce plan = lot 1 ; lot 2 (familles) inscrit au `deferred-work.md`, `after` lot 1.
- D2 : séance continue de 3 000 exemples par défaut, dont 1 200 de rappel (`--exemples` réglable).
- D3 : part du rappel 0,4 par défaut, plancher 0,4 (`--part-rappel`).
- D4 : la première vraie séance attend le lot 2 ; l'essai court (`--pas-max`) suffit à éprouver la mécanique.

## Boundaries & Constraints

**Always:**
- Toute garde du parent (lecture, comparaison, lignée, 4e maillon) s'exécute avant le GPU et avant de toucher au dossier de sortie, et se teste sans torch.
- Le parent est un dossier de sortie de `pipeline.py` : `rapport.json` + `travail/adaptateur-hf/{adapter_config.json, adapter_model.safetensors}`. Il n'est jamais modifié.
- Lignée = liste de maillons ; maillon 1 = entraînement de zéro (`depart: "base"`). Chaque maillon : commit Cassini, graine torch, SHA-256 de l'`adapter_model.safetensors` produit, et pour les maillons ≥ 2 le SHA-256 de l'adaptateur de départ. Un parent sans clé `lignee` (cas de B) est le maillon 1, reconstruit depuis son rapport.
- Les entraînements de zéro écrivent aussi `lignee` (un maillon), pour que la suite se lise directement.
- Le jeu de rappel est un préfixe du jeu graine 1 (déjà gardé par `recouvrement.rs`) ; le jeu ciblé, tiré à une autre graine, est gardé lui aussi par `recouvrement.rs`.
- La validation reste celle de graine 2, 300 exemples, moins les prompts vus : comparable au parent pour `non_oubli.py`.
- Le juge de chaque maillon est `non_oubli.py` contre `/home/kevin/cassini-entrainement/reference-non-oubli/` ; le script de nuit l'appelle, n'installe rien.

**Never:**
- Lancer un entraînement, un essai GPU ou un modèle pendant la construction (l'essai court du `verify` attend l'accord du propriétaire).
- `--depuis` avec `--exemples-externes` (refusé : l'adaptateur de domaine reste hors de la chaîne).
- Recopier, même à un nombre ou une commune près, une phrase de barre dans une famille.
- Écrire ou repondérer des familles dans `families.rs` (lot 2, D1).
- Toucher au dépôt principal, à `CASSINI-nuit`, aux dossiers de modèles ; pas de push.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Départ de B | `--depuis ajuste-apres-5-49-b --familles f1,f2` | jeu ciblé + rappel ≥ 40 %, taux 5e-5, rapport `lignee` à 2 maillons (B reconstruit, puis celui-ci), `depart` = SHA de l'adaptateur de B | — |
| Base différente | rapport parent `base.depot` ou `base.revision` ≠ épinglés, ou `adapter_config` `base_model_name_or_path` d'une autre révision | refus « chaîne coupée », avant GPU | `PipelineStop`, code 1 |
| Modules différents | `hyperparametres.modules`/`target_modules`, `rang`/`r` ou `alpha` ≠ courants | refus « chaîne coupée » | `PipelineStop` |
| 4e maillon | parent avec `lignee` de 3 maillons | refus : « recalage : entraînement de zéro sur le jeu complet » | `PipelineStop` |
| Adaptateur substitué | SHA de `adapter_model.safetensors` ≠ dernier maillon du rapport parent | refus | `PipelineStop` |
| Parent incomplet | rapport, config ou safetensors absent / illisible | refus en nommant le fichier | `PipelineStop` |
| Famille inconnue | `--familles nexiste_pas` | refus en listant les noms connus | générateur code 2, `PipelineStop` |
| Rappel trop faible | `--part-rappel 0.3` | refus (≥ 0,4) | `PipelineStop` |
| `--familles` sans `--depuis` | — | refus | `PipelineStop` |
| `--reprendre` | autre parent ou autres familles que le rapport commencé | refus | `PipelineStop` |

</frozen-after-approval>

## Code Map

- `training/entrainement/pipeline.py` -- `check_options` (l.201) : nouvelles combinaisons d'options ; `launch` (l.983) : lecture du parent juste après `check_options`, avant `gpu_memory` ; `report.update` (l.1008) : `hyperparametres.taux_apprentissage` effectif, `lignee`, `depart` ; `check_resume` (l.299) : comparer aussi parent et familles ; `generate` (l.388) : ajouter le jeu ciblé (graine 3, `--familles`) et prendre le rappel comme préfixe graine 1 ; `train` (l.538) : après `get_peft_model` (Unsloth ou peft, même config), charger les poids du parent par `peft.set_peft_model_state_dict` sur `safetensors.torch.load_file`, refuser toute clé inattendue ou paramètre LoRA non chargé ; lr pris dans les hyperparamètres effectifs (l.592). `sha256` (l.361), `repository_state` (l.969), `PipelineStop` à réutiliser. `main` (l.1190) : `--depuis`, `--familles`, `--part-rappel`.
- `training/src/main.rs` + `training/src/lib.rs` (`generate` l.277, `FAMILIES` dans `families.rs:53`) -- ajouter `--familles a,b` : tirage pondéré restreint aux familles nommées (`generate` inchangé pour le jeu complet, octet pour octet) ; nom inconnu → code 2 avec la liste.
- `training/tests/recouvrement.rs` -- `overlap` (l.~225) lit `generate(DEFAULT_SEED, DEFAULT_COUNT)` : ajouter les mêmes contrôles sur chaque famille tirée seule à la graine du jeu ciblé (constante partagée, ex. `FOCUS_SEED = 3`).
- `training/entrainement/tests/` -- nouveau `test_ajustement_continu.py`, style de `test_exemples_externes.py` (unittest, fichiers synthétiques, `options(...)`, `python3 -I -m unittest discover -s training/entrainement/tests`).
- `docs/entrainement-du-modele.md` -- nouvelle section après « Barre de non-oubli » (l.2579-2669), et rappel dans « Une commande pour ré-entraîner » (l.1144-1206).
- `/home/kevin/cassini-entrainement/nuit-5-54.sh` (hors dépôt) -- modèle : `nuit-apres-5-49.sh` (un seul `mesurer`, état dans un `.etat`) et les `gardes()` de `mesures-5-53-3.sh`.
- Parent réel, lu seulement : `/home/kevin/cassini-entrainement/ajuste-apres-5-49-b/` (rapport : base Qwen/Qwen3.5-4B @ 851bf6e…, graine torch 2, commit b8c3bdc ; `adapter_config` r 16, alpha 32, `revision: null`, révision dans `base_model_name_or_path`).
- Familles existantes utiles à la 1re séance (pour la liste par défaut du script) : `export_pdf_format`, `export_pdf_scale`, `refus`, `refus_hors_registre`, `champ_de_vision`, `champ_de_vision_cumule`, `reclassement_raster`, `conversion_coordonnees`, `reprojection_point`. Manquent : `page_size` + échelle (`export_pdf_format` dit « jamais `scale` », families.rs:2490), courbure (aucune occurrence), N/E inversé, `transform_point` avant `geo.*` (à vérifier dans `point_projete`, `distance_azimut`).

## Tasks & Acceptance

**Execution:**
- [x] `training/src/lib.rs`, `training/src/main.rs` -- `--familles` (sous-ensemble du tirage pondéré) + test unitaire : nom inconnu refusé, jeu complet inchangé -- le jeu ciblé vient du même générateur.
- [x] `training/tests/recouvrement.rs` -- contrôles étendus au jeu ciblé (graine 3) -- garde des barres sur tout ce qui est entraîné.
- [x] `training/entrainement/pipeline.py` -- `read_parent`, `check_parent`, `lineage`, mélange ciblé + rappel, taux 5e-5, chargement des poids, reprise -- cœur du ticket.
- [x] `training/entrainement/tests/test_ajustement_continu.py` -- une ligne de la matrice par test, plus : rapport à lignée complète, rappel ≥ 40 % compté, `lignee` d'un entraînement de zéro.
- [x] `docs/entrainement-du-modele.md` -- commande, règle des 3 maillons, juge `non_oubli.py`.
- [x] `/home/kevin/cassini-entrainement/nuit-5-54.sh` -- gardes GPU, une séance depuis B (graine 1, 3 000 exemples, rappel 0,4), liste des familles en argument obligatoire (remplie au lot 2, D4), copie dans `cassini-modeles/ajuste-continu-5-54-1` non installé, essais GPU + recette Annecy, puis `non_oubli.py --parent reference-non-oubli/* --candidat …`, verdict dans l'état -- écrit, jamais lancé.

**Acceptance Criteria:**
- Given aucun `--depuis`, when la chaîne tourne, then jeu, hyperparamètres et sorties sont ceux d'avant (seule `lignee` s'ajoute au rapport).
- Given l'accord du propriétaire et le GPU libre, when l'essai court `--depuis …/ajuste-apres-5-49-b --familles … --pas-max 2` tourne, then le GGUF fusionné et sa déclaration se chargent dans le noyau (première section des essais GPU) et le rapport porte 2 maillons.

## Design Notes

Lignée d'un 2e maillon depuis B :

```json
"lignee": [
  {"maillon": 1, "depart": "base", "commit": "b8c3bdc…", "graine_torch": 2, "adaptateur_sha256": "<B>"},
  {"maillon": 2, "depart": "<B>", "commit": "<HEAD>", "graine_torch": 1, "adaptateur_sha256": "<rempli après train>",
   "familles": ["…"], "part_rappel": 0.4}
]
```

Pourquoi `set_peft_model_state_dict` après `get_peft_model` plutôt que `PeftModel.from_pretrained` : le chemin Unsloth (checkpointing, noyaux) reste identique à un entraînement de zéro ; le contrôle des clés rend visible un chargement partiel, que l'essai court confirme sur GPU.

## Verification

**Commands:**
- `python3 -I -m unittest discover -s training/entrainement/tests` -- expected: tout passe, sans torch.
- `cargo test -p cassini-training` -- expected: tout passe, `recouvrement` compris.
- `cargo fmt --check && cargo clippy -p cassini-training -- -D warnings` -- expected: propre.
- `bash -n /home/kevin/cassini-entrainement/nuit-5-54.sh` -- expected: syntaxe valide.

**Manual checks (if no CLI):**
- Essai court (accord du propriétaire) : critère d'acceptation 2.

## Implementation Notes

- Générateur : `generate` passe par `generate_from` sur toutes les familles ; jeu complet vérifié
  identique à l'octet (graine 1 × 8 600 et graine 2 × 300, SHA-256 avant/après égaux), et le
  rappel graine 1 × 1 200 est bien le préfixe du jeu graine 1 × 8 600 (`cmp`). `--familles`
  inconnue : code 2 avec la liste des noms.
- Constat de la garde du jeu ciblé (`recouvrement.rs`) : tirées seules à la graine 3 jusqu'à
  1 800 exemples, cinq familles engendrent une phrase de barre à un nombre ou une commune près
  (`filtre_numerique`, `pente`, `reprojection_couche`, `retrait_couche`, `carroyage_mgrs` ; ids
  SEL2, PL15, RP2, LR1, PDF3). Réparer leurs tournures est du lot 2 (D1, « Never : écrire des
  familles ») : elles sont listées dans `FOCUS_EXCLUDED` (`training/src/lib.rs`), refusées au jeu
  ciblé par le générateur et, avant le GPU, par `pipeline.py` ; le test tient la liste exacte dans
  les deux sens. Aucune n'est dans la liste par défaut de la 1re séance. Inscrit au
  `deferred-work.md`.
- `pipeline.py` : `check_continuation` (options, `read_parent`, `check_parent`) juste après
  `check_options`, avant `gpu_memory` et `prepare_output` ; `check_resume_parent` lit le rapport
  commencé en lecture seule, avant le GPU aussi. Les noms de familles se vérifient avant le GPU
  en lisant `families.rs` (et `FOCUS_EXCLUDED` dans `lib.rs`) ; le générateur reste l'autorité.
  `--depuis` exige `--familles`. Sans `--depuis`, le rapport ne gagne que `lignee` (`depart` et
  `depuis` n'apparaissent qu'en séance continue).
- Parent réel `ajuste-apres-5-49-b` lu (sans écriture) : passe les gardes, lignée reconstruite
  (commit b8c3bdc, graine 2, adaptateur 0ff14a1d…), 352 tenseurs LoRA dans l'en-tête safetensors.
- Script de nuit écrit, `bash -n` valide, jamais lancé. Le script Windows de recette
  (`recette-5-49\recette.ps1`, `recette-5-56.ps1`) n'existe plus dans `%TEMP%` : `RECETTE_PS1`
  (défaut `%TEMP%\recette-5-54.ps1`) est à recréer, le script refuse de partir sans lui
  (`deferred-work.md`).
- Après relecture (2026-11-11) : rappel tiré avec une marge de 60, coupé au compte après filtrage ;
  bornes `FOCUS_COUNT` / `DEFAULT_COUNT` lues dans `lib.rs`, avant le GPU ; module
  `training/src/barres.rs` (sources et normalisation sorties de `recouvrement.rs`) et refus du
  générateur, code 3, d'écrire un jeu ciblé qui copie une phrase de barre (contrôle fait à la
  génération, après la lecture de la mémoire du GPU et avant l'entraînement, comme toute la
  génération) ; lignée vérifiée maillon par maillon ; parent d'essai court ou sans validation
  refusé ; identité complète de l'adaptateur ; `commit_cassini` en repli ; empreinte revérifiée et
  matrices `lora_B` comparées au chargement ; doublons par prompt ; compte des jeux vérifié ;
  reprise : familles triées, nombre d'exemples, commit gardé ; option `--familles-connues` ; la
  regex des noms manquait `decompte_par_zone` (commentaire avant `name:`), corrigée et tenue par
  un test Rust ; tests de `launch` au-delà du GPU (mocks).
- Non fait (attend l'accord du propriétaire) : l'essai court GPU du critère d'acceptation 2, qui
  confirmera aussi le chargement des poids par `set_peft_model_state_dict` sous Unsloth.

## Plan Change Log

## Review Triage Log

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment).
Verdicts : high 0, medium 10, low 8, false 0, maybe-false 0. Corrigés 17 (patch, même implémenteur), reportés 1, rejetés 0.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| Séance par défaut au plancher exact : un rappel filtré arrête la chaîne après le GPU | medium | patch | marge de 60 exemples, coupe après filtrage |
| Jeu ciblé et rappel non bornés aux comptes gardés | medium | patch | bornes `FOCUS_COUNT`/`DEFAULT_COUNT` lues dans `lib.rs`, refus avant le GPU |
| Garde des barres sur familles seules, pas sur le mélange tiré | medium | patch | `training/src/barres.rs` ; le binaire refuse (code 3) un jeu ciblé qui copie une phrase de barre |
| Lignée non vérifiée maillon par maillon ; parent d'essai court accepté | medium | patch | `check_lineage` ; `pas_max` ou validation absente refusés |
| Identité du parent incomplète (dropout, bias, DoRA, rsLoRA, modules_to_save) | medium | patch | comparée |
| Chargement des poids contrôlé sur les clés seulement | medium | patch | empreinte revérifiée, `lora_B` comparées au parent ; essai GPU : 176 matrices vérifiées |
| Chemin `launch` au-delà du GPU non testé (taux, adaptateur, lignée, rapport) | medium | patch | classe `LaunchPastTheGpu` |
| `--familles` du binaire jamais appelé | medium | patch | test de `commande.rs` |
| Propriété « préfixe » du rappel non testée | medium | patch | test `generate(1,1200)` préfixe |
| Liste des familles par regex non reliée à `FAMILIES` | medium | patch | test Rust ; bogue trouvé (`decompte_par_zone`) et corrigé |
| `mix` contourné en séance | low | patch | test : `mix` ne transforme jamais l'entraînement |
| Doublons retirés sur l'exemple entier | low | patch | retirés sur la demande, comptés |
| Ciblé plus court que demandé | low | patch | `check_counts` |
| Reprise : ordre des familles, comptes, commit du maillon | low | patch | corrigés |
| `rebuilt_first_link` silencieux | low | patch | repli `commit_cassini`, sinon arrêt |
| Script de nuit : worktree sale, état effacé, résumé fragile | low | patch | corrigés |
| Mise en forme de `deferred-work.md` | low | patch | corrigée |
| Garde des barres et génération après le contrôle du GPU (écrit dans la sortie) | low | defer | la génération écrit le dossier de sortie, garanti intact avant le contrôle ; arrêt avant téléchargement et entraînement |

Essai court (accord du propriétaire, 2026-11-11, `--familles export_pdf_scale,refus_hors_registre,champ_de_vision_cumule --pas-max 2` depuis ajuste-apres-5-49-b) : poids du parent chargés, 176 matrices `lora_B` vérifiées ; lignée à 2 maillons (base → B 0ff14a1d…) ; validation 278/300 ; complétion de contrôle valide ; `cassini.exe run model.list` charge le GGUF. Critère d'acceptation 2 tenu.
