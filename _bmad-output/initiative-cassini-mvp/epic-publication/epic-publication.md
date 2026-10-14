---
type: epic
title: "Publication : depot d'extensions QGIS, installeurs, video"
parent: initiative-cassini-mvp
covers: []
after: []
assignee: ""
risk: low
---

# Publication : dépôt d'extensions QGIS, installeurs, vidéo

## Description

Rendre le travail trouvable et installable. Le canal principal est le **dépôt d'extensions QGIS**, qui donne accès à l'installé des bureaux d'études et des collectivités sans rien construire de plus. S'y ajoutent le paquet du noyau pour les trois systèmes, la vidéo de démo, et un dépôt qu'un inconnu peut comprendre et essayer.

## Outcome

Quelqu'un qui découvre Cassini peut l'installer et refaire la démo seul ; le signal est une première installation réussie par une personne extérieure au projet.

## Requirements

À compléter à l'inception de cet epic. La source est la spec de l'initiative,
`../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities.

## Done when

1. L'extension est publiée sur le dépôt d'extensions QGIS et s'installe depuis le gestionnaire d'extensions de QGIS.
2. Le noyau est empaqueté pour Windows, macOS et Linux, produit automatiquement et téléchargeable depuis une release du dépôt.
3. Une vidéo de démo montre le scénario complet, du fichier ouvert au PDF, dans QGIS.
4. Une personne extérieure installe l'extension et le noyau, puis refait la démo en suivant la seule documentation du dépôt.
5. Les licences de toutes les dépendances livrées sont listées dans la release, et la frontière GPL entre l'extension et le noyau y est expliquée.
6. Le modèle local est trouvable et installable sur un poste isolé, par un chemin documenté qui ne passe pas par le dépôt d'extensions.

## Boundaries

Frontière par activité : l'empaquetage et la mise en ligne. Le README d'architecture est livré bien plus tôt, dans l'epic 1 ; cet epic porte la documentation d'installation et d'usage, et la publication sur le dépôt d'extensions.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities
- constraint — `CLAUDE.md` §10 licence, §2.7 rien de militaire dans le dépôt public

## Notes

- Decision: le README d'architecture avec schémas Mermaid est avancé à l'epic 1, cet epic ne portant que l'installation et l'usage (2026-10-09).
- Decision: le dépôt d'extensions QGIS devient le canal de distribution principal ; l'installeur d'application autonome disparaît du MVP avec la V2 (2026-10-14).
- Unknown: le dépôt d'extensions QGIS n'acceptera pas plusieurs Go de modèle GGUF ; le chemin de livraison du modèle sur poste isolé reste à trancher. Question ouverte de la spec.
- Waits on epic 6 because: il n'y a rien à filmer ni à documenter avant que le scénario de démo passe.
