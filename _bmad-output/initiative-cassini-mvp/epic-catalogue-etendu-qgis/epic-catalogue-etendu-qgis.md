---
type: epic
title: "Catalogue étendu par les algorithmes de traitement de QGIS"
parent: initiative-cassini-mvp
covers: [CAP-3, CAP-6, CAP-9]
after: []
assignee: ""
risk: high
---

# Catalogue étendu par les algorithmes de traitement de QGIS

## Description

Après le MVP, Cassini doit répondre à la plupart des demandes réelles de ses utilisateurs, et non aux seules cinq analyses de CAP-3. QGIS fournit déjà, dans son cadre de traitement, l'essentiel de ce que demandent collectivités et bureaux d'études : comptage pondéré dans un polygone, plus proche voisin, plus court chemin et zone de desserte sur réseau, statistiques par catégorie, reprojection, conversion de format, atlas PDF, statistiques zonales. Cet epic expose ces algorithmes comme commandes du registre, décrites une seule fois et typées, pour que l'agent les enchaîne, sans recoder aucun calcul.

## Outcome

Un utilisateur occasionnel obtient une réponse à la plupart de ses questions courantes en langage naturel ; le signal est la part des demandes réelles couvertes par le registre, mesurée sur des phrases réelles d'utilisateurs.

## Requirements

À compléter à l'inception de cet epic, après la décision d'architecture (E1). Portée prévue, à confirmer :

- E1 (CAP-6) : décision d'architecture sur l'exposition d'algorithmes de traitement de QGIS comme commandes du registre — une description par commande, paramètres typés à schéma JSON, aucune expression ni code émis par le modèle (AD-23), délégation déclarée (AD-6) ; les fournisseurs sous GPL (GRASS, extensions) ne s'exécutent que dans le processus de QGIS, jamais dans le noyau (CLAUDE.md §2.2–2.3).
- E2 (CAP-3) : premières primitives courantes — comptage et somme pondérée dans un polygone, plus proche voisin et matrice de distances, plus court chemin sur réseau, statistiques par catégorie et longueurs, reprojection, conversion de format, atlas PDF, statistiques zonales ; tout seuil (distance réglementaire, pente, surface minimale) est un paramètre, jamais une constante.
- E3 (CAP-3) : construire un réseau utilisable pour le calcul d'itinéraires à partir d'une couche de chemins brute (accrochage des extrémités, découpe aux croisements, nettoyage de la topologie).
- E4 (CAP-9) : géocodage hors ligne sur un fichier de la Base Adresse Nationale fourni par l'utilisateur ; le géocodeur intégré de QGIS interroge un service en ligne et est exclu.

## Done when

1. La décision d'architecture E1 est écrite dans le contrat d'architecture et citée par chaque commande ajoutée.
2. Chaque primitive retenue est une commande du registre, exécutée par le cadre de traitement de QGIS, avec un test hors ligne, et l'agent la compose dans un plan valide.
3. Sur un jeu de phrases réelles d'utilisateurs, au moins 70 % des demandes sont couvertes par le registre (un plan valide ou une question justifiée), mesuré et consigné.
4. Aucune commande ajoutée n'accède au réseau, vérifié par la recette hors ligne de CAP-1.
5. Le modèle de composition est ré-entraîné ou ré-éprouvé sur le catalogue étendu, sans baisse du taux de plans conformes.
6. Le catalogue étendu est livré dans une version publiée de l'extension et du paquet du noyau, par la même chaîne que l'epic 7.

## Boundaries

Frontière par capacité : l'élargissement du registre par délégation à QGIS. L'epic 9 garde les cinq analyses de CAP-3 qu'il livre (tampon, intersection, sélection par attribut, statistiques par zone, isochrones sur un réseau apporté par l'utilisateur) ; cet epic ajoute les autres primitives, la préparation d'un réseau à partir de chemins bruts (E3), qui alimente ensuite les isochrones existantes sans les refaire, et le géocodage local. Pas de nouveau calcul dans le noyau (le terrain reste à l'epic 4), pas de parité avec QGIS visée ni revendiquée (spec, Non-goals), pas de téléchargement de données. Le panneau et l'agent sont réutilisés tels quels.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, CAP-3, CAP-6, CAP-9, Non-goals
- constraint — `../architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`, AD-6, AD-14, AD-16, AD-22 et AD-23
- constraint — `CLAUDE.md` §2.2 et §2.3 (aucune GPL dans le noyau, extension à distance de bras), §2.5 (hors ligne total)

## Notes

- Decision: epic post-MVP, construit après la démo (epic 6) pour garder le MVP petit et solide ; demandé par le propriétaire, qui veut à terme un outil complet s'appuyant sur les algorithmes de QGIS (2026-10-17).
- Decision: l'objectif de couverture est d'au moins 70 % des demandes réelles, mesuré sur des phrases réelles d'utilisateurs, pas sur des phrases fabriquées (2026-10-17).
- Decision: l'inception attend la décision d'architecture E1 ; les entrées E2 à E4 ne sont pas découpées avant, leur forme en dépendant (2026-10-17).
- Open question: d'où viennent les phrases réelles de mesure (collecte auprès d'utilisateurs, communauté QGIS) ; le point 3 du Done when en dépend. Le propriétaire fournit ce jeu avant l'inception (au plus tard à la publication, epic 7, qui ouvre le contact avec les premiers utilisateurs).
- Unknown: la fraction des demandes réellement couverte par E2 ; la recherche sur les pratiques SIG suggère que ces primitives reviennent dans la plupart des demandes, sans mesure sur des phrases réelles.
- Source conflict: spec, CAP-3 et CAP-9 et Assumption « compléter QGIS pour les usages courants » — la spec du MVP borne CAP-3 à cinq analyses et CAP-9 à l'ouverture de fichiers ; cet epic va au-delà (primitives supplémentaires, géocodage). Decision: la spec est amendée par `bmad-spec` (nouvelle capacité post-MVP ou CAP-3 et CAP-9 élargies) avant l'inception de cet epic, qui attend de toute façon la publication ; la parité avec QGIS reste un non-objectif (2026-10-17).
- Waits on epic 7 because: les deux touchent l'extension et son paquet, et cet epic se livre par la chaîne de publication.
- Waits on epic 6 because: le MVP passe d'abord ; la démo complète fige le registre et l'agent sur lesquels cet epic s'appuie.
