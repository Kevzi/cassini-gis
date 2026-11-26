---
epic: epic-catalogue-etendu-qgis
date: 2026-11-26
reconciled: 2026-11-26 (tickets 10.36 à 10.38 et cycles base-24-1 à base-24-3 ajoutés)
verdict: accepted-with-open-items
criteria: declared
headless: false
---

# Rétrospective — epic 10, catalogue étendu par les algorithmes de traitement de QGIS

## Epic summary

- **Tickets** : 38 lignes dans `tickets.py status` (mise à jour du 2026-11-26) — 31 `built`, 6 `done`, 1 `dropped`; 10.36, 10.37 et 10.38 sont les trois entrées créées le 2026-11-24 pour F1 (commits 75f48a2, 10e2bd0, 24d4504), 10.36 `done`, 10.37 et 10.38 `built`. Version du 2026-11-23 : 35 lignes, 29 `built`, 5 `done`, 1 `dropped` (10.25, reclassement par QGIS, repris par le noyau dans l'epic 11). Aucun ticket inachevé (`pending_tickets` vide) : le verdict n'est donc pas forcé à *rejected*.
- **Tickets restés à `built`** : les 29, dont 10.18 (refactor sweep) et 10.35 (garde-fou). 10.17 et 10.34 sont `done`, 10.19 (livraison) vient d'être marqué `done` le 2026-11-23 (commit f65350c) ; son plan ne contient que le frontmatter.
- **Plages** : 33 plages de commits, de `10a929646` à `HEAD` (coupure au dernier commit de 10.19, `f65350c`), 277 commits d'écart. Chaque plan va de sa baseline à la suivante ; la baseline est l'état à la planification, pas au build, donc l'attribution commit → ticket est grossière (la plage de 10.29, 142 commits, mêle l'entraînement du modèle et d'autres tickets). Plans sans baseline : 10.19 et 10.25 (stubs).
- **Évidence disponible** : epic, `tickets.toml`, 36 plans, diffs et commits, `deferred-work.md`, `docs/entrainement-du-modele.md`, `docs/couverture-phrases-reelles.md`, mémoire locale du propriétaire (citée comme « mémoire », non versionnée), essai de la 0.1.5 du 2026-11-23.
- **Évidence manquante** : journaux de session de chaque ticket (seule la conversation courante existe : l'analyse des leçons de processus s'appuie sur les plans, pas sur les sessions) ; relevés bruts d'entraînement hors dépôt (`~/cassini-entrainement/*-mesures/`) ; release GitHub et Hugging Face non relues en ligne par l'analyse ; Verification de la plupart des plans ; contenu du dépôt privé.

## Findings

Les constats viennent de deux analyses déléguées, **revérifiées à la source** quand elles pèsent sur le verdict (marqué ✔). Les autres sont des rapports non revérifiés, marqués ○.

### Critères « Done when »

| # | Critère | Verdict | Preuve |
|---|---|---|---|
| 1 | AD-24 à AD-28 écrites et citées | Écrit : atteint. Cité par chaque commande : partiel | « E1 fait » le 2026-10-18. En-têtes de `analysis_difference.rs`, `analysis_area.rs`, `analysis_dissolve.rs`, `analysis_join_table.rs`, `analysis_nearest.rs`, `data_reproject.rs` : AD-22/AD-10/AD-19, 0 occurrence d'AD-24 à AD-28 ○ |
| 2 | Chaque primitive retenue est une commande, testée hors ligne, composée par l'agent | **Atteint (mise à jour 2026-11-26)** | F1 est résolu : `analysis.distance_matrix`, `raster.polygonize` et `raster.align` sont construites (75f48a2, 10e2bd0, 24d4504), chacune avec sa famille d'entraînement ; composées par le modèle installé ? non, voir F-M6. Avant la mise à jour : trois primitives retenues sans code. `training/tests/couverture.rs` (04984e0) garantit une famille d'entraînement par commande du registre |
| 3 | ≥ 70 % sur phrases réelles | Atteint sur modèle non publié | 44 phrases (df20812) : registre 34/44 (77,3 %) ; agent 32/44 (72,7 %) avec `base-17-1` (`docs/couverture-phrases-reelles.md` l. 84-100). La couverture de `base-22-2`, le modèle publié, n'est pas consignée |
| 4 | Aucune commande ajoutée n'accède au réseau | Atteint, preuve indirecte | Garde sur la table d'imports (`cli/tests/hors_ligne.rs`, `recette.yml` l. 78-90) ; essai du 2026-11-23 : pas de vraie coupure réseau |
| 5 | Modèle ré-entraîné ou ré-éprouvé, sans baisse | Partiel | Voir ci-dessous |
| 6 | Livré par la chaîne de l'epic 7 | Atteint | Tag `v0.1.5`, `installer/modele.json` → GGUF 0.3.0 (SHA 97bdc957…), installeur complet vérifié le 2026-11-23 (SHA 2d1afaaf…672f7). Caveat : le correctif B110 (f1117af) est dans `main`, pas dans le tag |

### Ré-entraînements du modèle (poids demandé par le propriétaire)

| Date | Modèle | Chiffres consignés | Sort |
|---|---|---|---|
| 2026-11-14 | `ajuste-continu-5-58-1` | `plan_reel` 44/48, `question_reel` 46/47, `ecart_reel` 38/40, `echelle_utm_reel` 42/44, `terrain_reel` 37/44, `epic11_reel` 45/49, `style_reel` 36/42, `par_commande_reel` 89/99, Annecy 18/20 | Référence du juge |
| 2026-11-15 | `continu-5-59-1` | `plan_reel` 43, `ecart_reel` 36, `epic11_reel` 43, `par_commande_reel` 86 ; `terrain_reel` +3, `style_reel` +3 | Refusé |
| 2026-11-16 | `base-5-55` | `par_commande_reel` 80/99 (−9), Annecy 20/20, `ecart_reel` 35 | Refusé |
| 2026-11-17 | `continu-5-61b` | `plan_reel` 44, `terrain_reel` 39 (+2), `par_commande_reel` 89, v4 15/20 (barre 16) | Publié 0.2.0 |
| 2026-11-21 | `base-17-1` | `plan_reel` 44/48, couverture agent 32/44, `echelle_utm_reel` 42→37 | Refusé |
| 2026-11-22 | `base-22-1` | `echelle_utm_reel` 42→40, `ecart_reel` 38→35 ; 29 phrases perdues, 24 gagnées | Refusé ; cause réelle : 7 commandes sans famille |
| 2026-11-24 | `base-24-1` | `plan_reel` 47/48, `ecart_reel` 36/40, `echelle_utm_reel` 38/44 (baisse), `par_commande_reel` 124/148 ; `geo.bearing_intersection` 3/3 → 0/3 ; Annecy 20/20, gros 7/7 | Refusé (juge) |
| 2026-11-25 | `base-24-2` | `ecart_reel` 33/40 (baisse, H17-H19 + 4 bascules isolées), refus 2/4 ; `geo.bearing_intersection` 2/3 (après le correctif b479e13) ; Annecy 19/20 | Refusé (juge) |
| 2026-11-25 | `base-24-3` (graine torch 2) | `ecart_reel` 34/40 et `echelle_utm_reel` 38/44 (baisses), `analysis.area` 0/3 | Refusé (juge) ; décision du propriétaire du 2026-11-25 : garder `base-22-2`, rien installé |
| 2026-11-22 | `base-22-2` | juge mode base 285 contre 288 sur 314 ; Annecy 20/20, gros 7/7, v4 15/20 (mémoire seule) | Refusé de 3 phrases par le juge, accepté par le propriétaire (bruit de tirage) ; publié 0.3.0 |

- **Le constat « chaque ré-entraînement déplace des échecs ailleurs » se vérifie** dans les chiffres consignés : chaque candidat gagne et perd à la fois (voir les lignes ci-dessus). Les 3 phrases par commande de `par_commande_reel` font qu'un seul tirage bascule une commande.
- **Deux faits tempèrent** : le défaut structurel de `base-22-1` (sept commandes sans famille) a été trouvé et verrouillé par `training/tests/couverture.rs` ; les entraînements « de zéro » sont devenus la seule voie quand le bras continu est refusé à trois maillons.
- **F-M1 — Le modèle publié n'a aucune mesure versionnée** ○. `plan_reel`, couverture 44 phrases et `par_commande_reel` complet manquent pour `base-22-2` ; la mémoire donne seulement le juge, Annecy, gros projet, v4. « Sans baisse des plans conformes » (critère 5) n'est donc pas démontré dans un document du dépôt. Disposition : fix now (consigner les chiffres) ; leçon : un modèle qui part en release a ses mesures dans `docs/` avant l'étiquette.
- **F-M2 — L'acceptation de `base-22-2` repose sur une décision, pas sur une barre tenue** ○ : juge refusant de 3 phrases, v4 15/20 sous la barre de 16 (déjà vrai pour `continu-5-61b`). Pas de test de significativité consigné. Disposition : accept, avec décision du propriétaire écrite ; leçon : fixer le seuil de bruit du juge avant le cycle.
- **F-M3 — Ratés persistants sans correction durable** ○ : v4 sous la barre, GN07 « Trouve les cols », GC3. Disposition : defer ; GC3 est consigné (f65350c).
- **F-M4 — `analysis.nearest` (NEA1-3), CHE/PRE/ATL de `par_commande_reel` jamais passés contre un vrai modèle** ○ (mémoire, plan 10.17). Disposition : defer avec échéance, avant le prochain ré-entraînement.
- **F-M6 — Le ré-entraînement de fin d'epic est clos sans installation** ✔ (tableaux du juge `~/cassini-entrainement/5-55-mesures/base-24-{1,2,3}/non-oubli.txt`, hors dépôt) : trois entraînements de zéro, trois refus, chacun avec un recul net différent (une commande, une série, une autre commande). Un correctif ciblé (b479e13) a remonté `geo.bearing_intersection` sans faire accepter le modèle ; une autre graine n'a rien réglé. `raster.polygonize`, `raster.align` et `analysis.distance_matrix` sont donc hors de l'entraînement du modèle installé (`base-22-2`, qui fait 36/40 à `ecart_reel`). Le juge compare à `ajuste-continu-5-58-1`, lignée ajustée, pas au modèle installé. Disposition : defer ; voie comparable par le juge : un ajustement continu sur `base-22-2` avec les seules familles nouvelles ; décision du propriétaire.
- **F-M7 — Défaut de 10.36 rattrapé au ticket suivant** ✔ (message du commit 10e2bd0) : `analysis.distance_matrix` est livrée sans ses trois phrases de `par_commande`, ce qui fait échouer deux tests de `cassini-ai` ; elles sont ajoutées par 10.37. Disposition : accept, déjà corrigé. Leçon : le plan d'une commande liste ses trois phrases NV/DM dans les tâches.
- **F-M8 — Lecture du jeu tenu à l'écart** ✔ (`ai/tests/ecart_reel.rs`, en-tête : l'auteur du générateur ne lit pas le jeu) : pendant l'analyse du refus de `base-24-2`, la session a lu les phrases et plans attendus de H17, H18 et H19 pour comprendre les échecs. Rien n'a été ajusté dans le générateur d'après elles, et la mesure reste valable pour le juge ; consigné pour que ces trois phrases ne servent pas à régler les familles. Leçon : lire les échecs par la forme de l'écart seulement.
- **F-M5 — Ré-entraînement groupé des familles de refus de 10.35 jamais fait** ○ : le garde du noyau a été retenu seul. Disposition : defer, à grouper avec le prochain cycle.

### Aggregate views

- **F1 ✔ — RÉSOLU le 2026-11-24 (10.36, 10.37, 10.38 construits).** Constat d'origine : trois primitives du catalogue figé, rattachées à l'epic 10, n'ont ni code ni ticket** : `analysis.distance_matrix` (`catalogue-des-commandes.md` l. 376, statut `planifiée`, « epic 10, entrée 10 »), `raster.polygonize` et `raster.align` (l. 382-383, « epic 10 (entrée ajoutée après la 12) »). `core/src` et `qgis-plugin/cassini_qgis` n'en contiennent aucune ; `tickets.toml` n'a aucune occurrence de polygonize/align. L'epic les ajoute à la voie le 2026-11-05. Disposition : **fix now côté décision** (créer les tickets, ou les sortir officiellement de l'epic et corriger le critère 2 et le catalogue). Leçon : une primitive découpée d'un ticket (10.10) naît en ticket dans la même session, et le sweep compare le catalogue (« planifiée ») au code.
- **F2 ○ — Le refactor sweep 10.18 n'a rien balayé** : corrige un avertissement clippy (`training/src/lib.rs:1580`), revue « aucun constat », 33 entrées reportées toutes jugées hors balayage. Disposition : accept, en le consignant. Leçon : le plan d'un sweep liste ses vues (taille, duplication).
- **F3 ○ — Croissance de fichiers** (baseline `10a929646`) : `analysis.py` 579 → 2411 lignes ; `summary.py` 529 → 1302 ; `panel.py` 1353 → 2262 ; `layout_export.py` 798 → 1418 ; `training/src/families.rs` 7733 → 19263 ; `core/src/serve.rs` 4123 → 4944 ; `core/src/journal/tests.rs` 2981 → 7671 ; `ai/src/plan.rs` 1933 → 2852. Disposition : defer (découper `families.rs` par famille et `analysis.py` par genre avec le prochain epic qui touche la table fermée). Non vérifié : taille avant la baseline.
- **F4 ○ — Environ dix fichiers touchés par primitive, même gabarit recopié** (diffs difference, area, dissolve : `analysis_*.rs`, `analysis.py`, `summary.py`, `texts.py`, `fixtures/resultats-extension.json`, `phrases.json`, `barres.rs`, `table_annulable.json`, README, sonde, test). Le motif est cohérent ; aucun test commun ne vérifie qu'une commande a ses dix points. Disposition : accept le motif, defer un test « toute commande du registre a ses points ».
- **F5 ○ — Signatures hétérogènes des `translate_*` dans `analysis.py`** (`(params, layer)`, `(params, layer, backend)`, `(params, previous)`, `(params, raster)`). Disposition : defer ; AD-24 devrait fixer la signature d'un geste.
- **F6 ○ — Le traceur 10.7 a été construit avant la mise à niveau 10.1-10.6** (« à reprendre par chaque entrée »), sans point de contrôle daté de la reprise. Disposition : defer avec vérification.

### Lentille revue (relectures)

- **F7 ✔ (partiel) — Revues `quick` sur tickets `risk: high`, contre CLAUDE.md §8** : 10.14 (préparation de réseau, `lenses_ran: [quick]`, le plan note que `thorough` n'existe pas dans ce build et que le propriétaire a été prévenu) ; 10.17 (`[quick]` seul, non revérifié) ; 10.16 (géocodage) et 10.32 (`[quick, edge-cases]`, deux lentilles, 10.16 a trouvé deux constats medium). Disposition : fix now pour 10.14 et 10.17 (relecture `thorough` rétroactive ou accord signé du propriétaire) ; accept 10.16 et 10.32. Leçon : appliquer la règle par l'outil (échec si `risk: high` et `lenses_ran` = `[quick]`).
- **F8 ○ — Plans repris** : 10.2, 10.10, 10.24, 10.12. Défauts de conception visibles seulement contre le vrai QGIS (champ `distance_2`, ex æquo de `joinbynearest`, paramètre `fields` saisi à la main contre la règle « choisir, pas saisir »). Disposition : accept, déjà corrigés. Leçon : sonde sous vrai QGIS avec cas dégénérés avant le plan ; « paramètre désignant une couche ou un champ → sélecteur » dans la liste de contrôle du plan.
- **F9 ○ — 36 entrées reportées de l'epic dans `deferred-work.md`, 33 sans `resolution`.** À traiter avant clôture ou à consigner avec propriétaire : #2 `step:k.file` rejoué depuis le mauvais dossier (10.9) ; #4 `check_plan` n'interdit pas `step:k.layer_id` d'une étape `layer.remove` ; #12 CLAUDE.md §2/§8 liste MIT/Apache/BSD, `deny.toml` admet MIT-0 et Zlib depuis 2026-11-19 (écart de documentation, **édition de CLAUDE.md réservée au propriétaire**) ; #16 démarrage de l'extension non prouvé sous vrai QGIS ; #17 `network` d'isochrone sans exemple d'entraînement ; #22 régression `plan_reel` 44/48 → 41/48 (PL04, PL08, PL17, PL22, S24) jamais diagnostiquée. Disposition : defer avec échéance, sauf #12 et #22 (décision du propriétaire).
- **F10 ○ — Le plan du ticket de livraison 10.19 est vide** (frontmatter seul). La preuve est dans les commits `f428191`, `c13957c`, `47e831b`, `f65350c`. Disposition : accept, avec une ligne de preuve à ajouter au plan. Leçon : un ticket « livraison » consigne versions, empreintes et phrases dans son plan.
- **F11 ✔ — Pas d'infraction à CLAUDE.md §8 (ticket de fermeture)** : `after` de 10.18 liste les 32 entrées sauf 10.25 (`dropped`).
- **F12 — Écart de contenu de la release** : le tag `v0.1.5` ne contient pas f1117af (B110), et l'installeur complet sur Hugging Face embarque l'ancien zip de l'extension ; le zip 0.1.5 à jour est à déposer sur plugins.qgis.org par le propriétaire. Disposition : accept (une 0.1.6 les réunirait, au choix du propriétaire).

## Behavior verification

- **Exercé le 2026-11-23** : installation silencieuse de `cassini-installeur-complet-0.1.5.exe` (SHA identique à la page), `installation.json` = 0.1.5 avec le modèle 0.3.0 ; 18 phrases du jeu `ai/tests/par_commande/phrases.json` (3 par famille : data, analysis, raster, network, layout, geocode) passées par `cassini plan` avec le modèle installé : 17 conformes (dont CHE3, question attendue et obtenue), 1 en échec stable, GC3 (le modèle ajoute `layer.describe` après `geocode.ban`, le noyau refuse le plan, `agent.plan_invalid`, deux essais sur deux ; rien n'est exécuté).
- **Non exercé** : vraie coupure réseau, second poste débranché, exécution des commandes sous QGIS dans cet essai (plan seul), recette Linux.

## Previous-retro follow-through

Rien à suivre : l'epic qui précède dans l'ordre (`epic-publication`) n'a **pas de fichier de rétrospective**, ce qui ne dit rien de ses éventuelles actions en suspens.

## Action items

| # | Action | Propriétaire | Nature |
|---|---|---|---|
| A1 | **Fait (2026-11-24)** : les trois tickets sont créés (37ca584) et construits ; décider du sort de `analysis.distance_matrix`, `raster.polygonize`, `raster.align` : créer les tickets dans l'epic ou les sortir officiellement (critère 2, catalogue, epic) | propriétaire | décision, puis ticket (`bmad-ticket`) |
| A2 | Consigner dans `docs/` les mesures de `base-22-2` : `plan_reel`, couverture des 44 phrases, `par_commande_reel` complet, avec les 3 phrases NEA/CHE/PRE/ATL | Claude (session dédiée) | remédiation proposée |
| A3 | Écrire la décision du propriétaire sur l'acceptation de `base-22-2` malgré le juge à −3 et v4 15/20 | propriétaire | décision |
| A4 | Relecture `thorough` rétroactive de 10.14 et 10.17, ou accord écrit du propriétaire | propriétaire / Claude | remédiation proposée |
| A5 | Diagnostiquer ou accepter signée la régression `plan_reel` 44 → 41 (#22) | propriétaire | décision |
| A6 | Renseigner le plan de 10.19 (versions, empreintes, 17/18) | Claude | remédiation proposée |
| A7 | CLAUDE.md §2/§8 : mentionner MIT-0 et Zlib admis par `deny.toml` (#12) | propriétaire | réconciliation de spec proposée, non appliquée |
| A8 | Reporter F3, F4, F5, F6 et les entrées #2, #4, #16, #17 de `deferred-work.md` avec échéance ; GC3 et le ré-entraînement groupé des refus (10.35) vont au prochain cycle d'entraînement | Claude | defer |
| A10 | Décider d'un ajustement continu sur `base-22-2` avec les seules familles nouvelles (polygonize, align, distance_matrix), ou d'attendre le prochain cycle (F-M6) | propriétaire | décision |
| A11 | Fixer le seuil de bruit du juge et son parent de comparaison avant le prochain cycle (question ouverte 3, F-M2, F-M6) | propriétaire | décision |
| A9 | Process : bloquer par l'outil une revue `quick` sur `risk: high` ; mesures d'un modèle dans `docs/` avant l'étiquette ; plan de sweep listant ses vues ; une primitive découpée naît en ticket tout de suite | propriétaire / Claude | leçons amont |

Rien de tout cela n'est appliqué par cette rétrospective.

## Acceptance verdict

**accepted-with-open-items** (critères *declared*, décision machine, le propriétaire peut l'écraser).

- Tous les tickets de l'epic sont terminés (aucun `pending_tickets`) ; critères 3, 4 (preuve indirecte) et 6 atteints, 1 partiel, 5 partiel.
- Mise à jour du 2026-11-26 : la réserve forte est levée, les trois primitives de F1 sont construites (critère 2 atteint). Restent ouverts : A2 à A5, A10 et A11 ; le critère 5 reste partiel (trois ré-entraînements refusés, modèle publié inchangé). Le verdict reste *accepted-with-open-items*.
- Aucune baisse mesurée n'est cachée : la preuve de non-régression du modèle publié (critère 5) n'est pas versionnée (A2), et son acceptation est une décision, pas une barre tenue (F-M2).

## Open questions

1. ~~Les trois primitives sans ticket (F1)~~ : résolu, elles sont construites.
2. Une version 0.1.6 (B110 + zip à jour dans l'installeur complet) est-elle voulue, ou le dépôt d'extensions QGIS suffit-il ?
3. Un seuil de bruit du juge (combien de phrases d'écart comptent pour « refusé ») doit-il être fixé avant le prochain cycle ?
4. Le fichier `tickets.toml` ne porte pas de `status` : les statuts vivent dans les plans (`built`/`done` mélangés) ; faut-il tout fermer en `done` ?
