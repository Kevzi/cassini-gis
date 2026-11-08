---
title: 'Guidage : À vous, repère d''avancement et Reformuler'
type: 'feature'
ticket: '2'
created: '2026-11-08'
status: 'built'
baseline_revision: '3b495a5a2cd5cc7929d73cbabd74f27fa12ff6cf'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/ux-panneau-cassini/EXPERIENCE.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/ux-panneau-cassini/DESIGN.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-refonte-du-panneau/story-le-fil-et-ses-cartes-plan.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le fil de 12.1 montre la demande et les cartes, mais ne dit pas quoi faire ensuite : pas de ligne « À vous », pas de repère d'avancement pendant l'exécution, et après une erreur, un refus ou une composition impossible il faut retaper toute la phrase.

**Approach:** ajouter au fil le guidage que fixe `EXPERIENCE.md` (State Patterns, Component Patterns, Interaction Primitives, Accessibility Floor) : la ligne « À vous : … » dans la carte où l'utilisateur doit agir, le repère « Étape k sur n en cours » dans la carte du plan, le bouton « Reformuler la demande » qui remet la phrase dans la saisie, et le focus et les annonces de chaque état.

## Boundaries & Constraints

**Always:** textes, états et règles de focus tels que les écrit `EXPERIENCE.md` (les déplacements marqués `[ASSUMPTION]` y sont la règle à suivre) ; la ligne « À vous » apparaît pour le plan proposé, la question, l'erreur d'étape, le refus et la composition impossible, et jamais en même temps qu'une autre ; « Reformuler la demande » est le seul bouton principal quand il est montré (Envoyer redevient natif), il remet la dernière demande dans la saisie sans jamais écraser un texte déjà tapé (même règle que `agentview.request_to_restore`), puis met le focus dans la saisie ; une question du noyau remet aussi la phrase dans la saisie, comme aujourd'hui ; une commande experte en erreur n'a pas de Reformuler ; après « Valider le plan », le focus va au titre de la carte du plan ; chaque changement d'état est annoncé (`_announce`) sans voler le focus pendant que l'utilisateur tape ; la logique (quel texte, quel bouton, où va le focus) vit dans `thread.py` ou `agentview.py`, testée sans Qt.

**Never:** pas de bouton « Rejouer le journal » (12.3) ; pas de changement du noyau, du protocole, du générateur ni du modèle ; pas de changement de la mise en page de 12.1 au-delà de ce que le guidage ajoute dans les cartes.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Plan proposé | plan de 4 étapes | « À vous : validez pour lancer, ou refusez pour reformuler. » ; focus sur Valider le plan | — |
| Exécution | étape 2 sur 4 | titre de la carte « Plan validé — 4 étapes », repère « Étape 2 sur 4 en cours » ; après Valider, focus sur ce titre | — |
| Erreur d'étape | étape 2 en erreur | ligne « À vous : reformulez… » et « Reformuler la demande » (principal) ; focus dessus | — |
| Refus | Refuser cliqué | carte de refus, « À vous », Reformuler | — |
| Composition impossible | le noyau ne compose pas de plan | carte d'échec, « À vous », Reformuler | — |
| Question | le noyau pose une question | « À vous : répondez dans la saisie. » ; phrase remise dans la saisie si elle est vide ; focus dans la saisie | — |
| Reformuler, saisie vide | clic | la phrase revient dans la saisie, focus dans la saisie | — |
| Reformuler, saisie remplie | texte déjà tapé | rien n'est écrasé ; le panneau le dit (texte de `EXPERIENCE.md`) | — |
| Commande experte en erreur | Lancer qui échoue | carte d'erreur sans Reformuler | — |

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/thread.py` -- module pur de 12.1 : `cards`, `plan_title`, `approved_title`, `folded_summary`, `plan_rows`, `result_title`, `reason_line`, `shown_result`, bouton principal unique ; y ajouter « À vous », repère d'avancement, présence de Reformuler et cible du focus par état.
- `qgis-plugin/cassini_qgis/agentview.py` l. 57, 91, 231 -- `restore_request` (question du garde-fou) et `request_to_restore(view, last_request, current_input)` : règle « ne jamais écraser » à réutiliser pour Reformuler.
- `qgis-plugin/cassini_qgis/panel.py` -- cartes posées par 12.1 (`_render_thread`, `_render_result`, `_render_agent`), `_send`, `_approve`, `_reject`, `_last_request`, `_announce` ; ajouter la ligne « À vous » (composant `act-cue` de `DESIGN.md`), le repère (`progress-cue`) et le bouton Reformuler dans les cartes.
- `qgis-plugin/cassini_qgis/texts.py` -- libellés FR/EN de `EXPERIENCE.md` Voice and Tone (« À vous : … », « Étape {k} sur {n} en cours », « Reformuler la demande », texte quand la saisie est déjà remplie).
- `qgis-plugin/tests/test_thread.py`, `test_agent_view.py`, `test_panel_logic.py` -- tests sans Qt.
- `C:\Users\kevin\AppData\Local\Temp\capture-panneau\capture.py` -- captures de 12.1 (six états, erreur sur Annecy avec « Trouve les parcelles à moins de 500 m d'une gare », gares en 4326) ; à étendre : après l'erreur, clic sur Reformuler (capture), puis une phrase qui réussit (« Fais la carte des pentes du MNT à 25 m en pourcentage »).

## Tasks & Acceptance

**Execution:**
- [x] `qgis-plugin/cassini_qgis/thread.py` (+ `agentview.py` si besoin) et tests -- « À vous », repère, Reformuler, cible du focus et annonce par état, règle « ne jamais écraser ».
- [x] `qgis-plugin/cassini_qgis/panel.py` -- poser la ligne « À vous », le repère et Reformuler dans les cartes ; focus et annonces.
- [x] `qgis-plugin/cassini_qgis/texts.py` -- libellés FR/EN.
- [x] `capture.py` étendu, captures refaites et regardées.

**Acceptance Criteria:**
- Given QGIS 3.44 et le projet Annecy, when la phrase des gares échoue à l'étape du tampon puis qu'on clique Reformuler, then la phrase revient dans la saisie avec le focus, et une phrase qui réussit ensuite remplace l'échange (captures).
- Given le dépôt, when on lance les tests et la garde de l'extension, then tout passe.

## Implementation Notes

- `thread.py` (pur) : `guidance` (carte qui porte la ligne « À vous », sa clé, présence de Reformuler ; une seule carte à la fois), `cue_text`, `focus_target` (Valider, titre de la composition, titre du plan, Reformuler, saisie, titre du résultat), `may_move_focus` (seulement si le focus est déjà dans le panneau, jamais pendant la frappe), `rephrase` (règle « ne jamais écraser »), `progress_text` (chiffres en demi-gras), `plan_title_name`, `outcome_announcement`, `failed_announcement` ; `primary(c, rephrase)` rend « rephrase » quand Reformuler est montré. `agentview.restorable` porte la règle commune ; `request_to_restore` l'appelle.
- `panel.py` : lignes `cassiniCue` dans les cartes du plan, de question, d'échec, de refus et de résultat ; Reformuler (principal) et sa ligne `meta` dans les cartes d'échec, de refus et de résultat ; repère sous le titre du plan ; titres du plan, du résultat et de la composition focalisables (anneau sarcelle, dans la tabulation). Le focus se déplace une fois par état, lu avant que les cartes ne se cachent. Annonces : étape k sur n, refus, échec de composition et erreur d'étape avec leur message, fin du plan.
- Écarts assumés : pendant la composition, le focus va au titre de la carte, Arrêter étant grisé (non annulable) ; après Reformuler, la ligne « À vous » reste et le focus reste dans la saisie ; si la saisie contient déjà la phrase (rendue après un échec de composition), Reformuler ne dit pas « rien n'est remplacé ». Texte « saisie remplie » non écrit par `EXPERIENCE.md` : « La saisie contient déjà du texte : rien n'est remplacé. Videz-la pour y remettre votre phrase. » (EN proposé). `{suggestion}` : le noyau n'en envoie pas, la consigne générale « reformulez votre demande » s'affiche ; le cas avec suggestion est prêt (`last["suggestion"]`). La question ne remet la phrase que pour le garde-fou, comme avant.
- Captures (`%TEMP%\capture-panneau\340-*.png`, focus imprimé) : repos → saisie, composition → titre, plan proposé → Valider, exécution → titre « Plan validé — 2 étapes » + « Étape 1 sur 2 en cours », erreur → Reformuler, 6b → phrase dans la saisie avec le focus, puis la phrase des pentes remplace l'échange. Projet archivé dans `donnees-test-cassini/_archives/sondes/capture-panneau-12-2/`.
- Correction d'un défaut de 12.1 relevé pendant 12.2 : lignes d'étape coupées (mot d'état « En atte », texte rogné en hauteur) sur l'écran mis à l'échelle à 150 %. Cause : les libellés de `_StepRow` étaient créés sans parent, leur police résolue pour l'écran principal (96 ppp, 11 px) et gardée une fois posés dans la carte, puis peinte à 144 ppp. Ils sont maintenant créés dans la ligne, elle-même créée dans sa carte, et les styles des lignes passent par la feuille de la carte (propriété `current`) ; aucune hauteur ni largeur fixe. Vérifié en mesurant la police des libellés dans le vrai panneau (17 px à 144 ppp, comme le reste) et par les captures, police normale (`planche-12-2-relecture.png`) et police +2 pt (`capture.py 340 grand`, `planche-12-2-340-grand.png`).
- Corrections après relecture (2026-11-08) :
  - un détail d'étape cliqué ne retire plus la ligne « À vous » ni Reformuler (échec, refus, erreur d'étape), et le clic restyle le bouton principal ;
  - plan proposé : pas de ligne « À vous » pendant un refus ni quand Valider et Refuser sont grisés (`guidance(..., controls)`) ;
  - le focus n'est retenu qu'une fois posé : un Valider grisé (IA externe au travail) le reçoit au rendu suivant ;
  - Reformuler : la phrase à rendre est choisie dans `thread.rephrase` (bulle, sinon dernière demande), curseur à la fin même si la phrase était déjà là, et la note « saisie remplie » s'efface à la frappe ;
  - le titre du plan perd « Un plan est proposé. » à la validation ; le repère est annoncé par sa propre ligne, lu une seule fois sur le titre ;
  - tabulation du résultat : titre, Reformuler, saisie, Envoyer, puis le reste de la carte et les replis ;
  - repère d'avancement décalé sous le texte du titre (logo 16 px + 6 px) ;
  - test « au plus une ligne À vous » sur tous les états du fil, la carte qui la porte étant affichée.

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture quick) — high 0, medium 5, low 7, false 0, maybe-false 0 ; corrigés 10, rejetés 2. Choix faits seul par Claude cette nuit (autorisation du propriétaire du 2026-11-08), à relire au réveil.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Clic sur une étape après une erreur : « À vous » et Reformuler disparaissent | medium | patch | `guidance` ne dépend plus du détail cliqué |
| Focus enregistré avant d'être posé : jamais reporté sur un bouton grisé | medium | patch | cible enregistrée une fois le focus posé |
| « validez… ou refusez… » quand aucune action n'est possible | medium | patch | `guidance(..., controls)`, deux tests |
| Choix de la phrase à remettre décidé dans le panneau | medium | patch | `thread.rephrase`, testé |
| Test « une seule ligne À vous » qui ne le vérifiait pas | medium | patch | parcours de 13 états |
| Curseur au début après Reformuler | low | patch | curseur en fin |
| Description accessible périmée du titre du plan | low | patch | effacée à Valider, progression lue une fois |
| Ordre de tabulation de l'état Résultat | low | patch | titre, Reformuler, saisie, Envoyer, puis le reste |
| Repère d'avancement non aligné sur le titre | low | patch | marge de 22 px |
| Note « saisie remplie » restée après effacement | low | patch | effacée à la frappe |
| Focus de la composition sur le titre au lieu d'Arrêter | low | rejeté | Arrêter est grisé (composition non annulable) et ne peut pas prendre le focus |
| Phrase remise seulement pour une question du garde-fou | low | rejeté | l'intention gelée dit « comme aujourd'hui » |

Hors relecture : défaut de 12.1 (lignes d'étapes coupées à 144 dpi, libellés créés sans parent) corrigé ici.

## Verification

**Commands:**
- `python3 -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python3 -I qgis-plugin/tests/check_extension.py` -- expected: propre

**Manual checks (if no CLI):**
- Copier l'extension dans le profil QGIS (`rsync -a --delete --exclude __pycache__ qgis-plugin/cassini_qgis/ /mnt/c/Users/kevin/AppData/Roaming/QGIS/QGIS3/profiles/default/python/plugins/cassini_qgis/`), copier le projet Annecy en `annecy/projet-capture-panneau.qgz`, lancer `capture.py 340` sous le Python de QGIS (`QT_QPA_PLATFORM=windows`), regarder les PNG, archiver le projet dans `_archives/sondes/`.
