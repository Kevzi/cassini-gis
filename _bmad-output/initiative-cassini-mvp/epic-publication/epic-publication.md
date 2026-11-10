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

Rendre le travail trouvable et installable. Le canal principal est le **dépôt d'extensions QGIS**, qui donne accès à l'installé des bureaux d'études et des collectivités sans rien construire de plus. S'y ajoutent le paquet du noyau, par deux portes d'installation, pour Windows et Linux d'abord et macOS plus tard, la vidéo de démo, et un dépôt qu'un inconnu peut comprendre et essayer.

## Outcome

Quelqu'un qui découvre Cassini peut l'installer et refaire la démo seul ; le signal est une première installation réussie par une personne extérieure au projet.

## Requirements

À compléter à l'inception de cet epic. La source est la spec de l'initiative,
`../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities.

## Done when

1. L'extension est publiée sur le dépôt d'extensions QGIS et s'installe depuis le gestionnaire d'extensions de QGIS.
2. Le noyau est empaqueté pour Windows et Linux (macOS plus tard, quand un Mac est disponible), produit automatiquement et téléchargeable depuis une release du dépôt ; chaque système a ses deux portes : un installeur léger en ligne et un installeur complet hors ligne (changement de cap du 2026-11-10).
3. Une vidéo de démo montre le scénario complet, du fichier ouvert au PDF, dans QGIS.
4. Une personne extérieure installe l'extension et le noyau, puis refait la démo en suivant la seule documentation du dépôt.
5. Les licences de toutes les dépendances livrées sont listées dans la release, et la frontière GPL entre l'extension et le noyau y est expliquée.
6. Le modèle local est publié sur un dépôt public Hugging Face avec sa licence et sa notice ; il s'installe par l'installeur en ligne, qui vérifie son empreinte SHA-256, ou, sur un poste isolé du réseau, par l'installeur complet hors ligne, un seul fichier ; aucun des deux chemins ne passe par le dépôt d'extensions (changement de cap du 2026-11-10).
7. Une page d'accueil publique, une seule page hébergée gratuitement par GitHub Pages, présente Cassini (deux phrases et la vidéo de démo), donne l'installation en trois étapes (extension depuis QGIS, puis installeur en ligne ou installeur complet hors ligne), les liens de téléchargement, une courte foire aux questions (données gardées sur le poste, aucun envoi sur le réseau, carte graphique conseillée) et un contact ; le champ `homepage` de `metadata.txt` pointe vers elle (décision du propriétaire, 2026-11-10).

## Boundaries

Frontière par activité : l'empaquetage et la mise en ligne. Le README d'architecture est livré bien plus tôt, dans l'epic 1 ; cet epic porte la documentation d'installation et d'usage, et la publication sur le dépôt d'extensions.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities
- constraint — `CLAUDE.md` §10 licence, §2.7 rien de militaire dans le dépôt public
- spine — `../architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`, AD-17 distribution en deux artefacts et ordre de recherche du noyau, et l'enveloppe d'exploitation

## Notes

- Decision: le README d'architecture avec schémas Mermaid est avancé à l'epic 1, cet epic ne portant que l'installation et l'usage (2026-10-09).
- Decision: le dépôt d'extensions QGIS devient le canal de distribution principal ; l'installeur d'application autonome disparaît du MVP avec la V2 (2026-10-14).
- Decision: distribution en deux artefacts — l'extension au dépôt d'extensions QGIS, sans binaire et sous 25 Mo ; un paquet Cassini par système (Windows, macOS, Linux) qui installe le noyau et, en option, le modèle. Le noyau seul découvre les modèles. Une clé USB portant les deux suffit sur un poste isolé. Contrat d'architecture, AD-17 (propriétaire, 2026-10-14).
- Decision: distribution par deux portes (changement de cap `change-distribution-deux-portes`, AD-17 amendé). Porte 1, un installeur léger en ligne de quelques Mo, télécharge une seule fois, pendant l'installation, le noyau depuis une release GitHub et le modèle depuis Hugging Face, à des adresses fixes, et vérifie l'empreinte SHA-256 de chaque fichier avant de l'installer ; porte 2, un installeur complet hors ligne, un seul fichier qui contient le noyau et le modèle, pour les postes isolés du réseau ; pas de « pack USB ». L'installeur est un programme séparé ; le noyau et l'extension ne changent pas et ne touchent jamais au réseau. Le modèle est hébergé sur un dépôt public Hugging Face, avec sa licence. Installeurs non signés au début, avec une page qui explique l'avertissement de Windows ; signature plus tard, avant les premiers vrais clients. Windows et Linux d'abord, macOS plus tard, quand un Mac est disponible. Remplace, pour ces points, la décision du 2026-10-14 ci-dessus (propriétaire, 2026-11-10).
- Decision: compléments du 2026-11-10 aux deux portes (changement de cap `change-distribution-deux-portes`) : l'installeur hors ligne, qui dépasse 2 Gio, est hébergé sur Hugging Face à côté du modèle et contient aussi l'archive de l'extension QGIS, installable à la main ; le modèle reste facultatif dans l'installeur en ligne ; un manifeste des versions publié dans la release GitHub, à adresse fixe, donne la version et le SHA-256 de chaque fichier. « Mettre à jour Cassini », lancé à la main (menu Démarrer sous Windows, commande sous Linux), est le même programme que l'installeur en ligne : il lit le manifeste, compare à l'installé, propose, télécharge, vérifie l'empreinte, installe. Un nouveau modèle se télécharge à côté des autres, l'ancien n'est jamais supprimé, le sélecteur de modèle du panneau permet de revenir en arrière. L'extension se met à jour par le gestionnaire d'extensions de QGIS. Le panneau affiche la version installée et, passé un délai (par exemple trois mois depuis l'installation ou la dernière mise à jour, calcul de date local, sans réseau), rappelle de lancer « Mettre à jour Cassini ». Aucune vérification automatique en arrière-plan (propriétaire, 2026-11-10).
- Parked: une vérification automatique des mises à jour, désactivée par défaut, pourra venir plus tard si des clients la demandent ; piste, pas décision (2026-11-10).
- Open question: format des installeurs (Windows et Linux) et accélération Vulkan sous Linux, à trancher à l'inception de cet epic (2026-11-10).
- Decision: une page d'accueil publique, une seule page sur GitHub Pages, sans serveur, est livrée avec la publication (Done when 7). Motif : l'extension seule ne fait rien sans le noyau, l'utilisateur qui l'installe depuis QGIS doit trouver où prendre l'installeur, et un bureau d'études veut voir avant d'installer. Un site plus complet (cas d'usage, tarifs, références) viendra avec la commercialisation, hors de cet epic (propriétaire, 2026-11-10).
- Waits on epic 6 because: il n'y a rien à filmer ni à documenter avant que le scénario de démo passe.
