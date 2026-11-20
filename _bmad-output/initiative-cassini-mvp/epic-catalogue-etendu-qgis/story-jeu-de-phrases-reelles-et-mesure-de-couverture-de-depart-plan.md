---
title: 'Jeu de phrases réelles et mesure de couverture de départ'
type: 'feature'
ticket: '8'
created: '2026-11-20'
status: 'built'
baseline_revision: 'df2081217d1a0332707c8df9577df0bd6965d9c1'
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

**Problem:** 7.11 a livré les 44 phrases, le harnais et une mesure de couverture écrite à la main, mais aucun rapport ne consigne la mesure de départ de l'epic 10 : le taux de plans conformes de l'épreuve sur le catalogue d'avant l'epic manque, et la garde anti-fuite ignore le nouveau jeu.

**Approach:** Faire du rapport de 7.11 le rapport de départ (couverture phrase par phrase, taux de plans conformes, constats, ordre recommandé des primitives) ; le harnais écrit lui-même la ligne du taux ; le jeu entre dans la garde anti-fuite de l'entraînement.

## Boundaries & Constraints

**Always:** Hors ligne ; le noyau et le modèle installé (`continu-5-61b`) tels quels, aucun ré-entraînement (groupé après le dernier ticket de l'epic) ; le taux de départ de référence est celui de `plan_reel` consigné le 2026-11-16 (44/48, barre 38/48), relu sur le modèle installé et consigné à côté s'il diffère ; chaque phrase garde sa source ; les quatre phrases en notation « E/N » (CV14, CV18, CV28, CV29) restent telles quelles et sont consignées comme constat pour l'entrée 17, non reformulées ; l'ordre recommandé des primitives est une proposition dans le rapport, l'accord du propriétaire (étape à personne humaine de l'entrée) lui reste dû.

**Never:** Modifier `after` ou l'ordre dans `tickets.toml` ; toucher `qgis-plugin/`, `core/src/commands/`, `training/src/families.rs`, `ai/tests/par_commande/phrases.json`, `ai/tests/plan_reel.rs` (en cours de modification par 10.12, on ne fait que le lancer) ; mettre à jour CV26 avant la livraison de 10.12 ; gonfler la couverture en comptant en « question justifiée » une phrase qui donne ses coordonnées.

## I/O & Edge-Case Matrix

| Scénario | Entrée / État | Résultat attendu | Erreur |
|---|---|---|---|
| Sans modèle | `cargo test -p cassini-ai --test couverture_reelle_reel` | tests hors ligne verts, couverture 33/44 recalculée | aucune |
| Avec modèle | test ignoré, `CASSINI_TEST_MODEL` et `CASSINI_COUVERTURE_RAPPORT` donnés | rapport écrit : couverture, colonne agent, ligne `plans conformes n/48` | modèle absent : test ignoré, pas d'échec |
| Fuite | une phrase du jeu réel présente dans l'entraînement | `recouvrement.rs` échoue | échec nommant la phrase |
| Jeu modifié | nombre de phrases différent du compteur déclaré | `barres.rs` échoue | échec explicite |

</frozen-after-approval>

## Code Map

- `ai/tests/couverture_reelle_reel.rs` -- `report()` l.119-196 : ajouter la ligne du taux de plans conformes ; test ignoré l.321 : lancer aussi les phrases de l'épreuve (logique de `plan_reel`) ou relire sa ligne `SCORE` ; `write_report` l.198
- `docs/couverture-phrases-reelles.md` -- devient « mesure de départ » : couverture 33/44, colonne agent, taux de plans conformes (44/48 du 2026-11-16, et la relecture), constats (notation E/N, confusions de commandes voisines, CV26), verdict sur l'objectif de 70 % (déjà dépassé), ordre recommandé des primitives
- `training/tests/recouvrement.rs` (en-tête l.1-16, tests l.47-142, ~114) et `training/src/barres.rs` (table l.55, `real_trials()` ~185) -- ajouter `ai/tests/couverture_reelle/phrases.json` à la garde anti-fuite ; vérifier que son schéma `id/phrase` est accepté
- `ai/tests/plan_reel.rs` -- ne pas modifier, seulement lancer (`SCORE {right}/{total} plans conformes`)
- `_bmad-output/initiative-cassini-mvp/epic-catalogue-etendu-qgis/epic-catalogue-etendu-qgis.md` l.60, 64 -- décisions sur la mesure de départ et l'ordre : lire, ne pas modifier
- Mesure avec modèle : `CARGO_TARGET_DIR=C:\ct-5-57g`, `VULKAN_SDK=C:\VulkanSDK\1.4.363.0`, `LIBCLANG_PATH=C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native`, `CMAKE_GENERATOR=Ninja` (ninja dans PATH, jamais le générateur Visual Studio), modèle dans `%LOCALAPPDATA%\Cassini\bin\models` ; vérifier le nom de la fonctionnalité (`cuda` ou `gpu`) dans les `Cargo.toml` ; ne pas toucher `C:\ctd`

## Tasks & Acceptance

**Execution:**
- [ ] `ai/tests/couverture_reelle_reel.rs` -- la ligne du taux de plans conformes dans le rapport écrit, avec son test hors ligne -- le harnais produit lui-même la mesure
- [ ] `training/tests/recouvrement.rs`, `training/src/barres.rs` -- jeu réel dans la garde anti-fuite et le compteur -- aucune phrase réelle dans l'entraînement
- [ ] Mesure avec modèle (couverture et `plan_reel`), une seule fois, après la livraison de 10.12 si elle a lieu avant la fin, sinon avec ses fichiers dans l'état courant et la mention de la date -- chiffres de départ
- [ ] `docs/couverture-phrases-reelles.md` -- rapport de départ complet, ordre recommandé des primitives restantes d'après la couverture -- ce qui sert à réordonner

**Acceptance Criteria:**
- Given le jeu de 44 phrases, when le harnais tourne hors ligne, then la couverture 33/44 et chaque commande manquante sont écrites phrase par phrase.
- Given le modèle installé, when la mesure est lancée, then le rapport donne le taux de plans conformes de départ et sa date.
- Given une phrase du jeu réel copiée dans l'entraînement, when la garde tourne, then elle échoue.
- Given le rapport, when le propriétaire le lit, then l'ordre proposé des primitives et le verdict sur les 70 % y sont, et rien dans `tickets.toml` n'a changé.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 7 constats ; 3 correctifs, 1 reporté, 3 rejetés.

| Verdict | Route | Constat et preuve |
|---|---|---|
| low | patch | `docs/couverture-phrases-reelles.md` : un caractère de contrôle 0x08 dans le chemin du modèle (`Cassiniin\models`) ; `grep -c` en trouve une ligne |
| medium | patch | Même doc, ligne 5 : « Le 7.11 avait déjà aligné le jeu » est faux, la reformulation de CV03, CV21, CV22 est de 10.8 ; dire aussi si la colonne « Agent local » est mesurée sur le nouveau texte |
| low | patch | `plans_score_from_env` : sans `CASSINI_PLANS_CONFORMES`, le rapport n'a silencieusement aucune ligne de taux ; écrire « non mesuré » |
| medium | defer | Le 41/48 mélange l'état de 10.12 : S15 et S23 expliquent deux points, PL04, PL08, PL17, PL22 et S24 ne sont pas diagnostiqués ; le plancher reste le 44/48 du 2026-11-16 ; à diagnostiquer avant la ré-épreuve de 10.17 |
| false | rejeté | CV26 mise à jour avant la livraison de 10.12 (Never du plan) et 33/44 attendu : 10.12 est construit et vérifié, CV26 est couverte, 34/44 est le chiffre juste |
| false | rejeté | `barres.rs` 115 → 118 couplé aux phrases de 10.12 : les deux tickets se commitent ensemble dans cet ordre, les fichiers partagés dans le premier commit |
| low | rejeté | Aucun test ne montre que la garde échoue sur une fuite : elle a échoué sur 3 vraies fuites avant reformulation (constat de l'implémenteur), l'ajouter est plus de code que le défaut ne vaut |

## Design Notes

Référence du taux de départ : l'entrée 17 compare le taux final à « au moins celui consigné par l'entrée 8 ». On consigne donc le 44/48 du 2026-11-16, mesuré avant les commits de l'epic 10, comme valeur de référence ; une relecture sur le catalogue courant s'ajoute à côté, sans la remplacer.

Constat des quatre phrases « E/N » : elles tombent sur le garde-fou des coordonnées (une question au lieu d'un plan). Le registre les couvre ; c'est la lecture de la notation qui manque, à traiter avec 10.17 ou un ticket de garde-fou.

## Verification

**Commands:**
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement
- `cargo test -p cassini-ai --test couverture_reelle_reel` et `cargo test -p cassini-training --test recouvrement` -- expected: vert
- Test avec modèle (variables ci-dessus, `-- --ignored --nocapture`) -- expected: rapport écrit avec le taux
