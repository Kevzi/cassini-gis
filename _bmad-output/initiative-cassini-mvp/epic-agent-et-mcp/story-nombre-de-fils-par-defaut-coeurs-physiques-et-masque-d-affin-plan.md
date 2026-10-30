---
title: 'Nombre de fils par défaut : cœurs physiques et masque d''affinité, sous Windows'
type: 'bugfix'
ticket: '20'
created: '2026-10-30'
status: 'built'
baseline_revision: 'c67a5933c8df522795772b1aa2b6ad4f5f6c8aba'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le défaut du nombre de fils du modèle local (`default_threads`) vaut `min(cœurs physiques de la machine, available_parallelism)`. Sous Windows, ni l'un ni l'autre ne voit le masque d'affinité : sous `0x55` (4 cœurs physiques autorisés sur le 5800X), le noyau prend 8 fils au lieu de 4 et se marche dessus ; sous Linux, un masque qui ne laisse que les deux fils d'un cœur donne 2 au lieu de 1.

**Approach:** Compter les cœurs physiques dont au moins un processeur logique est autorisé au processus, sous Windows et sous Linux, par un calcul pur testé sans modèle et une lecture par plateforme ; repli sur la formule actuelle si la lecture échoue. Puis mesurer sur le 5800X : défaut contre `CASSINI_THREADS` au même nombre, et cœurs physiques contre fils logiques pour l'écriture du plan.

**Décision (2026-10-30, propriétaire) :** sous Windows, la lecture passe par `windows-sys` (Microsoft, MIT OR Apache-2.0, déjà dépendance transitive), ajoutée à `cassini-ai` pour Windows seulement ; sous Linux, lecture de `/proc` et `/sys`, sans dépendance.

## Boundaries & Constraints

**Always:** `CASSINI_THREADS` garde la priorité sur le défaut ; résultat au moins 1, jamais plus que `available_parallelism` (quotas) ; toute erreur de lecture retombe sur la formule actuelle, sans panique ni message à l'utilisateur ; mesures de durée sur CPU sans CUDA, mode opératoire de 5.19/5.22 (PowerShell depuis WSL, `CARGO_TARGET_DIR=C:\ct`, modèle livré, noyau installé dans `%LOCALAPPDATA%\Cassini\bin`).

**Never:** dépendance ajoutée sans l'accord du propriétaire (licence MIT, Apache-2.0 ou BSD) ; changer le défaut pour les fils logiques sans décision du propriétaire sur chiffres ; toucher au prompt, au modèle ou à l'inventaire ; machines à plus de 64 processeurs logiques (groupes de processeurs Windows) : repli sur la formule actuelle.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Sans masque | 5800X, 8 cœurs, 16 fils | 8 | — |
| Masque un fil par cœur | `0x55` | 4 | — |
| Masque deux fils d'un cœur | `0x3` | 1 | — |
| Masque à cheval | `0x7` | 2 | — |
| Machine sans SMT | 4 cœurs, 4 fils, sans masque | 4 | — |
| Lecture impossible | API ou fichiers absents | formule actuelle | repli silencieux |

</frozen-after-approval>

## Code Map

- `ai/src/lib.rs:277` `default_threads` -- seul point à changer : y brancher le comptage ; garder `min(available_parallelism)` et le plancher 1. Test existant `:976` à réécrire (il recopie la formule actuelle).
- `ai/src/lib.rs:624` -- seul appelant : `threads.unwrap_or_else(default_threads)` ; rien à changer.
- `cli/src/main.rs:302-316` `measured_generator` -- doc de `CASSINI_THREADS` qui dit que le défaut ne voit pas le masque : à corriger. Le comportement reste.
- Calcul pur (nouveau module `ai/src/threads.rs` ou dans `lib.rs`) : `cores_allowed(cores: &[u64], allowed: u64) -> usize`, un masque de processeurs logiques par cœur physique ; le compte est celui des cœurs dont le masque croise `allowed`.
- Windows : `GetProcessAffinityMask(GetCurrentProcess())` et `GetLogicalProcessorInformation` (entrées `RelationProcessorCore`, champ `ProcessorMask`), dans `kernel32`, toujours lié. `num_cpus 1.17` les appelle déjà sans le masque.
- Linux : `/proc/self/status` (`Cpus_allowed_list`) et `/sys/devices/system/cpu/cpuN/topology/{physical_package_id,core_id}` ; pas de code `unsafe`, pas de dépendance.
- Lockfile : `windows-sys 0.61.2` et `0.45.0` y sont déjà en dépendances transitives (MIT OR Apache-2.0) ; aucune n'est dépendance directe de `cassini-ai`. Aucun `forbid(unsafe_code)` dans le dépôt.
- Mesure : sonde de 5.22 `C:\Users\kevin\AppData\Local\Temp\sonde-5-22\` (`sonde.py`, `run.ps1`), à copier dans `sonde-5-20\` ; `docs/mesure-duree-par-phrase.md` pour consigner.

## Tasks & Acceptance

**Execution:**
- [x] `ai/src/lib.rs` (ou `ai/src/threads.rs`) -- calcul pur `cores_allowed` et tests de la matrice ; lecture Windows et Linux ; `default_threads` branché, repli sur la formule actuelle
- [x] `ai/Cargo.toml` -- `windows-sys` en dépendance `[target.'cfg(windows)'.dependencies]`, avec le commentaire de décision comme pour `num_cpus`
- [x] `cli/src/main.rs` -- doc de `CASSINI_THREADS` mise à jour
- [x] sonde jetable `sonde-5-20` -- sur le 5800X, nombre de fils pris sans réglage, sans masque et sous `0x55` (ligne `cassini-timings`, à compléter du nombre de fils) ; phrase de démo sous `0x55` sans réglage et avec `CASSINI_THREADS=4` ; sans masque, 8 contre 16 fils
- [x] `docs/mesure-duree-par-phrase.md` -- fils pris, durées, comparaison physiques et logiques

**Acceptance Criteria:**
- Given le 5800X, when le noyau se lance sans `CASSINI_THREADS`, then il prend 8 fils sans masque et 4 sous `0x55`.
- Given le 5800X sous `0x55`, sans CUDA, when la phrase de démo est composée sans réglage puis avec `CASSINI_THREADS=4`, then la première n'est pas plus longue que la seconde (à l'écart de mesure près), durées consignées.
- Given le 5800X sans masque, when la démo est composée avec 8 puis 16 fils, then les deux durées sont consignées, et si 16 est plus rapide, le choix est soumis au propriétaire.
- Given `cargo test`, `clippy`, `fmt`, `deny`, when ils tournent, then ils sont verts sous Windows et sous Linux.

## Implementation Notes

- Nouveau module `ai/src/threads.rs` : `default_threads` (déplacé de `lib.rs`, réexporté), calcul pur
  `cores_allowed`, lecture Linux (`/proc/self/status`, `/sys/.../online` et `topology`, sans `unsafe`)
  et Windows (`GetActiveProcessorGroupCount` = 1, sinon repli ; `GetProcessAffinityMask`,
  `GetLogicalProcessorInformation`), analyse des listes de processeurs (`parse_cpu_list`, repli
  au-delà du 64e). Lecture rendant 0 ou `None` : ancienne formule. Test machine-dépendant réduit
  aux bornes (Design Notes).
- `cassini-timings` porte en plus `fils=<n>` (fils effectivement passés au moteur), demandé par la
  tâche de sonde.
- WSL n'a pas libclang : `cargo test` du dépôt ne se construit pas sous Linux (llama-cpp-sys). La
  partie Linux a été vérifiée par une caisse jetable incluant `threads.rs` seul (tests, clippy
  `-D warnings`, `taskset` 0x3/0x55/0x7/sans masque : 1/4/2/8). Suite complète verte sous
  PowerShell (test, clippy, fmt, deny).
- Constat : la sonde de 5.19/5.22 ne posait pas le masque (`ctypes` sans `argtypes`, pseudo-handle
  tronqué, échec silencieux) ; leurs mesures « 0x55 » étaient sans masque à 4 fils. Consigné dans
  `docs/mesure-duree-par-phrase.md`.
- QGIS du propriétaire ouvert : le noyau installé était verrouillé et le journal du projet de test
  tenu ; la sonde a lancé `C:\ct\release\cassini.exe` copié dans `sonde-5-20\` et une copie du
  projet (supprimée après, avec son journal). Le noyau de `%LOCALAPPDATA%\Cassini\bin` n'a pas été
  remplacé.
- Mesures : sous 0x55, défaut (4 fils) = `CASSINI_THREADS=4` (150,3 / 149,5 s, phrases 2-10) ; sans
  masque, 8 fils (129,7 s) battent 16 (157,0 s) : pas de décision à soumettre.

## Plan Change Log

## Review Triage Log

Passe 1 (2026-10-30, lentille quick). Avant la relecture, l'audit de la matrice a ajouté `pick_threads` et le test de la ligne « lecture impossible » (repli), seule ligne sans test.

| # | Constat | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 1 | Critère 4 « verts sous Linux » : l'espace de travail complet ne se construit pas sous WSL (libclang absent pour llama-cpp-sys) ; seul `threads.rs` y a été testé, dans une caisse jetable | medium | defer | Réel, mais limite d'environnement antérieure (5.19 et 5.22 ne vérifiaient que sous PowerShell) ; le code propre à Linux est testé (`taskset` : 8, 4, 1, 2). Entrée dans `deferred-work.md`. |
| 2 | Mesure faite avec `C:\ct\release\cassini.exe` copié, et non le noyau installé dans `%LOCALAPPDATA%` | low | rejet | Même construction, même modèle ; QGIS du propriétaire verrouillait l'exe. Écart consigné dans la doc ; installation à faire une fois QGIS fermé. |
| 3 | `fils=` de `cassini-timings` recalcule `default_threads()` après la génération au lieu du nombre passé au moteur | low | patch | Réel (`cli/src/main.rs`, `measured_generator`) : calculer `n` une fois, le passer en `Some(n)` et l'afficher. |
| 4 | Linux : regrouper par `(physical_package_id, core_id)` peut fondre deux cœurs (puces à plusieurs dies, arm64 à -1), compte trop bas sans repli | low | patch | `core_id` dépend de la plateforme (doc du noyau) ; regrouper par `thread_siblings_list`, plus simple et exact. |
| 5 | Fils pris sous `0x3` et `0x7` lus par un petit programme non conservé, et non dans `cassini-timings` | low | rejet | Le critère ne porte que sur 8 et 4, lus dans `cassini-timings` ; le petit programme appelle la même fonction. |

## Design Notes

Le comptage pur évite d'avoir un test qui dépend de la machine : la matrice se teste sur des masques fabriqués (5800X : cœur k = bits `2k` et `2k+1`). Seule la lecture par plateforme dépend de la machine ; son test se contente de vérifier, sur la machine qui lance les tests, que `default_threads` est entre 1 et `available_parallelism`.

## Verification

**Commands:**
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `cargo deny check` (PowerShell et WSL) -- expected: verts.

**Manual checks (if no CLI):**
- Sur le 5800X : fils pris sans masque (8) et sous `0x55` (4), lus dans la sortie de mesure.
