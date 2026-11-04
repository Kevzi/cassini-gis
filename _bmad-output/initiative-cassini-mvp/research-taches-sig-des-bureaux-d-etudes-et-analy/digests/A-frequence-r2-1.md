# Digest A, tour 2 : offres d'emploi et documents de densification (assistant de recherche, 2026-11-04)

Lu : 12 offres récentes (18/08 au 30/10/2026) en entier, liste emploi GeoRezo https://listes.georezo.net/hyperkitty/list/job@ml.georezo.net/ — 7 publics (Evry, Redon, 2 × Nantes Métropole, CBN Pyrénées, unité SIG de l'État 22, ANCT), 4 privés (Onia Environnement, géomaticien DFCI à Bruges, technicien SIG assainissement à Buc, chef de projet SIG santé à Paris), 1 établissement public (Shom). Écartés : stages, thèses, télédétection universitaire, SUEZ (texte vide). emploi-collectivites.fr : pages de redirection. SDIS 61 (11/2022) hors fenêtre.
2 documents prescriptifs : fiches densification DDTM Somme (V6, non daté, après 2021 et avant 08/2023) ; fiche « Gisements fonciers » n° 71 AUDRR (PDF du 15/06/2026).
Méthode : regex puis arbitrage manuel (faux positifs corrigés).

## Comptes, N = 12 offres
Administrer / alimenter une BDD géographique 12/12 ; diffusion web SIG / flux OGC 7/12 ; automatisation FME / Python 7/12 ; produire des cartes 6/12 ; atlas 1/12 ; mise en page / export PDF nommés 0/12 ; « analyses spatiales » génériques 5/12 ; saisie / mise à jour 5/12 ; SQL nommé 3/12 ; réseaux eau / assainissement 1/12 ; PLU(i), ADS, PCRS 1/12 (Redon) ; raster 1/12 ; statistiques 1/12 (ANCT) ; tampon, intersection, jointure, surface, géocodage, isochrone, MNT/pente : 0/12 ; QGIS cité 4/12 ; ArcGIS / Esri cité 6/12.

## Comptes, N = 2 documents de densification
Enveloppe urbaine 2/2 ; dents creuses parmi les parcelles non bâties 2/2 ; seuil de surface (2 500 m²) 2/2 ; surface en m² par parcelle et table attributaire 1/2 ; croisement cadastre / fichiers fonciers 2/2 ; croisement avec le zonage constructible 1/2.

## Constats
- Les offres décrivent des missions de données (BDD, ETL, diffusion), pas des opérations nommées. moyenne (petit N, liste orientée profils techniques). frequency.
- Onia Environnement (29/10/2026) : « Analyses spatiales et cartographie » ; « Automatisation des traitements (Python apprécié) » ; « Valorisation des données : cartographie, notes techniques, outils interactifs ». haute, vocabulary.
- Redon (18/09/2026) : « Collecter, analyser et interpréter des données spatiales, réaliser des cartes et synthèses utiles à la décision » ; « gestion des PLU(i), de l'ADS et du PCRS ». haute.
- DFCI Bruges (25/09/2026) : « cartographie opérationnelle, atlas, cartes thématiques, et analyses spatiales en appui à la décision » ; « traitement de données vecteur et raster ». haute.
- CBN Pyrénées (31/08/2026) : « analyse de données et production de cartes thématiques » ; « scripts (Python notamment pour Qgis) ». haute.
- Buc, assainissement (18/09/2026) : « Saisir les nouveaux réseaux », « Saisir les branchements », « Mettre à jour les plans et la base de données des réseaux ». haute.
- Nantes Métropole (18/09/2026) : « industrialisation et maintenance des traitements de données géographiques », « ETL FME, le langage python et des requêtes SQL ». haute.
- SDIS 61 (29/11/2022, hors fenêtre) : « Produire des cartes à la demande… », « production de statistiques ». https://listes.georezo.net/hyperkitty/list/job@ml.georezo.net/message/CMERXHUOC2N3FP3I4XE4X723Y3GRZFML/
- DDTM 80 https://www.somme.gouv.fr/contenu/telechargement/46632/312586/file/Fiches_densification_V6.pdf : livrables « le ou les périmètres de l'enveloppe urbaine », « une carte, à la parcelle, du potentiel foncier mobilisable brut et net », table attributaire en tableur (surface disponible en m², numéro de parcelle). haute, practice.
- DDTM 80 : dent creuse = « espace non construit entouré de parcelles bâties (3 au minimum) à l'intérieur de l'enveloppe urbaine », desservi et constructible ; SRADDET Hauts-de-France : plafond 2 500 m². haute.
- DDTM 80 : données Fichiers fonciers (Cerema), BD TOPO, OSM, BASIAS, LOCOMVAC. haute.
- AUDRR https://audrr.fr/sites/default/files/2026-06/71_Gisements_fonciers.pdf (15/06/2026) : part du cadastre, des unités foncières et de l'enveloppe urbaine ; 2 500 m² comme « choix méthodologique » ; « Le parcellaire n'est pas utilisé comme référence systématique » pour l'enveloppe. moyenne.

Lecture de l'assistant : les documents prescriptifs nomment les opérations, pas les offres ; la densification enchaîne enveloppe urbaine → parcelles non bâties → zonage → seuil de surface → surface par parcelle → table → carte à la parcelle.

Pistes : fiches emploi-territorial.fr HTML ; répertoire des métiers CNFPT ; un CCTP sur marches-securises.fr ; fil GeoRezo Gaillac-Graulhet (DZUZ3OWL) ; étude de densification Bourgogne-Fresne (échec du téléchargement).
Cherché sans trouver : fiche CNFPT ; offres de bureaux d'études d'urbanisme sur leurs sites ; un CCTP lisible ; une opération vectorielle nommée dans une offre récente.
