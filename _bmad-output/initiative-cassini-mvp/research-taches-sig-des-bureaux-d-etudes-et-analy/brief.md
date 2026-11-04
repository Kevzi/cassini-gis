# Consigne de recherche approfondie — tâches SIG des professionnels français

## Contexte

Je conçois un logiciel de SIG de bureau, utilisé hors ligne, que l'on pilote par des demandes en français (« trouve les parcelles constructibles à moins de 500 m d'une gare et sors la carte en PDF »). Il s'appuie sur QGIS pour exécuter les traitements. Il s'adresse aux bureaux d'études (urbanisme, environnement, aménagement), aux collectivités (communes, intercommunalités, départements) et aux étudiants, en particulier aux utilisateurs occasionnels qui savent ce qu'ils veulent sans savoir où cliquer. L'utilisateur apporte ses propres données.

**Décision que cette recherche doit éclairer :** dresser la liste des tâches SIG réellement demandées et faites au quotidien par ces professionnels en France, avec la façon dont ils les formulent, pour (1) écrire les exemples d'entraînement du modèle de langue qui traduit une demande en suite d'opérations, et (2) savoir, pour chaque tâche, quel algorithme de QGIS ou de GDAL l'exécute et sous quelle licence.

Périmètre civil uniquement.

## Questions de recherche

### Axe 1 — Urbanisme, foncier, aménagement
- Quelles analyses SIG un bureau d'études ou un service urbanisme fait-il pour un PLU / PLUi, un diagnostic foncier, une étude de densification, le suivi du « zéro artificialisation nette » (ZAN), une étude de faisabilité, l'instruction du droit des sols (ADS) ?
- Quelles données utilise-t-il (cadastre Etalab / PCI, Géoportail de l'Urbanisme et standard CNIG PLU, BD TOPO, DVF, fichiers fonciers, OCS GE, BAN, INSEE carroyé…) et quelles opérations y applique-t-il (sélection par attribut, jointure, intersection, différence, calcul de surface, agrégation par zone, distance, accessibilité…) ?

### Axe 2 — Environnement
- Quelles analyses pour une évaluation environnementale, une étude d'impact, un inventaire de zones humides, Natura 2000, la trame verte et bleue, un PPRI / risque inondation, les continuités écologiques, l'occupation du sol ?
- Quelles opérations raster y sont courantes (reclassement, seuil, calculatrice raster, statistiques zonales, pente, exposition, bassin versant, écoulement, courbes de niveau, ombrage) ?

### Axe 3 — Collectivités
- Quelles tâches SIG font les services d'une commune ou d'une intercommunalité : cadastre et propriétaires, adressage, voirie, réseaux (eau, assainissement, éclairage), défense extérieure contre l'incendie (DECI), équipements, cimetières, espaces verts, statistiques par quartier ou par IRIS, géocodage d'un fichier d'adresses, cartes pour le conseil municipal ?

### Axe 4 — Fréquence et vocabulaire
- Lesquelles de ces tâches reviennent le plus souvent ? Appuie-toi sur des preuves : cahiers des clauses techniques (CCTP) de marchés publics, offres d'emploi de géomaticien ou chargé d'études SIG, programmes de formation QGIS (CNFPT, organismes privés), guides du Cerema, de l'IGN et du CNIG, fils de discussion du forum GeoRezo, questions posées en français sur QGIS.
- Pour chaque tâche fréquente, cite **trois à cinq formulations réelles ou typiques en français**, telles qu'un praticien les dirait ou les écrirait (vocabulaire métier, abréviations comme « zones U/AU », « EBC », « OAP », « parcelles bâties », « dents creuses »), et le glossaire des termes métier qui changent l'opération à faire (par exemple : « constructible » = zones U et AU).

### Axe 5 — Correspondance technique avec QGIS
- Pour chaque tâche, quel algorithme de la boîte à outils de traitements de QGIS l'exécute ? Donne **l'identifiant exact** (par exemple `native:buffer`, `native:extractbyattribute`, `gdal:contour`, `native:zonalstatisticsfb`) et le **fournisseur** : natif QGIS (C++), GDAL, GRASS, SAGA, ou une extension.
- Indique quand une tâche n'existe que par GRASS, SAGA ou une extension tierce, et quand elle demande plusieurs algorithmes enchaînés (donne la chaîne).
- Pour les versions de QGIS 3.40 LTR à 3.44 : signale les algorithmes ajoutés, renommés ou retirés.

## Exigences de sources

- **Chaque affirmation porte sa source** : URL, éditeur, date de publication. Pas de chiffre sans source.
- Préférer les sources primaires : documentation officielle de QGIS (docs.qgis.org, « Processing algorithms »), documentation GDAL, Cerema, IGN, CNIG, Légifrance pour le cadre réglementaire, BOAMP et profils d'acheteurs pour les CCTP, offres d'emploi récentes.
- Récence : pratiques métier et offres d'emploi de moins de 3 ans ; algorithmes QGIS pour les versions 3.40 à 3.44 ; textes réglementaires (ZAN, Climat et résilience, Code de l'urbanisme) vérifiés dans leur version en vigueur.
- Deux sources indépendantes pour toute affirmation sur la fréquence d'une tâche et pour tout identifiant d'algorithme dont la recommandation dépend.
- Rapporter les preuves contraires, et dire franchement quand les données publiques sont maigres plutôt que de combler le vide.

## Format de sortie attendu

1. **Tableau principal**, une ligne par tâche : `domaine | tâche | fréquence (élevée / moyenne / faible, avec la preuve) | données d'entrée typiques | opérations | algorithme(s) QGIS exact(s) et fournisseur | chaîne si plusieurs étapes | sources [n]`.
2. **Formulations en français** : pour chaque tâche fréquente, trois à cinq demandes types, avec leurs variantes (avec ou sans distance, avec filtre, avec export de carte).
3. **Glossaire métier** : terme → sens opérationnel (quelle sélection ou quelle opération il implique).
4. **Tâches qui n'existent que par GRASS, SAGA ou une extension tierce**, avec leur licence.
5. **Lacunes et incertitudes** : ce qui n'a pas pu être établi.
6. **Liste des sources** numérotée `[n]` : titre, éditeur, URL, date de publication, date de consultation.
