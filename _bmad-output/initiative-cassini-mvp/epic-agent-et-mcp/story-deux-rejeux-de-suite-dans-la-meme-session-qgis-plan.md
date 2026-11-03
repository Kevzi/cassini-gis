---
title: 'Deux rejeux de suite dans la même session QGIS'
type: 'bugfix'
ticket: '35'
created: '2026-11-03'
status: done
baseline_revision: '172f820df304388e587da3b72a1925a7d702c567'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Rejouer deux fois un journal sans rouvrir le projet QGIS échoue : le second rejeu rend `replay.diverged` à l'étape 1. `Replay::run` vide `replay/` par `let _ = fs::remove_dir_all(...)` et ignore l'échec ; sous Windows, QGIS tient encore ouverts les fichiers du premier rejeu, la suppression échoue, et GDAL refuse ensuite d'écrire par-dessus. Un fichier périmé non vide pourrait aussi passer pour la sortie d'une étape QGIS.

**Approach:** Le rejeu écrit toujours dans un dossier qu'il vient de vider ou de créer, jamais dans un dossier où un fichier a survécu. Si `replay/` ne se vide pas entièrement, le rejeu prend le premier dossier de repli `replay-<n>/` (n = 2, 3, …) absent ou vidable ; s'il n'en trouve aucun, il refuse avec une erreur qui nomme un fichier qui a résisté.

**Decision (2026-11-03, propriétaire) :** remède = dossier de repli `replay-<n>/` ; écartés : refus fichier par fichier (second rejeu en erreur sous QGIS), déchargement par l'extension (logique métier dans l'extension).

**Decision (2026-11-03, propriétaire, après la recette QGIS) :** 5.35 se clôt sur la correction des fichiers tenus (second rejeu sous `replay-2/`, étapes 1 à 4 identiques sous QGIS). L'écart de l'étape 5 (`layout.export_pdf`, couches ajoutées au projet par le rejeu précédent) part dans un ticket à part, qui reprend le critère « deux rejeux de suite rendent deux fois `identical: true` ».

## Boundaries & Constraints

**Always:** cas courant inchangé — quand `replay/` se vide, les sorties restent sous `replay/<étape>-<id>.<ext>` (tests du noyau et sonde `probe_end_to_end.py` inchangés) ; les chemins de résultat restent relativisés contre le dossier réellement utilisé, donc le remplacement des chemins `replay/` de 5.31 continue de marcher ; le journal n'est jamais écrit par le rejeu ; le nom de fichier apparaît dans le message d'erreur sous une forme relative au dossier de travail, jamais en chemin absolu (AD-9).

**Never:** rien côté extension (pas de déchargement des couches du rejeu précédent) ; pas de nouvelle dépendance ; ne pas toucher au verrou ni aux rejeux simultanés (entrée F14 de deferred-work, hors périmètre) ; ne jamais supprimer un dossier `replay-<n>/` autre que celui qu'on s'apprête à utiliser.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Premier rejeu | pas de `replay/` | sorties sous `replay/`, `identical: true` | — |
| Rejeu après rejeu, rien de tenu | `replay/` plein, supprimable | `replay/` vidé puis réécrit, `identical: true` | — |
| Second rejeu, fichiers tenus | un fichier de `replay/` non supprimable | sorties sous `replay-2/`, `identical: true`, le fichier périmé jamais lu | — |
| Troisième rejeu, tout tenu | `replay/` et `replay-2/` non vidables | sorties sous `replay-3/` | — |
| Aucun dossier possible | `replay/` et les 20 replis non vidables ou non créables | aucune étape exécutée | erreur `replay.output_locked` nommant un fichier resté (ex. `replay/1-analysis.buffer.gpkg`) |

</frozen-after-approval>

## Code Map

- `core/src/journal/mod.rs:634-643` -- `Replay::run` : remplacer `let _ = fs::remove_dir_all(&self.replay_dir)` + `create_dir_all` par le choix d'un dossier propre ; le `Resolver.out_dir`, `relativize(value, …)` (l.679) et la lecture des sorties (l.699) doivent utiliser ce dossier choisi, pas `self.replay_dir`.
- `core/src/journal/mod.rs:514-588` -- struct `Replay` et `load` : `replay_dir` est la racine `work_dir/replay` ; garder le champ comme premier candidat.
- `core/src/journal/mod.rs:55-122` -- `JournalError` : ajouter la variante `OutputLocked(String)` (chemin relatif au dossier de travail, POSIX), code `replay.output_locked`, message français qui nomme le fichier.
- `core/src/journal/paths.rs` -- `posix`, `relative_to` à réutiliser pour nommer le fichier.
- `core/src/error.rs` -- vérifier que `ErrorReport::from(JournalError)` reprend code et message sans liste à mettre à jour ; sinon l'ajouter.
- `core/src/journal/tests.rs:270-310` -- modèle de test de rejeu (`temp_project`, `replay_of`, `test.write_output`, `test.read_file`).
- Ne pas changer : `output_file`, `same_result`, la règle « sortie QGIS non vide ».

## Tasks & Acceptance

**Execution:**
- [x] `core/src/journal/mod.rs` -- ajouter `fn fresh_replay_dir(&self) -> Result<PathBuf, ErrorReport>` : candidats `replay`, puis `replay-2` … `replay-21` ; pour chacun, `remove_dir_all` (NotFound = ok), puis `create_dir_all`, puis contrôler que le dossier est vide ; le premier qui passe est rendu. En cas d'échec partout, parcourir `replay/` pour trouver un fichier restant et rendre `OutputLocked` le nommant (relatif au dossier de travail) ; si aucun fichier n'est trouvé, `Io`. Brancher `run` dessus et remplacer `self.replay_dir` par le dossier choisi dans `run`. -- un fichier périmé n'est jamais dans le dossier de sortie.
- [x] `core/src/journal/mod.rs` -- variante `JournalError::OutputLocked`, code et `Display`. -- erreur qui nomme le fichier.
- [x] `core/src/journal/tests.rs` -- tests `#[cfg(unix)]` (droits de dossier pour simuler le fichier tenu) : (a) `replay/` contient un fichier périmé au contenu différent et est rendu non modifiable (0o555) → rejeu `identical: true`, sorties sous `replay-2/`, fichier périmé intact et non lu ; (b) deux rejeux de suite sans blocage → les deux `identical: true`, sorties sous `replay/`, pas de `replay-2/` ; (c) dossier de travail rendu non modifiable avec `replay/` bloqué → erreur `replay.output_locked` dont le message nomme `replay/<fichier>` et ne contient pas le chemin absolu. Rendre les droits avant le nettoyage. -- couvre la matrice.

**Acceptance Criteria:**
- Given un fichier de `replay/` impossible à supprimer, when le journal est rejoué, then le rejeu rend `identical: true` sans lire ce fichier, ou une erreur `replay.output_locked` qui le nomme — jamais `replay.diverged`.
- Given le journal de démo dans une session QGIS ouverte (noyau et extension installés), when Claude lance deux rejeux de suite sans rouvrir le projet avec `probe_replay_5_9.py`, then les deux rendent `identical: true`.

## Implementation Notes

- `Replay::fresh_replay_dir` + `empty_dir` + `first_file` dans `core/src/journal/mod.rs` ; plafond `REPLAY_FALLBACKS = 20`. `run` n'utilise plus `self.replay_dir` que comme premier candidat.
- `JournalError::OutputLocked` sans étape (`step: None`) : aucune étape n'a commencé ; `ErrorReport::from` le reprend sans liste à tenir.
- Vérifié : `cargo test -p cassini-core journal` (WSL, 46 verts dont les 3 nouveaux, `#[cfg(unix)]`) ; `cargo clippy --workspace --all-targets -D warnings` et `cargo test --workspace` verts sous Windows (`cargo.exe`, `LIBCLANG_PATH`, `CARGO_TARGET_DIR=C:\ct`), le WSL n'ayant ni pkg-config ni libclang. Recette QGIS (deux `probe_replay_5_9.py`) non faite.

- Recette QGIS (2026-11-03, noyau `--features gpu` installé, `rejeu.bat` de `mesure-5-9`) : rejeu 1 `identical: true` ; rejeu 2 écrit sous `replay-2/` et passe les étapes 1 à 4 (l'échec GDAL « already exists » de l'étape 1 a disparu), puis `replay.diverged` à l'étape 5 `layout.export_pdf`. Cause probable : le geste rend `layer_count` = couches visibles du projet (`qgis-plugin/cassini_qgis/layout_export.py:192-198`) ; les couches ajoutées par le rejeu 1 restent dans le projet ouvert, le rejeu 2 exporte donc plus de couches que l'original. Défaut distinct du fichier tenu, masqué jusque-là par l'échec à l'étape 1. AC 2 non atteinte ; question posée au propriétaire.

## Plan Change Log

## Review Triage Log

**Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment)** — high 0, medium 2, low 9, false 1, maybe-false 0.

| # | Source | Verdict | Route | Constat et preuve |
|---|---|---|---|---|
| 1 | intent, blind, vgap | medium | patch | `work_files` (`core/src/serve/plans.rs:215`) ne lit que `replay/` : une sortie sous `replay-2/` d'un nom absent ailleurs passe pour une couche utilisateur dans l'inventaire. Correctif : lire aussi `replay-<n>/`. |
| 2 | vgap | medium | patch | Aucun test ne rejoue un journal QGIS (résultats portant des chemins, 5.31) en repli ; remettre `relativize(.., replay_dir)` ne casserait aucun test. Correctif : test `qgis_journal` avec `replay/` tenu. |
| 3 | edge, blind | low | patch | `first_file` suit les liens symboliques (`is_dir`) et peut nommer un dossier comme « fichier ». Correction directe : `file_type()`, `None` sans fichier. |
| 4 | blind | low | patch | Commentaires « sorties sous `replay/` » périmés (`journal/mod.rs`, `agent_inventory.rs`, `plans.rs`). Correction directe. |
| 5 | blind, edge | low | defer | Les `replay-<n>/` ne sont jamais nettoyés une fois `replay/` libéré : accumulation disque, au plus un dossier par rejeu pendant que QGIS tient les précédents. |
| 6 | blind, vgap | low | defer | `ReplayReport` ne dit pas le dossier utilisé ; la sonde de bout en bout lit `replay/` en dur. Ajout de surface publique, hors intention. |
| 7 | vgap | low | defer | Sortie `Io` de `fresh_replay_dir` (tous candidats en échec, `replay/` vide) non testée. |
| 8 | edge | low | rejet | Deux rejeux simultanés : défaut antérieur, déjà au deferred-work (F14 de la rétro de l'epic 2). |
| 9 | edge, blind | low | rejet | `OutputLocked` dit « tenu (par QGIS ?) » quand la cause est un droit ou un disque plein : rare, le message pose une question ; corriger demande une branche de plus. |
| 10 | blind | low | rejet | `remove_dir_all` vide en partie un candidat écarté : ses fichiers non tenus ne sont chargés par aucune couche (un fichier chargé est tenu) ; comportement déjà présent avant 5.35. |
| 11 | edge, blind | low | rejet | Tests en root ou sans garde `Drop` des droits : la recette ne tourne pas en root ; hygiène de test sans défaut constaté. |
| 12 | blind | low | rejet | `last_error` ne garde que l'erreur du dernier candidat : sans effet visible hors cas déjà rare. |
| 13 | blind | low | rejet | Plafond de 20 replis non testé : constante simple, le test (c) couvre la sortie d'erreur. |
| 14 | intent, vgap | false | — | « Windows non prouvé » : ce n'est pas un défaut du code ; la recette QGIS sous Windows est l'AC 2, faite à l'étape suivante. |

## Design Notes

Dossier de repli plutôt que suppression fichier par fichier : la suppression avec refus donnerait, sous QGIS, une erreur au second rejeu au lieu du succès demandé ; le déchargement des couches par l'extension mettrait une décision dans l'extension (AD : aucune logique métier côté extension). Le repli garde le cas courant identique au comportement actuel. Les dossiers de repli s'accumulent au plus d'un par rejeu tant que QGIS tient les précédents ; ils sont réutilisés dès qu'ils se vident. Plafond de 20 replis pour ne jamais boucler sans fin.

## Verification

**Commands:**
- `cargo test -p cassini-core journal` -- expected: tous verts, dont les trois nouveaux.
- `cargo test --workspace` -- expected: vert.
- `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` -- expected: aucun avertissement.

**Manual checks (if no CLI):**
- Recette QGIS : installer noyau et extension, ouvrir le projet de démo, lancer deux fois de suite `probe_replay_5_9.py` (hors dépôt, `../donnees-test-cassini/mesure-5-9/`) sans rouvrir le projet ; attendu : deux `identical: true`, le second sous `replay-2/`.
