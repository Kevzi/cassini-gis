---
type: epic
title: "Analyse de terrain dans le noyau"
parent: initiative-cassini-mvp
covers: [CAP-11]
after: []
assignee: ""
risk: high
---

# Analyse de terrain dans le noyau

## Description

Les trois commandes de relief du MVP — champ de vision depuis un point, ligne de
vue entre deux points, pente — implémentées **en Rust dans le noyau**, sans
aucune dépendance GPL ou LGPL. QGIS sait déjà faire du champ de vision par GRASS,
mais GRASS est en GPL : un moteur de terrain en licence permissive dans le noyau
est la seule base sur laquelle le module défense privé pourra être construit.
C'est la raison d'être de cet epic, et la raison pour laquelle il reste dans le
noyau au lieu d'être délégué.

## Outcome

L'utilisateur interroge le relief de sa zone d'étude depuis le panneau Cassini,
et le calcul se fait dans un code dont la licence n'interdit rien ; le signal est
que chacune des trois commandes donne un résultat vérifiable par un test, hors
ligne, avec une provenance de code documentée.

## Requirements

La source est la spec de l'initiative, `../spec-cassini-mvp/spec-cassini-mvp.md`, CAP-11,
citée telle quelle par les entrées de `tickets.toml` (inception du 2026-11-03) : à partir d'un
modèle numérique d'élévation ouvert dans le projet, champ de vision, ligne de vue entre deux
points et pente s'exécutent dans le noyau Rust, hors ligne, sans dépendance GPL ni LGPL, chacune
avec un résultat vérifiable par un test. Contraintes : AD-1, AD-8, AD-9, AD-10, AD-19, AD-20,
AD-22 du contrat d'architecture.

## Done when

1. CAP-11 : champ de vision, ligne de vue entre deux points et pente répondent
   sur un modèle numérique d'élévation ouvert dans le projet, chacune comme
   commande du registre et chacune avec son test.
2. Aucune dépendance GPL ni LGPL n'est entrée dans le noyau avec la chaîne
   raster, et la provenance du code de visibilité est documentée commit par
   commit.
3. Les résultats sont reproductibles : même MNT, mêmes paramètres, même sortie,
   vérifié par comparaison.
4. Les trois commandes s'exécutent sans QGIS, depuis la CLI, sur un MNT donné en
   chemin de fichier.
5. Un résultat du noyau se compare à celui de QGIS par GRASS sur le même MNT, et
   l'écart est expliqué ou nul.

## Boundaries

Frontière par unité livrée : les trois commandes de terrain du noyau et la
lecture du MNT dont elles ont besoin. **L'ouverture du MNT comme couche du projet
et son affichage en ombrage appartiennent à QGIS**, donc à l'epic 9. Les six
autres commandes de terrain sont hors MVP. Aucune fonction conçue
pour l'usage militaire : elles sont hors de ce dépôt.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, CAP-11 et section Non-goals
- change — `../change-facade-qgis-avant-app-autonome/change-facade-qgis-avant-app-autonome.md`, section 2.4 : pourquoi trois commandes et pas neuf
- constraint — `CLAUDE.md` §2.2 pas de GPL ni LGPL dans le noyau, §2.9 séparation civil/défense
- research — dossier de recherche initial, hors de ce dépôt (dépôt privé `cassini-defense-prive`, `recherche/research-sig-francais-hors-ligne-pour-la-defense/`), section D4
- spine — `../architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`, AD-1, AD-19, AD-20, AD-22

## Notes

- Decision: réduit de neuf commandes à trois au MVP (champ de vision, ligne de vue, pente). Motif : en mode extension QGIS fait déjà le reste, mais par GRASS qui est en GPL ; trois commandes suffisent à établir une base en licence permissive pour le module défense. Les six autres sont reportées, pas abandonnées (propriétaire, 2026-10-14).
- Decision: GRASS et SAGA restent exclus du noyau car sous GPL ; les algorithmes de visibilité sont repris du code MIT de Whitebox ou réimplémentés depuis la littérature publique (2026-10-09).
- Decision: `whitebox_next_gen` n'expose pas son champ de vision comme bibliothèque, seul le substrat raster l'est sur crates.io. Les trois commandes sont réimplémentées dans le noyau, le portage avec attribution restant permis, et le noyau lit lui-même le MNT dans un unique module remplaçable. Contrat d'architecture, AD-20 (propriétaire, 2026-10-14).
- Unknown: la maturité de `wbgeotiff` 0.1.2 et `wbraster` 0.2.1, bibliothèques jeunes. À trancher par un cas d'essai compilé en première entrée, avant de s'engager sur CAP-11.
- Decision: 2026-11-03 — propriétaire : le noyau accepte tout MNT en système projeté métrique doté d'un code EPSG (Lambert-93, UTM, Lambert zones…) et refuse un MNT en degrés (WGS84) avec une erreur typée ; les points restent dans le système du MNT (AD-19). Tranche la question ouverte de la spec ; « WGS84 au minimum » de CLAUDE.md §5 vaut pour les couches vecteur, pas pour le MNT.
- Waits on epic 2 because: les trois commandes sont des commandes du registre et ont besoin de son contrat.
- Decision: 2026-11-03 — inception : huit entrées en une piste dans `core/` ; traceur = lecture et écriture d'un MNT GeoTIFF dans un module unique, qui tranche la maturité de `wbgeotiff`/`wbraster` (entrée 1, hitl : approbation des dépendances) ; puis pente, chargement des sorties raster dans QGIS, ligne de vue, champ de vision, familles du générateur, comparaison à QGIS/GRASS, ménage.
- Decision: 2026-11-03 — propriétaire : le MNT se désigne par un chemin relatif au projet (AD-9, AD-20), recopié par le modèle depuis l'inventaire, pas par un id de couche.
- Decision: 2026-11-03 — propriétaire : la ligne de vue rend verdict, premier obstacle (Point) et profil d'altitudes borné (AD-8).
- Decision: 2026-11-03 — propriétaire : une entrée apprend les trois commandes au modèle ; le ré-entraînement se fait à part, dans la passe groupée de l'epic agent-et-mcp.
- Decision: 2026-11-03 — propriétaire : pour la démo, le MNT entre dans le projet QGIS à la main ; l'ouverture d'un raster par `layer.add` reste l'entrée 20 de l'epic 10, qui réutilise les MNT de recette (4.1) et l'inventaire des rasters (4.3).
