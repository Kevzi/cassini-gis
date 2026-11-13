---
title: 'Porte 2 sous Windows : un seul fichier installable réseau coupé'
type: 'feature'
ticket: '2'
created: '2026-11-13'
status: 'ready-for-dev'
baseline_revision: '0f7245c7a6eb0958f23edd512cf5a33cda06d0bd'
route: 'full'
route_source: 'auto'
risk: 'high'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-publication/epic-publication.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-publication/story-balle-tracante-etiquette-release-et-installeur-en-ligne-mini-plan.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** sur un poste isolé du réseau, 7.1 ne sert à rien : son installeur télécharge. Il faut un seul fichier qui contient le noyau, le modèle et l'extension.

**Approach:** la recette construit l'archive de l'extension (sans binaire, moins de 25 Mo) et un installeur complet Inno Setup qui embarque `cassini.exe`, le modèle GGUF et cette archive, sans aucun accès réseau à l'installation.

## Boundaries & Constraints

**Always:** la limite de taille du format est vérifiée d'abord, dans la documentation officielle d'Inno Setup : un seul fichier jusqu'à 4,2 Go compressés (au-delà, `DiskSpanning` obligatoire), donc le noyau (≈12 Mo) et le modèle (≈2,78 Go) tiennent ; le build échoue si la taille approche cette limite (repli zip et `install.cmd`) ; l'installeur ne contient ni n'appelle aucun code réseau ; même emplacement que 7.1 (`%LOCALAPPDATA%\Cassini\bin`, modèle dans `bin\models\`, ordre de recherche du dossier des modèles) ; `installation.json` écrit comme en 7.1 ; l'archive de l'extension ne contient aucun binaire et pèse moins de 25 Mo ; le modèle n'est jamais publié dans ce dépôt ni construit sur le poste du propriétaire. Décision du propriétaire (2026-11-13) : la recette télécharge le modèle depuis le dépôt Hugging Face `Kevzi/cassini-composition-4b-GGUF` (publié par 7.3, qui passe donc avant la construction de cette entrée), à une adresse et sous un SHA-256 fixés dans le dépôt, et échoue si l'empreinte diffère ; le fichier produit (≈2,8 Go, au-delà des 2 Gio d'une release GitHub) est livré comme artefact du workflow, que le propriétaire télécharge depuis l'onglet Actions pour son essai en VM.

**Never:** déclaration `*.modele.json` et empreintes du modèle (7.4) ; hébergement sur Hugging Face (7.4) ; noyau gpu (7.6) ; `metadata.txt`, icône, LICENSE de l'archive (7.10) ; signature de code.

## I/O & Edge-Case Matrix

| Scénario | État | Comportement | Erreur |
|----------|------|--------------|--------|
| Installation | machine sans réseau | noyau et modèle posés, archive d'extension déposée, `installation.json` écrit | — |
| Archive d'extension trop lourde | ≥ 25 Mo ou binaire dedans | le build échoue | message en français |
| Fichier trop gros | taille compressée > seuil | le build échoue | message en français |
| Modèle introuvable ou altéré | téléchargement impossible ou SHA-256 différent de celui fixé | le build échoue | message en français |

</frozen-after-approval>

## Code Map

- `installer/cassini-en-ligne.iss` -- modèle de l'installeur : mêmes emplacements, `installation.json`, `DateIsoUtc` ; son `UninstallDelete` supprime tout `{app}\bin` : à restreindre au fichier `installation.json`, les fichiers installés se désinstallent seuls.
- `installer/manifeste.py`, `installer/verifier_etiquette.py` -- réutilisables ; `.github/workflows/release.yml` -- job Windows (déclenchement manuel et étiquette), à étendre sans casser 7.1.
- `docs/dossier-des-modeles.md` -- le noyau cherche `CASSINI_MODELS`, puis `models\` à côté de `cassini.exe` ; `qgis-plugin/cassini_qgis` + `qgis-plugin/tests/check_extension.py` -- source de l'archive et garde statique.
- Documentation Inno Setup vérifiée : `DiskSpanning=no` jusqu'à 4,2 Go compressés ; un GGUF ne se comprime presque pas, `Compression=none` pour ce fichier.
- À créer : `installer/cassini-hors-ligne.iss`, `installer/archive_extension.py` (+ tests).

## Tasks & Acceptance

**Execution:**
- [ ] `installer/archive_extension.py`, tests -- zip de l'extension, refuse un binaire ou ≥ 25 Mo
- [ ] `installer/cassini-hors-ligne.iss` -- noyau, modèle, archive d'extension, `installation.json`, aucune fonction réseau
- [ ] `.github/workflows/release.yml` -- construit l'archive et l'installeur complet, vérifie la taille, preuve d'installation silencieuse réseau coupé sur le poste de GitHub
- [ ] `docs/publier-une-version.md` -- étapes de l'essai en VM

**Acceptance Criteria:**
- Given un poste sans réseau, when l'installeur complet s'exécute, then le noyau répond, le modèle est listé par `model.list`, l'archive d'extension est déposée.
- Given un modèle absent ou une archive trop lourde, when la recette construit, then elle échoue avant toute publication.

## Implementation Notes

## Plan Change Log

## Review Triage Log

## Design Notes

## Verification

**Commands:**
- `python -I -m unittest discover -s installer/tests` -- expected: OK

**Manual checks (if no CLI):**
- Étape du propriétaire : VM Windows propre, réseau coupé, installer le fichier, vérifier le noyau et le modèle, installer l'archive dans QGIS, rejouer la démo.
