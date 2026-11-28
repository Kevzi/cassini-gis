---
epic: epic-publication
date: 2026-11-28
verdict: rejected
criteria: declared
headless: false
---

# Rétrospective de l'epic 7 — Publication

## Epic summary

Interactif, mais les décisions ont été prises d'avance avec le propriétaire : (1) rétro lancée alors que le ticket 7.14 n'est pas terminé ; (2) aucun angle imposé ; (3) pas de discussion d'équipe (phase 3 sautée).

**Tickets inachevés sur lesquels la rétro passe (`pending_tickets`) : `7.14` seul.** « Une personne extérieure installe et refait la démo par les deux portes » : état `planned`, aucun plan, il attend une vraie personne (`tickets.py status`, ligne 7.14 ; le plan n'existe pas dans le dossier). Le verdict machine est donc **rejeté** (`acceptance-verdict.md`) ; seul 7.14 est ouvert, les quatorze autres entrées sont finies.

**Tickets finis par le build mais pas encore `done` (statut `built`, état `review`) : 7.4, 7.5, 7.7, 7.8, 7.9, 7.11, 7.12, 7.13.**

| Ticket | Titre | Statut | Risque / relecture | Baseline du plan | Commits qui le nomment |
|---|---|---|---|---|---|
| 7.1 | Balle traçante | done | high / thorough | `15ccd9d` | `443e7f0`, `3dfcdff`, `ac6ef92`, `8ecda3a` |
| 7.2 | Porte 2 sous Windows | done | high / thorough | `0f7245c` | `058bbe5` |
| 7.3 | Modèle sur Hugging Face | done | low / aucune | `ede7d0a` | `0b1c555`, `e261008`, `7d83fa8` |
| 7.6 | Noyau gpu Vulkan | done | **high / quick** | `543721e` | `b3a91ae`, `e098f6b`, `52f8ba9`, `8e2891a`, `92a7e7b`, `a342a91`, `104db43`, `f79e2fe` |
| 7.8 | Panneau : moteur introuvable, version, rappel | built | low / quick | `3dc0460` | `e6cc904` |
| 7.10 | Extension sur le dépôt QGIS | done | medium / quick | `e6cc904` | `e5f9568`, `682af7e`, `d342cee`, `386a04d`, `5464414` |
| 7.4 | Porte 1 complète | built | medium / quick | `b6a2da0` | `82b1dcb` |
| 7.15 | Republier le modèle (continu-5-61b) | done | medium / quick | `8cf2d47` | `8cf2d47`, `76e7a0b`, `d0eae55` |
| 7.5 | « Mettre à jour Cassini » | built | medium / quick | `d0eae55` | `aa3f9f0`, `4eeb213`, `1daa6a1` |
| 7.7 | Linux, deux portes | built | medium / quick | `aa3f9f0` | `847053c`, `cf09b8e` |
| 7.9 | Licences et frontière GPL | built | medium / quick | `a60ea4b` | `753d2f5` |
| 7.12 | Page d'accueil | built | low / quick | `753d2f5` | `0af18b9`, `ea15187`, `c746b07`, `430e45b`, `a0a3abf` |
| 7.11 | Jeu de phrases réelles | built | low / **aucune** | `2240fa9` | `df20812` |
| 7.13 | Refactor sweep | built | medium / quick | `f1117af` | `6ac53d7`, `33cf9b2` (plan `331768a`) |
| 7.14 | Une personne extérieure installe | **planned** | medium | aucun plan | aucun |

- **Plages.** Les plans sont classés par place de la baseline dans l'historique (7.1 à 7.13). L'historique est partagé avec les epics 5, 9, 10, 2 : la plage `15ccd9d..HEAD` compte 158 commits, dont **41 seulement nomment un ticket 7.x** (`git log --grep`). **Portée réduite, déclarée :** les plages par plan n'ont pas été exploitées telles quelles (elles mêlent d'autres epics) ; `git_evidence.py` a été lancé une fois sur la plage totale (158 commits, 317 fichiers, sortie non retenue : trop bruyante) et les volumes de l'epic ont été recalculés à la main sur les 41 commits qui nomment `7.x` : 74 fichiers, +9326 −404. Ordre de construction constaté : 7.1, 7.3, 7.2, 7.6, 7.8, 7.10 (13/11) ; 7.4, 7.15, 7.5, 7.7, 7.9, 7.12 (17-18/11) ; 7.11 (20/11) ; 7.13 (23 plan, 28/11 code). Plage de 7.13 : `f1117af..HEAD`, coupée aux commits `6ac53d7` et `33cf9b2`.
- **Aucune dépendance Rust ajoutée** par l'epic : seuls changements de `Cargo.toml` : montée de version (`682af7e`) et la fonctionnalité `elf` de la crate `object`, tests seulement (`847053c`, `cli/Cargo.toml`).
- **Inventaire disponible** : fichier de l'epic (Done when présent), `tickets.toml`, 14 plans, `deferred-work.md`, `git_evidence.py`, code courant, tags `v0.1.0`, `v0.1.1`, `v0.1.3`, `v0.1.5`. Pas de fichier d'histoire. 7.14 n'a pas de plan ; 7.11 n'a ni Verification ni Review Triage Log (plan réduit à Intent et Implementation Notes).
- **Inventaire manquant** : **journaux de session absents** (aucun cherché ni trouvé) : l'analyse des leçons de processus tirées des sessions n'a pas été faite, ce qui a été essayé et abandonné est inconnu. Pas de retour du GitHub Actions (aucune action GitHub lancée, par consigne) : les exécutions de la recette de release ne sont connues que par les tags et les messages de commit.
- **Portée réduite, déclarée** : la compétence `bmad-review` n'a pas été invoquée (je tourne comme sous-agent, sans relecteurs parallèles). Les lentilles adversariale, cas limites et trou de vérification ont été appliquées **en ligne** sur : le diff du 7.13 (`6ac53d7`), `installer/cassini.sh` (téléchargement), `installer/cassini-en-ligne.iss` (appels de téléchargement), `release.yml` (étapes touchées par 7.13 et par `needs`), `installer/manifeste.py`, la garde de l'extension, les plans de 7.4, 7.5, 7.6, 7.7, 7.8, 7.9, 7.11, 7.12, 7.13, 7.15. Le reste (Pascal complet de l'installeur, textes de la page, `licences.py`) n'a pas été relu ligne à ligne.

## Findings

Constat « vérifié » = relu dans le code ou exécuté ici.

### A. Vues agrégées

| # | Vue | Constat | Source | Disposition |
|---|---|---|---|---|
| F1 | Spec → implémentation | **Done when 4 et R10 non tenus** : aucune personne extérieure n'a installé ni refait la démo ; 7.14 est le seul ticket ouvert et dépend de 7.13 (`after`), lui-même `built`. C'est le « signal » que l'Outcome de l'epic fixe (première installation réussie par une personne extérieure). Les six autres critères sont tenus (voir verdict). | `epic-publication.md` (Outcome, Done when 4, R10) ; `tickets.toml` 7.14 ; `tickets.py status` | Reporter (7.14, attend une personne) |
| F2 | Spec → implémentation | **La spec de l'epic a dérivé du réel.** (a) l'epic dit « quatorze entrées » et ne cite pas 7.15, qui existe dans `tickets.toml` (ajoutée le 2026-11-17) ; (b) les Notes parlent de `v0.1.1` (Q9 amendée) et du modèle 0.1.0 (plan de 7.3), alors que les tags vont à `v0.1.5`, `installer/modele.json` porte le modèle `0.3.0` et `metadata.txt` la version `0.1.5` ; (c) `installer/assembler_linux.py:15` cite encore `--version 0.1.1` dans son exemple. **Vérifié.** | `epic-publication.md` l.111 ; `tickets.toml` ; `installer/modele.json` ; `qgis-plugin/cassini_qgis/metadata.txt` l.5 ; `installer/assembler_linux.py:15` | Corriger (réconciliation de spec proposée, A6) |
| F3 | Duplication | **Le nom versionné du modèle est écrit en dur à huit endroits** : `release.yml` l.443, 485, 487 ; `cassini-hors-ligne.iss` l.15, 25, 31 ; `docs/modele-hugging-face/publier.ps1:13` ; `installer/telecharger_modele.py:9` ; `installer/tests/test_hors_ligne.py:262` — alors que `installer/modele.json` (`nom_installe`) est la seule source et que le job Linux la lit (`release.yml` l.91-94). Chaque republication (7.15 : 0.1.0 → 0.2.0 ; 10.19 : 0.3.0) oblige à toucher tout cela ; un oubli ne casse que la release, à l'étiquette. `test_linux.py:38` garde même un `0.2.0` figé. **Vérifié** (`grep`). | fichiers cités ; `f428191` (10.19) | Corriger (A3) |
| F4 | Taille / fichiers volumineux | `.github/workflows/release.yml` : **577 lignes**, +645 −68 sur 15 commits de l'epic ; `installer/cassini-en-ligne.iss` : **548 lignes**, +603 −55 sur 9 commits ; `installer/cassini.sh` 317 ; `installer/page_accueil.py` 350. Le Pascal de l'installeur n'a aucun test unitaire (constat reporté de 7.6) et n'est prouvé que par l'essai à blanc de la release. Candidats à regarder, pas des verdicts. | `wc -l` ; stats par fichier sur les 41 commits ; `deferred-work.md` (entrée 7.6, mise à jour par `6ac53d7`) | Reporter (A7) |
| F5 | Divergence de motif / garde | **Récidive Bandit B110.** 7.10 a vu 0.1.1 bloquée par l'analyse du dépôt d'extensions (six `try/except/pass`, `d342cee`) ; le même défaut est revenu dans `inventory.py` et a été corrigé par `f1117af` (2026-11-23, « B110 »). Aucune règle équivalente n'existe dans la garde locale : `check_extension.py` ne contient pas `bandit` ni `except…pass`, et le mot n'apparaît nulle part dans `.github`, `docs`, `installer`, `qgis-plugin` (hors commits). Le scanner du dépôt reste le seul filet, et il agit à la soumission. **Vérifié** (`grep -rin bandit`). | `d342cee` ; `f1117af` ; `qgis-plugin/tests/check_extension.py` ; plan 7.10 (Review Triage Log, dernière ligne) | Corriger (A4) |
| F6 | Architecture | Rien de neuf côté noyau : aucun crate ajouté ; `cli/Cargo.toml` gagne la fonctionnalité `elf` d'`object` (tests seulement, licence Apache-2.0 OR MIT) ; la garde hors ligne couvre désormais ELF (branche dans `cli/tests/hors_ligne.rs`, `847053c`), plus faible que la garde Windows (pas d'interdiction de `socket`/`connect`, dit au plan de 7.7). Pas de cycle ni de dépendance de l'extension vers le noyau. | `847053c` ; plan 7.7 (Implementation Notes) | Accepter |
| F7 | Ordre `after` | **L'ordre déclaré a été tenu**, contrairement aux epics 2 et 9 : 7.13 (code `6ac53d7`, 28/11) vient après 7.11 (`df20812`, 20/11) et 7.12 (18/11) ; 7.4 attendait 5.51, présent dans l'historique (`31d1320`) avant `82b1dcb`. | `git log` ; `tickets.toml` | Accepter (bon point) |

### B. Lentilles sur le diff (portée réduite, voir plus haut)

| # | Constat | Source | Disposition |
|---|---|---|---|
| F8 | **Le durcissement de 7.13 ne protège que le chemin de la recette.** `telecharger_modele.py` (trois essais, délai global, https exigé redirections comprises, `6ac53d7`) n'est appelé que par `release.yml` (l.94 et l.443). Le téléchargement de l'utilisateur passe ailleurs : Inno Setup `DownloadTemporaryFile` (`cassini-en-ligne.iss` l.351, 444, 504), sans essai ni contrôle du schéma ; Linux : `curl -fsSL --retry 2` sans `--max-time`, `wget` sans reprise, et `recuperer_manifeste` accepte `http://*` (`cassini.sh` l.94-97, 105). L'intégrité reste couverte (SHA-256 vérifié avant pose), la disponibilité et la confidentialité du manifeste non. Le plan de 7.13 interdisait de changer l'installeur Windows (Boundaries), donc ce n'est pas un défaut de la livraison, mais la ligne `deferred-work.md` « RÉSOLU » peut se lire comme couvrant l'utilisateur final. | `installer/telecharger_modele.py` ; `release.yml` l.94, 443 ; `cassini-en-ligne.iss` ; `cassini.sh` ; plan 7.13 (Boundaries) | Reporter (A5), préciser la formulation |
| F9 | **7.13 est `built` sans la preuve qu'il s'était donnée.** Le plan le dit : « La vérification complète (recette verte sur une nouvelle étiquette, essai à blanc `gh workflow run release.yml`) n'est pas faite ». Deux étapes nouvelles n'ont jamais tourné sur GitHub : « Recette verte du commit » (`gh run list --workflow recette.yml --commit "$GITHUB_SHA" --status success`, `release.yml`, job `linux`) et l'altération par entrée (boucle `altere-$k`, avec un `throw` si le nombre d'empreintes diffère du nombre de fichiers). Je n'ai pas trouvé de défaut à la lecture (la recette se déclenche sur `push`, donc sur l'étiquette comme sur la branche), mais c'est une lecture, pas une exécution. Détail de forme : les cases des tâches du plan sont restées `[ ]` alors que le statut est `built` (`story-refactor-sweep-plan.md` l.55-58). | plan 7.13 (Review Triage Log, dernier paragraphe ; Verification) ; `6ac53d7` ; `recette.yml` l.11-14 | Reporter (A1) |
| F10 | **7.5 a été « built » avec un installeur qui plantait.** Le plan dit « Le script Pascal n'a jamais été compilé ni lancé : première exécution réelle = recette ou VM ». Le lendemain : `1daa6a1` « /MISEAJOUR plantait dans InitializeWizard (constante {app} lue trop tôt) » et `4eeb213` (commentaire Inno avec `{app}` imbriqué). Le plan n'a pas de Plan Change Log pour ces corrections et ses notes disent toujours « jamais compilé ». Corrigé, donc pas de défaut ouvert ; le risque est le motif : un ticket `built` sur du code jamais exécuté. | plan 7.5 (Implementation Notes l.71-73) ; `aa3f9f0`, `4eeb213`, `1daa6a1` ; plan 7.4 (même formule) | Accepter (corrigé) ; leçon A8 |
| F11 | **Vérifications manuelles des plans sans trace.** Plans qui les demandent et n'écrivent rien après : 7.4 (« VM Windows propre, avec puis sans modèle, un octet changé »), 7.5 (« VM : mise à jour contre un manifeste plus récent, retour à l'ancien modèle »), 7.7 (« Ubuntu 22.04 propre : porte 1, porte 2 réseau coupé, mise à jour »), 7.8 (QGIS : bandeau, bouton, sonde de 3 s, note de trois mois), 7.12 (adresse fixe : vidéo et empreinte). Des traces indirectes existent (releases `v0.1.3`, `v0.1.5`, page régénérée avec les quatre installeurs, commit `430e45b` qui cite Linux) mais rien ne dit qu'une personne a fait ces parcours. *Aucune preuve trouvée*, pas « non fait ». | plans 7.4, 7.5, 7.7, 7.8, 7.12 (Verification) ; `site/index.html` l.101-104 | Reporter dans 7.14, qui les couvre (A2) |
| F12 | **Le choix de relecture n'a pas suivi le risque.** 7.6 (risque `high`, la recette Windows qui construit le noyau gpu) a eu `quick` (`review: 'quick'`, `review_source: 'pinned'`), alors que 7.1 et 7.2 (high) ont eu `thorough` ; 7.11 (risque `low`) n'a eu **aucune** relecture et a été construit « sans relecture ni plan préalable par un agent non supervisé » (`df20812`). 7.6 date du 13/11, avant la leçon A9 de la rétro du catalogue (24/11) : ce n'est donc pas une récidive, mais le garde-fou « refuser `quick` sur `high` » n'existait pas. | plans 7.6, 7.11 (front matter, Implementation Notes) ; rétro de l'epic 10, A9 | Accepter (antérieur à la règle) ; leçon couverte par A9 de l'epic 10 |
| F13 | **7.11 : la question ouverte n'est pas tranchée et les sources ne sont pas vérifiées.** `tickets.toml` porte toujours `unknown = "La source du jeu de phrases réelles d'utilisateurs…"` ; le plan écrit que les URL de source « ne sont pas vérifiées une à une » ; les 44 phrases viennent de « documents ou thèmes publics » (`source.type: doc`), paraphrasées. Mesure : 33/44 (75 %) de couverture du registre, **21/44 (47,7 %)** bien rendues par le modèle installé, le seuil de 70 % étant renvoyé à l'epic 10 comme R9 le dit : conforme à R9. Le plan n'a ni triage ni Verification. | `tickets.toml` l.136 ; plan 7.11 (Implementation Notes) ; `ai/tests/couverture_reelle/phrases.json` ; `docs/couverture-phrases-reelles.md` | Reporter (A9) |

### C. Process

| # | Constat | Source | Disposition |
|---|---|---|---|
| F14 | **Le sweep 7.13 ne liste pas ses vues.** Son plan a quatre points à corriger et quatre consignés, tous tirés de `deferred-work.md` ; aucune mesure de taille, de duplication ni d'écart de motif sur l'ensemble de l'epic (c'est ce que F3, F4 et F5 montrent). La rétro du catalogue (A9) demandait déjà « plan de sweep listant ses vues » ; 7.13 a été planifié le 23/11, avant cette rétro. | plan 7.13 (Intent, Code Map) ; `331768a` | Accepter pour 7.13 ; règle à suivre par A9 de l'epic 10 |
| F15 | **Journaux de session absents** : impossible de dire pourquoi 7.6 a demandé sept corrections de recette en une journée (`e098f6b` à `f79e2fe` : générateur Vulkan, parallélisme, Ninja, 0xC000007B) ni pourquoi 7.11 est passé sans plan. Le plan de 7.6 donne la cause de trois défauts, pas du reste. | `git log` du 13/11 ; plan 7.6 (Implementation Notes) | Accepter, lacune d'évidence |

## Behavior verification

L'epic change du comportement (installeurs, release, panneau). Ce qui a été exécuté ici, sur le dépôt à `HEAD` (`33cf9b2`), arbre inchangé avant et après (seul `.memlog.md`, préexistant, est modifié) :

- `uv run --no-project python -I -m unittest discover -s installer/tests` : **122 tests, OK (1 ignoré)** ; le test de licences a assemblé un paquet réel (`LICENCES-TIERCES.txt`, `FRONTIERE-GPL.txt`, `cassini.exe`, DLL ggml).
- `python -I qgis-plugin/tests/check_extension.py` : « Garde de l'extension : propre. » ; `unittest discover -s qgis-plugin/tests` : **1205 tests, OK**.
- Lecture du diff de 7.13 et du workflow : voir F8, F9.
- **Non exercé** : toute exécution de `release.yml` (essai à blanc ou étiquette : interdit ici), l'installeur Inno Setup (compilateur absent du poste, test ignoré), `cassini.sh` sur Ubuntu 22.04, l'installation par les deux portes sur une machine propre, le panneau dans QGIS, la page publique en ligne (`https://kevzi.github.io/cassini-gis`), la soumission sur plugins.qgis.org (7.10 note la publication en 0.1.2, confirmée par le propriétaire le 2026-11-13, non revérifiée). Les 7.14 et F11 sont précisément ces parcours.

## Previous-retro follow-through

**Rien à suivre, et le cas est celui du fichier absent** : l'epic qui précède `epic-publication` dans l'ordre de `status` est `epic-export-et-demo`, et son dossier ne contient **aucun** fichier `*-retrospective.md`. Cela ne dit rien de ses éventuelles actions en suspens. (Pour mémoire : l'epic qui *suit*, `epic-catalogue-etendu-qgis`, a une rétro du 2026-11-26 qui écrit la même absence ; ses actions A9 sur la relecture `quick` pour `high` et le plan de sweep touchent F12 et F14, voir plus haut. Les rétros des epics 2 et 9, dont `epic-facade-qgis-retrospective.md` pris pour modèle de forme, ne sont pas des précédents de cet epic.)

## Action items

Tous **proposés, non appliqués** ; le propriétaire décide. « Remédiation » et « réconciliation de spec » attendent une application humaine.

| # | Action | Constats | Propriétaire | Nature |
|---|---|---|---|---|
| A1 | Faire l'essai à blanc `gh workflow run release.yml --ref main` (vert attendu, rien n'est publié) puis, à la prochaine étiquette, vérifier que « Recette verte du commit » et les altérations par entrée tournent ; noter le résultat dans le plan de 7.13, cocher ses tâches, puis seulement le passer à `done` | F9 | Propriétaire (action GitHub) | Preuve manquante |
| A2 | Faire exécuter 7.14 par une vraie personne en y couvrant les parcours des plans 7.4, 7.5, 7.7, 7.8, 7.12 (VM Windows, Ubuntu 22.04, panneau QGIS, page publique) ; en consigner chaque résultat, puis passer 7.4, 7.5, 7.7, 7.8, 7.9, 7.11, 7.12, 7.13 à `done` | F1, F11 | Propriétaire (avec `bmad-ticket`) | Clôture de l'epic |
| A3 | Faire de `installer/modele.json` la seule source du nom du modèle : `release.yml` l.443/485/487, `cassini-hors-ligne.iss`, `publier.ps1`, docstring de `telecharger_modele.py`, tests ; ajouter un test qui échoue si un nom en dur diffère | F3 | Dev (`bmad-build`, petite entrée) | Remédiation |
| A4 | Ajouter à la garde locale (ou à la recette) l'analyse Bandit sur `qgis-plugin/cassini_qgis`, au moins B110, pour attraper `try/except/pass` avant la soumission | F5 | Dev (`bmad-build`) | Remédiation |
| A5 | Si l'on veut la même robustesse chez l'utilisateur : temps limite et reprise pour `curl`/`wget` de `cassini.sh`, refus d'`http://` hors `file://` ; côté Inno Setup, décider si le schéma des adresses du manifeste se contrôle. Et reformuler dans `deferred-work.md` « RÉSOLU » en « RÉSOLU pour la recette » | F8, F12 | Propriétaire (décision), puis Dev | Décision puis remédiation |
| A6 | Réconcilier l'epic : 15 entrées dont 7.15, version du paquet (`v0.1.5`), modèle courant (0.3.0), `assembler_linux.py:15` ; ne pas écrire d'office, le propriétaire applique | F2 | Propriétaire (avec `bmad-ticket`) | Réconciliation de spec proposée |
| A7 | Reporter dans `deferred-work.md` : tailles de `release.yml` et `cassini-en-ligne.iss`, absence de test unitaire du Pascal ; à regarder avant le prochain ajout de job ou d'étape de release (macOS, Vulkan sous Linux) | F4 | Dev | Suivi |
| A8 | Leçon de processus : ne pas passer un ticket à `built` quand son plan écrit « jamais exécuté » pour le code livré (7.4, 7.5, 7.7) ; ou consigner « première exécution réelle » dans le plan, avec ses corrections (cas 7.5) | F10 | Propriétaire (avec `bmad-ticket`) | Leçon de processus |
| A9 | Trancher la source des phrases de 7.11 (`unknown` du ticket), vérifier les URL de source, ajouter Verification et triage au plan ; la mesure de départ à 70 % reste à l'epic 10 (10.8) | F13 | Propriétaire, puis Dev | Décision puis suivi |

## Acceptance verdict

**Rejeté**, critères **déclarés** (Done when de l'epic), règle `acceptance-verdict.md` : un ticket de l'epic est inachevé (`pending_tickets` = `7.14`). Ce n'est pas un défaut caché : le ticket attend une personne extérieure et le propriétaire a choisi de faire la rétro avant. **Les quatorze autres entrées sont finies** (six `done`, huit `built`).

Done when, un par un, sur ce qui a été lu ou exécuté :

1. Extension sur le dépôt d'extensions, installable depuis QGIS : **tenu** d'après le plan 7.10 (publiée en 0.1.2, confirmée par le propriétaire) ; `metadata.txt` complet (`homepage`, `experimental=False`, version 0.1.5). Non revérifié en ligne.
2. Noyau empaqueté Windows et Linux, produit par release, deux portes par système : **tenu sur le papier** (tags `v0.1.0` à `v0.1.5`, `release.yml`, `manifeste.py`, page avec quatre installeurs) ; les parcours d'installation propres ne sont pas tracés (F11).
3. Vidéo de démo : **tenu** (`site/cassini-presentation.mp4`, 6,8 Mo, servie par la page).
4. Personne extérieure installe et refait la démo : **non tenu** (7.14, F1).
5. Licences et frontière GPL dans la release : **tenu** (7.9, `LICENCES-TIERCES.txt` et `FRONTIERE-GPL.txt` assemblés par le test exécuté ici).
6. Modèle sur Hugging Face, installable par les deux portes, empreinte vérifiée : **tenu** (7.3, 7.15, `installer/modele.json`, preuves d'altération de 7.13 écrites mais non rejouées sur GitHub, F9).
7. Page d'accueil unique, deux phrases, vidéo, installation en trois étapes, empreintes, FAQ, contact ; `homepage` pointe dessus : **tenu** (`site/index.html`, `faq.html`, `avertissement-windows.html`).

Points ouverts, aucun bloquant hors 7.14 : F9 (preuve de 7.13), F11 (parcours manuels), F8 (téléchargement de l'utilisateur), F5 (garde Bandit), F3 (nom du modèle en dur), F13 (source des phrases), F4. Une décision humaine l'emporte sur ce verdict machine ; si le propriétaire accepte l'epic sans 7.14, il l'écrit lui-même.

## Open questions

1. Un parcours d'installation complet a-t-il déjà été fait par le propriétaire sur machine propre (VM Windows, Ubuntu 22.04) pour 7.4, 7.5 et 7.7 ? Ce que la réponse changerait : F11 deviendrait « tenu » et A2 se réduirait à 7.14.
2. L'essai à blanc `release.yml` a-t-il été relancé depuis `6ac53d7` ? (non vérifiable sans action GitHub.) Il décide de F9.
3. Le durcissement du téléchargement doit-il atteindre l'utilisateur (F8) ou la signature des installeurs, annoncée « avant les premiers vrais clients », prend-elle le relais ?
4. Sessions de build : aucun journal disponible, aucune leçon sur ce qui a été essayé et abandonné (7.6 en particulier).
5. 7.14 : qui est la personne extérieure et quel jeu de consignes publiées lui donne-t-on, puisque Done when 4 exige « la seule documentation du dépôt » ?
