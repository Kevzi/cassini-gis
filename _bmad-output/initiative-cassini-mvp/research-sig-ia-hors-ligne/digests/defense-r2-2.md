# Digest — défense, marchés publics, ANSSI, tour 2, assistant 2

Consultation : 2026-10-15. Environ 12 appels d'outils. Limite : plusieurs conclusions viennent de résumés de WebFetch ou de WebSearch, pas de lectures intégrales.

| Affirmation | Source (URL) | Éditeur | Publication | Consultation | Confiance | Classe |
|---|---|---|---|---|---|---|
| SitaWare Insight intègre un chatbot LLM « wingman », dans une fenêtre de chat SitaWare standard. La page dit : « The LLM can be … kept away from the open internet ». Elle rapporte aussi une démo d'entraînement sur serveur SECRET autonome avec modèle d'IA local. La page ne dit pas « offline » ni « disconnected ». Elle ne dit pas non plus si c'est livré ou prototype (formulation « latest update »). | https://systematic.com/us/industries/defense/news-knowledge/news/sitaware-insight-delivers-new-ai-features-for-military-operations/ | Systematic | 2025-09-01 | 2026-10-15 | moyenne | capacité |
| Systematic prépare de l'IA agentique pour l'analyse de courses d'action, avec un outil de briefing amélioré prévu en juillet 2026. | https://www.janes.com/defence-intelligence-insights/defence-news/defence/systematic-develops-new-features-for-sitaware-c2-software | Janes | date non vue | 2026-10-15 | faible | capacité (annonce) |
| La page Eurosatory 2026 de Systematic mentionne l'IA agentique intégrée aux flux de commandement et présente SitaWare comme un système fermé où les nations gardent leurs données. Rien sur un LLM hors ligne. | https://systematic.com/int/industries/defence/news-knowledge/events/eurosatory/ | Systematic | 2026 | 2026-10-15 | moyenne | capacité (annonce) |
| ANSSI-PA-102, texte anglais lu (PDF 1,8 Mo, extrait localement). R11 : hébergement cohérent avec les besoins de sécurité, confidentialité et intégrité, sur les 3 phases. R12 : isoler chaque phase dans un environnement dédié (réseau, système, stockage, comptes). R13 : passerelle Internet sécurisée pour un système hébergé sur Internet. R14 : prioriser SecNumCloud en cloud public si les données sont sensibles, l'impact critique ou les utilisateurs non fiables. R15 : mode dégradé sans IA. R16 : GPU dédiés, ou filtrage IOMMU en virtualisation. Le guide mentionne aussi un déploiement local et le cloud pour l'entraînement. | https://messervices.cyber.gouv.fr/documents-guides/security_recommandations_for_a_generative_ai_system.pdf | ANSSI | 2024-04-29 | 2026-10-15 | haute | réglementation |
| Le guide ne recommande ni n'exige un modèle hors ligne ou isolé du réseau. Les recherches dans le texte ont trouvé « isolation », « hosting » et « Internet » seulement dans le contexte des rôles R11 à R13. Il ne contient aucune règle « air-gap ». Le texte précise que « AI models are considered to have the same level of sensitivity as the data used to design and train them » (circulaire « Cloud au Centre », R9, pour l'administration). | même PDF | ANSSI | 2024-04-29 | 2026-10-15 | haute (absence constatée par recherche de mots-clés, pas par lecture intégrale) | réglementation |
| NumSpot (offre IaaS) et OVHcloud (SNC Cloud Platform) ont obtenu la qualification SecNumCloud le 1er septembre 2026. L'article ne dit pas que la qualification couvre des services d'IA générative. NumSpot dit viser son PaaS (données et IA), non qualifié à ce jour. | https://www.solutions-numeriques.com/secnumcloud-avec-ovhcloud-numspot-rejoint-le-cercle-restreint-des-clouds-qualifies/ | Solutions Numériques | 2026-09-02 | 2026-10-15 | moyenne | réglementation |
| Le décret n° 2026-272 du 14 avril 2026 fixe le cadre technique et contractuel du cloud, avec un référentiel ANSSI approuvé par arrêté. Un prestataire qui sert l'administration peut devoir proposer une offre qualifiée. | https://kohenavocats.fr/?p=2107172 | Kohen Avocats | 2026 | 2026-10-15 | faible (page non lue, vu dans un résumé de recherche) | réglementation |
| Un blog dit que SecNumCloud v3.2 intègre des exigences liées à l'IA, sans détail. Il avance aussi une date de qualification mars 2026, qui contredit la ligne ci-dessus. | https://tech-insider.org/fr/?p=119 | tech-insider | 2026 | 2026-10-15 | faible | autre |
| Le CNIG a tenu un atelier sur l'IA et l'information géolocalisée, avec des échanges sur les LLM comme interface en langage naturel des SIG. Document de réflexion, pas un marché. | https://cnig.gouv.fr/IMG/pdf/2._intelligence_artificielle_et_information_geolocalisee-9-07-2026.pdf | CNIG | 2026-07-09 | 2026-10-15 | moyenne (non lu, vu dans un résumé de recherche) | autre |

**Statut 2026 de SecNumCloud :** il qualifie un hébergeur d'infrastructure, pas un modèle. Les qualifications datées du 1er septembre 2026 portent sur des offres IaaS ou cloud. Aucune qualification d'un service d'IA générative hébergé n'a été trouvée, et aucun texte ANSSI sur v3.2 n'a été lu.

**Seule trouvaille solide côté défense :** SitaWare Insight, avec un LLM tenu hors Internet et une démo sur serveur SECRET autonome (page du 2025-09-01). Aucune annonce « LLM hors ligne » n'a été trouvée pour Thales, Airbus, Safran, Sopra Steria, Esri, Hexagon ou Palantir. Aucune annonce Eurosatory 2026 n'a été trouvée pour un assistant en langage naturel couplé à un SIG fonctionnant en local.

## Pistes à creuser

- Lire intégralement la page SitaWare pour distinguer livré et prototype, puis les communiqués Systematic de juin 2026.
- Consulter le référentiel SecNumCloud v3.2 sur cyber.gouv.fr, et la circulaire « Cloud au Centre » (R9).
- Chercher sur les portails marchés (BOAMP, TED, achats défense) avec les mots « assistant », « LLM » et « SIG ».
- Examiner les annonces Esri, Hexagon et Palantir, avec leurs sites produit.
- Examiner les annonces Thales, Airbus et Safran à Milipol et DSEI 2025-2026.
- Vérifier les détails de l'accord Mistral et du ministère des Armées (GenIAl.intradef).

## Cherché et non trouvé

- Annonces Eurosatory 2026 d'assistant IA générative hors ligne : 1 recherche large, plus 1 sur Systematic. Aucune annonce concernée trouvée. La recherche est mince.
- Appels d'offres et appels à projets (AID, BOAMP, TED) citant un SIG avec IA générative ou langage naturel : 1 recherche, aucun résultat pertinent. Cette absence n'est pas concluante. BOAMP et TED n'ont pas été interrogés directement.
- Autres éditeurs (Thales, Airbus, Safran, Sopra Steria, Esri, Hexagon, Palantir, GenIAl.intradef/Mistral) : 0 recherche ciblée, non vérifiés. Les trouvailles du premier tour restent non confirmées.
- Texte du référentiel SecNumCloud v3.2 sur l'IA : 1 recherche, source primaire non trouvée.
