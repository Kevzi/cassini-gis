---
name: ux-panneau-cassini
title: Panneau Cassini ancré dans QGIS
status: final
updated: 2026-10-16
sources:
  - ../spec-cassini-mvp/spec-cassini-mvp.md
  - ../architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md
spine:
  design: DESIGN.md
  experience: EXPERIENCE.md
---

# UX du panneau Cassini

- **DESIGN.md** : héritage total du thème et des widgets de QGIS ; seul ajout, l'accent sarcelle (`#0E8F86` clair, `#3FD0C4` sombre) et le logo globe de bureau.
- **EXPERIENCE.md** : un panneau en colonne (saisie naturelle d'abord, résultat, journal, mode expert replié, Réglages FR/EN), ses états, sa voix, et trois parcours ; marque ce que livre la story 9.4 et ce qui attend l'epic 5.
- **Maquette** : [mockups/panneau.html](mockups/panneau.html), trois états côte à côte (accueil avec mode expert, commande lente, noyau absent).
- **Logo** : [assets/logo-cassini-light.svg](assets/logo-cassini-light.svg) et [assets/logo-cassini-dark.svg](assets/logo-cassini-dark.svg) ; pistes écartées dans `.working/`.
