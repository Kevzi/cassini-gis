---
name: Cassini
status: final
sources:
  - ../spec-cassini-mvp/spec-cassini-mvp.md
  - ../architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md
  - ../epic-facade-qgis/story-panneau-lancer-une-commande-du-registre-voir-son-resultat-l-plan.md
updated: 2026-11-08
---

# Cassini — Experience Spine

> Panneau ancré dans QGIS, en **fil de la demande** (direction B, 2026-11-08). Associé à `DESIGN.md`, dont il cite les jetons par leur nom. Convention de marquage, gardée pour l'historique : **[9.4]** = livré par la story 9.4 ; **[Epic 5]** = livré par l'agent local (epic 5) ; **[9.1–9.3]** = déjà en place avant. **[Fil]** = ce que change la refonte du 2026-11-08, à construire.
>
> Référence visuelle : [mockups/panneau-fil.html](mockups/panneau-fil.html) ([PNG](mockups/panneau-fil.png)), approuvée le 2026-11-08 ; elle illustre l'architecture d'information, les états Au repos, Composition, Plan proposé, Exécution, Résultat et Erreur d'étape, et le Flux 1. L'ancienne maquette [mockups/panneau.html](mockups/panneau.html) (2026-10-16) est remplacée. **Les deux spines l'emportent sur toute maquette en cas de conflit.**

## Foundation

Un seul panneau, dans QGIS (Qt via `qgis.PyQt`, extension séparée du noyau). Il est ancré à droite et peut être détaché en fenêtre flottante. Il n'a aucune logique métier : le noyau compose, valide et dicte ; le panneau affiche, demande et exécute les gestes QGIS (AD-4, AD-5). Une seule commande à la fois. Il suit le thème clair ou sombre de QGIS ; ses signes propres sont décrits dans `DESIGN.md` (logo, sarcelle, bouton principal, cartes, bulle). Zéro réseau, aucune ressource distante (AD-14).

Public : tout le monde, avec le langage naturel d'abord. L'utilisateur occasionnel sait ce qu'il veut mais pas où cliquer. Le cartographe averti passe par le mode expert. Aucune fonction à usage militaire dans ce panneau public.

La refonte du 2026-11-08 répond à trois gênes relevées sur le panneau réel à 340 px : des zones trop petites (plan, résultat, journal en cadres de quelques lignes), la demande perdue à l'envoi, et pas assez de repères sur la suite à donner (valider, attendre, relire). Elle ne change ni le noyau ni le protocole.

Glossaire : le **Journal de session** (libellé à l'écran : « Journal de la session ») est la liste affichée par le panneau, effacée à la fermeture de la session ; le **journal Cassini** est le fichier rejouable écrit par le seul noyau à côté du projet (AD-11). « Rejouer le journal » rejoue le second.

## Information Architecture

Une colonne, de haut en bas. Aucune navigation à onglets.

| Zone | Contenu | Livraison |
|---|---|---|
| En-tête | Logo, « Cassini », bouton Réglages | [9.4] |
| Réglages (bouton d'en-tête, ouvre une zone sous l'en-tête) | Langue FR / EN ; Modèle ; Autoriser une IA externe | [9.4] puis [Epic 5] ; inchangés |
| Bandeau d'état | Noyau absent, planté, protocole refusé, occupé, session en lecture seule, projet absent, IA externe autorisée ; actions Relancer / Choisir le noyau / Couper | [9.1–9.3] puis [9.4] ; inchangé |
| **Fil** | Prend toute la hauteur libre ; seul à défiler. Contient, dans l'ordre d'arrivée : | [Fil] |
| · Carte d'accueil | « Bonjour », consigne, exemples de demandes. Seule dans le fil au repos | [Fil] |
| · Bulle de demande | La phrase envoyée, à droite, avec « Vous · heure ». Reste visible | [Fil] |
| · Bulle « Mode expert » | Après un Lancer expert : « Mode expert : {libellé court de l'étape} », à la place de la bulle de demande | [Fil] |
| · Carte du plan | Plan proposé (étapes, « À vous », Valider le plan / Refuser, Détails techniques) ; pendant l'exécution, repère « Étape k sur n en cours » et Arrêter ; une fois fini, repliée en « Plan validé — n étapes › Voir » | [Epic 5], forme [Fil] |
| · Carte de question | Quand Cassini a besoin d'une précision : sa question et « À vous : répondez dans la saisie. » | [Epic 5], forme [Fil] |
| · Carte de résultat | Résultat en langage clair par étape, Ouvrir le dossier, Détails techniques ; en cas d'erreur, l'étape fautive, « À vous » et Reformuler la demande | [9.4], forme [Fil] |
| · Carte de refus | « Plan refusé. Reformulez votre demande. », « À vous » et Reformuler la demande | [Fil] |
| · Carte d'échec de composition | Message du noyau, « À vous » et Reformuler la demande | [Fil] |
| Repli Journal | « Journal de la session (n étapes) », replié par défaut ; contient le bouton **Rejouer le journal** | [9.4], place et rejeu [Fil] |
| Repli Mode expert | « Mode expert : lancer une commande », replié par défaut | [9.4], place [Fil] |
| Saisie épinglée | Champ en langage naturel et bouton Envoyer, toujours en bas, toujours visible | [Epic 5], place [Fil] |
| Pied hors ligne | « Cassini fonctionne sans connexion… » | [9.4] |

Ce que montre le fil est la **demande en cours** : la bulle, puis les cartes qui lui répondent. À l'envoi d'une nouvelle demande, l'échange précédent est **remplacé** dans le fil ; les demandes précédentes restent dans le Journal de session (memlog, 2026-11-08).

Déplié, le repli Journal ou Mode expert s'ouvre **sur place, au-dessus de la saisie** ; le fil rétrécit d'autant. Un clic sur la ligne du repli le referme (memlog, 2026-11-08).

Mode lecture seule : seconde fenêtre QGIS sur le même projet. Le panneau le dit dans le bandeau, ne laisse que **Rejouer le journal** (dans le repli Journal) et grise le reste (AD-11).

[NOTE FOR UX] Où s'affichent les demandes d'accord de l'IA externe (« Me demander avant chaque appel » : texte, Autoriser, Refuser) dans la nouvelle colonne : carte du fil, ligne d'état du Journal de session ou bandeau ? Le panneau actuel a une zone à part ; la refonte ne la place pas.

## Voice and Tone

Vouvoiement, phrases courtes et complètes, verbes simples, aucun point d'exclamation, peu de jargon. [NOTE FOR UX] le mot « noyau » est celui du projet et reste en l'état ; à reconsidérer si l'utilisateur occasionnel le juge trop technique. Jamais de code, d'expression ni de chemin absolu à l'écran (AD-9, AD-23). Une erreur dit ce qui s'est passé, puis quoi faire. Quand c'est à l'utilisateur d'agir, la phrase commence par « À vous : » et nomme le geste.

**Libellés d'étapes.** Une étape du plan, du résultat ou du journal se lit comme une courte phrase en langage courant, tirée de `summary.label` (« Zone tampon de 500 m autour du résultat de l'étape 1 »). La description du registre n'est jamais utilisée comme libellé d'étape (lot 1, commit 20a52f4) ; elle reste celle de la liste du mode expert.

| À faire | À éviter |
|---|---|
| « À vous : validez pour lancer, ou refusez pour reformuler. » | « Action requise ! » |
| « Le noyau Cassini est introuvable. » | « Erreur 127 » |
| « Terminée. » | « Succès ✓ » |
| « Zone tampon de 500 m autour des gares » | « Calcule une zone tampon autour des entités d'une couche vectorielle » |
| « Vous pouvez relancer. » | « Veuillez contacter l'administrateur. » |

Microcopy par état (FR / EN). La langue du panneau suit le réglage ; FR par défaut. [ASSUMPTION] les traductions anglaises des libellés marqués [Fil] sont proposées ici, non relues par le propriétaire.

| Élément ou état | FR | EN |
|---|---|---|
| Carte d'accueil, titre [Fil] | Bonjour | Hello |
| Carte d'accueil, consigne [Fil] | Décrivez en français ce que vous voulez faire sur les couches de votre projet. Je vous propose un plan, vous le validez, puis je l'exécute. | Describe in French what you want to do with the layers of your project. I propose a plan, you approve it, then I run it. |
| Exemples, titre [Fil] | Exemples de demandes | Example requests |
| Saisie, texte indicatif au repos | Décrivez ce que vous voulez faire sur vos données. | Describe what you want to do with your data. |
| Saisie, texte indicatif après un résultat [Fil] | Nouvelle demande… | New request… |
| Saisie, raison pendant la composition [Fil] | Patientez : un plan est en préparation. | Please wait: a plan is being prepared. |
| Saisie, raison plan proposé [Fil] | Validez ou refusez d'abord le plan ci-dessus. | First approve or reject the plan above. |
| Saisie, raison pendant l'exécution [Fil] | Patientez : une étape est en cours. | Please wait: a step is running. |
| Envoyer | Envoyer | Send |
| Bulle, en-tête [Fil] | Vous · {heure} | You · {time} |
| Bulle du mode expert [Fil] | Mode expert : {libellé court de l'étape} | Expert mode: {short step label} |
| Composition, titre [Fil] | Je prépare un plan… | Preparing a plan… |
| Composition, détail [Fil] | Cela prend en général quelques secondes. | This usually takes a few seconds. |
| Composition impossible, titre [Fil] [ASSUMPTION : formulation] | Je n'ai pas pu préparer de plan. | I could not prepare a plan. |
| Composition arrêtée [Fil] [ASSUMPTION : formulation] | Préparation arrêtée. | Preparation stopped. |
| Question, « À vous » [Fil] | À vous : répondez dans la saisie. | Your turn: answer in the input. |
| Plan, titre | Voici ce que je propose — {n} étapes | Here is what I propose — {n} steps |
| Plan, « À vous » [Fil] | À vous : validez pour lancer, ou refusez pour reformuler. | Your turn: approve to run, or reject to rephrase. |
| Valider / Refuser | Valider le plan / Refuser | Approve plan / Reject |
| Plan, titre pendant l'exécution [Fil] | Plan validé — {n} étapes | Plan approved — {n} steps |
| Repère d'avancement [Fil] | Étape {k} sur {n} en cours | Step {k} of {n} running |
| Carte du plan validé (repliée) [Fil] | Plan validé — {n} étapes · Voir | Plan approved — {n} steps · Show |
| Statuts d'étape | En attente / En cours… / Terminée / Erreur / Non lancée / Annulée | Pending / Running… / Done / Error / Not run / Cancelled |
| Résultat, titre [Fil] | Plan exécuté. Étapes terminées : {n}. | Plan done. Steps completed: {n}. |
| Résultat, clôture [Fil] | Les couches sont ajoutées au projet. | The layers are added to the project. |
| Ouvrir le dossier [Fil] | Ouvrir le dossier | Open folder |
| Erreur, titre [Fil] | Plan arrêté à l'étape {k}. | Plan stopped at step {k}. |
| Erreur, « À vous » [Fil] | À vous : reformulez, par exemple « {suggestion} ». | Your turn: rephrase, for example “{suggestion}”. |
| Erreur d'étape | Étape {n} en erreur : {message} | Step {n} failed: {message} |
| Plan annulé, titre [Fil] [ASSUMPTION : formulation] | Plan arrêté à l'étape {k} à votre demande. | Plan stopped at step {k} at your request. |
| Refus du plan | Plan refusé. Reformulez votre demande. | Plan rejected. Rephrase your request. |
| Refus, « À vous » [Fil] | À vous : reformulez votre demande. | Your turn: rephrase your request. |
| Reformuler la demande [Fil] | Reformuler la demande | Rephrase the request |
| Reformuler, détail [Fil] | Votre phrase revient dans la saisie, prête à être modifiée. | Your sentence goes back to the input, ready to edit. |
| Détails techniques | Détails techniques | Technical details |
| Journal, titre [Fil] | Journal de la session ({n} étapes) | Session log ({n} steps) |
| Journal vide [Fil] | Journal de la session (vide) | Session log (empty) |
| Rejouer le journal [Fil] | Rejouer le journal | Replay the log |
| Rejeu en cours [Fil] [ASSUMPTION : formulation] | Rejeu : étape {k} sur {n} en cours | Replay: step {k} of {n} running |
| Rejeu identique [Fil] [ASSUMPTION : formulation] | Rejeu terminé : résultat identique. | Replay done: identical result. |
| Rejeu différent [Fil] [ASSUMPTION : formulation] | Rejeu terminé : résultat différent à l'étape {k}. {message} | Replay done: different result at step {k}. {message} |
| Rejeu impossible [Fil] [ASSUMPTION : formulation] | Rejeu arrêté à l'étape {k} : {message} | Replay stopped at step {k}: {message} |
| Mode expert, titre | Mode expert : lancer une commande | Expert mode: run a command |
| Lancer / Arrêter | Lancer / Arrêter | Run / Stop |
| Commande indisponible | Indisponible : {raison} | Unavailable: {reason} |
| Étape non annulable | Arrêt impossible : cette étape ne peut pas être interrompue. | Stop unavailable: this step cannot be interrupted. |
| Résultat de `system.about` [ASSUMPTION : formulation] | Le noyau Cassini répond et son protocole est compatible. | The Cassini core is responding and its protocol is compatible. |
| Projet absent | Ouvrez ou enregistrez un projet QGIS : Cassini se lance avec le projet. | Open or save a QGIS project: Cassini starts with the project. |
| Démarrage | Démarrage du noyau… | Starting the core… |
| Noyau absent | Le noyau Cassini est introuvable. Installez le paquet Cassini, ou désignez le fichier. Rien n'est téléchargé. | The Cassini core cannot be found. Install the Cassini package, or point to the file. Nothing is downloaded. |
| Noyau planté | Le noyau s'est arrêté de façon inattendue. Votre projet n'est pas modifié. | The core stopped unexpectedly. Your project is unchanged. |
| Protocole refusé | Ce noyau et cette extension ne se comprennent pas (versions incompatibles). Installez des versions assorties. | This core and this extension do not understand each other (incompatible versions). Install matching versions. |
| Occupé | Une commande est déjà en cours. Attendez sa fin ou arrêtez-la. | A command is already running. Wait for it to finish or stop it. |
| Lecture seule | Ce projet est ouvert dans une autre fenêtre QGIS. Ici, vous pouvez seulement rejouer. | This project is open in another QGIS window. Here you can only replay. |
| Modèle absent | Aucun modèle utilisable dans le dossier des modèles. | No usable model in the models folder. |
| Réglages, langue | Langue | Language |
| Hors ligne | Cassini fonctionne sans connexion. Aucune donnée ne quitte votre poste. | Cassini works offline. No data leaves your computer. |

`{code}` est un identifiant `domaine.cause` (par exemple `analysis.crs_not_metric`), **jamais dans la vue principale** : il n'apparaît que sous « Détails techniques » ; `{message}` est le message français renvoyé par le noyau, montré en clair sous l'étape fautive. `{suggestion}` vient du noyau. Le pied hors ligne garde le texte de la spine ci-dessus ; la variante de la maquette (« Vos données restent sur votre poste. ») ne fait pas foi (memlog, 2026-11-08).

## Component Patterns

Comportement. Les spécifications visuelles sont dans `DESIGN.md` (Components). Les noms ci-dessous sont les mêmes dans les deux spines, et dans le même ordre : celui du panneau, de haut en bas.

| Composant | Usage | Règles de comportement |
|---|---|---|
| En-tête | Identité, accès aux Réglages | Logo clair ou sombre choisi en lisant la palette de QGIS. Réglages ouvre ou referme sa zone. |
| Réglages | Langue, modèle, IA externe | Inchangés. Changer la langue prend effet tout de suite et se mémorise. |
| Bandeau d'état | Situation du noyau ou de la session | Inchangé : un seul bandeau à la fois, sous l'en-tête, au-dessus du fil. Texte + icône + action. |
| Carte d'accueil [Fil] | Point de départ | Visible seulement quand le fil est vide. [ASSUMPTION] Les exemples sont du texte d'aide, non cliquables ; la maquette ne leur donne pas d'action. Disparaît à l'envoi d'une demande. |
| Bulle de demande [Fil] | Garder la demande sous les yeux | Créée à l'envoi avec la phrase exacte et l'heure ; la saisie se vide. Lecture seule : on ne l'édite pas en place (on reformule). Elle reste en tête du fil tant que les cartes lui répondent. [NOTE FOR UX] reste-t-elle collée en haut quand le fil défile sous un long plan ? |
| Bulle « Mode expert » [Fil] | Montrer ce que l'utilisateur a lancé en mode expert | Un Lancer remplace l'échange en cours comme une demande : la bulle « Mode expert : {libellé court de l'étape} » (libellé de `summary.label`), puis la carte de résultat (memlog, 2026-11-08). Pas de carte du plan : une commande experte est une seule étape. |
| Carte de composition [Fil] | Dire que Cassini travaille | Apparaît sous la bulle dès l'envoi, jusqu'à l'arrivée du plan, d'une question ou d'un échec. Arrêter interrompt la préparation sans tuer le noyau ; la carte devient alors « Préparation arrêtée. » (état « Composition arrêtée »). |
| Carte d'échec de composition [Fil] | Le noyau n'a pas pu composer de plan | Remplace la carte de composition : titre, message français du noyau, code sous « Détails techniques », « À vous : reformulez votre demande. » et Reformuler la demande. Comme aujourd'hui (`panel.py`, échec du plan), la phrase d'origine revient dans la saisie si celle-ci est vide. |
| Carte de question [Fil] | Le noyau demande une précision au lieu d'un plan | Carte de Cassini sous la bulle, qui remplace la carte de composition : la question en clair, puis « À vous : répondez dans la saisie. ». La phrase d'origine revient dans la saisie pour être complétée, selon la règle `agentview.request_to_restore` : seulement si la saisie est vide, jamais par-dessus un texte tapé. La réponse s'envoie par Envoyer, comme une nouvelle demande. |
| Carte du plan | Étapes proposées par le noyau | Lecture seule : ni édition d'étape, ni de paramètre (AD-5, AD-23). Valider ou refuser **en bloc**. Pendant l'exécution, le noyau dicte une étape à la fois ; l'étape en cours porte le `step-marker` et le mot « En cours… », et la carte affiche le repère d'avancement et Arrêter. Une fois le plan fini (réussi ou arrêté), la carte se replie en « Plan validé — n étapes » ; « Voir » la redéplie, « Masquer » la replie [ASSUMPTION : libellé du repli inverse]. |
| Repère d'avancement [Fil] | Dire où en est l'exécution | Une seule ligne « Étape k sur n en cours », mise à jour à chaque étape dictée. Pas de barre d'étapes, pas de pourcentage. Disparaît quand la carte se replie. |
| Ligne « À vous » [Fil] | Dire que c'est à l'utilisateur d'agir | Présente **si et seulement si** l'utilisateur doit agir : plan proposé, question de Cassini, erreur d'étape, plan refusé, composition impossible. Toujours suivie de l'action qu'elle nomme, et cette action reçoit le focus (voir Accessibility Floor). Jamais pendant la composition ni l'exécution. |
| Carte de résultat | Sortie du plan ou de la commande | Langage clair d'abord : une ligne par étape, son libellé et ce qu'elle a produit (couche, nombre d'entités, fichier par son seul nom). Tout résumé chiffré s'affiche selon le gabarit unique d'AD-27 : une ligne par élément de `rows` (son `label`, puis chaque valeur avec son unité) ; si `truncated` est vrai, une ligne finale dit que la liste est tronquée et que la couche produite garde tout [ASSUMPTION : formulation de cette ligne]. « Ouvrir le dossier » ouvre le dossier des sorties dans l'explorateur du système [ASSUMPTION : dossier des sorties à côté du projet, sans afficher son chemin]. « Détails techniques » replié, jamais en code, expression ou chemin absolu. En erreur : voir État « Erreur d'étape ». |
| Carte de refus [Fil] | Après Refuser | Remplace la carte du plan : « Plan refusé. Reformulez votre demande. », « À vous : reformulez votre demande. », Reformuler la demande (bouton principal). Disparaît à l'envoi suivant. |
| Détails techniques | Ce qu'un cartographe averti veut vérifier | Repli fermé par défaut, dans la carte du plan, la carte de résultat (y compris en erreur) et la carte d'échec de composition. Contient l'identifiant de commande de chaque étape et, en erreur, le code `domaine.cause` ; jamais de code, d'expression ni de chemin absolu (AD-9, AD-23). Nom accessible « Détails techniques, réduit / développé ». [NOTE FOR UX] Y montrer aussi les paramètres de chaque étape ? Non tranché. |
| Repli Journal / Mode expert | Ranger le journal et le mode expert hors du fil | Les deux replis sont fermés par défaut, au-dessus de la saisie, le Journal d'abord. Déplié, un repli s'ouvre sur place, au-dessus de la saisie, et le fil rétrécit d'autant ; un clic le referme. Repli Journal : son compte d'étapes est dans le libellé, mis à jour en direct ; il contient le bouton **Rejouer le journal**. Repli Mode expert : son état est mémorisé. |
| Journal de session (ligne d'état) | Trace de ce qui a été lancé depuis l'ouverture du panneau | Une ligne par étape ou commande (statut icône + mot, libellé, heure), y compris les appels marqués « IA externe ». Le Journal de session garde les demandes précédentes, remplacées dans le fil. Il ne relit pas le journal du noyau. Il s'efface à la fermeture de la session. |
| Rejouer le journal [Fil] | Refaire toutes les étapes du journal Cassini | Bouton natif dans le repli Journal (memlog, 2026-11-08). Demande au noyau de rejouer **toutes** les étapes de son journal, par le même chemin que l'exécution d'origine (AD-11) ; le panneau n'enchaîne rien lui-même. Pendant le rejeu, le bouton devient Arrêter si c'est annulable. À la fin, il dit si le résultat est identique. Seul geste laissé en session en lecture seule (rejeu sans écriture, AD-11). Grisé, avec sa raison, pendant une commande, un plan ou un journal vide [ASSUMPTION]. [NOTE FOR UX] Le rejeu passe-t-il aussi par le fil (bulle et carte) ou reste-t-il dans le repli ? Non tranché. |
| Mode expert | Pour qui sait ce qu'il veut | Contenu du repli Mode expert (voir Repli Journal / Mode expert). Liste des commandes du catalogue reçu à `session.hello` ; indisponibles grisées avec raison. Choisir une commande affiche son formulaire. |
| Formulaire par schéma | Paramètres d'une commande | Champs engendrés depuis le schéma pour un sous-ensemble (texte, nombre, entier, booléen, énumération). Propriété hors du sous-ensemble : un champ JSON brut le remplace. Le panneau ne valide rien : le noyau valide. JSON brut invalide : message local, rien n'est envoyé. |
| Autoriser / Refuser (IA externe) | Accord avant chaque appel, si « Me demander avant chaque appel » est allumé | Chaque appel externe attend la réponse et ne part qu'après Autoriser ; Refuser renvoie une erreur claire à l'IA externe (AD-29). Texte « L'IA externe demande à lancer : {description} Paramètres : {params} » en clair. Annonce « L'IA externe demande votre accord. ». Boutons natifs, aucun n'est le bouton principal [ASSUMPTION]. Emplacement : voir la note d'Information Architecture. |
| Saisie principale | Demande en langage naturel | Épinglée en bas, toujours visible. Entrée envoie, Maj+Entrée saute une ligne. Désactivée pendant la composition, tant qu'un plan attend validation et pendant l'exécution, avec la raison à la place du texte indicatif. Ce qui y est tapé n'est jamais écrasé par Cassini. |
| Bouton principal [Fil] | Le geste attendu maintenant | Porté par Envoyer, Valider le plan ou Reformuler la demande, **un seul visible à la fois** : Envoyer au repos et après un résultat ; Valider le plan quand un plan attend ; Reformuler la demande après un refus, une erreur ou un échec de composition (Envoyer redevient alors un bouton natif). Pendant une commande experte, aucun : Envoyer est grisé. Son anneau de focus est le trait extérieur décrit dans `DESIGN.md` (Components, `focus-ring-primary`). Désactivé, il perd sa couleur et dit pourquoi. |
| Reformuler la demande [Fil] | Repartir de la même phrase | Voir Interaction Primitives. Après le geste, le bouton disparaît et Envoyer redevient le bouton principal [ASSUMPTION]. |
| Bouton Lancer / Arrêter | Une seule commande à la fois (AD-5) | Lancer remplace l'échange en cours du fil (voir Bulle « Mode expert »). Lancer devient Arrêter pendant l'exécution si la commande est annulable ; sinon Arrêter reste grisé avec sa raison. Le même Arrêter figure dans la carte de composition et dans la carte du plan en exécution. |
| Anneau de focus | Montrer le contrôle actif | Suit le focus clavier partout dans le panneau ; s'ajoute au focus natif quand celui-ci est trop discret. Sur le bouton principal, il prend la forme du trait extérieur `focus-ring-primary` (memlog, 2026-11-08). |
| Pied hors ligne | Rassurer | Toujours visible, information et non panne. |

## State Patterns

États du fil (nouveaux ou changés le 2026-11-08). Dans la colonne Focus, seule la ligne Exécution est décidée (memlog, 2026-11-08) ; les autres déplacements sont [ASSUMPTION].

| État | Fil | « À vous » | Bouton principal | Saisie | Focus |
|---|---|---|---|---|---|
| Au repos | Carte d'accueil seule ; journal « (vide) » | — | Envoyer | Active, texte indicatif | Saisie [ASSUMPTION] |
| Composition | Bulle + carte « Je prépare un plan… » avec Arrêter | — | aucun (Envoyer grisé) | Grisée : « Patientez : un plan est en préparation. » | Arrêter [ASSUMPTION] |
| Composition impossible | Bulle + carte d'échec de composition (message du noyau) | « reformulez votre demande » | Reformuler la demande | Active ; la phrase d'origine y revient si elle est vide | Reformuler la demande [ASSUMPTION] |
| Composition arrêtée | Bulle + carte « Préparation arrêtée. » | — | Envoyer | Active | Saisie [ASSUMPTION] |
| Question de Cassini | Bulle + carte de question | « répondez dans la saisie » | Envoyer | Active ; la phrase d'origine y revient si elle est vide | Saisie [ASSUMPTION] |
| Plan proposé | Bulle + carte du plan (étapes « En attente ») | « validez pour lancer, ou refusez pour reformuler » | Valider le plan | Grisée : « Validez ou refusez d'abord le plan ci-dessus. » | Valider le plan [ASSUMPTION] |
| Exécution | Bulle + carte du plan « Plan validé — n étapes », repère « Étape k sur n en cours », étape en cours marquée, Arrêter | — | aucun (Envoyer grisé) | Grisée : « Patientez : une étape est en cours. » | Titre de la carte du plan (repère « Étape k sur n en cours ») après Valider le plan (memlog, 2026-11-08) |
| Résultat | Bulle + carte du plan validé (repliée) « Plan validé — n étapes › Voir » + carte de résultat | — | Envoyer | Active, « Nouvelle demande… » | Titre de la carte de résultat, puis la saisie au Tab [ASSUMPTION] |
| Commande experte en cours | Bulle « Mode expert : {libellé} » + carte « En cours… » avec Arrêter si annulable | — | aucun (Envoyer grisé) | Grisée : « Patientez : une étape est en cours. » | Reste sur Arrêter du mode expert [ASSUMPTION] |
| Commande experte, résultat | Bulle « Mode expert : {libellé} » + carte de résultat | — | Envoyer | Active, « Nouvelle demande… » | Titre de la carte de résultat [ASSUMPTION] |
| Commande experte, erreur | Bulle « Mode expert : {libellé} » + carte de résultat en erreur (message français, code sous Détails techniques) | — [NOTE FOR UX] Reformuler n'a pas de phrase à rendre ; rien n'est proposé à la place | Envoyer | Active | Titre de la carte [ASSUMPTION] |
| Erreur d'étape | Bulle + carte du plan validé (repliée) + carte de résultat « Plan arrêté à l'étape k » : étapes terminées, étape fautive (icône, « Erreur », message français), suivantes « Non lancée » | « reformulez, par exemple … » | Reformuler la demande | Active ; Envoyer natif | Reformuler la demande [ASSUMPTION] |
| Plan refusé | Bulle + carte « Plan refusé. Reformulez votre demande. » | « reformulez votre demande » | Reformuler la demande | Active | Reformuler la demande [ASSUMPTION] |
| Plan annulé (Arrêter) | Comme un plan fini : bulle + carte du plan validé (repliée) « Plan validé — n étapes › Voir » + carte de résultat « Plan arrêté à l'étape k à votre demande. » : étapes terminées, étape interrompue « Annulée », suivantes « Non lancée » | — [NOTE FOR UX] faut-il proposer Reformuler après un arrêt volontaire ? | Envoyer | Active | Titre de la carte de résultat [ASSUMPTION] |
| Rejeu en cours | Inchangé ; dans le repli Journal : « Rejeu : étape k sur n en cours », Arrêter si annulable | — | aucun (Envoyer grisé) | Grisée : « Patientez : une étape est en cours. » | Reste sur le bouton du rejeu [ASSUMPTION] |
| Rejeu identique | Repli Journal : « Rejeu terminé : résultat identique. » | — | Envoyer | Active | Inchangé |
| Rejeu différent | Repli Journal : « Rejeu terminé : résultat différent à l'étape k. » et ce qui diffère (par exemple un fichier d'entrée changé, AD-11) | — | Envoyer | Active | Inchangé |
| Rejeu impossible | Repli Journal : « Rejeu arrêté à l'étape k : {message} » ; la commande qui manque est nommée (AD-11) ; code sous Détails techniques | — | Envoyer | Active | Inchangé |

Une erreur d'étape suit AD-21 : étape fautive, code `domaine.cause` (seulement sous Détails techniques), message français, ligne d'état « Erreur » au Journal de session ; les étapes suivantes ne partent pas ; aucune exception ne remonte jusqu'à l'utilisateur ; le panneau est de nouveau libre.

États du noyau et de la session (inchangés, portés par le bandeau) :

| État | Où | Traitement |
|---|---|---|
| Projet absent | Bandeau | Texte « Ouvrez ou enregistrez un projet… ». Saisie et mode expert grisés. |
| Démarrage | Bandeau | « Démarrage du noyau… », sans bouton. |
| Prêt | Panneau | Pas de bandeau. État « Au repos » du fil. |
| Noyau absent | Bandeau | Message « introuvable », bouton **Choisir le noyau**. Saisie et mode expert grisés. Jamais de téléchargement (AD-14). |
| Noyau planté / mort | Bandeau | Message « s'est arrêté », bouton **Relancer**. Si une étape tournait, elle passe à « Erreur » dans la carte et au Journal de session. Panneau grisé. |
| Protocole refusé | Bandeau | Message « versions incompatibles », bouton **Choisir le noyau**. Pas de Relancer. Panneau grisé. |
| Occupé | Saisie et mode expert | Envoyer et Lancer grisés, raison « Une commande est déjà en cours » (ou « L'IA externe travaille » pendant un appel externe, AD-29), Arrêter proposé. Le panneau reste réactif (aucune lecture bloquante). |
| Commande indisponible | Liste experte | Ligne grisée, raison en détail ; non lançable (AD-11). |
| Étape non annulable | Bouton Arrêter | Grisé, raison « cette étape ne peut pas être interrompue » (AD-12). On ne tue jamais le noyau pour annuler. |
| Session en lecture seule | Bandeau | Texte explicite ; seul **Rejouer le journal** reste actif (rejeu sans écriture) ; Envoyer, Lancer, saisie et modèle grisés (AD-11). |
| Modèle absent | Saisie + Réglages | « Aucun modèle utilisable dans le dossier des modèles. » Saisie et Envoyer grisés avec ce texte pour raison [ASSUMPTION] ; le mode expert reste utilisable. |
| Hors ligne | Pied du panneau | Information, pas une panne. Jamais de bandeau d'erreur réseau. |
| IA externe autorisée [5.24–5.26] | Bandeau + Journal de session | Interrupteur « Autoriser une IA externe » dans les réglages, éteint par défaut et jamais rallumé seul. Allumé : bandeau « Une IA externe peut agir sur ce projet », bouton **Couper**. Chaque appel externe s'ajoute au Journal de session en direct (commande, paramètres en clair, résultat, durée), marqué « IA externe », avec **Arrêter** (AD-12, AD-29). Case « Me demander avant chaque appel », éteinte par défaut : allumée, chaque appel attend **Autoriser** / **Refuser**. |
| IA externe déjà branchée ailleurs | Réglages | Interrupteur grisé, « Une autre session QGIS accepte déjà l'IA externe. » (AD-29) |

## Interaction Primitives

- **Envoyer** une demande : Entrée ou bouton Envoyer. La phrase devient la bulle de demande, la saisie se vide, la composition commence ; l'échange précédent quitte le fil et reste au Journal de session. [Fil]
- **Valider / refuser le plan en bloc** : le plan s'affiche en entier, on l'accepte ou on le refuse en bloc. Pas d'édition d'étape ni de paramètre. [Epic 5]
- **Reformuler la demande** : remet la phrase de la bulle dans la saisie, curseur à la fin, focus dans la saisie ; l'utilisateur la modifie puis Envoie. Proposé après un refus, une erreur d'étape ou un échec de composition. **Ne remplace jamais du texte déjà tapé** : si la saisie n'est pas vide, rien n'est écrasé [ASSUMPTION : le bouton reste sans effet et le dit, plutôt que d'ajouter la phrase à la suite]. C'est la même règle que le comportement actuel `agentview.request_to_restore`, qui rend la phrase d'origine après une question de Cassini seulement si la saisie est vide. [Fil]
- **Voir / masquer** la carte du plan validé (repliée) : redéplie ou replie la carte du plan. [Fil]
- **Lancer** une commande experte : choisir, remplir, Lancer. Une seule à la fois. L'échange en cours du fil est remplacé par la bulle « Mode expert : … » puis le résultat. [9.4], fil [Fil]
- **Rejouer le journal** : dans le repli Journal ; le noyau rejoue toutes les étapes et dit si le résultat est identique (AD-11). [Fil]
- **Arrêter** : envoie `request.cancel` au noyau pour la requête en cours ; ne tue jamais le processus. Proposé seulement quand c'est annulable. [9.4]
- **Relancer** : relance le noyau après plantage. Le panneau repart de l'état prêt ; le journal de session est conservé. [9.4]
- **Clavier** : Tab suit l'ordre du fil puis des replis et de la saisie (voir Accessibility Floor) ; Entrée active le bouton ; Échap referme le repli ou la zone Réglages ; Espace déplie ou replie.
- **Interdits** : confirmation modale en cascade, glisser-déposer, infobulle seule porteuse d'information, tout contenu qui s'ouvre sans geste, tout texte de Cassini qui écrase la saisie.

## Accessibility Floor

Aucune exigence écrite dans les sources (AD) : le plancher ci-dessous est la décision du memlog.

- **Tout au clavier** : chaque fonction du panneau, y compris Voir / masquer la carte du plan validé, les replis Journal et Mode expert, Rejouer le journal, Autoriser / Refuser, la liste des commandes, les Réglages, Envoyer, Valider, Refuser, Reformuler, Lancer, Arrêter et Relancer.
- **Ordre de tabulation = ordre du fil** : en-tête, bandeau, bulle, cartes dans l'ordre d'arrivée (et dans chaque carte : étapes, « À vous », boutons, Détails techniques), replis Journal puis Mode expert, saisie, Envoyer, pied.
- **Focus sur l'action « À vous »** [ASSUMPTION : aucune entrée du memlog ne le décide] : quand une ligne « À vous » apparaît, le focus va à l'action qu'elle nomme (Valider le plan, Reformuler la demande, ou la saisie pour une question).
- **Après Valider le plan**, le bouton disparaît : le focus va au titre de la carte du plan, qui porte le repère « Étape k sur n en cours » (memlog, 2026-11-08). Il n'est jamais volé pendant que l'utilisateur tape dans la saisie [ASSUMPTION : dans ce cas, l'annonce seule suffit].
- **Annonces** par une zone vivante (`QAccessible`), sans déplacer le focus : plan proposé (« Un plan est proposé. »), début de chaque étape (« Étape k sur n en cours »), fin du plan, erreur d'étape avec son message, question de Cassini, échec de composition, début et fin du rejeu (identique ou non), demande d'accord de l'IA externe, changement d'état du noyau, IA externe allumée ou coupée.
- **Noms accessibles** sur chaque contrôle, lus avec rôle et état (« Arrêter, bouton, désactivé : cette étape ne peut pas être interrompue »). La bulle se lit « Votre demande, {heure} : {phrase} » ; la carte du plan validé (repliée) « Plan validé, n étapes, réduit » ; les replis avec leur compte.
- **Jamais la couleur seule** : chaque statut a une icône et un mot ; la ligne « À vous » commence par ces mots ; le bouton principal se reconnaît aussi à sa place et à son libellé ; la bulle se reconnaît aussi à « Vous · heure ».
- Contraste : surfaces et texte hérités du thème QGIS ; contrastes propres à Cassini dans `DESIGN.md` (Colors). `{colors.accent}` n'est jamais le seul porteur de texte ou d'état.
- Mise à l'échelle des polices du système respectée ; les cartes grandissent, aucun libellé tronqué sans repli.
- [NOTE FOR UX] Pas de cible tactile imposée : poste de bureau. À confirmer.

## Key Flows

### Flux 1 — La phrase de démo (Camille, urbaniste en bureau d'études, Villeneuve-le-Roi)

Statut : **[Epic 5]**, forme **[Fil]**. Couvre : CAP-4, CAP-5. Camille a ouvert ses fichiers (PLU, parcelles, gares) dans un projet QGIS ; elle connaît son métier, pas les menus d'analyse.

1. Camille ouvre le panneau Cassini, ancré à droite. Le fil montre la carte « Bonjour » et ses exemples ; la saisie est en bas, Envoyer en sarcelle.
2. Elle écrit : « Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à Villeneuve-le-Roi et sors-moi la carte en PDF. » et appuie sur Entrée.
3. Sa phrase reste à l'écran, en bulle à droite (« Vous · 19:49 »). Dessous, « Je prépare un plan… » ; la saisie dit « Patientez : un plan est en préparation. »
4. La carte « Voici ce que je propose — 4 étapes » arrive, en phrases courantes (sélection des zones constructibles, zone tampon de 500 m autour des gares, intersection avec les parcelles, export PDF). Dessous : « À vous : validez pour lancer, ou refusez pour reformuler. » Le focus est sur **Valider le plan**.
5. Camille lit le plan en entier et valide.
6. La carte dit « Étape 1 sur 4 en cours », puis 2, 3, 4 ; l'étape en cours porte le marqueur sarcelle, les précédentes « Terminée » ; le repli Journal compte les étapes.
7. **Climax :** la carte du plan se replie en « Plan validé — 4 étapes › Voir », et la carte de résultat dit en clair le nombre de parcelles retenues et que la carte PDF est prête ; les couches sont dans son projet QGIS. Sa demande est toujours au-dessus, la saisie propose « Nouvelle demande… ».

Échec — erreur d'étape : la zone tampon échoue parce que la couche des gares n'est pas en mètres. La carte dit « Plan arrêté à l'étape 2. », l'étape 2 porte « Erreur » et le message français, l'étape 3 « Non lancée ». « À vous : reformulez, par exemple “reprojette d'abord les gares en Lambert-93”. » Le focus est sur **Reformuler la demande** ; Camille clique, sa phrase revient dans la saisie, elle ajoute la reprojection et renvoie.

Branche — question : au lieu d'un plan, Cassini pose une question (par exemple, quelle couche porte le PLU). La carte de question s'affiche sous la bulle avec « À vous : répondez dans la saisie. » ; la phrase d'origine revient dans la saisie, Camille la complète et Envoie.

Échec — refus : Camille clique **Refuser**. « Plan refusé. Reformulez votre demande. » et **Reformuler la demande** ; même suite.

### Flux 2 — Arrêter une commande lente (Marc, cartographe, ce que livre 9.4)

Statut : **[9.4]**. Couvre : CAP-13, story 9.4. Marc est dans QGIS ; il a déplié le mode expert et choisi la commande de test lente `test.qgis_probe`.

1. Marc clique **Lancer**. L'échange en cours du fil est remplacé par la bulle « Mode expert : {libellé de l'étape} » et une carte « En cours… ». Le bouton devient **Arrêter** ; le repli Journal passe à « (1 étape) » et sa ligne d'état dit « En cours… ».
2. Il constate que la commande dure ; QGIS reste fluide, il peut zoomer sur la carte.
3. Il clique **Arrêter**.
4. Le panneau envoie `request.cancel` au noyau (le processus n'est pas tué).
5. **Climax :** la ligne d'état du Journal de session passe à « Annulée », le bouton redevient **Lancer**, et Marc peut lancer autre chose tout de suite.

Échec : sur une commande non annulable, **Arrêter** reste grisé avec « Arrêt impossible : cette étape ne peut pas être interrompue. » Marc attend la fin.

### Flux 3 — Noyau absent (Inès, étudiante, première installation)

Statut : **[9.1–9.3]** pour le comportement, **[9.4]** pour la forme. Couvre : CAP-13, story 9.4.

1. Inès installe l'extension depuis le dépôt de QGIS et ouvre un projet.
2. Le panneau affiche, dans le bandeau sous l'en-tête : « Le noyau Cassini est introuvable. Installez le paquet Cassini, ou désignez le fichier. Rien n'est téléchargé. » La saisie et le mode expert sont grisés.
3. Elle installe le paquet Cassini à part (hors du panneau).
4. Elle clique **Choisir le noyau**, désigne le fichier, ou **Relancer**.
5. **Climax :** le bandeau disparaît après « Démarrage du noyau… », le fil montre la carte « Bonjour » et la saisie devient active.

Échec : le fichier choisi n'est pas un noyau valide ou parle un autre protocole → bandeau « versions incompatibles » avec **Choisir le noyau**.

### Flux 4 — Une IA externe travaille sous les yeux de l'utilisateur (Camille, ChatGPT pour ordinateur)

Statut : **[5.24–5.27]**. Couvre : CAP-8, CAP-7 (rejeu). Camille a QGIS ouvert sur son projet Villeneuve-le-Roi et ChatGPT pour ordinateur branché sur `cassini mcp`.

1. Dans les réglages du panneau, elle allume **Autoriser une IA externe**. Le bandeau l'annonce.
2. Dans ChatGPT, elle tape la phrase de démo.
3. Les appels s'ajoutent au Journal de session, marqués « IA externe », et le compte du repli Journal monte ; les couches produites apparaissent sur la carte.
4. Elle déplie le repli Journal et clique **Rejouer le journal**.
5. **Climax :** le PDF est écrit, le rejeu se termine sur « Rejeu terminé : résultat identique. », et Camille a vu chaque étape.

Échec : Camille envoie une demande dans le panneau pendant que ChatGPT travaille → l'appel arrivé second reçoit « Une commande est déjà en cours » ; rien n'est mis en file. Elle clique **Couper** : le canal se ferme, ChatGPT reçoit une erreur claire.

[NOTE FOR UX] Les appels de l'IA externe vont au Journal de session, pas au fil (qui reste celui des demandes tapées dans le panneau) ; à confirmer, la maquette ne montre pas ce cas.

## Responsive & Platform

Poste de bureau, QGIS 3.44 à 4.99 (Qt 5 ou Qt 6 via `qgis.PyQt`). Pas de mobile.

| Mode | Comportement |
|---|---|
| Ancré à droite (par défaut) | Colonne d'environ 340 px, redimensionnable. Fil au centre, seul à défiler ; replis et saisie épinglés en bas. |
| Étroit (< 300 px) | La bulle prend toute la largeur ; Valider le plan et Refuser passent l'un sous l'autre ; les libellés passent à la ligne ; pas de défilement horizontal. |
| Détaché (fenêtre flottante) | Même contenu, sans changement de logique. Peut être large : la colonne reste unique. [ASSUMPTION] pas de mise en deux colonnes au MVP. |

Thème : clair, sombre ou personnalisé de QGIS, suivi sans réglage propre ; les couleurs de Cassini choisies en lisant la palette (voir `DESIGN.md`). Langue : FR par défaut, EN au choix dans Réglages.

## Hors périmètre et trous

- [NOTE FOR UX] Pas de bouton d'export du journal de session ; le journal Cassini, rejoué par « Rejouer le journal », reste un fichier du noyau que le panneau n'exporte pas.
- [NOTE FOR UX] Choix du modèle GGUF dans Réglages : la façon d'ouvrir et de valider un modèle n'est pas redessinée par la refonte du fil.
- [NOTE FOR UX] Formulation exacte des résultats en langage clair par commande, hors résumé chiffré (qui suit le gabarit d'AD-27) : à définir avec le registre (une phrase par commande).
- [NOTE FOR UX] Valeurs sombres de `DESIGN.md` approchées : à relever sur un vrai QGIS Night Mapping au moment du code.
