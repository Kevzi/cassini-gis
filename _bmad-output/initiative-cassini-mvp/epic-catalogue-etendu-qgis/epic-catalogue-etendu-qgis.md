---
type: epic
title: "Catalogue étendu par les algorithmes de traitement de QGIS"
parent: initiative-cassini-mvp
covers: [CAP-6, CAP-14]
after: []
assignee: ""
risk: high
---

# Catalogue étendu par les algorithmes de traitement de QGIS

## Description

Après le MVP, Cassini doit répondre à la plupart des demandes réelles de ses utilisateurs, et non aux seules cinq analyses de CAP-3. QGIS fournit déjà, dans son cadre de traitement, l'essentiel de ce que demandent collectivités et bureaux d'études : comptage pondéré dans un polygone, plus proche voisin, plus court chemin et zone de desserte sur réseau, statistiques par catégorie, reprojection, conversion de format, atlas PDF, statistiques zonales. Cet epic expose ces algorithmes comme commandes du registre, décrites une seule fois et typées, pour que l'agent les enchaîne, sans recoder aucun calcul de QGIS. Seul le géocodage sur la Base Adresse Nationale est calculé par le noyau (AD-26).

## Outcome

Un utilisateur occasionnel obtient une réponse à la plupart de ses questions courantes en langage naturel ; le signal est la part des demandes réelles couvertes par le registre, mesurée sur des phrases réelles d'utilisateurs.

## Requirements

La source est la spec, `CAP-14` (catalogue étendu, après le MVP) et les contraintes qui la nomment, avec `CAP-6` pour le registre. La forme est fixée par le contrat d'architecture, AD-24 à AD-28 (décision E1, 2026-10-18) :

- E1 (CAP-6, CAP-14) : **fait** — AD-24 à AD-28 écrites dans le contrat d'architecture ; chaque commande ajoutée les cite.
- Mise à niveau (CAP-6, CAP-14) : le code passe au niveau d'AD-24, AD-25, AD-27 et AD-28 avant la première primitive (voir `deferred-work.md`, gate de revue E1).
- E2 (CAP-14) : primitives courantes — comptage et somme pondérés dans un polygone, plus proche voisin et matrice de distances, plus court chemin sur réseau, statistiques par catégorie et longueurs, reprojection, conversion de format, atlas PDF, statistiques zonales d'un raster (MNT compris) ; tout seuil est un paramètre, jamais une constante.
- E3 (CAP-14) : préparer un réseau d'itinéraires à partir d'une couche de chemins brute, lu sans autre traitement par les isochrones et le plus court chemin (AD-28).
- E4 (CAP-14) : géocodage hors ligne, **dans le noyau**, sur un CSV de la Base Adresse Nationale apporté par l'utilisateur ; résultat en EPSG:4326, ajouté ensuite par QGIS (AD-26).

## Done when

1. La décision d'architecture E1 est écrite dans le contrat d'architecture et citée par chaque commande ajoutée.
2. Chaque primitive retenue est une commande du registre, exécutée par le cadre de traitement de QGIS — sauf le géocodage, calculé par le noyau et testé sans QGIS —, avec un test hors ligne, et l'agent la compose dans un plan valide.
3. Sur un jeu de phrases réelles d'utilisateurs, au moins 70 % des demandes sont couvertes par le registre (un plan valide ou une question justifiée), mesuré et consigné.
4. Aucune commande ajoutée n'accède au réseau, vérifié par la recette hors ligne de CAP-1.
5. Le modèle de composition est ré-entraîné ou ré-éprouvé sur le catalogue étendu, sans baisse du taux de plans conformes.
6. Le catalogue étendu est livré dans une version publiée de l'extension et du paquet du noyau, par la même chaîne que l'epic 7.

## Boundaries

Frontière par capacité : l'élargissement du registre par délégation à QGIS. L'epic 9 garde les cinq analyses de CAP-3 qu'il livre (tampon, intersection, sélection par attribut, statistiques par zone, isochrones sur un réseau apporté par l'utilisateur) ; cet epic ajoute les autres primitives, la préparation d'un réseau à partir de chemins bruts (E3), qui alimente ensuite les isochrones existantes sans les refaire, et le géocodage local. Aucun calcul de QGIS n'est refait dans le noyau ; le seul calcul nouveau du noyau est l'appariement d'adresses du géocodage (AD-26) ; le terrain reste à l'epic 4. Pas de parité avec QGIS visée ni revendiquée (spec, Non-goals), pas de téléchargement de données. Le panneau et l'agent sont réutilisés ; seul change ce que le modèle voit du catalogue.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, CAP-6, CAP-14, Constraints (seuils, fournisseurs GPL, géocodage, statistiques zonales), Non-goals
- constraint — `../architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`, AD-6 à AD-12, AD-14, AD-16, AD-19, AD-21 à AD-28, Consistency Conventions (familles de commandes), Deferred (catalogue montré au modèle, couches distantes à l'export, fichier de la BAN)
- reference — `../deferred-work.md`, entrées issues des revues E1 du 2026-10-18 (summary.rs:24, renommage, mise à niveau avant E2)
- constraint — `CLAUDE.md` §2.2 et §2.3 (aucune GPL dans le noyau, extension à distance de bras), §2.5 (hors ligne total)

## Notes

- Decision: epic post-MVP, construit après la démo (epic 6) pour garder le MVP petit et solide ; demandé par le propriétaire, qui veut à terme un outil complet s'appuyant sur les algorithmes de QGIS (2026-10-17).
- Decision: l'objectif de couverture est d'au moins 70 % des demandes réelles, mesuré sur des phrases réelles d'utilisateurs, pas sur des phrases fabriquées (2026-10-17).
- Decision: l'inception attend la décision d'architecture E1 ; les entrées E2 à E4 ne sont pas découpées avant, leur forme en dépendant (2026-10-17). E1 faite le 2026-10-18 (AD-24 à AD-28).
- Source conflict: spec, CAP-3 et CAP-9 et Assumption « compléter QGIS pour les usages courants » — la spec du MVP borne CAP-3 à cinq analyses et CAP-9 à l'ouverture de fichiers ; cet epic va au-delà. Résolu le 2026-10-18 : la spec porte la nouvelle capacité CAP-14 (après le MVP), CAP-3 et CAP-9 restent inchangées ; l'epic couvre désormais CAP-6 et CAP-14.
- Source conflict: AD-26 — le géocodage est calculé par le noyau, contre « pas de nouveau calcul dans le noyau » de l'ancienne frontière de cet epic. Résolu le 2026-10-18 : la spec nomme l'exception (CAP-14 success et Constraints), la frontière ci-dessus est réécrite.
- Decision: un seul epic malgré environ 19 entrées — un seul porteur, et la mesure des 70 % porte sur tout le catalogue (propriétaire, 2026-10-18).
- Decision: le jeu de phrases réelles est collecté tôt, par une entrée hitl placée juste après le traceur ; la mesure de départ sert à réordonner les primitives qui suivent (propriétaire, 2026-10-18).
- Decision: le modèle ne voit que les commandes disponibles dans le QGIS hôte, la partie fixe du prompt étant gardée en cache par ensemble annoncé ; le sous-ensemble choisi par demande est écarté (propriétaire, 2026-10-18). Tranche la question « Catalogue montré au modèle et taille du prompt » du Deferred de l'architecture.
- Decision: le noyau ne lit que des `.csv` de la Base Adresse Nationale ; l'utilisateur décompresse le `.csv.gz` lui-même, aucune crate de décompression n'est ajoutée ; le propriétaire dépose le CSV d'un département dans `../donnees-test-cassini/` avant l'entrée de géocodage (propriétaire, 2026-10-18).
- Decision: le traceur est `data.reproject` (entrée 7), placé après la mise à niveau (entrées 1 à 6) imposée par la gate de revue E1 (« mettre le code au niveau d'AD-24, AD-25, AD-27 et AD-28 avant la première primitive ») ; l'entrée 6 est donc dans son `after` sans être un prérequis technique de la reprojection (2026-10-18).
- Decision: les primitives (entrées 9 à 16 et 20) forment une seule voie, chacune après la précédente, car elles partagent la table fermée de l'extension, les modules du registre et le prompt ; leur ordre est révisé après la mesure de départ de l'entrée 8, et un genre (table, raster, layout) revient à la première primitive qui en a besoin après ce réordonnancement (2026-10-18).
- Decision: pas de suite de bout en bout séparée ; l'entrée 17 (ré-épreuve et mesure de couverture) et l'entrée 19 (livraison vérifiée sur poste débranché) en tiennent lieu (2026-10-18).
- Decision: la préparation de réseau (entrée 14) n'utilise que des algorithmes `native:` — `native:snapgeometries` pour accrocher les extrémités, `native:splitwithlines` de la couche sur elle-même pour découper aux croisements — sans GRASS (`v.clean`), pour rester disponible sur tout poste (AD-25) ; identifiants vérifiés dans la documentation de QGIS 3.44 (propriétaire, qui délègue le choix, 2026-10-18).
- Decision: un export de mise en page ou d'atlas ne dessine que les couches locales, passées au paramètre `LAYERS` de `native:atlaslayouttopdf` et de `native:printlayouttopdf` ; les couches distantes (WMS, WMTS, XYZ, tuiles vectorielles distantes) sont écartées et nommées dans le résultat et dans le plan, jamais en silence (AD-14, AD-21). Tranche « Couches distantes à l'export » du Deferred de l'architecture, pour `layout.export_atlas` (entrée 15) et `layout.export_pdf` (propriétaire, qui délègue le choix, 2026-10-18).
- Open question: d'où viennent les phrases réelles de mesure (collecte auprès d'utilisateurs, communauté QGIS) ; le point 3 du Done when en dépend. Porté par l'entrée hitl de collecte.
- Unknown: la fraction des demandes réellement couverte par E2 ; la recherche sur les pratiques SIG suggère que ces primitives reviennent dans la plupart des demandes, sans mesure sur des phrases réelles.
- Waits on epic 7 because: les deux touchent l'extension et son paquet, et cet epic se livre par la chaîne de publication.
- Waits on epic 6 because: le MVP passe d'abord ; la démo complète fige le registre et l'agent sur lesquels cet epic s'appuie.
