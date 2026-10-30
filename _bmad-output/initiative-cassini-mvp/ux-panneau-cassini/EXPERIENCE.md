---
name: Cassini
status: final
sources:
  - ../spec-cassini-mvp/spec-cassini-mvp.md
  - ../architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md
  - ../epic-facade-qgis/story-panneau-lancer-une-commande-du-registre-voir-son-resultat-l-plan.md
updated: 2026-10-16
---

# Cassini — Experience Spine

> Panneau ancré dans QGIS. Associé à `DESIGN.md`. Convention de marquage : **[9.4]** = livré par la story 9.4 ; **[Epic 5]** = attend l'agent local (epic 5), conçu ici pour que le dessin n'ait pas à être refait ; **[9.1–9.3]** = déjà en place.

## Foundation

Un seul panneau, dans QGIS (Qt via `qgis.PyQt`, extension séparée du noyau). Il est ancré à droite et peut être détaché en fenêtre flottante. Il n'a aucune logique métier : le noyau compose, valide et dicte ; le panneau affiche, demande et exécute les gestes QGIS (AD-4, AD-5). Une seule commande à la fois. Il suit le thème clair ou sombre de QGIS ; son seul signe propre est le logo (globe de bureau) et l'accent sarcelle. Zéro réseau, aucune ressource distante (AD-14).

Public : tout le monde, avec le langage naturel d'abord. L'utilisateur occasionnel sait ce qu'il veut mais pas où cliquer. Le cartographe averti passe par le mode expert. Aucune fonction à usage militaire dans ce panneau public.

Ce que la story 9.4 livre : le lanceur de commandes expert (liste du catalogue, formulaire par schéma), le résultat, le journal de la session, l'arrêt, la langue FR/EN. Ce que l'epic 5 livre : la saisie en langage naturel active, le plan à valider, le modèle.

## Information Architecture

Une colonne, de haut en bas. Aucune navigation à onglets.

| Zone | Contenu | Livraison |
|---|---|---|
| En-tête | Logo, « Cassini », bouton Réglages | [9.4] |
| Bandeau d'état | Noyau absent, planté, protocole refusé, occupé, session en lecture seule, projet absent ; actions Relancer / Choisir le noyau | [9.1–9.3] puis [9.4] pour la forme |
| Saisie principale | Champ en langage naturel, bouton Envoyer. Visible mais désactivée : « Disponible avec l'agent » | Visible [9.4] ; active [Epic 5] |
| Plan | Étapes numérotées, chacune avec statut ; boutons Valider le plan / Refuser | [Epic 5] |
| Résultat | Résumé en langage clair + repli « Détails techniques » | [9.4] |
| Journal | Liste des étapes de la session : icône + mot d'état + libellé + heure | [9.4] (session seulement) |
| Mode expert (repliable, replié par défaut) | Liste des commandes du catalogue, formulaire engendré depuis le schéma, Lancer, Arrêter | [9.4] |
| Réglages (bouton d'en-tête, ouvre une zone sous l'en-tête) | Langue FR / EN ; Modèle (grisé : « Choisi avec l'agent ») | Langue [9.4] ; modèle grisé [9.4], actif [Epic 5] |

Mode lecture seule : seconde fenêtre QGIS sur le même projet. Le panneau le dit dans le bandeau, ne laisse que le rejeu et grise le reste (AD-7, AD-17).

→ Composition : `mockups/panneau.html` (trois états côte à côte). Le présent texte l'emporte en cas de conflit.

## Voice and Tone

Vouvoiement, phrases courtes et complètes, verbes simples, aucun point d'exclamation, peu de jargon. [NOTE FOR UX] le mot « noyau » est celui du projet et reste en l'état ; à reconsidérer si l'utilisateur occasionnel le juge trop technique. Jamais de code, d'expression ni de chemin absolu à l'écran (AD-9, AD-23). Les erreurs disent ce qui s'est passé, puis quoi faire.

| À faire | À éviter |
|---|---|
| « Disponible avec l'agent. » | « Fonction bientôt disponible !!! » |
| « Le noyau Cassini est introuvable. » | « Erreur 127 » |
| « Terminée. » | « Succès ✓ » |
| « Vous pouvez relancer. » | « Veuillez contacter l'administrateur. » |

Microcopy par état (FR / EN). La langue du panneau suit le réglage ; FR par défaut.

| Élément ou état | FR | EN |
|---|---|---|
| Saisie, texte indicatif (désactivée) | Décrivez ce que vous voulez faire sur vos données. | Describe what you want to do with your data. |
| Saisie, raison | Disponible avec l'agent. | Available with the agent. |
| Mode expert, titre | Mode expert : lancer une commande | Expert mode: run a command |
| Lancer | Lancer | Run |
| Arrêter | Arrêter | Stop |
| En cours | En cours… | Running… |
| Terminée | Terminée | Done |
| Annulée | Annulée | Cancelled |
| Erreur | Erreur | Error |
| Résultat, titre | Résultat | Result |
| Détails techniques | Détails techniques | Technical details |
| Journal, titre | Journal de cette session | This session's log |
| Journal vide | Rien n'a encore été lancé. | Nothing has run yet. |
| Résultat de `system.about` [ASSUMPTION : formulation] | Le noyau Cassini répond et son protocole est compatible. | The Cassini core is responding and its protocol is compatible. |
| Projet absent | Ouvrez ou enregistrez un projet QGIS : Cassini se lance avec le projet. | Open or save a QGIS project: Cassini starts with the project. |
| Démarrage | Démarrage du noyau… | Starting the core… |
| Noyau absent | Le noyau Cassini est introuvable. Installez le paquet Cassini, ou désignez le fichier. Rien n'est téléchargé. | The Cassini core cannot be found. Install the Cassini package, or point to the file. Nothing is downloaded. |
| Noyau planté | Le noyau s'est arrêté de façon inattendue. Votre projet n'est pas modifié. | The core stopped unexpectedly. Your project is unchanged. |
| Protocole refusé | Ce noyau et cette extension ne se comprennent pas (versions incompatibles). Installez des versions assorties. | This core and this extension do not understand each other (incompatible versions). Install matching versions. |
| Occupé | Une commande est déjà en cours. Attendez sa fin ou arrêtez-la. | A command is already running. Wait for it to finish or stop it. |
| Commande indisponible | Indisponible : {raison} | Unavailable: {reason} |
| Étape non annulable | Arrêt impossible : cette étape ne peut pas être interrompue. | Stop unavailable: this step cannot be interrupted. |
| Erreur d'étape | Étape {n} en erreur ({code}) : {message} | Step {n} failed ({code}): {message} |
| Lecture seule | Ce projet est ouvert dans une autre fenêtre QGIS. Ici, vous pouvez seulement rejouer. | This project is open in another QGIS window. Here you can only replay. |
| Hors ligne | Cassini fonctionne sans connexion. Aucune donnée ne quitte votre poste. | Cassini works offline. No data leaves your computer. |
| Réglages, langue | Langue | Language |
| Réglages, modèle (grisé) | Modèle : choisi avec l'agent. | Model: chosen with the agent. |
| Plan, titre [Epic 5] | Voici ce que je propose. | Here is what I propose. |
| Valider / Refuser [Epic 5] | Valider le plan / Refuser | Approve plan / Reject |
| Refus du plan [Epic 5] | Plan refusé. Reformulez votre demande. | Plan rejected. Rephrase your request. |

`{code}` est un identifiant `domaine.cause` (par exemple `qgis.cancelled`) ; `{message}` est le message français renvoyé par le noyau. Les descriptions des commandes viennent du registre, déjà bilingues.

## Component Patterns

Comportement. Les spécifications visuelles sont dans `DESIGN.md.Components`.

| Composant | Usage | Règles de comportement |
|---|---|---|
| Saisie principale | Demande en langage naturel | [9.4] visible, désactivée, raison affichée à côté. [Epic 5] Entrée envoie, Maj+Entrée saute une ligne ; désactivée tant qu'un plan attend validation ou qu'une commande tourne. |
| Plan [Epic 5] | Étapes proposées par le noyau | Lecture seule : ni édition d'étape, ni de paramètre. Valider ou refuser en bloc (AD-5, AD-23). Une étape à la fois est dictée ; l'étape en cours porte le marqueur. |
| Résultat | Sortie de la dernière commande ou étape | Langage clair d'abord. Les « Détails techniques » sont repliés et jamais affichés en code, expression ou chemin absolu. |
| Journal de session | Trace de ce qui a été lancé depuis l'ouverture du panneau | Une ligne par étape ou commande : statut (icône + mot), libellé, heure. Ne relit pas le journal du noyau (celui du panneau est celui de la session). S'efface à la fermeture de la session. |
| Mode expert | Pour qui sait ce qu'il veut | Replié par défaut, état mémorisé. Liste des commandes du catalogue reçu à `session.hello` ; indisponibles grisées avec raison. Choisir une commande affiche son formulaire. |
| Formulaire par schéma | Paramètres d'une commande | Champs engendrés depuis le schéma pour un sous-ensemble (texte, nombre, entier, booléen, énumération). Propriété hors du sous-ensemble : un champ JSON brut le remplace. Le panneau ne valide rien : le noyau valide. JSON brut invalide : message local, rien n'est envoyé. |
| Bouton Lancer / Arrêter | Une seule commande à la fois (AD-5) | Lancer devient Arrêter pendant l'exécution si la commande est annulable ; sinon Arrêter reste grisé avec sa raison. |
| Bandeau d'état | Situation du noyau ou de la session | Un seul bandeau à la fois, en haut. Texte + icône + action (Relancer ou Choisir le noyau). |
| Réglages | Langue, modèle | Changer la langue prend effet tout de suite et se mémorise. Le choix du modèle est grisé avec sa raison jusqu'à l'epic 5. |

## State Patterns

| État | Où | Traitement |
|---|---|---|
| Projet absent | Bandeau | Texte « Ouvrez ou enregistrez un projet… ». Saisie et mode expert grisés. |
| Démarrage | Bandeau | « Démarrage du noyau… », sans bouton. |
| Prêt | Panneau | Pas de bandeau. Mode expert disponible. [Epic 5] saisie active. |
| Noyau absent | Bandeau | Message « introuvable », bouton **Choisir le noyau**. Saisie et mode expert grisés. Jamais de téléchargement (AD-14). |
| Noyau planté / mort | Bandeau | Message « s'est arrêté », bouton **Relancer**. Si une commande tournait, sa ligne du journal passe à « Erreur ». Panneau grisé. |
| Protocole refusé | Bandeau | Message « versions incompatibles », bouton **Choisir le noyau**. Pas de Relancer (relancer le même noyau ne servirait à rien). Panneau grisé. |
| Occupé | Mode expert | Lancer grisé, raison « Une commande est déjà en cours », Arrêter proposé. Le panneau reste réactif (aucune lecture bloquante). |
| Commande indisponible | Liste experte | Ligne grisée, raison en détail ; non lançable (AD-11). |
| Étape non annulable | Bouton Arrêter | Grisé, raison « cette étape ne peut pas être interrompue » (AD-12). On ne tue jamais le noyau pour annuler. |
| Erreur d'étape | Résultat + journal | Étape fautive, code `domaine.cause`, message français, ligne « Erreur » au journal (AD-21). Aucune exception ne remonte jusqu'à l'utilisateur ; le panneau est de nouveau libre. |
| Annulée | Journal | Ligne « Annulée » ; bouton libéré ; QGIS reste réactif. |
| Session en lecture seule | Bandeau | Texte explicite, rejeu seulement ; Lancer, saisie et modèle grisés (AD-7, AD-17). |
| Saisie désactivée avant l'agent | Saisie principale | Champ visible et grisé, « Disponible avec l'agent. » [9.4] ; devient actif à l'epic 5. |
| Hors ligne | Pied du panneau | Information, pas une panne : « Cassini fonctionne sans connexion. » Jamais de bandeau d'erreur réseau. |
| IA externe autorisée [5.24–5.25] | Bandeau + fil des appels | Interrupteur « Autoriser une IA externe » dans les réglages, éteint par défaut et jamais rallumé seul. Allumé : bandeau « Une IA externe peut agir sur ce projet », bouton **Couper**. Chaque appel externe s'ajoute au journal du panneau en direct (commande, paramètres en clair, résultat, durée), marqué « IA externe », avec **Arrêter** (AD-12, AD-29). Case « Me demander avant chaque appel », éteinte par défaut : allumée, chaque appel attend **Autoriser** / **Refuser**. |
| IA externe déjà branchée ailleurs | Réglages | Interrupteur grisé, « Une autre session QGIS accepte déjà l'IA externe. » (AD-29) |

## Interaction Primitives

- **Lancer** une commande experte : choisir, remplir, Lancer. Une seule à la fois. [9.4]
- **Arrêter** : envoie `request.cancel` au noyau pour la requête en cours ; ne tue jamais le processus. Proposé seulement pour une commande annulable. [9.4]
- **Relancer** : relance le noyau après plantage. Le panneau repart de l'état prêt ; le journal de session est conservé. [9.4]
- **Valider / refuser le plan en bloc** : le plan s'affiche en entier, on l'accepte ou on le refuse en bloc, puis l'utilisateur reformule en langage naturel s'il refuse. Pas d'édition d'étape ni de paramètre. [Epic 5]
- **Clavier** : Tab suit l'ordre de lecture ; Entrée active le bouton ; Échap referme le repli ou la zone Réglages ; Espace déplie ou replie.
- **Interdits** : confirmation modale en cascade, glisser-déposer, infobulle seule porteuse d'information, tout contenu qui s'ouvre sans geste.

## Accessibility Floor

Aucune exigence écrite dans les sources (AD) : le plancher ci-dessous est la décision du memlog.

- Tout au clavier : chaque fonction du panneau, y compris le repli du mode expert, la liste des commandes, les Réglages, Lancer, Arrêter et Relancer.
- Nom accessible sur chaque contrôle (bouton Réglages, champ de saisie, liste des commandes, repli, bouton Arrêter), lu avec rôle et état (« Arrêter, bouton, désactivé : cette étape ne peut pas être interrompue »).
- Aucune information portée par la couleur seule : chaque statut a une icône et un mot.
- Contraste hérité du thème QGIS. Le sarcelle n'est jamais le seul porteur de texte ou d'état (voir `DESIGN.md`).
- Annonces : changement d'état du noyau et fin de commande annoncés par une zone vivante (`QAccessible`) ; l'ordre de tabulation suit l'ordre vertical.
- Mise à l'échelle des polices du système respectée ; aucun libellé tronqué sans repli.
- [NOTE FOR UX] Pas de cible tactile imposée : poste de bureau. À confirmer.

## Key Flows

### Flux 1 — La phrase de démo (Camille, urbaniste en bureau d'études, Villeneuve-le-Roi)

Statut : **[Epic 5]**. Camille a ouvert ses fichiers (PLU, parcelles, gares) dans un projet QGIS ; elle connaît son métier, pas les menus d'analyse.

1. Camille ouvre le panneau Cassini, ancré à droite. La saisie est active.
2. Elle écrit : « Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à Villeneuve-le-Roi et sors-moi la carte en PDF. »
3. Cassini répond : « Voici ce que je propose. » et affiche un plan en étapes numérotées (sélection des zones constructibles, zone tampon de 500 m autour des gares, intersection avec les parcelles, export PDF). Aucune formule, aucun chemin.
4. Camille lit le plan en entier et clique **Valider le plan**.
5. Les étapes s'exécutent une à une ; le marqueur sarcelle suit l'étape en cours, le journal se remplit (icône + « Terminée »).
6. **Climax :** le résultat annonce en langage clair le nombre de parcelles retenues et que la carte PDF est prête ; la carte est dans son projet QGIS. Le journal affiche quatre lignes « Terminée », rejouables.

Échec : une étape échoue (par exemple, couche manquante) → « Étape 2 en erreur (domaine.cause) : message français ». Les étapes suivantes ne partent pas. Camille reformule ou corrige ses données. Si elle refuse le plan : « Plan refusé. Reformulez votre demande. »

### Flux 2 — Arrêter une commande lente (Marc, cartographe, ce que livre 9.4)

Statut : **[9.4]**. Marc est dans QGIS ; il a déplié le mode expert et choisi la commande de test lente `test.qgis_probe`.

1. Marc clique **Lancer**. Le bouton devient **Arrêter**, une ligne « En cours… » apparaît au journal.
2. Il constate que la commande dure ; QGIS reste fluide, il peut zoomer sur la carte.
3. Il clique **Arrêter**.
4. Le panneau envoie `request.cancel` au noyau (le processus n'est pas tué).
5. **Climax :** la ligne du journal passe à « Annulée », le bouton redevient **Lancer**, et Marc peut lancer autre chose tout de suite.

Échec : sur une commande non annulable, **Arrêter** reste grisé avec « Arrêt impossible : cette étape ne peut pas être interrompue. » Marc attend la fin.

### Flux 3 — Noyau absent (Inès, étudiante, première installation)

Statut : **[9.1–9.3]** pour le comportement, **[9.4]** pour la forme.

1. Inès installe l'extension depuis le dépôt de QGIS et ouvre un projet.
2. Le panneau affiche, dans le bandeau : « Le noyau Cassini est introuvable. Installez le paquet Cassini, ou désignez le fichier. Rien n'est téléchargé. » La saisie et le mode expert sont grisés.
3. Elle installe le paquet Cassini à part (hors du panneau).
4. Elle clique **Choisir le noyau**, désigne le fichier, ou **Relancer**.
5. **Climax :** le bandeau disparaît, « Démarrage du noyau… » puis le mode expert s'ouvre, et une première exécution de `system.about` répond que le noyau est prêt.

Échec : le fichier choisi n'est pas un noyau valide ou parle un autre protocole → bandeau « versions incompatibles » avec **Choisir le noyau**.

### Flux 4 — Une IA externe travaille sous les yeux de l'utilisateur (Camille, ChatGPT pour ordinateur)

Statut : **[5.24–5.26]**. Camille a QGIS ouvert sur son projet Villeneuve-le-Roi et ChatGPT pour ordinateur branché sur `cassini mcp`.

1. Dans les réglages du panneau, elle allume **Autoriser une IA externe**. Le bandeau l'annonce.
2. Dans ChatGPT, elle tape la phrase de démo.
3. Les appels défilent dans le journal du panneau, marqués « IA externe » ; les couches produites apparaissent sur la carte.
4. **Climax :** le PDF est écrit, le journal se rejoue à l'identique, et Camille a vu chaque étape.

Échec : Camille lance une analyse dans le panneau pendant que ChatGPT travaille → l'appel arrivé second reçoit « Une commande est déjà en cours » ; rien n'est mis en file. Elle clique **Couper** : le canal se ferme, ChatGPT reçoit une erreur claire.

## Responsive & Platform

Poste de bureau, QGIS 3.44 à 4.99 (Qt 5 ou Qt 6 via `qgis.PyQt`). Pas de mobile.

| Mode | Comportement |
|---|---|
| Ancré à droite (par défaut) | Colonne d'environ 340 px, redimensionnable. Blocs en pile, mode expert replié. |
| Étroit (< 300 px) | Les boutons passent sous leur libellé ; les journaux et résultats se replient à la ligne ; pas de défilement horizontal. |
| Détaché (fenêtre flottante) | Même contenu, sans changement de logique. Peut être large : la colonne reste unique, le texte se lit plus à l'aise. [ASSUMPTION] pas de mise en deux colonnes au MVP. |

Thème : clair, sombre ou personnalisé de QGIS, suivi sans réglage propre. Langue : FR par défaut, EN au choix dans Réglages.

## Hors périmètre et trous

- [NOTE FOR UX] Pas de bouton d'export du journal de session ; le journal rejouable du noyau est un autre objet, hors du panneau.
- [NOTE FOR UX] Choix du modèle GGUF : seulement grisé jusqu'à l'epic 5 ; la façon d'ouvrir et de valider un modèle reste à concevoir.
- [NOTE FOR UX] Formulation exacte des résultats en langage clair par commande : à définir commande par commande avec le registre (une phrase par commande).
