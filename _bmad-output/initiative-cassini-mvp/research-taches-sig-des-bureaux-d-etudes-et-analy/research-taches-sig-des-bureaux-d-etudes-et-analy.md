---
title: 'Recherche domaine et technique : tâches SIG des professionnels français et analyse d''écart du registre'
type: 'domain+technical'
topic: 'Tâches SIG des bureaux d''études, collectivités et cabinets de géomatique en France ; correspondance avec les algorithmes QGIS/GDAL'
decision: 'Bâtir les familles du jeu d''entraînement final et trier chaque tâche : déjà au registre, à écrire dans le noyau Rust, à déléguer à QGIS, ou hors d''atteinte'
source: 'Process du rapport NotebookLM du 2026-11-04 (imports/), puis Deepen par recherche web le 2026-11-04 (digests/A, B, C)'
status: complete
preset: 'standard'
validation: 'normal'
created: '2026-11-04'
updated: '2026-11-04'
claims: {verified: 8, unverified: 1, disputed: 1, overturned: 1} # recon_kit tally, par source [n], dernier statut
---

# Tâches SIG des professionnels français et analyse d'écart du registre Cassini

**Décision servie :** quelles demandes ajouter au jeu d'entraînement final, et pour chacune, si Cassini sait déjà la traiter, s'il faut l'écrire dans le noyau ou la confier à un algorithme de QGIS.

## Résumé

1. **Les gestes de base sont ceux qu'enseignent toutes les formations QGIS** [9] :
   - mise en page et export de carte (3 programmes sur 3) ;
   - sélection attributaire et sélection spatiale, zone tampon, calcul de surface, jointure avec un tableur, analyse thématique (2 sur 3 chacun).

   Cassini couvre déjà la mise en page, les sélections et le tampon. **Il lui manque le calcul de surface et la jointure.**
2. **La tâche d'urbanisme la mieux documentée est l'étude de densification** [11]. Elle enchaîne : enveloppe urbaine, parcelles non bâties, croisement avec le cadastre et le zonage, seuil de surface, surface en m² par parcelle, puis table et carte à la parcelle. Il faut pour cela **la différence et le calcul de surface**, que QGIS a en natif sous les identifiants `native:difference` et `native:fieldcalculator` [3].
3. **Les offres d'emploi ne servent pas à choisir les phrases.** Sur 12 offres récentes, aucune ne nomme une opération précise. Elles décrivent la gestion de bases de données, l'automatisation et des « cartes thématiques à la demande » [10]. Les phrases à enseigner viennent des formations et des documents qui prescrivent une méthode.
4. **Le vocabulaire métier se traduit en codes vérifiés** [6] :
   - l'EBC est la prescription `TYPEPSC 01`, l'OAP la prescription `TYPEPSC 18`, l'emplacement réservé `05`, le patrimoine et le paysage `07` ;
   - les zones sont `TYPEZONE` U, AUc, AUs, A et N.

   « Constructible » n'a **aucune définition officielle** : c'est un choix de méthode. Le rapport NotebookLM se trompait sur ces codes [1].
5. **Mise en garde principale :** la règle DECI n'est pas nationale. Chaque département fixe la distance, et la mesure se fait le long des voies [7]. Une zone tampon de 200 m n'est donc qu'une approximation, que le modèle ne doit pas présenter comme la règle.

## Les tâches, triées

Légende du statut :
- **Registre** : faisable aujourd'hui.
- **QGIS** : à déléguer, l'algorithme natif existe.
- **Noyau** : à écrire en Rust.
- **Hors** : incompatible avec le principe hors ligne, ou n'existe que sous GPL hors de QGIS.

| Tâche | Ce qu'il faut | Statut | Algorithme vérifié |
|---|---|---|---|
| Mise en page et export PDF | `layout.export_pdf` | **Registre** | — |
| Sélection par attribut, par localisation, à moins de N m | sélection, tampon, intersection | **Registre** | — |
| Bâtiments en zone d'aléa, zones humides qui croisent un périmètre, extrait parcellaire | sélection, intersection | **Registre** | — |
| Décompte de points par zone | `analysis.zonal_statistics` | **Registre** | `native:countpointsinpolygon` |
| Pente, ligne de vue, champ de vision | commandes `terrain.*` | **Registre** (noyau) | — |
| **Calcul de surface** (« de plus de 2 500 m² », « surface en hectares ») | champ calculé ou filtre de surface | **QGIS**, priorité 1 | `native:fieldcalculator` [2] |
| **Différence** (« hors EBC », « hors zones humides », dents creuses, bâti hors couverture) | soustraction de couches | **QGIS**, priorité 1 | `native:difference` [3] |
| **Jointure avec un tableur** (DVF, données INSEE) | jointure attributaire | **QGIS**, priorité 2 | `native:joinattributestable` [2] |
| Agrégation, fusion par commune | | **QGIS**, priorité 2 | `native:aggregate`, `native:dissolve` [2] |
| Statistiques d'un raster par zone (artificialisation par commune) | | **QGIS**, priorité 2 | `native:zonalstatisticsfb` [2] |
| Seuil ou reclassement d'un raster (« pente > 15 % ») | | **QGIS**, priorité 3 | `native:rastercalc`, `native:reclassifybytable` [2] |
| Courbes de niveau, ombrage, exposition | | **QGIS** (GDAL), priorité 3 | `gdal:contour`, `gdal:hillshade`, `gdal:aspect` [2] |
| Analyse thématique (catégories, dégradés, symboles proportionnels) | style d'une couche | **QGIS**, à décider (refusé aujourd'hui) | aucun algorithme de traitement : c'est du style |
| Exporter une couche en fichier (Excel, GPKG) | | **QGIS**, à décider (refusé aujourd'hui) | `native:savefeatures` [2] |
| Validité des géométries (préparation GPU) | | **QGIS**, faible priorité | `qgis:checkvalidity`, `native:fixgeometries` [2] |
| Comblement des dépressions, bassins versants | | **QGIS 3.44** | `native:fillsinkswangliu` (pas en 3.40) [3] |
| Accumulation de flux, indice d'humidité (TWI) | | **Noyau** (portage) ou attendre QGIS 4.4 | rien en natif en 3.44 [3] ; `wbtools_oss` (MIT OR Apache-2.0) le fait [4] |
| DECI exacte (distance par les voies) | réseau | **Registre** partiel (isochrone) ; tampon = approximation | — [7] |
| Géocodage d'un fichier d'adresses | | **Hors** en ligne ; hors ligne possible avec la BAN locale et Addok | `native:batchnominatimgeocoder` appelle nominatim.qgis.org [3] ; BAN sous Licence Ouverte, Addok sous MIT [5] |
| Flux WMS/WFS | réseau | **Hors** | — |
| Nettoyage topologique de réseaux | GRASS | **Hors** au MVP | — |

## Ce que cela change pour l'entraînement

- **Familles à ajouter sans nouvelle commande :**
  - extrait parcellaire par numéro ;
  - bâti en zone d'aléa ;
  - zones humides qui croisent un périmètre ;
  - filtres sur les vrais codes CNIG : « en zone AUc », « en EBC » sur une couche de prescriptions où `TYPEPSC = '01'`, « dans une OAP » où `TYPEPSC = '18'`, quand l'inventaire montre ces champs.
- **« Constructible » reste une question ou un choix explicite.** Aucune source ne donne de recette ; le générateur actuel le traduit par les zones U et AU. Il vaut mieux garder cette traduction comme une convention de Cassini, écrite et assumée, plutôt que la présenter comme une règle.
- **Familles qui attendent leur commande** (des refus en attendant) : « de plus de N m² », « en hectares », « hors de… », « à plus de N m de… », « joins le tableau », « courbes de niveau », « par commune » sur un raster.
- **Seuils :** les 2 500 m² des dents creuses et les 200 m de la DECI sont des **paramètres écrits par l'utilisateur**, jamais des constantes [7][11]. Le seuil de surface varie selon les sources : 300 m² contre 2 500 m² [1][11].

## Recommandations

1. **Nouvelle épopée « analyses vectorielles complémentaires », déléguée à QGIS**, dans cet ordre :
   - `analysis.difference` et un calcul ou filtre de surface (ensemble, ils débloquent la densification et les « hors de ») ;
   - la jointure de table ;
   - l'agrégation ;
   - les statistiques de raster par zone ;
   - le seuil raster et les courbes de niveau.

   Confiance haute sur les algorithmes [2][3], moyenne sur la priorité [9][11]. Cela alimente AD-1 (frontière noyau / QGIS) et le registre.
2. **Identifiants :** écrire `native:*` dans l'extension. `qgis:*` n'est qu'un repli de compatibilité [3].
3. **Hydrologie :** comblement et bassins par `native:fillsinkswangliu` sous 3.44. Pour l'accumulation et le TWI, porter dans le noyau depuis `wbtools_oss` (MIT OR Apache-2.0). Il faut d'abord vérifier fichier par fichier qu'aucun outil porté ne relève de la partie propriétaire ni de `wblicense_core` [4]. Hors MVP.
4. **Géocodage hors ligne :** piste réelle (BAN téléchargée par l'utilisateur, Addok en local, sous MIT) [5]. À étudier comme une commande à part, après le MVP.
5. **Documentation et prompt :** la DECI et « constructible » doivent être présentés comme des conventions paramétrables, pas comme du droit [6][7].

## Questions ouvertes

- Le standard CNIG **v2025-06** a-t-il déplacé des codes ? Il n'a pas été lu, seule la v2024-01 l'a été [6].
- Version consolidée du décret 2023-1096 et des textes ZAN de 2025 : Légifrance était inaccessible [8].
- Aucun **CCTP** ni **fiche métier CNFPT** n'a pu être lu. La fréquence des tâches en bureau d'études privé d'urbanisme reste peu documentée.
- Pour l'analyse thématique (style), qui revient dans 2 formations sur 3 : la faire entrer dans le registre ? C'est une décision produit.

## Sources

| [n] | Ce qu'elle appuie | Éditeur | Date | Consultée | Confiance |
|---|---|---|---|---|---|
| [1] | Tâches, formulations, glossaire (en partie faux) | [NotebookLM Deep Research (import)](imports/notebooklm-2026-11-04.md) | 2026-11 | 2026-11-04 | faible |
| [2] | Existence des algorithmes en 3.40 | [QGIS Project, Processing algorithms 3.40](https://docs.qgis.org/3.40/en/docs/user_manual/processing_algs/) | 2024-10 | 2026-11-04 | haute |
| [3] | Identifiants `native:`, repli `qgis:`, ajouts 3.42/3.44, `fillsinkswangliu`, Nominatim en ligne | [QGIS, code source release-3_44](https://github.com/qgis/QGIS/tree/release-3_44/src/analysis/processing) ; [changelog 3.44](https://qgis.org/project/visual-changelogs/visualchangelog344/) | 2025 à 2026-09 | 2026-11-04 | haute |
| [4] | Licence et outils hydrologiques de Whitebox | [whitebox_next_gen](https://github.com/jblindsay/whitebox_next_gen) | 2026-08 | 2026-11-04 | haute (licence), moyenne (outils) |
| [5] | BAN téléchargeable, licence ; Addok sous MIT | [adresse.data.gouv.fr](https://adresse.data.gouv.fr/outils/telechargements) ; [addok](https://github.com/addok/addok) | 2026-10 | 2026-11-04 | haute |
| [6] | Codes TYPEZONE, TYPEPSC, TYPEINF | [CNIG, standard PLU v2024-01](https://cnig.gouv.fr/IMG/pdf/231220_standard_cnig_plu_v2024-01.pdf) ; [GeoRezo](https://listes.georezo.net/hyperkitty/list/geomatique@ml.georezo.net/thread/3IIEY6BJIICAOIUHFM2GWG5YGCBQHRSR) | 2023-12 | 2026-11-04 | haute |
| [7] | DECI : distances départementales, mesure par les voies | [Sénat, réponse ministérielle](https://www.senat.fr/questions/base/2020/qSEQ200717509.html) ; [RDDECI Puy-de-Dôme](https://www.puy-de-dome.gouv.fr/contenu/telechargement/11701/119750/file/rddeci_2017.pdf) ; RDDECI Charente, Ardèche | 2017 à 2021 | 2026-11-04 | haute |
| [8] | Nomenclature d'artificialisation, OCS GE | [Fiche Artificialisation V5](https://artificialisation.developpement-durable.gouv.fr/sites/artificialisation/files/inline-files/2024_0122_Fiche_Artificialisation_V5.pdf) ; JORF du 28/11/2023 ; [OCS GE artificialisation](https://www.data.gouv.fr/datasets/ocs-ge-artificialisation) | 2023-11 à 2025-09 | 2026-11-04 | moyenne |
| [9] | Contenu des formations QGIS | [ARNia BFC](https://backoffice-portail.arnia-bfc.fr/wp-content/uploads/2026/04/Programme-Creer-vos-premieres-cartes-avec-QGIS-2026.pdf) ; [CFPPA Montmorot](https://montmorot.educagri.fr/wp-content/uploads/2025/03/INITIATION-QGIS-21-05.pdf) ; [Sorbonne FC](https://fc.sorbonne-universite.fr/pdf/fiche/?ID=23556) | 2025 à 2026 | 2026-11-04 | haute (petit N) |
| [10] | Contenu des offres d'emploi | [Liste emploi GeoRezo, 08 à 10/2026](https://listes.georezo.net/hyperkitty/list/job@ml.georezo.net/) | 2026-10 | 2026-11-04 | moyenne |
| [11] | Méthode de densification et dents creuses | [DDTM de la Somme](https://www.somme.gouv.fr/contenu/telechargement/46632/312586/file/Fiches_densification_V6.pdf) ; [AUDRR n° 71](https://audrr.fr/sites/default/files/2026-06/71_Gisements_fonciers.pdf) | 2023 à 2026-06 | 2026-11-04 | haute |

## Fraîcheur

- **Identifiants et algorithmes de QGIS** [2][3] : à revérifier à chaque version de QGIS (prochain contrôle le 2026-12-04, ou à la sortie de 4.4).
- **Standard CNIG** [6] et **textes ZAN** [8] : à revérifier à chaque nouvelle version. La v2025-06 est à lire dès maintenant.
- **Offres et formations** [9][10] : à revérifier sous 18 mois (mai 2028).
