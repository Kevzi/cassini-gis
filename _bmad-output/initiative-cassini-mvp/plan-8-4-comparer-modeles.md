---
title: 'Comparer modeles et quantifications'
type: 'chore'
ticket: '4'
created: '2026-10-15'
status: 'ready-for-dev'
baseline_revision: '3031462'
route: 'oneshot'
route_source: 'pinned'
risk: 'medium'
review: ''
review_source: ''
lenses_ran: []
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
- [ ] Télécharger et contrôler les deux GGUF Ministral (licence lue sur la fiche).
- [ ] `harness.py` -- gabarit `mistral3`, mémoire au pic, processeur et fils.
- [ ] Essai de contrôle du gabarit sur une phrase simple, avec examen des jetons d'invite.
- [ ] `run_models.py` et `check_harness.py` -- comparaison, contrôlée en rejeu.
- [ ] Lancer les trois mesures, commiter `resultats-8-4.*` et `lecture-8-4.md`.

**Acceptance Criteria:**
- Given `resultats-8-4.md`, when on le lit, then il donne pour chacune des trois combinaisons : licence, taux d'emblée et après relance, mémoire vive et vidéo au pic, temps de chargement, de lecture et de génération, processeur et nombre de fils, sur les mêmes 25 phrases.
- Given les fichiers téléchargés, when on compare leur empreinte SHA-256 à celle publiée, then elles sont égales.

## Implementation Notes

Route oneshot épinglée : aucun sous-agent de codage autorisé. Les trois mesures durent environ 3 minutes chacune et sont lancées en arrière-plan.

## Verification

**Commands:**
- `python -I tests/epreuve-modele-local/check_harness.py` -- expected: sortie 0
