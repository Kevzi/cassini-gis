---
title: 'Empreinte du modèle vérifiée au lancement'
type: 'feature'
ticket: '51'
created: '2026-11-15'
status: 'built'
baseline_revision: 'd16c5bfe1f10cee9224f1f872d3f98034768e567'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-empreinte-du-modele-verifiee/change-empreinte-du-modele-verifiee.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** un GGUF ou un adaptateur abîmé, remplacé ou posé à la main est chargé tel quel : rien ne dit que le modèle qui compose le plan est celui qu'on croit.

**Approach:** la déclaration `*.modele.json` porte `model_sha256` et `adapter_sha256` ; le noyau les vérifie au premier chargement de la session et refuse un fichier non conforme (`models.tampered`) ; les producteurs de déclaration (chaîne d'entraînement, installeur complet) les écrivent.

## Boundaries & Constraints

**Always:** le noyau seul lit la déclaration (AD-15) ; l'erreur nomme le fichier par son seul nom (AD-9) et dit de réinstaller par l'installeur ou de recopier depuis la porte hors ligne ; résultat gardé en mémoire de session, aucun cache sur disque ; déclaration sans empreintes ou GGUF seul acceptés et signalés « empreinte du modèle non déclarée » ; crate `sha2` (MIT OR Apache-2.0, accordée le 2026-11-10) ; les empreintes viennent du seul manifeste des versions.

**Never:** aucun téléchargement de réparation (AD-17) ; ne pas changer la forme du manifeste lu par `cassini-en-ligne.iss` ; pas de signature ni de gestion de clés ; empreinte non obligatoire.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Déclaration juste | empreintes égales aux fichiers | le modèle charge | — |
| Octet changé | GGUF ou adaptateur modifié | rien n'est chargé | `models.tampered`, nom du fichier seul |
| Sans empreintes | déclaration ancienne | charge ; « empreinte du modèle non déclarée » | — |
| GGUF seul | pas de déclaration | charge, même signalement | — |
| Empreinte mal formée | pas 64 hexadécimaux | déclaration refusée | `models.bad_declaration` |

**Decisions (propriétaire, 2026-11-15) :** (1) 5.51 remplit les empreintes de l'installeur complet hors ligne et de la chaîne d'entraînement ; la déclaration de l'installeur en ligne reste à 7.4. (2) La durée du lancement est mesurée sur le poste de la session, machine consignée ; la mesure sur l'i5 de référence reste au propriétaire.

</frozen-after-approval>

## Code Map

- `core/src/commands/model_list.rs` -- `ModelDeclaration` (~l. 48, `deny_unknown_fields`) : ajouter `model_sha256`, `adapter_sha256` (Option, hex validé) ; `read_declaration` (l. 90) ; `diagnose` : champ facultatif d'avertissement dans les entrées `{file,size_bytes,problem}`.
- `ai/src/plan.rs` -- `resolve_model` (~l. 775), seule voie de lecture, partagée par la CLI et les essais réels.
- `ai/src/lib.rs` -- `AiError` (~l. 42) + `code()` : variante `Tampered` → `models.tampered` ; `ModelFiles` (~l. 117) ; `engine()` (~l. 274, `OnceLock`) pour loger le résultat de session.
- `cli/src/main.rs` -- `resolve_model` (l. 421) : conversion en `ErrorReport` ; `scrub` ne laisse que les noms.
- `qgis-plugin/cassini_qgis/models.py`, `panel.py` (~l. 2085) -- affichage de l'avertissement ; texte i18n.
- `training/entrainement/pipeline.py` -- `external_outputs` (l. 187) ajoute les empreintes (`sha256()` l. 877, `adapter_sha256` déjà calculé).
- `installer/cassini-hors-ligne.iss` (l. 98-101) -- déclaration minimale écrite sans empreintes.
- `core/Cargo.toml`, `ai/Cargo.toml`, `deny.toml` -- `sha2` (déjà au `Cargo.lock` via `rmcp`).
- Tests à imiter : `model_list.rs` (l. 529-655), `plan.rs` (~l. 2300), `cli/src/main.rs` (~l. 1195), `qgis-plugin/tests/test_models.py`, `installer/tests/`.
- Ne pas toucher : `installer/manifeste.py` (forme), journal (trace en option, non retenue ici).

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/model_list.rs` -- champs d'empreinte, validation, avertissement -- contrat de la déclaration
- [ ] `ai/src/lib.rs`, `ai/src/plan.rs` -- calcul SHA-256, vérification au premier chargement, mémoire de session, `Tampered` -- cœur de la garantie
- [ ] `cli/src/main.rs` -- `models.tampered` en `ErrorReport` sans chemin
- [ ] `qgis-plugin/cassini_qgis/models.py`, `panel.py` -- message « empreinte du modèle non déclarée »
- [ ] `training/entrainement/pipeline.py`, `installer/cassini-hors-ligne.iss` -- écrire les empreintes
- [ ] tests : un par ligne de la matrice, plus la mémoire de session (un seul calcul par session) ; mesure du lancement consignée

**Acceptance Criteria:**
- Given une déclaration juste, when le modèle se charge, then il charge ; un octet changé dans le GGUF ou l'adaptateur rend `models.tampered` sans charger.
- Given une erreur `models.tampered`, when on lit son message, then il ne contient aucun chemin et ne propose aucun téléchargement.

## Implementation Notes

- Vérification dans `resolve_model` (ai/src/plan.rs) via `verify_fingerprint` (ai/src/lib.rs) ; `sha256_file` et la validation vivent dans `core/src/commands/model_list.rs` (seule crate qui dépend de `sha2`, 0.10). Mémoire de session : liste (chemin, taille, date, empreinte attendue) en `OnceLock<Mutex<..>>`, sans disque.
- Avertissement « empreinte du modèle non déclarée » : champ `warning` facultatif de `DiagnosedModel` (donc de `agent.models`), affiché par l'extension (`models.label`).
- `adapter_sha256` sans `adapter` : déclaration refusée (`models.bad_declaration`).
- Mesure du lancement (2026-11-15, poste de la session, lecture en flux de 2 783 446 944 octets, disque local) : environ 2,0 s de SHA-256 (hashlib/OpenSSL, ordre de grandeur de la crate `sha2`), une fois par session. Mesure sur l'i5 de référence : au propriétaire. Pas de calcul en parallèle retenu.
- Rouge connu, hors ticket : `test_liens_idempotents_sous_le_dossier_des_roues` (symlink refusé sous Windows sans privilège).

- Mesure du lancement, poste de la session (2026-11-15), crate `sha2` 0.10 en release, lecture en blocs de 1 Mio : 2 783 446 944 octets en 1,65 s, une fois par session. Mesure sur l'i5 de référence : au propriétaire (décision 2). Pas de calcul en parallèle du chargement.
- Après relecture : classe de test d'installeur remontée avant `__main__`, nom du fichier temporaire de `sha_of` rendu unique par fil. `cargo fmt`, `clippy -D warnings` et `cargo test -p cassini-core -p cassini-ai -p cassini` verts.

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture quick) — medium 1, low 4 ; corrigés 3, rejetés 6.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Classe `TestEmpreinteDeclaration` après `unittest.main()` | low | patch | confirmé (l. 208/211) ; classe remontée avant le bloc `__main__` |
| `sha_of` : chemin identique pour deux contenus de 11 octets, tests en parallèle | medium | patch | confirmé ; le nom porte maintenant l'identifiant du fil |
| Erreur de lecture rangée sous `models.tampered`, commentaire inexact | low | patch | commentaire corrigé ; refuser sur erreur de lecture reste voulu |
| Mesure du lancement faite avec hashlib, pas la crate `sha2` | medium | patch | remesuré avec `sha2` 0.10 en release : 2 783 446 944 octets en 1,65 s (voir Implementation Notes) |
| Mémoire de session : taille/date seulement | low | rejeté | limite assumée, plan « en mémoire pour la session » |
| Empreinte du `.iss` copiée à la main | false | rejeté | `release.yml` compare le GGUF installé à `modele.sha256` ; le test lie `.iss` et `modele.json` |
| `diagnose` ne signale pas une empreinte fausse | false | rejeté | le plan ne l'exige pas ; hacher à chaque liste coûterait ~2 s |
| `--model` / `plan.propose` sans test `models.tampered` | low | rejeté | même `resolve_model`, déjà testé côté `ai` et `composer.choose` |
| `write_declarations` re-hache à chaque appel | low | rejeté | exécuté une fois par entraînement ; correction plus lourde que le gain |
| `write_declarations` : GGUF absent sans empreinte | low | rejeté | voulu et documenté dans sa docstring |

## Design Notes

Le calcul se fait à la première vérification de la session, par lecture en flux (jamais le fichier entier en mémoire) ; si la mesure gêne, calcul en parallèle du chargement, la première phrase attendant la fin.

## Verification

**Commands:**
- `cargo test -p cassini-core -p cassini-ai` (Windows, voir CLAUDE.md : libclang, `CARGO_TARGET_DIR=C:\ct`) -- expected: vert
- `cargo fmt --check` ; `cargo clippy -p cassini-core -p cassini-ai -p cassini --all-targets -- -D warnings` -- expected: propres
- `cargo deny check licenses` -- expected: `sha2` admis
- tests Python de `qgis-plugin`, `installer`, `training/entrainement` -- expected: verts
