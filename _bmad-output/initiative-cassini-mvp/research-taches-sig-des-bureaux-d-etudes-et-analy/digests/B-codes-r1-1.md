# Digest B, tour 1 : codes CNIG, DECI, ZAN (assistant de recherche, 2026-11-04)

Légifrance inaccessible (HTTP 403) ; textes lus via JORF republié par une préfecture, Sénat, data.gouv, portail de l'artificialisation, RDDECI préfectoraux.

## Standard CNIG PLU
Source principale : https://cnig.gouv.fr/IMG/pdf/231220_standard_cnig_plu_v2024-01.pdf (v2024-01, 2023-12-20, validé le 14/12/2023). Une v2025-06 (validée le 12/06/2025) existe, changements mineurs selon extrait (notes de version en 404, non lues ; low).
- Classes spatiales : ZONE_URBA, PRESCRIPTION_SURF/_LIN/_PCT, INFO_SURF/_LIN/_PCT, HABILLAGE_* ; DOC_URBA non spatiale ; tables `<INSEE ou SIREN>_<CLASSE>_<DATAPPRO>`. high.
- `ZONE_URBA.TYPEZONE` (C3, ZoneUrbaType) : U, AUc (à urbaniser), AUs (à urbaniser bloquée), A, N ; nom local dans LIBELLE/LIBELONG (« Uc », « 2AUci ») ; RNU : TYPEZONE="U", LIBELLE="ZURNU". p. 21-26. high.
- Prescriptions : `TYPEPSC` (C2) + `STYPEPSC` (C2), PrescriptionUrbaType millésime 2023-12 ; sous-code 00 générique ; NATURE libre. high.
- Informations : `TYPEINF` + `STYPEINF` (InformationUrbaType). high.
- EBC = TYPEPSC 01 (01-00 générique, 01-01 à protéger, 01-02 à créer, 01-03 significatif loi Littoral). p. 30 ; corroboré GeoRezo https://listes.georezo.net/hyperkitty/list/geomatique@ml.georezo.net/thread/3IIEY6BJIICAOIUHFM2GWG5YGCBQHRSR. high.
- Emplacement réservé = TYPEPSC 05 (05-01 voies, 05-02 ouvrages publics, 05-03 installations d'intérêt général, 05-04 espaces verts et continuités, 05-05 logement social, 05-06 servitude de localisation, 05-07 attente de projet global, 05-08 recul du trait de côte) ; anciens 09, 10, 12 transférés. high.
- Patrimoine / paysage = TYPEPSC 07 (07-00 à 07-05) ; 08 terrain cultivé ou non bâti à protéger en zone U ; 25 continuités écologiques / TVB. high.
- OAP = TYPEPSC 18 (18-00 périmètre d'OAP, 18-01 OAP sans règlement) : une prescription, pas une information. p. 31, 48. high.
- Autres : 02-01 interdiction de constructibilité, 02-02 constructibilité sous conditions, 03, 29 densité minimale, 38 emprise au sol, 39 hauteur, 50, 51, 97, 99. high.
- TYPEINF : 16 site archéologique, 21 projet de PPR, 25 PAEN, 27 PEB, 32 taxe d'aménagement, 37 régime forestier, 38 SIS, 41 recul des axes à grande circulation, 98, 99. high.
- Les codes changent entre versions (ex. 11 → 15) : le filtre dépend de la version de la donnée. high.
- « Constructible » : aucun attribut ni recette officielle ; inférence non vérifiée : TYPEZONE ∈ {U, AUc} moins prescriptions 01, 02-01, 05-*, 07-*, éventuellement 08.

## DECI
- Le cadre national (CGCT L2225-1 à 4, R2225-1 à 10, arrêté du 15/12/2015) ne fixe ni distance, ni volume, ni débit : c'est le RDDECI (SDIS, arrêté préfectoral). https://www.senat.fr/questions/base/2020/qSEQ200717509.html (réponse du 20/05/2021). high.
- Distances variables : Eure 200 m vs 400 m chez les voisins (question Sénat 2020-07-30). medium.
- Puy-de-Dôme (2017) : 200 m mesurés par cheminements carrossables ; 400 m au total si plusieurs PEI ; tolérances 400 m / 800 m en rural. https://www.puy-de-dome.gouv.fr/contenu/telechargement/11701/119750/file/rddeci_2017.pdf. high (version en vigueur non vérifiée).
- Charente : distance PEI → entrée principale, par cheminement praticable aux dévidoirs à roues. https://www.charente.gouv.fr/index.php/contenu/telechargement/21459/143134/file/Réglement%20départemental%20DECI.pdf. high.
- Ardèche : « par les cheminements praticables ». https://www.ardeche.gouv.fr/contenu/telechargement/9864/103218/file/rddeci_pour_publication.pdf. medium.
- Conséquence : un tampon euclidien est une approximation optimiste ; les RDDECI lus mesurent le long des voies (réseau) ; le seuil est départemental.

## ZAN, ENAF, OCS GE
- L101-2-1 : artificialisation nette = solde artificialisation / renaturation ; définition des surfaces artificialisées et non artificialisées. https://artificialisation.developpement-durable.gouv.fr/sites/artificialisation/files/inline-files/2024_0122_Fiche_Artificialisation_V5.pdf (2024-05-15) ; https://www.data.gouv.fr/datasets/ocs-ge-artificialisation. high.
- ENAF 2021-2031 : réduction de la consommation d'ENAF, « création ou extension effective d'espace urbanisé » (art. 194 loi 2021-1104), mesurée en consommation d'espace, sans la nomenclature. Notice du décret 2023-1096 (JORF 28/11/2023, republié préfecture du Rhône). medium.
- Décret 2023-1096 : nomenclature R101-1 en 10 catégories (1° bâti ≥ 50 m², 2° revêtement ≥ 2 500 m², 3° stabilisé/compacté, 4° herbacé à usage résidentiel/secondaire/tertiaire/infrastructure, 5° chantier ou abandon ; 6° à 10° non artificialisés) ; linéaires ≥ 5 m ; herbacé si < 25 % arboré ; selon l'occupation effective, pas le zonage. high (version d'origine ; consolidée non vérifiée).
- Exceptions R101-1 III (parcs publics, photovoltaïque) non prises en compte par la mesure OCS GE. high.
- Méthode OCS GE : artificialisé = CS1.1.* sauf CS1.1.2.1 × US1.3 (extraction), plus CS2.2.* à usage résidentiel, mixte, transport, transition, abandonné, secondaire, tertiaire ; regroupement puis seuils. Fiche V5 p. 4. high.
- Codes CS/US (matrice 2022-05-03, libellés tronqués, antérieure au décret). medium.
- OCS GE nouvelle génération (IGN, DGALN) : couches d'artificialisation sur toute la France, GeoPackage, deux millésimes, depuis 09/2025. high.

Pistes : standard v2025-06 ; texte de l'arrêté DECI 2015 ; versions consolidées R101-1 / L101-2-1 (loi 2023-630, textes ZAN 2025, croyance non vérifiée) ; nomenclature OCS GE v2 complète ; question Sénat 02/2025 ; définition de l'« espace urbanisé » (fichiers fonciers).
Cherché sans trouver : définition officielle de « constructible » ; texte de l'arrêté 2015 ; RDDECI Yvelines ; notes de version CNIG v2025-06 ; Légifrance (403) ; dates des RDDECI Charente et Ardèche.
