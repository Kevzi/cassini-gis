---
name: Cassini
description: Panneau Cassini ancré dans QGIS. Il hérite du thème, des polices et des widgets de QGIS (Qt) ; ce document ne décrit que ce que Cassini ajoute, à savoir une couleur d'accent sarcelle et son logo.
status: final
updated: 2026-10-16
sources:
  - ../spec-cassini-mvp/spec-cassini-mvp.md
  - ../architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md
colors:
  # Seule couleur propre à Cassini : l'accent sarcelle (logo, repère de focus, marqueur d'étape).
  accent: '#0E8F86'
  accent-dark: '#3FD0C4'
  # Tous les autres rôles sont hérités du thème QGIS (palette Qt) ; aucune valeur n'est restituée ici.
  surface:
    note: 'Hérité du thème QGIS — rôle Window / Base de la palette Qt'
  surface-raised:
    note: 'Hérité du thème QGIS — rôle AlternateBase / fond des groupes'
  ink-primary:
    note: 'Hérité du thème QGIS — rôle WindowText / Text'
  ink-secondary:
    note: 'Hérité du thème QGIS — rôle PlaceholderText (texte atténué)'
  ink-disabled:
    note: 'Hérité du thème QGIS — groupe Disabled de la palette'
  border:
    note: 'Hérité du thème QGIS — rôle Mid / Dark'
  highlight:
    note: 'Hérité du thème QGIS — rôle Highlight (sélection de liste)'
  danger:
    note: 'Hérité du thème QGIS — aucune couleur d''erreur propre à Cassini ; l''erreur est portée par le texte et une icône'
typography:
  title:
    note: 'Police et corps hérités de Qt/QGIS — police de titre de groupe, graisse demi-grasse'
  body:
    note: 'Police et corps hérités de Qt/QGIS — police par défaut de l''application'
  meta:
    note: 'Police et corps hérités de Qt/QGIS — police par défaut, couleur atténuée (ink-secondary)'
  technical:
    note: 'Police à chasse fixe du système, réservée aux identifiants de commande et aux codes domaine.cause'
rounded:
  DEFAULT:
    note: 'Hérité du style Qt de QGIS'
spacing:
  '1': 4px
  '2': 8px
  '3': 12px
  '4': 16px
  gutter: 8px
components:
  logo:
    color: '{colors.accent}'
    colorDark: '{colors.accent-dark}'
    size: 24px
  focus-ring:
    color: '{colors.accent}'
    colorDark: '{colors.accent-dark}'
    width: 2px
  step-marker:
    color: '{colors.accent}'
    colorDark: '{colors.accent-dark}'
  input-main:
    backgroundColor: '{colors.surface}'
    textColor: '{colors.ink-primary}'
    disabledTextColor: '{colors.ink-disabled}'
  status-line:
    textColor: '{colors.ink-primary}'
    detailColor: '{colors.ink-secondary}'
---

## Brand & Style

Cassini est un invité dans QGIS, pas un second logiciel. Le panneau doit avoir l'air d'une partie de QGIS : mêmes polices, mêmes boutons, mêmes contrastes, même thème clair ou sombre. Ce qui est propre à Cassini tient en deux choses, un logo (un globe de bureau sur pied incliné, qu'on fait tourner) et une couleur d'accent sarcelle.

Le ton visuel est sobre et outillé. Pas de dégradé, pas d'illustration, pas d'animation décorative. L'utilisateur occasionnel sait ce qu'il veut mais pas où cliquer : la page doit lui présenter une seule chose à faire, la saisie en langage naturel, et ranger le reste (mode expert, détails techniques) derrière un repli.

## Colors

La palette de QGIS fait foi. Cassini n'en redéfinit aucun rôle (surface, texte, bordure, sélection, désactivé) : ils sont lus dans la palette Qt au moment de l'affichage, ce qui fait suivre le thème clair ou sombre sans code propre. [NOTE FOR UX] les rôles listés plus haut sont des noms de rôle, pas des valeurs : l'implémentation les lit dans `QPalette` ; aucun hexadécimal n'est à copier.

- **Sarcelle (`#0E8F86` clair / `#3FD0C4` sombre)** est l'unique couleur de Cassini. Elle sert au logo, à l'anneau de focus clavier et au marqueur de l'étape en cours. Elle ne sert jamais à dire « erreur », « succès » ou « avertissement » : ces états sont portés par une icône et un texte (voir Do's and Don'ts).
- **Contraste.** Le sarcelle clair (`#0E8F86`) sur fond blanc donne environ 4,0 : 1, sous le seuil de 4,5 : 1 du texte courant. [ASSUMPTION] En thème clair, le sarcelle n'est donc utilisé que pour des éléments non textuels (logo, trait, marqueur, anneau de focus, seuil de 3 : 1) ; jamais pour du texte ni comme fond d'un bouton portant du texte blanc. En thème sombre, `#3FD0C4` a un contraste largement suffisant sur les fonds sombres usuels. À mesurer sur les thèmes réels de QGIS avant de figer.
- Les boutons gardent le style natif de QGIS, sans fond sarcelle.

## Typography

Tout vient de Qt : police et corps de l'application, mise à l'échelle du système respectée. Aucune police n'est embarquée (le paquet de l'extension reste léger et sans ressource distante).

- **title** : titre de section du panneau (« Résultat », « Journal »). Demi-gras, pas de capitales.
- **body** : texte courant, consignes, résultats en langage clair.
- **meta** : détail secondaire (horodatage du journal, raison d'un bouton grisé), couleur atténuée du thème, jamais sous le corps par défaut moins un cran.
- **technical** : chasse fixe, réservée aux identifiants de commande (`system.about`) et aux codes d'erreur `domaine.cause`. Ce sont des identifiants publics du registre, pas du code : aucune expression, aucun chemin absolu, aucun extrait de programme n'est jamais affiché (AD-9, AD-23).

## Layout & Spacing

Échelle : 4 / 8 / 12 / 16 px, avec un intervalle de 8 px entre blocs liés.

Le panneau est une colonne unique, ancrée à droite de QGIS, étroite (de l'ordre de 340 px) et redimensionnable ; il peut être détaché en fenêtre flottante. Rien ne dépend d'une largeur fixe : les blocs passent à la ligne, les libellés longs se replient, aucun défilement horizontal. L'ordre vertical est constant : en-tête, saisie principale, plan (epic 5), résultat, journal, mode expert, Réglages.

## Elevation & Depth

Aucune élévation propre. Pas d'ombre : la hiérarchie vient de l'ordre vertical, d'un filet de séparation hérité du thème et de la graisse des titres. Le détachement en fenêtre flottante laisse à QGIS le soin du cadre.

## Shapes

Les rayons et bordures des champs, boutons et listes sont ceux du style Qt de QGIS. Le logo est un tracé au trait (épaisseur uniforme, extrémités arrondies) ; le panneau ne reprend pas cette arrondi ailleurs.

## Components

Spécifications visuelles ; le comportement est dans `EXPERIENCE.md`.

- **En-tête** : logo 24 px à gauche (version claire ou sombre selon le thème), titre « Cassini », bouton Réglages (icône d'engrenage native avec nom accessible) à droite.
- **Saisie principale** : champ multiligne natif, texte indicatif atténué ; désactivée, elle prend le style désactivé du thème et garde un texte visible qui dit pourquoi.
- **Plan** (epic 5) : liste numérotée ; l'étape en cours porte le marqueur sarcelle et un texte « en cours » ; chaque étape a une icône et un mot d'état.
- **Ligne d'état** (journal) : icône d'état + mot d'état + libellé ; l'horodatage en `meta` à droite.
- **Résultat** : texte en langage clair, puis un repli « Détails techniques » en `technical`.
- **Mode expert** : groupe repliable (flèche native), contenant liste de commandes, formulaire généré, bouton Lancer. Commande indisponible : ligne grisée avec la raison en `meta`.
- **Bandeau d'état** (noyau absent, planté, protocole refusé, session en lecture seule) : bloc de texte avec icône, en haut du panneau, bouton d'action à côté du texte.
- **Anneau de focus** : 2 px sarcelle autour du contrôle actif, par-dessus le focus natif quand il est trop discret.

## Do's and Don'ts

| À faire | À éviter |
|---|---|
| Suivre le thème de QGIS (clair, sombre, personnalisé) sans le contredire | Fixer des couleurs de fond ou de texte |
| Réserver le sarcelle au logo, au focus et au marqueur d'étape | Colorer des états (erreur, succès) ou des boutons en sarcelle |
| Dire l'état par une icône et un mot (« Erreur », « Annulée », « Terminée ») | Porter une information par la couleur seule |
| Afficher les identifiants de commande et les codes `domaine.cause` en chasse fixe | Afficher du code, une expression ou un chemin absolu (AD-9, AD-23) |
| Griser un contrôle indisponible et écrire la raison à côté | Masquer un contrôle sans explication |
| Rester dans les widgets Qt natifs, via `qgis.PyQt` | Dessiner des widgets maison, embarquer polices ou images lourdes, charger une ressource distante (AD-14, AD-18) |
