---
name: Cassini
description: Panneau Cassini ancré dans QGIS, en fil de la demande. Il hérite du thème, des polices et des widgets de QGIS (Qt) ; ce document ne décrit que ce que Cassini ajoute, à savoir sa couleur sarcelle, son bouton principal, ses cartes légères, la bulle de la demande et son logo.
status: final
updated: 2026-11-08
sources:
  - ../spec-cassini-mvp/spec-cassini-mvp.md
  - ../architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md
  - ../epic-facade-qgis/story-panneau-lancer-une-commande-du-registre-voir-son-resultat-l-plan.md
colors:
  # Couleurs propres à Cassini, toujours par paire clair / sombre. La paire est choisie en lisant
  # la palette de QGIS (fenêtre claire ou sombre), comme le logo (`panel.py`, `_logo`).
  # Valeurs sombres approchées : à relever sur un vrai QGIS Night Mapping au moment du code.
  accent: '#0E8F86'
  accent-dark: '#3FD0C4'
  # Fond du bouton principal et son libellé.
  accent-strong: '#0B7A72'
  accent-strong-dark: '#3FD0C4'
  accent-strong-border: '#08605A'
  accent-strong-border-dark: '#3FD0C4'
  on-accent-strong: '#FFFFFF'
  on-accent-strong-dark: '#0F1F1E'
  # Teinte légère : fond de la bulle de demande et de l'étape en cours.
  accent-tint: '#E4F2F0'
  accent-tint-dark: '#1E3B39'
  accent-tint-border: '#9CCFC9'
  accent-tint-border-dark: '#2F6E68'
  # Tous les autres rôles sont hérités du thème QGIS (palette Qt) ; aucune valeur n'est restituée ici.
  surface:
    note: 'Hérité du thème QGIS — rôle Window (fond du panneau)'
  surface-card:
    note: 'Hérité du thème QGIS — rôle Base (fond des champs et des cartes)'
  surface-raised:
    note: 'Hérité du thème QGIS — rôle AlternateBase (fond de la ligne « À vous »)'
  ink-primary:
    note: 'Hérité du thème QGIS — rôle WindowText / Text'
  ink-secondary:
    note: 'Hérité du thème QGIS — rôle PlaceholderText (texte atténué)'
  ink-disabled:
    note: 'Hérité du thème QGIS — groupe Disabled de la palette'
  border:
    note: 'Hérité du thème QGIS — rôle Mid / Dark (cadre 1 px des cartes et des champs)'
  highlight:
    note: 'Hérité du thème QGIS — rôle Highlight (sélection de liste)'
  danger:
    note: 'Hérité du thème QGIS — aucune couleur d''erreur propre à Cassini ; l''erreur est portée par le texte et une icône'
typography:
  title:
    note: 'Police et corps hérités de Qt/QGIS — titre de carte, graisse demi-grasse'
  body:
    note: 'Police et corps hérités de Qt/QGIS — police par défaut de l''application'
  meta:
    note: 'Police et corps hérités de Qt/QGIS — un cran sous le corps par défaut, couleur atténuée (ink-secondary)'
  technical:
    note: 'Police à chasse fixe du système, réservée aux identifiants de commande et aux codes domaine.cause'
rounded:
  DEFAULT:
    note: 'Hérité du style Qt de QGIS (champs, boutons, listes)'
  card: 3px
  bubble: 8px
  bubble-tail: 2px
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
  focus-ring-primary:
    color: '{colors.ink-primary}'
    width: 2px
    offset: 2px
  step-marker:
    color: '{colors.accent}'
    colorDark: '{colors.accent-dark}'
    backgroundColor: '{colors.accent-tint}'
    backgroundColorDark: '{colors.accent-tint-dark}'
    borderLeftWidth: 3px
  input-main:
    backgroundColor: '{colors.surface-card}'
    textColor: '{colors.ink-primary}'
    disabledTextColor: '{colors.ink-disabled}'
    borderColor: '{colors.border}'
    focusBorderColor: '{colors.accent}'
    focusBorderColorDark: '{colors.accent-dark}'
  status-line:
    textColor: '{colors.ink-primary}'
    detailColor: '{colors.ink-secondary}'
  button-primary:
    backgroundColor: '{colors.accent-strong}'
    borderColor: '{colors.accent-strong-border}'
    textColor: '{colors.on-accent-strong}'
    backgroundColorDark: '{colors.accent-strong-dark}'
    borderColorDark: '{colors.accent-strong-border-dark}'
    textColorDark: '{colors.on-accent-strong-dark}'
    fontWeight: 600
    rounded: '{rounded.DEFAULT}'
  request-bubble:
    backgroundColor: '{colors.accent-tint}'
    borderColor: '{colors.accent-tint-border}'
    backgroundColorDark: '{colors.accent-tint-dark}'
    borderColorDark: '{colors.accent-tint-border-dark}'
    textColor: '{colors.ink-primary}'
    metaColor: '{colors.ink-secondary}'
    rounded: '{rounded.bubble}'
    roundedTail: '{rounded.bubble-tail}'
    padding: '{spacing.2}'
    maxWidth: 85%
  card:
    backgroundColor: '{colors.surface-card}'
    borderColor: '{colors.border}'
    borderWidth: 1px
    rounded: '{rounded.card}'
    padding: '{spacing.2}'
    shadow: none
  act-cue:
    backgroundColor: '{colors.surface-raised}'
    textColor: '{colors.ink-primary}'
    borderLeftColor: '{colors.accent}'
    borderLeftColorDark: '{colors.accent-dark}'
    borderLeftWidth: 3px
    padding: '{spacing.1} {spacing.2}'
  progress-cue:
    textColor: '{colors.ink-secondary}'
    emphasisColor: '{colors.ink-primary}'
    typography: '{typography.meta}'
---

## Brand & Style

Cassini est un invité dans QGIS, pas un second logiciel. Mais c'est un invité **reconnaissable**. Le panneau garde les polices, les widgets, les surfaces et les contrastes de QGIS, et suit son thème clair ou sombre. Ce qui est propre à Cassini tient en peu de choses : un logo (un globe de bureau sur pied incliné, qu'on fait tourner), une couleur sarcelle, un bouton principal sarcelle, et une mise en forme en **fil de la demande** — la demande de l'utilisateur en bulle, les réponses de Cassini en cartes légères.

Le ton visuel est sobre, sans extravagance. Pas de dégradé, pas d'ombre, pas d'illustration, pas d'animation décorative. L'utilisateur occasionnel sait ce qu'il veut mais pas où cliquer : à chaque instant, une seule chose est mise en avant (un seul bouton principal visible, une ligne « À vous : … » quand c'est à lui d'agir) ; le reste (journal, mode expert, détails techniques) est rangé derrière un repli.

Ce document remplace, sur ces points, la règle du 2026-10-16 « 100 % natif, sans aucun encadré ni fond sarcelle » (memlog, 2026-11-08).

## Colors

La palette de QGIS fait foi pour les surfaces, le texte, les bordures, la sélection et le désactivé : Cassini n'en redéfinit aucun rôle. Ces rôles sont lus dans `QPalette` au moment de l'affichage. Dans l'en-tête YAML, ce sont des noms de rôle, pas des valeurs : aucun hexadécimal n'est à copier pour eux.

Les couleurs propres à Cassini vont toujours par paire clair / sombre. La paire est choisie **en lisant la palette de QGIS** (fenêtre claire ou sombre), exactement comme le logo aujourd'hui (`qgis-plugin/cassini_qgis/panel.py`, méthode `_logo`). Les valeurs sombres sont approchées : [NOTE FOR UX] les relever sur un vrai QGIS Night Mapping au moment du code et corriger ici si un contraste passe sous son seuil.

| Jeton | Clair | Sombre | Sert à | Ne sert jamais à |
|---|---|---|---|---|
| `{colors.accent}` / `{colors.accent-dark}` | `#0E8F86` | `#3FD0C4` | Logo, anneau de focus, marqueur de l'étape en cours, trait de la ligne « À vous », puces des exemples | Du texte en thème clair ; dire « erreur », « succès » ou « avertissement » |
| `{colors.accent-strong}` / `{colors.accent-strong-dark}` | `#0B7A72` | `#3FD0C4` | Fond du bouton principal seulement | Tout autre fond, tout texte |
| `{colors.on-accent-strong}` / `{colors.on-accent-strong-dark}` | `#FFFFFF` | `#0F1F1E` | Libellé du bouton principal | — |
| `{colors.accent-tint}` / `{colors.accent-tint-dark}` | `#E4F2F0` | `#1E3B39` | Fond de la bulle de demande et de l'étape en cours | Fond d'une carte de Cassini |
| `{colors.accent-tint-border}` / `{colors.accent-tint-border-dark}` | `#9CCFC9` | `#2F6E68` | Cadre de la bulle de demande | — |

En thème clair, `{colors.accent}` reste réservé aux éléments non textuels. Aucune couleur d'erreur ni de succès : l'état se lit par l'icône et le mot.

En thème sombre, `{colors.accent-dark}`, `{colors.accent-strong-dark}` et `{colors.accent-strong-border-dark}` ont **volontairement** la même valeur `#3FD0C4` (memlog, 2026-11-08). Le bouton principal s'y distingue par sa forme, sa place et son libellé foncé. Ce n'est pas une erreur à corriger.

**Contrastes** (WCAG 2.1, calculés sur les valeurs de la maquette). Fonds clairs relevés sur les captures QGIS du 2026-11-08 : fenêtre `#F0F0F0`, Base `#FFFFFF`. Fonds sombres approchés : fenêtre `#2B2B2B`, Base `#232323`.

| Combinaison | Ratio | Seuil |
|---|---|---|
| Libellé blanc sur bouton principal `#0B7A72` | 5,2 : 1 | 4,5 : 1 (texte) |
| Libellé `#0F1F1E` sur bouton principal sombre `#3FD0C4` | 8,9 : 1 | 4,5 : 1 (texte) |
| Bouton principal sombre `#3FD0C4` sur fenêtre `#2B2B2B` | 7,4 : 1 | 3 : 1 (élément) |
| Sarcelle `#0E8F86` sur fenêtre `#F0F0F0` | 3,5 : 1 | 3 : 1 (élément non textuel) |
| Sarcelle `#0E8F86` sur Base `#FFFFFF` | 4,0 : 1 | sous 4,5 : 1, donc jamais pour du texte |
| Sarcelle `#3FD0C4` sur fenêtre `#2B2B2B` | 7,4 : 1 | 3 : 1 |
| Texte `#1F1F1F` sur bulle `#E4F2F0` | 14,3 : 1 | 4,5 : 1 |
| Texte `#E6E6E6` sur bulle sombre `#1E3B39` | 9,7 : 1 | 4,5 : 1 |
| Sarcelle `#0E8F86` sur teinte `#E4F2F0` (marqueur d'étape en cours) | 3,5 : 1 | 3 : 1 |
| Sarcelle `#3FD0C4` sur teinte sombre `#1E3B39` (marqueur d'étape en cours) | 6,3 : 1 | 3 : 1 |
| « Vous · heure » `#5F5F5F` sur bulle `#E4F2F0` / `#A8A8A8` sur bulle sombre `#1E3B39` | 5,5 : 1 / 5,1 : 1 | 4,5 : 1 |
| Trait « À vous » `#0E8F86` sur `#F4F4F4` / `#3FD0C4` sur `#303030` (AlternateBase approché) | 3,6 : 1 / 6,9 : 1 | 3 : 1 |
| Anneau du bouton principal `{colors.ink-primary}` sur fenêtre : `#1F1F1F` sur `#F0F0F0` / `#E6E6E6` sur `#2B2B2B` | 14,5 : 1 / 11,3 : 1 | 3 : 1 (WCAG 1.4.11) |

L'anneau `{colors.accent}` ordinaire serait invisible sur le bouton principal (1,3 : 1 en clair, 1 : 1 en sombre) : d'où `focus-ring-primary`, un trait extérieur séparé du bouton par 2 px de fond de fenêtre, et mesuré contre ce fond (memlog, 2026-11-08).

## Typography

Tout vient de Qt : police et corps de l'application, mise à l'échelle du système respectée. Aucune police n'est embarquée (le paquet de l'extension reste léger et sans ressource distante).

- **title** : titre d'une carte (« Voici ce que je propose — 4 étapes », « Plan exécuté. »). Demi-gras, pas de capitales.
- **body** : texte courant, bulle de demande, étapes, résultats en langage clair, ligne « À vous ».
- **meta** : détail secondaire (« Vous · 19:49 », repère d'avancement, raison d'un bouton grisé, pied hors ligne), couleur atténuée du thème, jamais plus petit qu'un cran sous le corps par défaut.
- **technical** : chasse fixe, réservée aux identifiants de commande (`system.about`) et aux codes d'erreur `domaine.cause`, et seulement dans le mode expert ou derrière « Détails techniques ». Aucune expression, aucun chemin absolu, aucun extrait de programme n'est affiché (AD-9, AD-23).

## Layout & Spacing

Échelle : 4 / 8 / 12 / 16 px, avec un intervalle de `{spacing.gutter}` entre blocs liés et entre cartes du fil.

Le panneau est une colonne unique, ancrée à droite de QGIS, étroite (de l'ordre de 340 px) et redimensionnable ; il peut être détaché en fenêtre flottante. De haut en bas :

1. **En-tête** (logo, « Cassini », Réglages) et, s'il y a lieu, **bandeau d'état** juste dessous.
2. **Fil** : il prend toute la hauteur libre. La bulle de demande est alignée à droite (au plus 85 % de la largeur) ; les cartes de Cassini occupent toute la largeur, empilées dessous dans l'ordre d'arrivée.
3. **Replis** Journal puis Mode expert, chacun sur une ligne, avec leur compte. Déplié, un repli s'ouvre sur place, au-dessus de la saisie, et le fil rétrécit d'autant.
4. **Saisie épinglée en bas** : champ multiligne et bouton Envoyer sur une même ligne, Envoyer aligné sur le bas du champ.
5. **Pied hors ligne**, séparé par un filet du thème.

Règles :

- Les cartes **grandissent avec leur contenu**. Aucune zone à hauteur fixe de quelques lignes avec son propre défilement : c'était la gêne principale des captures du 2026-11-08.
- Seul le fil défile, d'un seul bloc, quand il dépasse la hauteur ; la saisie, les replis et le pied restent visibles. [ASSUMPTION] le fil se cale sur la carte la plus récente à chaque nouvelle carte.
- Rien ne dépend d'une largeur fixe : les libellés passent à la ligne (`overflow-wrap`), aucun défilement horizontal.
- Marges intérieures : `{spacing.2}` dans les cartes et la bulle, `{spacing.1}` `{spacing.2}` dans la ligne « À vous ».

## Elevation & Depth

Pas d'ombre, nulle part. La hiérarchie vient de trois choses : le **cadre 1 px** des cartes (couleur `{colors.border}` du thème, fond `{colors.surface-card}`) qui les détache du fond de fenêtre, la **bulle teintée** qui distingue ce que l'utilisateur a dit de ce que Cassini répond, et la graisse des titres. Une carte n'est jamais posée dans une autre carte ; à l'intérieur d'une carte, les étapes sont séparées par un filet du thème. Le détachement en fenêtre flottante laisse à QGIS le soin du cadre.

Ce paragraphe remplace la règle du 2026-10-16 « aucune élévation, hiérarchie par l'ordre vertical seul » (memlog, 2026-11-08).

## Shapes

Champs, boutons et listes gardent les rayons du style Qt de QGIS (`{rounded.DEFAULT}`), bouton principal compris. Les cartes ont un rayon discret de `{rounded.card}`. La bulle de demande a un rayon de `{rounded.bubble}`, sauf le coin bas-droit à `{rounded.bubble-tail}`, qui la rattache au bord droit comme une réplique. Le logo est un tracé au trait (épaisseur uniforme, extrémités arrondies).

## Components

Spécifications visuelles, dans l'ordre du panneau, de haut en bas. Le comportement est dans `EXPERIENCE.md` (Component Patterns), sous les mêmes noms.

Référence visuelle : [mockups/panneau-fil.html](mockups/panneau-fil.html) (et [mockups/panneau-fil.png](mockups/panneau-fil.png)). Elle montre six états en thème clair (repos, composition, plan proposé, exécution, résultat, erreur), puis le plan proposé et le résultat en thème sombre. L'ancienne maquette [mockups/panneau.html](mockups/panneau.html) (2026-10-16) est **remplacée** et ne sert plus de référence. En cas de conflit entre une maquette et ce document, ce document l'emporte.

| Composant | Jeton | Spécification visuelle |
|---|---|---|
| En-tête | `logo` | Logo 24 px à gauche ([assets/logo-cassini-light.svg](assets/logo-cassini-light.svg) ou [assets/logo-cassini-dark.svg](assets/logo-cassini-dark.svg) selon la palette lue), titre « Cassini », bouton Réglages (icône d'engrenage native, nom accessible) à droite. |
| Réglages | — | Zone native sous l'en-tête ; contrôle désactivé au style désactivé du thème, raison en `meta`. |
| Bandeau d'état | — | Inchangé : bloc de texte avec icône, sous l'en-tête et au-dessus du fil, bouton d'action à côté du texte. Style natif, pas de carte. |
| Carte d'accueil | `card` | Carte au repos : titre « Bonjour », consigne en `body`, liste « Exemples de demandes » précédée d'un chevron `{colors.accent}`, texte en `{colors.ink-secondary}`. |
| Bulle de demande | `request-bubble` | Alignée à droite, fond `{colors.accent-tint}`, cadre `{colors.accent-tint-border}`, coin bas-droit resserré. Au-dessus du texte, « Vous · heure » en `meta`, aligné à droite. |
| Bulle « Mode expert » | `request-bubble` | Même bulle que la demande, texte « Mode expert : {libellé court de l'étape} ». |
| Carte de composition | `card` | Titre « Je prépare un plan… » avec petit logo 16 px, détail en `meta`, bouton natif Arrêter. Aucune animation décorative. |
| Carte d'échec de composition | `card`, `act-cue` | Titre avec petit logo 16 px, message du noyau en `body`, ligne « À vous », Reformuler la demande (principal), repli Détails techniques. |
| Carte de question | `card`, `act-cue` | Titre avec petit logo 16 px, la question en `body`, puis la ligne « À vous : répondez dans la saisie. ». Pas de bouton propre : Envoyer reste le bouton principal. |
| Carte du plan | `card`, `step-marker`, `progress-cue`, `act-cue` | Titre avec petit logo 16 px. Étapes numérotées : icône + mot d'état en demi-gras + libellé ; filet du thème entre deux étapes. Étape en cours : fond `{colors.accent-tint}` et trait gauche 3 px `{colors.accent}`. Puis la ligne « À vous », la rangée Valider le plan (principal) / Refuser (natif), et le repli « Détails techniques ». |
| Repère d'avancement | `progress-cue` | Une ligne en `meta` sous le titre de la carte du plan, alignée sur le texte du titre : « Étape **k** sur **n** en cours », chiffres en `{colors.ink-primary}`. Pas de barre de progression. |
| Ligne « À vous » | `act-cue` | Bande pleine largeur dans la carte, fond `{colors.surface-raised}`, trait gauche 3 px `{colors.accent}`, texte en `body` commençant par « À vous : ». |
| Carte du plan validé (repliée) | `card` | Une seule ligne au cadre 1 px : « Plan validé — n étapes », lien « › Voir » à droite. |
| Carte de résultat | `card` | Titre (« Plan exécuté. » ou « Plan arrêté à l'étape k. »), liste des étapes avec icône et résultat en langage clair, phrase de clôture, bouton natif « Ouvrir le dossier », repli « Détails techniques » en `technical`. En erreur : l'étape fautive porte l'icône d'erreur, le mot « Erreur » et son message dessous ; les suivantes « Non lancée ». |
| Carte de refus | `card`, `act-cue` | Titre « Plan refusé. Reformulez votre demande. », ligne « À vous », Reformuler la demande (principal). |
| Détails techniques | — | Repli natif (flèche) en bas de carte, fermé par défaut ; contenu en `technical`. |
| Repli Journal / Mode expert | — | Ligne repliable native (flèche), fond de bouton du thème, cadre 1 px ; libellé suivi du compte en `meta` (« (vide) », « (4 étapes) »). |
| Journal de session (ligne d'état) | `status-line` | Icône d'état + mot d'état + libellé ; l'horodatage en `meta` à droite. |
| Rejouer le journal | — | Bouton natif dans le repli Journal déplié ; le message de fin du rejeu en `body` sous le bouton, avec icône et mot. |
| Mode expert | — | Contenu du repli : liste de commandes, formulaire par schéma, bouton Lancer. Commande indisponible : ligne grisée avec la raison en `meta`. |
| Formulaire par schéma | — | Widgets Qt natifs (champ texte, compteur, case, liste déroulante) ; propriété hors sous-ensemble : champ multiligne en `technical`. |
| Autoriser / Refuser (IA externe) | — | Deux boutons natifs côte à côte sous le texte de la demande d'accord ; aucun n'est en `button-primary`. |
| Saisie principale | `input-main` | Champ multiligne natif, fond `{colors.surface-card}`, texte indicatif atténué ; au focus, cadre 2 px `{colors.accent}`. Désactivée : style désactivé du thème, la raison remplace le texte indicatif. |
| Bouton principal | `button-primary` | Fond `{colors.accent-strong}`, libellé `{colors.on-accent-strong}` demi-gras (sombre : `{colors.accent-strong-dark}` et `{colors.on-accent-strong-dark}`). Porté seulement par Envoyer, Valider le plan, Reformuler la demande — **un seul visible à la fois**. Désactivé, il reprend le style désactivé natif (pas de sarcelle). |
| Reformuler la demande | `button-primary` | Bouton principal de la carte de résultat en erreur ou de la carte de refus, suivi d'une ligne `meta` « Votre phrase revient dans la saisie… ». |
| Bouton Lancer / Arrêter | — | Bouton natif du thème, comme Refuser et Ouvrir le dossier. |
| Anneau de focus | `focus-ring`, `focus-ring-primary` | `{components.focus-ring.width}` `{colors.accent}` autour du contrôle actif, par-dessus le focus natif quand il est trop discret. Sur le bouton principal : trait extérieur `{colors.ink-primary}` de 2 px, à 2 px du bouton (`focus-ring-primary`). |
| Pied hors ligne | — | Texte `meta` sous un filet du thème, tout en bas. |

## Do's and Don'ts

| À faire | À éviter |
|---|---|
| Suivre le thème de QGIS (clair, sombre, personnalisé) pour les surfaces, le texte et les bordures | Fixer des couleurs de fond ou de texte autres que les jetons de Cassini |
| Choisir la paire clair / sombre de Cassini en lisant la palette de QGIS | Coder en dur une seule variante |
| Réserver `{colors.accent-strong}` au bouton principal, un seul visible à la fois | Mettre deux boutons sarcelle côte à côte, ou colorer Refuser, Arrêter, Lancer |
| Réserver `{colors.accent}` au logo, au focus, aux marqueurs et aux traits | Écrire du texte en `{colors.accent}` sur fond clair ; colorer des états (erreur, succès) |
| Cartes au cadre 1 px du thème, sans ombre, qui grandissent avec leur contenu | Ombres, cartes imbriquées, zones à hauteur fixe avec défilement interne |
| Dire l'état par une icône et un mot (« Erreur », « Annulée », « Terminée ») | Porter une information par la couleur seule |
| Afficher les identifiants de commande et les codes `domaine.cause` en chasse fixe, derrière un repli | Afficher du code, une expression ou un chemin absolu (AD-9, AD-23) |
| Griser un contrôle indisponible et écrire la raison à côté | Masquer un contrôle sans explication |
| Rester dans les widgets Qt natifs, via `qgis.PyQt` ; cartes et bulle en simples cadres stylés | Dessiner des widgets maison, embarquer polices ou images lourdes, charger une ressource distante (AD-14, AD-18) |
