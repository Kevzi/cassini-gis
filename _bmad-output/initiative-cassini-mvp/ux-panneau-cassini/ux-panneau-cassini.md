---
name: ux-panneau-cassini
title: Panneau Cassini ancré dans QGIS
status: final
updated: 2026-11-08
sources:
  - ../spec-cassini-mvp/spec-cassini-mvp.md
  - ../architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md
spine:
  design: DESIGN.md
  experience: EXPERIENCE.md
---

# UX du panneau Cassini

- **DESIGN.md** : héritage du thème et des widgets de QGIS ; ajouts propres à Cassini : sarcelle (`#0E8F86` / `#3FD0C4`), bouton principal (`#0B7A72` / `#3FD0C4`), cartes légères sans ombre, bulle de demande, logo globe de bureau.
- **EXPERIENCE.md** : un panneau en fil de la demande (bulle de demande, cartes de Cassini, ligne « À vous », journal et mode expert repliés, saisie épinglée en bas), ses états, sa voix et quatre parcours.
- **Maquette** : [mockups/panneau-fil.html](mockups/panneau-fil.html) ([PNG](mockups/panneau-fil.png)), six états clairs et deux sombres (2026-11-08) ; [mockups/panneau.html](mockups/panneau.html) est remplacée. Les spines l'emportent sur les maquettes.
- **Logo** : [assets/logo-cassini-light.svg](assets/logo-cassini-light.svg) et [assets/logo-cassini-dark.svg](assets/logo-cassini-dark.svg) ; pistes écartées dans `.working/`.
