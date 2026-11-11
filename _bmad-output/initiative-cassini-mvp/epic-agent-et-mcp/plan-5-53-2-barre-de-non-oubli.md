---
title: 'Barre de non-oubli : un candidat tient chaque barre de son parent (livrable 2 de 5.53)'
type: 'feature'
ticket: '53'
created: '2026-11-11'
status: 'built'
baseline_revision: '7459e2fb000f6da9af8a99e926eb32fe651b3759'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 1
context:
  - '{project-root}/docs/entrainement-du-modele.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** décider d'installer un modèle se fait aujourd'hui à la main, en relisant des
journaux de mesure : rien ne garantit qu'un candidat n'a pas oublié ce que son parent savait
faire, barre par barre (ticket 5.53 : « il tient chaque barre de son parent à une phrase près,
sinon il n'est pas installé »).

**Approach:** un outil hors ligne, sans modèle ni GPU, qui lit les relevés déjà produits (essais
GPU, recettes QGIS, rapport d'entraînement) du parent et du candidat, rend un verdict par barre
et un verdict d'ensemble (installable, refusé, à décider), en texte et en JSON lisible par
machine, avec l'identité de chaque modèle. Essai réel : `ajuste-5-49-a` (candidat) contre la
fusion de la nuit du 11-10 (parent), sur les relevés gardés.

Décisions du propriétaire (2026-11-11, après la relecture thorough) :
- D1 : la validation du `rapport.json` est **indicative**, sauf si les deux jeux de validation
  ont la même composition (`par_sorte` : mêmes totaux par sorte) ; elle n'est tirée du
  générateur et renouvelée à chaque entraînement, ce n'est pas une phrase écrite à la main.
- D2 : une barre dont le candidat échoue à son propre seuil alors que le parent y passait est en
  `baisse`, même à une phrase près ; le seuil est un code de fin de section non nul (essais GPU)
  ou « barre non tenue » d'une recette QGIS. Il est affiché dans le tableau.

## Boundaries & Constraints

**Always:** une barre tient si le candidat a au plus une phrase juste de moins que le parent, sur
le même total ; une barre du parent absente du candidat refuse ; une barre sans parent, ou de
totaux différents, ne refuse pas mais rend « à décider » ; le rejeu identique d'une recette ne
se perd pas ; la recette du gros projet reste indicative tant que 5.50 n'en fait pas une barre ;
Python standard seul (licence du noyau sans objet : outil d'entraînement) ; messages en français.

**Never:** charger un modèle, utiliser le GPU, installer ou déplacer un modèle ; toucher les
scripts de nuit en cours, `~/cassini-worktrees/reentrainement-5-49` ou `CASSINI-nuit` ;
modifier une barre existante ; pousser.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Barre tenue | parent 25/25, candidat 24/25 | `tenue` | — |
| Baisse | parent 25/25, candidat 23/25 | `baisse`, ensemble `refusé` | code de sortie 1 |
| Barre perdue | parent a `style_reel`, candidat non | `absente`, `refusé` | code 1 |
| Barre nouvelle | candidat a `par_commande_reel`, parent non | `sans parent`, `à décider` | code 3 |
| Totaux différents | Annecy parent 4/5, candidat 18/20 | `non comparable`, `à décider` | code 3 |
| Rejeu perdu | parent « rejeu identique », candidat non | `baisse` | code 1 |
| Par commande | `SCORE terrain.slope 1/3` contre `3/3` | barre `par_commande:terrain.slope` en `baisse` | code 1 |
| Fichier illisible | relevé vide ou de forme inconnue | message nommant le fichier | code 2 |
| Validation de jeux différents | `par_sorte` 253/25/22 contre 249/38/13 | `validation` indicative | — |
| Validation de même jeu | mêmes totaux par sorte, 291 → 289 | `baisse` | code 1 |
| Seuil perdu | parent `fin … : 0`, candidat `fin … : 101`, 25/25 → 25/25 | `baisse` | code 1 |
| Recette sous sa barre | parent « barre tenue », candidat « barre non tenue » | `baisse` | code 1 |

</frozen-after-approval>

## Code Map

- Relevé GPU (`~/cassini-entrainement/5-49-mesures/essais-gpu-a.log`, écrit par `%TEMP%\essais-5-49.ps1`) : sections `=== <barre> ===` … `=== fin <barre> : <code> ===`, une ligne `SCORE <justes>/<total> …` par barre (la première fraction compte ; `ecart_reel` porte des sous-scores après « : »), identité `Modèle : <chemin> ; adaptateur : … ;`. Texte UTF-8 avec BOM possible et accents abîmés : ne lire que l'ASCII.
- `ai/tests/par_commande_reel.rs` (livrable 1) -- lignes `SCORE <commande> <j>/<n>` et `SCORE total <j>/<n>` : une barre par commande plus le total.
- Recette QGIS (`5-49-mesures/recette-annecy-a.txt`, `recette-gros-a.txt`) -- `SCORE : <j>/<n> (barre …)` ou, plus ancienne, `PLANS EXECUTES : <j>/<n>` ; lignes `== <ID> : <phrase>` dont le préfixe (`RA`, `G`) nomme la recette ; `FIN : rejeu identique` quand le rejeu tient.
- `rapport.json` d'un entraînement (`~/cassini-entrainement/ajuste-*/rapport.json`) -- `validation.justes`, `validation.total` : barre `validation`.
- Parent de l'essai : `11-23-mesures/essais-gpu-nuit-11-10-fusion.log`, `11-23-mesures/recette-annecy-nuit-11-10-fusion.txt` (ancienne recette à 5 phrases), `ajuste-nuit-11-10-fusion/rapport.json`.
- `training/entrainement/scoring.py`, `tests/test_data_scoring.py` -- style à suivre (stdlib, docstrings en français, `unittest`).

## Tasks & Acceptance

**Execution:**
- [x] `training/entrainement/non_oubli.py` -- lecture des trois sortes de relevés (reconnues au contenu), comparaison, verdicts, sortie texte et `--json <fichier>`, codes 0 / 1 / 2 / 3 -- l'outil de décision.
- [x] `training/entrainement/tests/test_non_oubli.py` -- une ligne de la matrice par test, sur de petits relevés écrits dans le test ; plus un test sur des extraits des vrais formats (BOM, accents abîmés, sous-scores d'`ecart_reel`).
- [x] `docs/entrainement-du-modele.md` -- section « Barre de non-oubli » : usage, règle, verdicts, résultat de l'essai 5-49-a contre la fusion du 11-10.

**Acceptance Criteria:**
- Given les relevés gardés de `ajuste-5-49-a` et de la fusion du 11-10, when l'outil tourne, then il rend un verdict par barre (essais GPU, validation, Annecy) et un verdict d'ensemble, et nomme les deux modèles.
- Given `python3 -I -m unittest discover -s training/entrainement/tests`, then vert.

## Design Notes

Le verdict « à décider » existe parce qu'une barre neuve (par commande au premier relevé) ou
élargie (recette passée de 5 à 20 phrases) n'a pas de parent comparable : refuser bloquerait
toute installation, accepter ferait passer un oubli. Le propriétaire tranche, sur le tableau.

Sortie texte, une ligne par barre :
`plan_reel            25/25 → 24/25   tenue`

## Verification

**Commands:**
- `python3 -I -m unittest discover -s training/entrainement/tests` -- vert
- `python3 -I training/entrainement/non_oubli.py --parent <relevés fusion 11-10> --candidat <relevés 5-49-a> --json <scratch>/non-oubli.json` -- un verdict par barre

## Plan Change Log

- 2026-11-11, boucle 1 (intent_gap, relecture thorough) : la validation comparait deux jeux
  tirés différemment (constat edge-case : 253/25/22 contre 249/38/13) et le seuil propre d'une
  barre était lu sans effet (intent-alignment, blind-hunter). Le propriétaire a tranché D1 et D2
  (bloc gelé, matrice élargie). Code retiré puis refait. **KEEP** : l'architecture du premier
  jet (lecture des trois sortes reconnues au contenu, ASCII seul, `Score`/`Releves`, `comparer`,
  `en_texte`, codes 0/1/2/3, une barre lue dans deux fichiers = code 2, tests par ligne de
  matrice et extraits des vrais formats) ; copie de référence : copie de travail hors dépôt, effacée.
  **À corriger en plus** (constats de la passe 1, voir le journal) :
  1. toute erreur imprévue (écriture `--json`, `OSError`) sort en code 2, jamais en 1 ;
  2. recette sans ligne `FIN` : pas de barre `:rejeu` (absente), avertissement ; `FIN` sans
     « rejeu identique » : 0/1 ;
  3. section « fin » sans `SCORE` : avertissement ; une section relancée garde la dernière
     mesure, et efface aussi les `par_commande:*` de la précédente ;
  4. `SCORE <commande>` lu seulement dans la section `par_commande_reel` ;
  5. une seconde ligne `SCORE :` dans une recette : avertissement (la dernière compte, comme
     pour les essais) ;
  6. aucune barre comptée (toutes indicatives) : `à décider` ;
  7. même modèle nommé des deux côtés : avertissement ;
  8. fraction de total 0 ou justes > total : relevé illisible ;
  9. doc : exemple recopié de la vraie sortie (avertissements compris), code 2 = relevé
     illisible ou usage faux, la recette ancienne nommée d'après son fichier dit comme tel (ne
     plus écrire « jamais à son nom »), que faire d'un « à décider » (rejouer la mesure du
     parent, ou décision du propriétaire notée dans le plan du ré-entraînement) ;
  10. tests : priorité `refusé` sur `à décider` ; identité lue d'une recette ; nom tiré de la
      ligne `phrases :` (`phrases-gros.json` → `gros`, indicative) ; avertissement du nom de
      fichier ; section écrite deux fois ; ligne d'identité à adaptateur vide sans « ; » final ;
      fichier JSON effacé avant chaque lancement du test.

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment).
Verdicts : high 1, medium 7, low 14, false 0, maybe-false 0. intent_gap 2 (boucle), patch 10
(reportés au plan, refaits avec la boucle), defer 2, rejetés 8.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| Validation de deux jeux tirés différemment comparée comme une barre | high | intent_gap | `par_sorte` 253/25/22 contre 249/38/13 ; D1 |
| Code de fin non nul / « barre non tenue » sans effet | medium | intent_gap | `Score.code` jamais lu ; D2 |
| Erreur imprévue (écriture `--json`) sort en 1 = « refusé » | medium | patch | correction 1 |
| Recette interrompue lue comme rejeu perdu | medium | patch | correction 2 |
| Section relancée sans `SCORE` : la barre disparaît ; `par_commande:*` périmées | medium | patch | correction 3 |
| `SCORE <commande>` accepté hors de sa section | low | patch | correction 4 |
| Première ligne `SCORE :` d'une recette gagne en silence | low | patch | correction 5 |
| Toutes barres indicatives → installable | medium | patch | correction 6 |
| Même modèle des deux côtés sans avertissement | medium | patch | correction 7 |
| Fraction 0/0 ou justes > total acceptée | low | patch | correction 8 |
| Exemple de la doc sans la ligne `attention`, code 2 incomplet, « jamais à son nom » faux, rien sur « à décider » | low | patch | correction 9 |
| Tests manquants : priorité refusé/à décider, identité de recette, nom par `phrases :`, avertissements, section double, adaptateur vide, JSON périmé | medium | patch | correction 10 |
| Recette de même total mais fichier de phrases changé | low | defer | `deferred-work.md` : porter l'empreinte du fichier de phrases |
| Refus non branché sur l'installation (C2) | low | defer | les scripts de nuit sont hors dépôt ; à brancher au prochain script de nuit |
| Tolérance fixe d'une phrase quel que soit le total | low | reject | règle du ticket, bloc gelé ; D1 retire la barre où elle pesait le moins |
| Accents des chemins de modèle perdus (ASCII seul) | low | reject | noms des dossiers de modèles en ASCII ; aucun cas montré |
| `rapport.json` copié hors de son dossier mal nommé | low | reject | les rapports restent dans `ajuste-*` ; aucun cas montré |
| `modèles :` vers un dossier de plusieurs modèles | low | reject | la recette pointe toujours un dossier `ajuste-*` d'un modèle |
| GGUF des essais non relié à l'empreinte du rapport | low | reject | l'avertissement de modèles différents couvre le mélange de dossiers |
| Essai réel non figé par un test | low | reject | relevés hors dépôt (jamais commités) ; résultat consigné dans la doc |
| Barres `par_commande:*` jamais vues sur un relevé réel | low | reject | le livrable 1 n'a pas encore tourné ; ce soir sur les candidats |
| `ticket: '53'` ambigu | low | reject | convention des plans : l'`id` de l'entrée, pas la `ref` |

Passe 2 (même lentilles, après la boucle 1). Lignes de la passe 1 : reprises (`carried`), non
revérifiées. Verdicts nouveaux : high 0, medium 6, low 13, false 1, maybe-false 0. Corrigés 17
(patch, même implémenteur, plus un par le coordinateur), rejetés 3.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| `--json` égal à un relevé : le relevé est effacé avant lecture | medium | patch | code 2 avant tout effacement, test |
| JSON écrit avant l'affichage : verdict frais sur une sortie en 2 | medium | patch | texte d'abord, JSON en dernier par fichier provisoire renommé |
| Validation d'un seul côté comptée (absente / sans parent) malgré D1 | medium | patch | indicative, avertissement, test |
| Section sans `=== fin … ===` : seuil inconnu en silence, passe en `tenue` | medium | patch | avertissement ; seuil ok contre inconnu → `non comparable` |
| Modèles mélangés ou même modèle des deux côtés : `installable` possible | medium | patch | ensemble au plus `à décider` |
| Aucun test d'un JSON périmé effacé sur échec | medium | patch | test par appel direct de `main` |
| Deux `SCORE` dans une section : la première gagne en silence | low | patch | la dernière, avec avertissement |
| `SCORE` non reconnu ou `0/0` disparaît / rend tout illisible | low | patch | avertissements |
| Seuil de recette sur trois tournures figées | low | patch | `(barre N)` → `justes >= N`, tournures en repli |
| Nom de recette inconnu échappe à l'indicatif ; nombre de phrases annoncé non comparé | low | patch | avertissements |
| Fixture `FIN : rejeu différent` et `barre ratée` jamais écrites par la sonde | low | patch | vraies chaînes `rejeu NON identique`, `SOUS LA BARRE` |
| `VraisFormats.ESSAIS` dit « gardé » mais porte un code 101 inventé | low | patch | docstring : code ajouté pour le test |
| Seuil perdu sur totaux différents non testé (D2 avant les totaux) | low | patch | test |
| « Même composition » ne prouve pas « mêmes phrases » | low | patch | dit dans la doc |
| Chemin absolu du bloc-notes dans le plan | low | patch | « copie de travail hors dépôt, effacée » |
| Empreinte reportée ne couvrait que les recettes ; mise en forme de `deferred-work.md` | low | patch | entrée élargie aux essais GPU, ligne vide |
| Fichier JSON partiel en cas d'échec d'écriture | low | patch | fichier provisoire `.partiel`, renommé puis toujours effacé |
| Fin de recette d'infrastructure (`rejeu en échec`) comptée comme rejeu perdu | low | reject | barre à 0/1 ou absente, même verdict : la recette n'a pas prouvé son rejeu |
| `(chiffres)` dans le chemin des phrases tronque le nom | low | reject | chemins fixes du dépôt ; avertissement du nom inconnu en filet |
| Exemple de la doc contredit par la fixture 101 | false | reject | le vrai relevé porte `fin epic11_reel : 0` (vérifié) ; la fixture est maintenant dite inventée |
