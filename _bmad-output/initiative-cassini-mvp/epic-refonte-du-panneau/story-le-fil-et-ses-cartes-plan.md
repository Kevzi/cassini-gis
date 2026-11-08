---
title: 'Le fil et ses cartes'
type: 'feature'
ticket: '1'
created: '2026-11-08'
status: 'built'
baseline_revision: '8e2707e876262e887f4990d33b923e860b930963'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/ux-panneau-cassini/DESIGN.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/ux-panneau-cassini/EXPERIENCE.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-refonte-du-panneau/epic-refonte-du-panneau.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le panneau empile ses zones au même niveau, enferme plan, résultat et journal dans des cadres de 4-5 lignes qui défilent alors que le bas reste vide, et efface la demande dès l'envoi (captures `%TEMP%\capture-panneau\340-*.png`).

**Approach:** réorganiser le panneau en « fil de la demande » tel que la spine UX le fixe (`DESIGN.md`, `EXPERIENCE.md`, maquette `mockups/panneau-fil.html`) : en-tête, bandeau, fil défilant de cartes, replis Journal et Mode expert au-dessus d'une saisie épinglée en bas, pied hors ligne ; couleurs propres à Cassini en paire claire/sombre lue dans la palette de QGIS.

## Boundaries & Constraints

**Always:** l'ordre et le contenu des zones suivent `EXPERIENCE.md` Information Architecture ; les jetons et composants suivent `DESIGN.md` (bouton principal `accent-strong`, bulle, carte 1 px sans ombre, un seul bouton principal visible) ; la bulle de la demande reste affichée de l'envoi au résultat, et une nouvelle demande remplace l'échange précédent ; les cartes grandissent avec leur contenu, seul le fil défile, aucun cadre à hauteur fixe ; le plan validé se replie en « Plan validé — n étapes » avec « Voir » ; Journal et Mode expert sont des replis fermés par défaut, avec leur compte, qui s'ouvrent sur place au-dessus de la saisie ; fonds, textes et bordures restent ceux de la palette de QGIS, seules les couleurs de Cassini ont une paire claire et sombre, choisie d'après la palette comme le logo ; Qt par `qgis.PyQt` seulement, QGIS 3.44 et 4.x ; toute décision d'affichage testable sans Qt vit dans un module pur testé (comme `agentview.py`, `runlog.py`), `panel.py` ne fait que poser les widgets ; tous les comportements existants restent (Valider/Refuser, Arrêter, bandeau, lecture seule, IA externe, Réglages, mode expert, Détails techniques, Ouvrir le dossier, historique rechargé).

**Never:** pas de ligne « À vous », de repère « Étape k sur n », de bouton « Reformuler la demande » ni de nouveau déplacement de focus : c'est 12.2 ; pas de bouton « Rejouer le journal » ni de commande experte dans le fil : c'est 12.3 ; aucun changement du noyau, du protocole, du générateur ou du modèle ; aucune police ni image embarquée, aucune ressource distante ; pas de widget dessiné à la main quand un widget Qt avec feuille de style suffit.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Repos | noyau prêt, rien envoyé | carte d'accueil (exemples nommés comme tels), replis fermés, saisie en bas avec Envoyer principal | — |
| Composition | phrase envoyée | bulle de la demande + carte « Je prépare un plan… » avec Arrêter ; saisie grisée avec sa raison | composition impossible : carte d'échec sous la bulle |
| Plan proposé | plan de 4 étapes | bulle + carte du plan, étapes « En attente », Valider le plan (principal) / Refuser | — |
| Exécution | étape 2 en cours | carte du plan, étape 2 marquée (marqueur sarcelle + mot « En cours »), Arrêter | — |
| Résultat | plan fini | carte du plan validé repliée (« Voir » la rouvre), carte de résultat, Ouvrir le dossier, compte du journal à jour | — |
| Erreur d'étape | étape 2 en erreur | étape « Erreur » + message, suivantes « Non lancée », code seulement sous Détails techniques | — |
| Question / refus | le noyau pose une question ou refuse | carte de question ou de refus sous la bulle | — |
| Nouvelle demande | une 2e phrase après un résultat | le fil ne montre que la nouvelle bulle ; l'ancienne reste au journal | — |
| Thème sombre | palette de QGIS sombre | couleurs de Cassini en version sombre, logo sombre | — |

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/panel.py` -- `_build` l. 251-525 (une `QScrollArea` qui contient tout, zones empilées), `render` l. 757, `_render_result` l. 879, `_render_agent` l. 926, `_render_buttons` l. 1041, `_send` l. 1170, `_approve`/`_reject` l. 1190-1205, `_logo` l. 246 (choix clair/sombre par `palette().window().color().lightness() < 128` : même règle pour les couleurs de Cassini), `_Fold` l. 166 (repli existant, à réutiliser pour Journal/Mode expert/plan validé), `RequestEdit` l. 98. La saisie (`self.input`, `setMaximumHeight(72)`), `plan_list`, `log_list` et `result_text` y sont aujourd'hui dans le même défilement : à séparer (fil défilant, replis et saisie hors du défilement).
- `qgis-plugin/cassini_qgis/agentview.py` -- état de l'agent (`phase` : IDLE, generating, PROPOSED, executing…), `step_line`, `controls(...)` (boutons actifs et raisons) : à réutiliser ; un nouveau module pur (ex. `thread.py`) peut dire quelles cartes montrer pour un état, testable sans Qt.
- `qgis-plugin/cassini_qgis/runlog.py`, `summary.py` -- journal et phrases (lot 1, `summary.label`) : ne pas changer leur sortie.
- `qgis-plugin/cassini_qgis/texts.py` -- tous les textes FR/EN ; les nouveaux libellés de `EXPERIENCE.md` Voice and Tone (accueil, exemples, « Plan validé — {n} étapes », « Voir », « Journal de la session ({n} étapes) », etc.) s'y ajoutent.
- `qgis-plugin/cassini_qgis/assets/` -- logos clair et sombre existants.
- `qgis-plugin/tests/test_panel_logic.py`, `test_agent_view.py` -- tests sans Qt ; `panel.py` n'est pas testé directement (les modules purs le sont).
- `qgis-plugin/tests/check_extension.py` -- garde de l'extension (imports interdits, `qgis.PyQt` seul).
- `C:\Users\kevin\AppData\Local\Temp\capture-panneau\capture.py` -- capture du vrai panneau par état (`QT_QPA_PLATFORM=windows`, largeur en argument) ; à étendre pour les six états, l'erreur (phrase qui échoue sur Annecy : « Trouve les parcelles à moins de 500 m d'une gare », gares en 4326) et le thème sombre (palette sombre posée sur l'application, ou la feuille de style « Night Mapping » de QGIS `C:\Program Files\QGIS 3.44.15\apps\qgis-ltr\resources\themes\`).

## Tasks & Acceptance

**Execution:**
- [x] `qgis-plugin/cassini_qgis/thread.py` (nouveau, pur) et son test -- cartes du fil selon l'état de l'agent, compte des replis, choix des couleurs de Cassini selon la luminosité de la palette.
- [x] `qgis-plugin/cassini_qgis/panel.py` -- nouvelle mise en page : fil défilant de cartes, replis et saisie épinglée hors du défilement, styles de `DESIGN.md` (feuilles de style limitées aux éléments propres à Cassini).
- [x] `qgis-plugin/cassini_qgis/texts.py` -- libellés FR/EN du fil.
- [x] `qgis-plugin/tests/` -- tests des modules purs pour chaque ligne de la matrice.
- [x] `capture.py` étendu, captures refaites à 340 px (six états clairs + plan proposé et résultat sombres) et regardées.

**Acceptance Criteria:**
- Given QGIS 3.44 et le projet Annecy, when on prend les captures à 340 px, then chaque état ressemble à `mockups/panneau-fil.html` : bulle visible de l'envoi au résultat, aucun cadre à hauteur fixe, aucun texte coupé, saisie en bas.
- Given la palette sombre, when on capture le plan proposé et le résultat, then le bouton principal, la bulle et les marqueurs prennent leur version sombre et restent lisibles.
- Given le dépôt, when on lance les tests et la garde de l'extension, then tout passe.

## Implementation Notes

- `thread.py` (pur) décide des cartes (`cards`), des titres et lignes du plan et du résultat, du texte indicatif de la saisie, du bouton principal unique, du compte du repli Journal, de la hauteur d'un repli déplié (au plus la moitié du panneau) et de la paire de couleurs (`is_dark`, même règle que le logo). Les feuilles de style ne fixent que les couleurs de Cassini ; fonds et cadres des cartes passent par `palette(base)` / `palette(mid)`.
- `panel.py` : en-tête, bandeaux, fil (`QScrollArea`, seul à défiler), replis Journal et Mode expert, saisie épinglée, pied. Plus aucun `QListWidget` ni `QPlainTextEdit` à hauteur fixe dans le fil : étapes en lignes (`_StepRow`), détails techniques en libellé à chasse fixe.
- La composition n'est pas annulable côté client (`client.propose`) : Arrêter y est grisé avec sa raison (« Arrêt impossible : la préparation d'un plan ne peut pas être interrompue. »), sans toucher au protocole.
- Plan exécuté : « Ouvrir le dossier » vise le dernier fichier produit par une étape (avant, le bouton n'apparaissait que pour une commande experte).
- Une commande experte remplace l'échange (bulle « Mode expert : … ») et oublie le plan fini précédent du fil (`agent.reset`), qui reste au journal.
- Captures (`%TEMP%\capture-panneau\340-*.png`) : le thème sombre est simulé par le style Fusion et une palette sombre (le style natif Windows ignore la palette pour les boutons) ; valeurs sombres de `DESIGN.md` gardées, non relevées sur un vrai Night Mapping.
- Corrections après relecture (2026-11-08) :
  - une ligne de raison revient sous la saisie grisée quand du texte cache le texte indicatif (`thread.reason_line`) ;
  - « Vous · heure » n'a plus de largeur fixe (alignement à droite seul) ;
  - le fil se cale sur la carte la plus récente en défilant juste assez, garde la bulle à l'écran quand bulle et carte tiennent ensemble, et lit la géométrie après la mise en page ;
  - la carte repliée dit l'issue du plan, icône et mot (`thread.folded_summary` : exécuté, en erreur, arrêté, y compris après un arrêt du noyau) ; nom accessible au singulier (`plan_folded_name_one`) ;
  - les étapes du plan prennent le focus, sont dans l'ordre de tabulation et ouvrent leur détail par Entrée ou Espace ;
  - les deux replis ouverts se partagent au plus la moitié du panneau, le fil gardant toujours `thread.MIN_THREAD` pixels (`thread.fold_heights`) ; les replis ont une politique de taille « Maximum » et leurs mises en page sont refaites tout de suite (sinon un écart restait sous la ligne du repli) ;
  - la cible de « Ouvrir le dossier » (`thread.shown_result`) et la forme des lignes d'étapes (5 éléments, même forme pour le plan et le résultat) passent dans `thread.py` ;
  - l'Arrêter grisé de la carte de résultat montre sa raison à côté ;
  - bouton principal : survol, appui et focus (trait de 2 px à la couleur du texte du thème, `palette(window-text)`). QSS n'a pas de contour décalé : le trait borde le bouton au lieu d'en être séparé par 2 px de fond de fenêtre comme le veut `focus-ring-primary`. Les couleurs de survol et d'appui ne sont pas fixées par `DESIGN.md` (choisies : plus sombre en clair, plus clair en sombre) ;
  - Échap reprend son comportement d'avant (repli Mode expert seul) ;
  - un arrêt du noyau pendant la composition ou un plan proposé efface l'échange : le fil repart de la carte d'accueil.

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture quick) — high 0, medium 6, low 9, false 0, maybe-false 0 ; corrigés 12, rejetés 3.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Raison de la saisie grisée invisible quand du texte est tapé | medium | patch | ligne `input_reason` rendue par `thread.reason_line` |
| Largeur fixe de « Vous · heure » qui coupe les cartes quand la barre de défilement apparaît | medium | patch | largeur fixe retirée |
| Défilement qui peut cacher la bulle, géométrie lue avant la mise en page | medium | patch | défilement minimal après activation de la mise en page |
| Plan replié toujours « ✔ » même après erreur, arrêt ou panne du noyau | medium | patch | `thread.folded_summary`, quatre cas testés |
| Détail d'une étape au clavier perdu | medium | patch | focus, Entrée/Espace, ordre de tabulation |
| Bouton principal sans état de focus, survol ni appui | medium | patch | trait 2 px `window-text` au focus (pas de décalage possible en QSS, noté) |
| Deux replis ouverts qui écrasent le fil | low | patch | `thread.fold_heights`, fil ≥ 120 px |
| Cible de « Ouvrir le dossier » et lignes d'étapes décidées dans Qt | low | patch | `thread.shown_result`, forme unique des lignes |
| Raison d'Arrêter grisé absente de la carte de résultat | low | patch | `result_stop_reason` |
| Pas de singulier pour le plan replié | low | patch | `plan_folded_name_one` |
| Échap déplace le focus vers le Journal (réservé à 12.2) | low | patch | comportement d'avant rétabli |
| Bulle orpheline après un arrêt du noyau | low | patch | l'échange est effacé |
| Titre anglais « Examples to try » au lieu de « Example requests » | low | rejeté | « requests » fait échouer la garde anti-réseau ; la spine sera alignée |
| Branche `[RESULT]` sans bulle jamais atteinte | low | rejeté | inatteignable aujourd'hui ; aucun effet pour l'utilisateur |
| Ligne « Repos » de la matrice couverte à moitié par les tests | low | rejeté | état des replis posé par Qt, vérifié sur captures |

Écarts gardés hors constat : la commande experte dans le fil (prévue en 12.3) et « Ouvrir le dossier » après un plan, construits ici et gardés.

## Verification

**Commands:**
- `python3 -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python3 -I qgis-plugin/tests/check_extension.py` -- expected: propre

**Manual checks (if no CLI):**
- Copier l'extension (`rsync -a --delete --exclude __pycache__ qgis-plugin/cassini_qgis/ /mnt/c/Users/kevin/AppData/Roaming/QGIS/QGIS3/profiles/default/python/plugins/cassini_qgis/`), copier le projet Annecy en `annecy/projet-capture-panneau.qgz`, lancer `powershell.exe -NoProfile -Command '$env:QT_QPA_PLATFORM="windows"; & "C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat" "$env:TEMP\capture-panneau\capture.py" 340'`, regarder chaque PNG, puis archiver le projet dans `_archives/sondes/`.
