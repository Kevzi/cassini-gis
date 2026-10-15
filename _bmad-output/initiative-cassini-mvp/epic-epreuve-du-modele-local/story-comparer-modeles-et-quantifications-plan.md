---
title: 'Comparer modeles et quantifications'
type: 'chore'
ticket: '4'
created: '2026-10-15'
status: 'built'
baseline_revision: '3031462'
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

**Problem:** on ne sait pas si Qwen3.5-9B est le bon modèle local, ni ce que coûte, en mémoire et en temps, un modèle plus lourd ou plus léger. Le verdict de 8.6 doit nommer un modèle, une quantification et une configuration minimale, avec des chiffres.

**Approach:** le propriétaire a choisi Mistral, modèle français. Télécharger Ministral 3 8B Instruct 2512 (Apache 2.0) en Q4_K_M et Q6_K, hors du dépôt, avec contrôle des empreintes publiées. Lancer le harnais sur la meilleure forme de 8.5 (sortie contrainte par schéma, catalogue complet, avec exemples) pour trois combinaisons : Qwen3.5-9B Q4_K_M, Ministral Q4_K_M, Ministral Q6_K. Relever pour chacune la licence, les deux taux, la mémoire vive et la mémoire vidéo au pic, le temps de chargement, le temps de lecture de l'invite et le temps de génération séparément, le processeur et le nombre de fils.

## Boundaries & Constraints

**Always:**
- Même forme, mêmes 25 phrases, température 0, graine fixe pour les trois combinaisons ; Qwen est remesuré avec la même version du harnais pour que les mémoires soient comparables.
- Le format d'invite de Ministral est celui de sa fiche officielle (`[SYSTEM_PROMPT]…[/SYSTEM_PROMPT][INST]…[/INST]`), vérifié sur la fiche, avant toute mesure ; une invite mal formée fausserait la comparaison.
- Les fichiers de modèle restent hors du dépôt ; l'empreinte SHA-256 de chaque fichier est comparée à celle publiée par Hugging Face ; la licence de chaque modèle est lue sur sa fiche et notée.
- Aucune mesure ne s'appuie sur un accès réseau ; le téléchargement est la seule opération réseau de l'entrée, faite une fois et sur demande du propriétaire.
- Un écart de moins de deux phrases n'est pas un écart ; l'écrire.
- Avant de comparer, vérifier que le gabarit d'invite ne produit pas de double jeton de début de séquence.

**Never:**
- Choisir le modèle de l'epic : c'est le verdict de 8.6.
- Changer la forme de sollicitation, le catalogue ou les phrases pour améliorer un score.
- Verser un fichier GGUF au dépôt.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Téléchargement complet | fichiers GGUF | empreintes égales aux empreintes publiées | une empreinte différente arrête l'entrée, le fichier est supprimé |
| Trois combinaisons | modèles présents | un tableau de licence, taux, RAM, VRAM, temps, processeur et fils | une combinaison qui plante est signalée, jamais omise |
| Modèle trop gros pour la carte | Q6_K sur 8 Go de VRAM | mesuré tel quel, avec la mémoire vidéo au pic | si le chargement échoue, le dire et essayer une fenêtre de contexte plus petite, en le notant |
| Invite mal formée | gabarit Ministral | repéré avant la mesure par un essai de contrôle | l'essai donne un plan lisible sur une phrase simple |

</frozen-after-approval>

## Code Map

- `tests/epreuve-modele-local/harness.py` -- modifier : gabarit `mistral3` ; mesure du pic de mémoire vive (processus enfant) et de mémoire vidéo (`nvidia-smi`) ; relevé du processeur et du nombre de fils ; temps de lecture et de génération déjà séparés.
- `tests/epreuve-modele-local/run_models.py` -- à créer : enchaîne les trois combinaisons sur la forme retenue, écrit `resultats-8-4.md` et `.json`.
- `tests/epreuve-modele-local/check_harness.py` -- ajouter : le gabarit `mistral3`, le rendu du tableau de comparaison.
- `tests/epreuve-modele-local/lecture-8-4.md` -- à créer à la main : lecture des chiffres.
- `C:/Users/kevin/cassini-modeles/modeles/` -- modèles, hors dépôt.
- `run_forms.py`, `resultats-8-5.*`, `catalog.json`, `inventory.json`, `sentences.json` -- à réutiliser sans modifier.

## Tasks & Acceptance

**Execution:**
- [x] Télécharger et contrôler les deux GGUF Ministral (licence lue sur la fiche).
- [x] `harness.py` -- gabarit `mistral3`, mémoire au pic, processeur et fils.
- [x] Essai de contrôle du gabarit sur une phrase simple, avec examen des jetons d'invite.
- [x] `run_models.py` et `check_harness.py` -- comparaison, contrôlée en rejeu.
- [x] Lancer les trois mesures, commiter `resultats-8-4.*` et `lecture-8-4.md`.

**Acceptance Criteria:**
- Given `resultats-8-4.md`, when on le lit, then il donne pour chacune des trois combinaisons : licence, taux d'emblée et après relance, mémoire vive et vidéo au pic, temps de chargement, de lecture et de génération, processeur et nombre de fils, sur les mêmes 25 phrases.
- Given les fichiers téléchargés, when on compare leur empreinte SHA-256 à celle publiée, then elles sont égales.

## Implementation Notes

Route oneshot épinglée : aucun sous-agent de codage autorisé. Les trois mesures durent environ 3 minutes chacune et sont lancées en arrière-plan.

Fait : empreintes des trois GGUF égales à celles d'unsloth ; gabarit `mistral3` sans `<s>` (le chargeur l'ajoute, vérifié à `llama-tokenize`) ; `harness.py` relève RAM au pic (jeu de travail, ctypes), VRAM au pic (`nvidia-smi`), fils ; `run_models.py` créé ; `resultats-8-4.*` et `lecture-8-4.md`. Surprise : le Q6_K déborde les 8 Go de la carte, d'où une lecture d'invite sept fois plus lente.

## Verification

**Commands:**
- `python -I tests/epreuve-modele-local/check_harness.py` -- expected: sortie 0

## Review Triage Log

- Traçabilité des empreintes : réel, patché (hashes et dépôts écrits dans `lecture-8-4.md`).
- Lecture trop ferme sur la carte de 8 Go : réel, patché (formulation sans verdict, journal non conservé dit).
- Pas de mise en garde sur le Q6_K dans le tableau généré : réel, patché (`run_models.py` signale une lecture plus de trois fois plus lente).
- `--catalog=x` contourne le garde-fou : réel, bas, patché (comparaison sur la partie avant `=`) ; le même défaut subsiste dans `run_forms.py` (8.5), non touché.
- Lecture « rapporte trois phrases » contre sa propre prudence : patché (reformulé).
- Comparaison à la seule première ligne : rejeté, c'est la référence par construction.
- Échantillonnage de VRAM grossier : noté dans les limites, sans correction.
- Fils « 8 » contre 16 logiques : noté dans les limites.
- Virgule ou point décimal : rejeté, cosmétique.
- Fichiers hors périmètre dans l'arbre (`skills-lock.json`, `.claude/`, `deferred-work.md`) : non commités avec 8.4.
- Plan non mis à jour : patché.
