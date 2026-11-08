# Spine Pair Review — Cassini

## Overall verdict

La paire tient comme contrat. Un consommateur peut extraire les jetons, les composants, les états et les textes sans rouvrir la maquette : chaque couleur propre à Cassini a sa valeur hexadécimale claire et sombre, les rôles hérités de QGIS sont nommés et non chiffrés (décision du 2026-10-16), les références `{…}` se résolvent toutes, et les décisions du memlog du 2026-11-08 sont toutes reprises. Aucun constat critique. Quatre constats élevés sont à fermer avant de construire la refonte « Fil » : le rejeu, cité mais jamais spécifié ; l'anneau de focus invisible sur le bouton principal ; la place du résultat d'une commande experte dans le fil ; le focus perdu quand Valider le plan disparaît.

## 1. Flow coverage — adequate

Vérifié : les sources sont des capacités (CAP-n de `spec-cassini-mvp.md`, la story 9.4), pas des parcours UJ. Les CAP qui touchent le panneau sont CAP-4, CAP-5, CAP-7, CAP-8 et CAP-13. Les quatre flux ont chacun un protagoniste nommé (Camille, Marc, Inès, Camille), des étapes numérotées, un **Climax** et un chemin d'échec. CAP-4/CAP-5 sont couverts par le Flux 1, CAP-13 et la story 9.4 par les Flux 2 et 3, CAP-8 par le Flux 4. CAP-7 n'est couvert par aucun flux.

### Findings
- **high** Le rejeu est cité sans être spécifié nulle part. Le mode lecture seule « ne laisse que le rejeu » (EXPERIENCE.md l.49, l.112, l.176) ; CAP-4 et CAP-7 parlent de « rejouer » ; le climax du Flux 4 dit « le journal se rejoue à l'identique » (l.257). Pourtant aucune zone, aucun contrôle, aucun composant, aucun état ni aucun flux ne dit où et comment on rejoue depuis le panneau, et `panel.py` n'a aujourd'hui aucun rejeu. *Fix:* soit spécifier un contrôle « Rejouer » (où il se trouve, ses états, son comportement en lecture seule), soit écrire que le rejeu passe hors du panneau (`cassini replay`), puis réécrire en conséquence le texte de la lecture seule et le climax du Flux 4.
- **medium** La carte de question n'a pas de flux. C'est une décision du 2026-11-08 (memlog l.34), liée à AD-23 (point absent, donc le noyau pose une question), mais elle n'existe que comme ligne d'état (l.153). *Fix:* ajouter au Flux 1 une branche « Question », par exemple un point sans système, avec la phrase rendue dans la saisie et la réponse par Envoyer.
- **medium** Aucun flux ne nomme la CAP qu'il couvre, si bien que la traçabilité vers la spec repose sur la lecture. *Fix:* une ligne « Couvre : CAP-4, CAP-5, CAP-3 » sous chaque statut de flux.
- **low** Le Flux 2 (story 9.4) suit l'arrêt par le journal, mais ne montre jamais où arrive le résultat d'un Lancer qui réussit (voir §4). *Fix:* ajouter l'issue nominale (`system.about`, carte de résultat).

## 2. Token completeness — adequate

Vérifié : les 14 couleurs propres à Cassini ont toutes une valeur hexadécimale et leur paire claire / sombre ; les 9 rôles hérités sont en `note` ; typography (4 rôles en `note`), rounded (DEFAULT en `note`, card, bubble, bubble-tail), spacing (1–4, gutter) ; les 10 composants à jetons. Toutes les références `{colors.*}`, `{rounded.*}`, `{spacing.*}` et `{typography.meta}` du texte et des composants se résolvent. Les 9 ratios du tableau des contrastes ont été recalculés et sont justes (5,20 ; 8,93 ; 7,44 ; 3,48 ; 3,97 ; 14,33 ; 9,67 ; 3,45).

### Findings
- **high** L'anneau de focus est invisible sur le bouton principal. `focus-ring` vaut `{colors.accent}`, posé « autour du contrôle actif » (DESIGN.md l.72-75, l.234). Contre le fond du bouton principal, il donne `#0E8F86` sur `#0B7A72`, soit **1,31 : 1** en clair, et `#3FD0C4` sur `#3FD0C4`, soit **1 : 1** en sombre. Or le focus est placé d'office sur ce bouton dans trois états (Plan proposé, Erreur d'étape, Plan refusé ; EXPERIENCE.md l.154-158), ce qui manque WCAG 1.4.11 et 2.4.13. *Fix:* soit un écart de 2 px de la couleur de surface entre l'anneau et le bouton, soit un anneau `{colors.ink-primary}` sur le bouton principal ; ajouter la ligne au tableau des contrastes.
- **low** Plusieurs combinaisons porteuses manquent au tableau des contrastes : marqueur sombre `#3FD0C4` sur teinte sombre `#1E3B39` (6,3 : 1), « Vous · heure » en `ink-secondary` sur la bulle (5,5 : 1 sur les valeurs de la maquette), trait « À vous » sur AlternateBase (3,6 : 1). *Fix:* les ajouter.
- **low** En sombre, `accent`, `accent-strong` et `accent-strong-border` valent tous `#3FD0C4` (l.14-19). Le « réservé au bouton principal » ne repose donc plus que sur la forme. C'est la décision du memlog l.32, mais elle n'est pas écrite comme voulue. *Fix:* une phrase dans Colors, pour que personne ne « corrige » l'égalité.
- **low** La notation `{colors.accent-tint-border}` / `-dark` (DESIGN.md l.152) n'est pas un chemin qu'on peut résoudre. *Fix:* écrire `{colors.accent-tint-border-dark}` en entier.
- **low** `spacing.3` et `spacing.4` sont définis mais jamais cités (l.64-65). *Fix:* les citer ou les retirer.
- **low** Le texte nomme `ink-secondary` et `ink-primary` sans accolades (l.216, l.221). *Fix:* écrire `{colors.ink-secondary}` et `{colors.ink-primary}`.

## 3. Component coverage — adequate

Vérifié : 22 lignes dans DESIGN.md (Components) et 20 dans EXPERIENCE.md (Component Patterns). Les deux tables concordent sur 18 composants, avec des règles réelles. Les écarts sont d'abord une question de nom (plan replié, replis), puis trois composants utilisés dans le texte qui n'ont de ligne dans aucune des deux tables.

### Findings
- **medium** La « carte de refus » n'a de ligne dans aucune table. Elle est nommée dans DESIGN.md l.233 et apparaît dans l'état Plan refusé (EXPERIENCE.md l.158). *Fix:* lui donner une ligne dans chaque spine (jeton `card` + `act-cue`, titre, Reformuler).
- **medium** Le repli « Détails techniques » est utilisé dans le plan, le résultat, la typographie et l'accessibilité, mais n'a de ligne dans aucune table. Son contenu exact (code `domaine.cause`, identifiant de commande, autre chose ?) n'est pas arrêté. *Fix:* une ligne par spine : son contenu, replié par défaut, `technical`, son nom accessible.
- **medium** Les boutons Autoriser / Refuser de « Me demander avant chaque appel » (EXPERIENCE.md l.178, AD-29) n'ont ni composant ni emplacement : ligne du journal ? carte dans le fil ? bandeau ? *Fix:* les placer, puis les spécifier dans les deux spines (visuel, focus, annonce).
- **low** Les noms divergent entre les spines : DESIGN.md a « Carte du plan validé (repliée) », « Repli Journal / Mode expert » et « Journal de session (ligne d'état) », qu'EXPERIENCE.md fond dans « Carte du plan », « Journal de session » et « Mode expert ». Pourtant EXPERIENCE.md l.120 affirme que « les noms sont les mêmes dans les deux spines ». *Fix:* aligner les deux listes.
- **low** Le memlog l.28 et DESIGN.md l.187 donnent un « compte » aux deux replis, mais le libellé du Mode expert (l.97) n'en a pas, et rien ne dit ce qui y est compté. *Fix:* fixer ce que compte le Mode expert, ou retirer le compte pour ce repli.

## 4. State coverage — adequate

Vérifié, zone par zone : le fil a 9 états, avec pour chacun « À vous », le bouton principal, la saisie et le focus (très complet) ; le noyau et la session ont 13 états, dont hors ligne et lecture seule (l'équivalent ici d'un refus de permission) ; le journal vide et rempli ; le mode expert (indisponible, JSON invalide) ; les Réglages (IA externe déjà prise). Les manques sont autour du fil, pas dedans.

### Findings
- **high** On ne sait pas où va le résultat d'une commande experte dans le fil. La carte de résultat est la « sortie du plan ou de la commande » (l.133), mais aucun état ne dit si un Lancer remplace l'échange en cours, s'il crée une bulle, ni ce que deviennent le bouton principal et la saisie. La matrice de la story 9.4 (« Lancer `system.about` → résultat clair ») n'a plus de place dans le fil. *Fix:* ajouter les états « Commande experte en cours / Résultat / Erreur » au tableau du fil.
- **high** Le focus se perd après Valider le plan. Le bouton disparaît, et l'état Exécution dit que le focus « reste où il était » [ASSUMPTION] (l.155), c'est-à-dire sur un contrôle qui n'existe plus. *Fix:* envoyer le focus sur Arrêter quand l'étape est annulable, sinon sur le titre de la carte du plan.
- **medium** L'échec de composition n'a pas d'état. La carte de composition s'efface « ou d'une erreur » (l.128), mais rien ne dit que faire quand le noyau n'arrive pas à composer (sortie rejetée selon AD-23, délai dépassé, plantage) : ni ligne d'état, ni texte, ni « À vous », ni bouton principal. *Fix:* ajouter un état « Composition impossible » avec son texte FR / EN et Reformuler.
- **medium** Le cas du modèle absent ou non chargé n'est pas décrit. Il y manque l'état de la saisie et le texte qui l'explique ; la seule trace est la note l.278. *Fix:* ajouter une ligne au tableau du noyau et de la session.
- **medium** L'état Annulée se contredit. La carte du plan « se replie une fois finie (réussie ou arrêtée) » (l.130), alors que l'état Annulée fait passer « la carte en cours à Annulée » (l.159) sans dire si elle se replie ni si une carte de résultat suit. Il n'y a de titre ni pour un plan annulé ni pour une composition annulée. *Fix:* trancher, et donner les textes.
- **low** La saisie grisée doit afficher sa raison à la place du texte indicatif (DESIGN.md l.230), mais ces raisons ne sont pas écrites pour Projet absent, Noyau absent, Lecture seule et Occupé. *Fix:* les ajouter au tableau des textes.
- **low** Reformuler quand la saisie n'est pas vide doit « le dire » (l.186), mais aucun texte n'est fourni. *Fix:* ajouter le texte.

## 5. Visual reference coverage — strong

Vérifié : `mockups/` contient `panneau-fil.html` et `panneau-fil.png`, tous deux liés dans le texte avec ce qu'ils illustrent (DESIGN.md l.210, EXPERIENCE.md l.15), et `panneau.html`, lié et marqué comme remplacé. `imports/` est vide et `wireframes/` absent : aucun fichier orphelin. La règle « les spines l'emportent » est posée une fois dans chaque spine. L'écart sur le pied de page (« Vos données restent sur votre poste. ») est écrit et tranché (l.116).

### Findings
- **low** DESIGN.md (l.214) parle du « fichier clair ou sombre » du logo sans lier `assets/logo-cassini-light.svg` ni `-dark.svg`, que seul l'index lie. *Fix:* lier les deux fichiers dans la ligne En-tête.
- **low** Pendant l'exécution, la maquette titre la carte « Plan validé — 4 étapes » au-dessus du repère d'avancement, mais le tableau des textes n'a que la forme repliée. *Fix:* une ligne « Plan, titre pendant l'exécution ».
- **low** Le memlog l.32 cite `.working/key-panneau-fil.html`, alors que les spines citent `mockups/panneau-fil.html`, une copie identique (même taille). *Fix:* aucun dans les spines ; noter dans le memlog que la copie fait foi.

## 6. Bloat & overspecification — adequate

### Findings
- **medium** EXPERIENCE.md transporte l'historique des livraisons : les marques [9.4], [Epic 5] et [9.1–9.3] (« gardée pour l'historique », l.13) et la colonne « Livraison » de l'IA. Seule la marque [Fil] dit ce qui reste à construire. *Fix:* garder [Fil], retirer le reste ou le ramener à une note.
- **low** Il reste de la narration d'historique : Foundation §3 (l.23), et DESIGN.md l.138 et l.202 (« remplace la règle du 2026-10-16 »). Le memlog la porte déjà. *Fix:* réduire à une référence au memlog.
- **low** Le texte répète des mesures déjà portées par les jetons : trait de 3 px, focus de 2 px, 85 % (DESIGN.md l.219, l.222, l.234). *Fix:* citer le jeton à la place.

## 7. Inheritance discipline — adequate

Vérifié : les trois sources se résolvent, et les AD cités existent. Les références aux jetons depuis EXPERIENCE.md (`step-marker`, `{colors.accent}`) se résolvent. Les décisions du memlog du 2026-11-08 sont toutes respectées : bulle, cartes sans ombre, bouton `#0B7A72` et `#3FD0C4`/`#0F1F1E`, remplacement dans le fil, question avec `request_to_restore`, replis sur place, pied de page, bandeau inchangé, Reformuler, repère d'avancement. `agentview.request_to_restore` et `summary.label` existent bien dans le code.

### Findings
- **medium** La lecture seule est rattachée à AD-7 et AD-17 (EXPERIENCE.md l.49, l.176), mais la règle (verrou exclusif, seconde session en lecture seule qui peut rejouer) est dans AD-11 ; AD-17 traite de la distribution. *Fix:* citer AD-11.
- **medium** Le texte « Étape {n} en erreur ({code}) : {message} » (l.111) affiche le code dans la vue principale, ce qui contredit l.116 et DESIGN.md l.177 (code seulement sous Détails techniques). *Fix:* retirer `({code})` du texte principal.
- **medium** AD-27 n'est pas repris. Il fixe un gabarit unique de résumé chiffré (`rows` / `label` / `values` / `truncated`), alors que la carte de résultat renvoie « commande par commande » la formulation (l.279) et ne dit pas comment s'affiche `truncated`. Le climax du Flux 1, « le nombre de parcelles retenues », en dépend. *Fix:* une règle d'affichage unique pour le gabarit d'AD-27.
- **medium** Une décision n'a pas de trace. Le déplacement du focus sur l'action « À vous » (l.200) et la colonne Focus du tableau des états ne viennent d'aucune entrée du memlog et ne sont pas tous marqués [ASSUMPTION]. *Fix:* les marquer, ou les consigner dans le memlog après accord du propriétaire.
- **low** Les sources diffèrent d'un fichier à l'autre : EXPERIENCE.md liste la story 9.4, DESIGN.md et l'index non. *Fix:* aligner les trois listes.
- **low** Le glossaire n'est pas posé. « Journal de session » (composant), « Journal de la session » (texte affiché) et « journal Cassini / rejouable » (CAP-7) désignent deux objets différents. *Fix:* définir les deux une fois, dans Foundation.

## 8. Shape fit — strong

Vérifié : les sections de DESIGN.md sont dans l'ordre canonique, au complet. EXPERIENCE.md a les huit sections par défaut, plus Responsive & Platform. Inspiration & Anti-patterns est retirée, ce qui se défend : les interdits vivent dans Interaction Primitives. La section inventée « Hors périmètre et trous » mérite sa place, puisqu'elle regroupe les notes ouvertes. Aucun constat.

## Mechanical notes

- **Frontmatter** : DESIGN.md (name, description, status, updated, sources, jetons) et EXPERIENCE.md (name, status, sources, updated) sont complets ; les sources ne sont pas les mêmes (§7).
- **Noms** : « Carte du plan validé (repliée) », « Repli Journal / Mode expert » et « Journal de session (ligne d'état) » n'existent que dans DESIGN.md ; « Carte de refus » et « Détails techniques » n'ont de ligne dans aucune des deux tables.
- **Renvois cassés ou faux** : AD-7/AD-17 pour la lecture seule (c'est AD-11) ; `{colors.accent-tint-border}` / `-dark` ; « rejeu » sans cible.
- **Marques ouvertes** : 6 [ASSUMPTION] et 7 [NOTE FOR UX] dans EXPERIENCE.md, 1 [ASSUMPTION] et 1 [NOTE FOR UX] dans DESIGN.md. Aucune ne bloque un consommateur, sauf l'[ASSUMPTION] de la l.155 (focus pendant l'exécution, §4).
- **Contrastes** : les 9 ratios déclarés ont été recalculés et sont exacts ; il manque l'anneau de focus sur le bouton principal (§2).
