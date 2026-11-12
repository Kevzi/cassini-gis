---
type: epic
title: "Publication : depot d'extensions QGIS, installeurs, video"
parent: initiative-cassini-mvp
covers: ["CAP-1", "CAP-4", "CAP-13", "CAP-14"]
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

Source : la spec de l'initiative, `../spec-cassini-mvp/spec-cassini-mvp.md`, sections
Capabilities et Constraints, et le contrat d'architecture, AD-16 et AD-17. Écrites à
l'inception du 2026-11-12.

- R1 : l'extension est publiée sur le dépôt d'extensions QGIS ; son `metadata.txt` est
  complet, `homepage` compris ; son archive ne contient aucun binaire et pèse moins de
  25 Mo. (CAP-13 ; Constraints, distribution en deux artefacts ; AD-17)
- R2 : sur une étiquette de version, la recette construit le noyau Windows (gpu, Vulkan) et
  Linux (processeur), publie une release GitHub avec le manifeste des versions (version et
  SHA-256 de chaque fichier), et la garde hors ligne tourne sur le binaire livré. (CAP-13,
  CAP-1)
- R3 : porte 1, l'installeur en ligne, télécharge une seule fois, à des adresses fixes,
  vérifie le SHA-256 de chaque fichier avant de l'installer, laisse le modèle facultatif et
  écrit la déclaration `*.modele.json` avec ses empreintes ; « Mettre à jour Cassini » est
  le même programme et ne supprime jamais un ancien modèle. (CAP-13, CAP-4, CAP-1 ; AD-17)
- R4 : porte 2, l'installeur complet hors ligne, est un seul fichier qui contient le noyau,
  le modèle et l'archive de l'extension ; il est hébergé sur Hugging Face et s'installe
  machine débranchée du réseau. (CAP-1, CAP-13 ; AD-17)
- R5 : le modèle est publié sur un dépôt public Hugging Face avec la licence Apache-2.0, la
  NOTICE de Qwen, la mention des modifications et une fiche qui décrit un usage civil.
  (CAP-4)
- R6 : quand l'extension ne trouve pas le noyau, le panneau dit « Cassini a besoin de son
  moteur pour fonctionner » et un bouton ouvre la page d'accueil dans le navigateur ; au
  retour, le panneau retrouve le noyau sans redémarrer QGIS ; le panneau affiche la version
  installée et, passé un délai, rappelle de lancer « Mettre à jour Cassini », par un calcul
  de date local, sans réseau. (CAP-13, CAP-1 ; AD-17)
- R7 : la release et les installeurs listent les licences de tout ce qui est livré (crates
  du noyau, llama.cpp, Qwen, outil de l'installeur) et expliquent la frontière GPL entre
  l'extension et le noyau. (CAP-13 ; AD-16)
- R8 : une page d'accueil sur GitHub Pages, tirée du dossier `site/`, donne deux phrases de
  présentation, la vidéo de démo en 1080p hébergée avec la page (pas de YouTube),
  l'installation en trois étapes, les liens de téléchargement et leurs empreintes SHA-256,
  une FAQ (données gardées sur le poste, aucun envoi sur le réseau, carte graphique
  conseillée), une page sur l'avertissement SmartScreen publiée avant le premier
  téléchargement, et un contact. (CAP-13)
- R9 : le jeu de phrases réelles d'utilisateurs que CAP-14 mesure, dû au plus tard à la
  publication, est écrit, et sa couverture par le registre actuel (un plan valide ou une
  question justifiée) est mesurée et consignée phrase par phrase ; le seuil de 70 % reste
  à atteindre par l'epic 10, après le catalogue étendu. (CAP-14 ; question ouverte de la
  spec, `.memlog.md`)
- R10 : une personne extérieure au projet installe Cassini et refait la démo par chacune des
  deux portes, avec la seule documentation publiée. (CAP-4, CAP-1)

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
- Decision: quand l'extension ne trouve pas le noyau (ordre d'AD-17), le panneau dit « Cassini a besoin de son moteur pour fonctionner » et offre un bouton « Télécharger l'installeur Cassini » qui ouvre la page d'accueil (Done when 7) dans le navigateur de l'utilisateur ; au retour, le panneau retrouve le noyau sans redémarrer QGIS. L'extension ne télécharge rien elle-même : c'est le navigateur, à la demande de l'utilisateur (AD-17 inchangé). Écarté : un installeur intégré à l'extension, qui mettrait du code réseau dans l'extension de tous les postes, isolés compris, et se heurterait aux antivirus (propriétaire, 2026-11-10).
- Waits on epic 6 because: il n'y a rien à filmer ni à documenter avant que le scénario de démo passe. Depuis l'inception du 2026-11-12, l'attente est portée par les entrées : la page et la vidéo (7.12) attendent la démo répétée et filmée (6.6, fait) et la preuve « aucun envoi » de la sonde (6.5). La méthode garde la déclaration au niveau de l'epic dans le `tickets.toml` de l'initiative ; ce sont les entrées qui l'épinglent.
- Waits on epic 5 because: la porte 1 complète (7.4) écrit la déclaration `*.modele.json` avec les empreintes que le noyau vérifie ; 5.51 doit être construit avant.
- Decision: 2026-11-12 — Q1 : l'installeur Windows est fait avec Inno Setup, sous réserve que l'entrée 1 vérifie dans sa documentation officielle (Context7 ou site de l'éditeur) qu'il sait télécharger un fichier et en vérifier l'empreinte SHA-256 ; sinon, repli sur un petit installeur maison dans sa propre crate, hors du noyau et exclue de la garde hors ligne comme `cassini-app` (party du 2026-11-12, par délégation du propriétaire).
- Decision: 2026-11-12 — Q2 : sous Linux, les deux portes sont une archive tar et un script POSIX qui installent dans `~/.local/share/cassini`, sans droits root (party, par délégation du propriétaire).
- Decision: 2026-11-12 — Q3 : le noyau Linux est construit pour le processeur seul au départ ; l'accélération Vulkan sous Linux est une entrée ultérieure, hors de cet epic, à mettre sur la liste « après publication » (party, par délégation du propriétaire). Tranche l'Open question du 2026-11-10 sur Vulkan sous Linux.
- Decision: 2026-11-12 — Q4 : le noyau gpu est construit par GitHub Actions, avec un SDK Vulkan à version et empreinte fixées, jamais sur le poste du propriétaire (party, par délégation du propriétaire).
- Decision: 2026-11-12 — Q5 : le modèle est publié sur Hugging Face dans `Kevzi/cassini-composition-4b-GGUF`, avec des noms de fichiers versionnés (party, par délégation du propriétaire).
- Decision: 2026-11-12 — Q6 : les installeurs ne sont pas signés au début ; leurs empreintes SHA-256 sont publiées sur la page d'accueil, et la page sur l'avertissement SmartScreen est en ligne avant le premier téléchargement (party, par délégation du propriétaire). Tranche, avec Q1 et Q2, l'Open question du 2026-11-10 sur le format des installeurs.
- Decision: 2026-11-12 — Q7 : le dépôt GitHub passe en public à l'entrée 1, après un audit de tout l'historique fait par une personne (étape hitl de l'entrée 1) (party, par délégation du propriétaire).
- Decision: 2026-11-12 — Q8 : la page d'accueil est `https://kevzi.github.io/cassini-gis`, publiée depuis `site/` ; les adresses fixes sont écrites au contrat d'architecture, sous AD-17 (précision du 2026-11-12) (party, par délégation du propriétaire).
- Decision: 2026-11-12 — Q9 : le noyau et l'extension partagent la version 0.1.0, étiquette `v0.1.0`, et l'extension passe à `experimental=False` (party, par délégation du propriétaire).
- Decision: 2026-11-12 — Q10 : l'epic 6 avance en parallèle de l'epic 7, sur des fichiers disjoints ; 6.1 à 6.3 sont fusionnés, 6.5 et 6.4 sont gardés (voir les Notes de `epic-export-et-demo.md`). Dérogation à `CLAUDE.md` §7 approuvée par délégation du propriétaire.
- Decision: 2026-11-12 — vidéo : `cassini-presentation.mp4` est réencodée en 1080p, sous 25 Mo, et hébergée avec la page d'accueil (party, par délégation du propriétaire).
- Decision: 2026-11-12 — l'epic garde la « Refactor sweep » (7.13) et une suite de bout en bout faite par une personne extérieure (7.14) ; quatorze entrées, au-delà de la fourchette habituelle, parce qu'elles forment peu de voies et un seul propriétaire (party, par délégation du propriétaire).
- Tracer bullet : 7.1 — sur un poste Windows, une étiquette de version fait construire le noyau processeur par la recette et publier une release avec son manifeste ; l'installeur en ligne minimal télécharge le noyau, vérifie son SHA-256 et le pose dans `%LOCALAPPDATA%\Cassini\bin` ; l'extension copiée à la main le trouve ; une empreinte altérée fait refuser l'installation.
- Sequencing: voies. Workflow de release et installeurs, en file : 7.1 → 7.2 → 7.6 → 7.4 → 7.5 → 7.7 → 7.9 (même fichier de workflow et mêmes scripts d'installeur). En parallèle : 7.3 (dépôt Hugging Face), 7.8 puis 7.10 (extension), 7.11 (phrases réelles), puis 7.12 (page d'accueil), 7.13 et 7.14 en fin.
- Sequencing: 7.6 attend 7.2 et 7.4 attend 7.6 parce qu'ils modifient le même workflow de release (job Windows, manifeste) ; c'est une collision, pas un ordre. 7.4 installe donc le noyau gpu par défaut.
- Sequencing: 7.4 publie aussi sur Hugging Face le fichier de la porte 2 construit par 7.2, et ajoute au manifeste les empreintes du modèle et de ce fichier.
- Handoff: l'installeur de 7.1 écrit, à côté du noyau, la version et la date d'installation, dans un format qu'il fixe ; 7.5 les met à jour ; le panneau (7.8) les lit pour afficher la version et calculer le rappel.
- Handoff: l'archive de l'extension est construite par la recette dès 7.2 (elle entre dans la porte 2) ; 7.10 complète `metadata.txt` et l'icône, et vérifie l'archive contre les exigences du dépôt d'extensions.
- Handoff: le jeu de phrases réelles écrit par 7.11 est celui que reprend 10.8 pour sa mesure de départ ; 10.8 l'attend dans son `after`.
- Check outside: 7.1 (risque high) — l'audit de l'historique est fait par une personne avant le passage en public, et la recette GitHub (garde hors ligne) reste verte sur la release.
- Check outside: 7.2 (risque high) — le propriétaire installe le fichier de la porte 2 dans une VM Windows propre, réseau coupé.
- Decision: 2026-11-12 — adresse publique de contact du projet : cassini-sig@proton.me (Proton Mail, créée par le propriétaire) ; elle va dans `metadata.txt` (7.10) et sur la page d'accueil (7.12), jamais une adresse personnelle.
- Open question: la source du jeu de phrases réelles d'utilisateurs (collecte, communauté QGIS, sources publiques), reprise de l'Open question de l'epic 10 ; portée par 7.11 (2026-11-12).
- Source conflict: spec Constraints, « Le noyau se compile sur Windows, macOS et Linux » vs `.github/workflows/recette.yml`, qui n'a aucun job macOS (2026-11-12).
- Source conflict: commentaire d'en-tête de `.github/workflows/recette.yml`, « Rien d'autre que les actions officielles de GitHub » vs le job `noyau`, qui utilise `EmbarkStudios/cargo-deny-action` (2026-11-12). Confié à 7.1, qui touche le même workflow.
- Source conflict: R1 et AD-17 (dépôt d'extensions) vs `qgis-plugin/cassini_qgis/metadata.txt`, où `email`, `repository` et `tracker` sont vides, `homepage` et `icon` absents, la description dit « (traceur) » et la version est 0.0.1 contre 0.1.0 pour le noyau (2026-11-12). Réglé par 7.10 (Q9).
- Source conflict: amendement d'AD-15 (2026-11-10), qui fait remplir la déclaration `*.modele.json` par l'installeur vs 5.51 (empreinte du modèle), pas encore construit (2026-11-12). 7.4 attend 5.51. Relais inversé : la description et le `verify` de 5.51 demandent que « l'installeur en ligne » écrive les empreintes, alors que l'installeur n'existe qu'à 7.4 ; la partie installeur revient à 7.4, et 5.51 est à corriger à sa revue pour ne garder que la chaîne d'entraînement.
