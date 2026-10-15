---
title: 'Recherche concurrentielle : SIG pilotés en langage naturel, hors ligne, plan validé et journal rejouable'
type: 'competitive'
topic: 'SIG pilotés en langage naturel, hors ligne, plan validé et journal rejouable'
decision: 'Le créneau SIG hors ligne piloté par une IA locale, plan validé et journal rejouable est-il peu occupé, et par qui ?'
source: 'run'
status: complete
preset: 'standard'
validation: 'normal'
verified: 14
unverified: 6
created: '2026-10-15'
updated: '2026-10-15'
---

# Recherche concurrentielle : SIG pilotés en langage naturel, hors ligne, plan validé et journal rejouable

**Décision servie :** le créneau « SIG hors ligne piloté par une IA locale, plan validé et journal rejouable » est-il peu occupé, et par qui ?

## Résumé

**Réponse courte : « parler à l'IA » n'est pas un créneau libre, mais la combinaison complète n'apparaît chez personne dans ce qui a pu être lu.**

1. **Le créneau « IA en langage naturel dans QGIS » est déjà très occupé.** Le dépôt d'extensions compte au moins 20 extensions étiquetées `llm`, avec des mises à jour de la semaine dernière [1]. Plusieurs annoncent un modèle local : AtQuery dit « exécution entièrement locale par Ollama » [2], GeoAgent (MIT) et OpenGeoAgent prennent en charge Ollama [3][4]. Présenter Cassini comme « le premier SIG où l'on parle à l'IA » ne tient pas.
2. **Aucune des extensions examinées ne réunit la combinaison complète** : hors ligne par construction, plan validé en bloc avant exécution, journal rejouable à l'identique, registre de commandes unique. La lecture du code (cinq dépôts, [21]) nuance pourtant les pages : GeoAgent **valide les paramètres avec QGIS avant chaque étape** et exporte un modèle `.model3`, mais exécute sans arrêt pour l'utilisateur et parle à un **serveur HTTP local** (Ollama), avec installation de paquets et téléchargement de modèles possibles ; QGIS Agent a déjà des **modèles de workflow enregistrés et rejouables**, mais génère du PyQGIS relu par un LLM ; OpenGeoAgent confirme outil par outil, avec « Trusted auto-approve » en tête de ses profils ; TerraLab montre chaque étape, mais dans le cloud [5]. Ce qui reste distinct : un **plan complet validé contre des schémas avant tout geste**, un **journal rejouable à l'identique**, et un **modèle lié au processus sans serveur** (AD-15).
3. **Les offres commerciales sont dans le cloud** : Felt [9][10], CARTO [11] et l'assistant d'ArcGIS Pro (bêta sur invitation) [7]. Esri « explore » un mode déconnecté sans date [8]. Côté défense, la seule trouvaille solide est SitaWare Insight, un système de commandement qui garde son LLM hors Internet [13] ; rien trouvé pour un SIG de bureau.

**Plus grand bémol :** ces conclusions reposent surtout sur des **pages d'éditeur et des résumés de moteur**, pas sur du code lu ni des produits essayés. « Hors ligne » est auto-déclaré partout. Les recherches défense et marchés publics étaient minces (une à quatre par angle) : leur absence de résultat n'est pas une preuve.

## 1. Assistants IA des SIG du marché

- **QGIS :** au moins 20 extensions `llm` [1]. Les plus actives : QGIS MCP (118 outils, v0.15.1, 2026-10-07), AI Agent by TerraLab (cloud, v1.7.1, 2026-10-11), QGIS Agent (v2.4.15, 2026-10-09, cloud, confirmation des opérations dangereuses) [6].
- **Locales d'après leurs pages :** AtQuery (outils routés, v1.1, 2026-05-17) [2] ; GeoAgent (MIT, enchaîne les algorithmes Processing, onglet Logs, export `.model3`) [3] ; OpenGeoAgent (Ollama et « openai-compatible », MIT, v1.8.2) [4]. Promptly a Ollama parmi d'autres fournisseurs et exécute du code Python généré. Aucune ne cible les postes isolés ou la défense.
- **Gouvernance :** QEP428, fusionné le 2026-10-07, interdit toute contribution assistée par IA dans le dépôt `qgis/qgis` ; le texte cité ne vise pas les extensions tierces [12]. Deux propositions ouvertes (QEP429 et QEP430) pourraient en modifier la portée.
- **Esri :** assistant ArcGIS Pro 3.7 en bêta sur invitation, avec génération de code ArcPy, SQL et OpenCypher [7] ; les assistants exigent une connexion ArcGIS Online, un mode déconnecté étant « exploré » pour Enterprise [8] (pages Esri en 403, via résumés de moteur : confiance moyenne).
- **Felt et CARTO :** cloud ; Felt exécute sans attendre d'approbation et ses modifications ne sont pas annulables [9] ; CARTO appelle des outils typés et expose un serveur MCP [11].

## 2. Hors ligne, modèle local, postes isolés, défense

- **SitaWare Insight (Systematic, 2025-09-01) :** chatbot LLM qui « peut être tenu à l'écart de l'Internet », démo sur serveur SECRET autonome avec modèle local ; la page ne dit pas si c'est livré ou prototype [13]. C'est un système de commandement, pas un SIG de bureau.
- **ANSSI-PA-102 (2024-04-29, texte lu) :** ne recommande ni n'exige un modèle hors ligne ; recommande SecNumCloud en cloud public pour des données sensibles (R14) et un mode dégradé sans IA (R15) [14]. SecNumCloud qualifie des hébergeurs d'infrastructure ; aucune qualification d'un service d'IA générative hébergé n'a été trouvée [19].
- **Ministère des Armées :** un agent interne (GenIAl.intradef) sur réseau interne, sans cartographie [20 partiel] ; accord-cadre Mistral publié en 2026-01 [20].
- **Non trouvé :** annonce d'un SIG avec assistant en langage naturel local chez Thales, Airbus, Safran, Sopra Steria, Hexagon ou Palantir ; marchés publics pertinents. Recherches minces, non concluantes.

## 3. Sortie contrainte, plan validé, journal rejouable

- **Génération de code dominante :** GISclaw (Python, GeoPandas) rapporte 97 % de succès moyen sur 50 tâches, avec un modèle local de 14 B qui profite de l'orchestration [17]. GeoBenchX suit la voie des appels typés (23 outils) mais n'a testé que des modèles commerciaux [18]. GIS Copilot génère workflows et code.
- **La contrainte garantit la forme, pas le sens :** deux préimpressions à auteur unique, sur des modèles de 0,5 à 4 B, trouvent une validité à 100 % mais des erreurs sémantiques persistantes, et dans un cas une exactitude plus faible sous contrainte dure [15][16]. Aucune mesure trouvée pour des modèles de 7 à 14 B quantifiés sur des tâches SIG.
- **Journal rejouable :** la pratique répandue est l'enregistrement de ce que le modèle et les outils ont rendu, rejoué sans régénérer ; aucune norme trouvée. Dans le code, QGIS Agent enregistre et rejoue des modèles de workflow à paramètres, GeoAgent exporte un `.model3` [21] : le rejeu existe donc, mais pas sous la forme d'un journal d'audit ajouté en fin de fichier et rejoué à l'identique.

## 4. Positionnement et prix

- **Felt :** Free sans IA, Team 199 $/mois (annuel) pour 250 crédits IA, Business 599 $, Enterprise dès 1 250 $, requêtes arrêtées à épuisement des crédits [10]. **CARTO :** paiement à l'usage puis paliers sur devis, aucun montant, auto-hébergement possible en Enterprise [11]. **ArcGIS :** prix non vérifié. **QGIS et ses extensions :** gratuits.
- **Conséquence :** vendre le seul accès à l'IA contre des crédits est la voie des offres cloud. Une extension gratuite ou libre fixe donc le prix de référence près de zéro pour tout ce qui ressemble à un assistant ; la valeur monnayable est ailleurs (garanties, qualification, module défense).

## Enseignements croisés

- **L'argument « hors ligne » est facile à copier.** Une extension peut brancher Ollama en une semaine, plusieurs l'ont fait [2][3][4]. Ce qui ne se copie pas en une semaine : le plan **validé contre des schémas**, le **journal rejouable**, la **qualification** et la **confiance** d'un module défense. La défendabilité est dans la rigueur, pas dans la nouveauté technique.
- **Le hors ligne n'est pas une obligation légale.** Le guide ANSSI n'en fait pas une exigence [14] : la demande doit venir de postes réellement isolés, là où l'évidence publique est la plus mince. La taille de ce marché n'est pas établie ici.
- **Le risque technique est le même que dans ta mesure.** La littérature dit que la forme garantie ne suffit pas [15][16], et les premiers essais sur 8.5 vont dans ce sens (plans conformes mais faux).

## Preuve contraire

Pas de passe adverse (désactivée). Les contre-arguments les plus forts trouvés : la densité de l'offre QGIS [1] ; une approche par génération de code qui atteint 97 % avec un modèle local de 14 B [17], ce qui affaiblit l'idée que seuls des appels typés fonctionnent (sa faiblesse : l'exécution de code généré est précisément ce que tu veux exclure pour la sûreté).

## Recommandations

1. **Ne pas annoncer « premier SIG piloté par l'IA ».** Annoncer « plan validé, journal rejouable, 100 % hors ligne ». *Confiance : moyenne (absence constatée, non prouvée).* Cible : brief et positionnement.
2. **Lancer GeoAgent et AtQuery soi-même avec Ollama, hors ligne.** Le code a déjà répondu à une partie des questions [21] ; il reste à voir ce que ça donne à l'usage (qualité des plans, gestion des erreurs). AtQuery n'a pas de dépôt identifié : sa page reste la seule source. *Confiance : haute sur le besoin.*
3. **Faire du verdict 8.6 la mesure du différenciateur.** Le taux de plans corrects et la preuve que la validation attrape les plans faux sont l'argument à montrer. *Confiance : moyenne.*
4. **Défense : parler à des utilisateurs avant de coder davantage.** SitaWare montre que des industriels s'en préoccupent, mais ils sont sur le commandement, pas sur l'analyse de bureau. Recherche à reprendre sur BOAMP, TED, Thales, Airbus à Milipol et DSEI. *Confiance : faible.*
5. **Avant de publier au dépôt officiel QGIS, relire la politique du dépôt d'extensions sur les contributions assistées par IA.** QEP428 ne vise pas les extensions d'après le texte cité, mais QEP430 pourrait élargir la portée, et le climat est hostile [12]. *Confiance : faible à moyenne.*

## Questions ouvertes

- AtQuery, GeoAgent et OpenGeoAgent fonctionnent-ils réellement sans réseau, et demandent-ils confirmation avant d'exécuter ? Il faut les installer.
- SitaWare Insight est-il livré ou en démonstration ? Lire la page entière et les communiqués de 2026.
- L'assistant ArcGIS Pro est-il disponible à tous aujourd'hui, et à quel prix ? Les pages Esri sont en 403.
- Existe-t-il des appels d'offres citant un SIG avec IA générative ou langage naturel ? BOAMP et TED non interrogés.
- Quelle est la portée exacte de QEP428 et de QEP430 ? Texte intégral non lu.

## Appendice des sources

| [n] | Affirmation soutenue | Éditeur | Publication | Consultation | Confiance |
|---|---|---|---|---|---|
| [1] | Au moins 20 extensions QGIS `llm` | [plugins.qgis.org](https://plugins.qgis.org/plugins/tags/llm) | 2026-10 | 2026-10-15 | haute |
| [2] | AtQuery local par Ollama | [plugins.qgis.org](https://plugins.qgis.org/plugins/AtQuery/) | 2026-05-17 | 2026-10-15 | moyenne |
| [3] | GeoAgent : Processing, Logs, `.model3`, MIT | [GitHub iamtekson](https://github.com/iamtekson/GeoAgent) | n.d. | 2026-10-15 | haute |
| [4] | OpenGeoAgent : confirmation, auto-approve par défaut | [geoagent.gishub.org](https://geoagent.gishub.org/qgis-plugin) | 2026-08 | 2026-10-15 | moyenne |
| [5] | TerraLab : cloud, étapes visibles | [plugins.qgis.org](https://plugins.qgis.org/plugins/AI_Agent/) | 2026-10-11 | 2026-10-15 | moyenne-haute |
| [6] | QGIS Agent : confirmation, fournisseurs cloud | [plugins.qgis.org](https://plugins.qgis.org/plugins/qgis_agent/) | 2026-10-09 | 2026-10-15 | moyenne |
| [7] | Assistant ArcGIS Pro 3.7 en bêta | [Esri Early Adopter](https://earlyadopter.esri.com/ArcGISProAssistant) | 2026 | 2026-10-15 | haute (existence) |
| [8] | Assistants Esri : connexion requise, déconnecté exploré | [Esri documentation](https://doc.esri.com/en/arcgis-pro/latest/get-started/faq.html) | 2025-2026 | 2026-10-15 | moyenne (via moteur) |
| [9] | Felt AI : cloud, sans approbation | [help.felt.com](https://help.felt.com/felt-ai/ai-assistant) | n.d. | 2026-10-15 | haute |
| [10] | Prix Felt | [felt.com/pricing](https://felt.com/pricing) | n.d. | 2026-10-15 | haute |
| [11] | CARTO : prix et agents | [carto.com/pricing](https://carto.com/pricing) | n.d. | 2026-10-15 | haute |
| [12] | QEP428, fusion le 2026-10-07 | [GitHub QGIS-Enhancement-Proposals #387](https://github.com/qgis/QGIS-Enhancement-Proposals/pull/387) | 2026-10-07 | 2026-10-15 | haute (date), moyenne (portée) |
| [13] | SitaWare Insight, LLM hors Internet | [Systematic](https://systematic.com/us/industries/defense/news-knowledge/news/sitaware-insight-delivers-new-ai-features-for-military-operations/) | 2025-09-01 | 2026-10-15 | moyenne |
| [14] | ANSSI-PA-102 | [ANSSI](https://messervices.cyber.gouv.fr/documents-guides/security_recommandations_for_a_generative_ai_system.pdf) | 2024-04-29 | 2026-10-15 | haute |
| [15] | Constraint Tax | [arXiv 2605.26128 via alphaxiv](https://www.alphaxiv.org/abs/2605.26128) | 2026-05-20 | 2026-10-15 | moyenne |
| [16] | Outlines et XGrammar | [arXiv 2609.23742](https://arxiv.org/abs/2609.23742) | 2026-09-20 | 2026-10-15 | moyenne |
| [17] | GISclaw | [arXiv 2603.26845](https://arxiv.org/abs/2603.26845) | 2026-03-27 | 2026-10-15 | haute (abstract) |
| [18] | GeoBenchX | [arXiv 2503.18129](https://arxiv.org/abs/2503.18129) | 2025-03 | 2026-10-15 | haute |
| [19] | SecNumCloud IaaS, 2026-09-01 | [Solutions Numériques](https://www.solutions-numeriques.com/secnumcloud-avec-ovhcloud-numspot-rejoint-le-cercle-restreint-des-clouds-qualifies/) | 2026-09-02 | 2026-10-15 | moyenne |
| [21] | Lecture du code : GeoAgent, OpenGeoAgent, QGIS Agent, TerraLab, QGIS MCP (réseau, confirmation, rejeu, licences) | Dépôts GitHub clonés en lecture seule ; voir `digests/code-r3-verification.md` | 2026-10-05 à 2026-10-11 (derniers commits) | 2026-10-15 | haute (code lu), sauf points listés comme non vérifiés |
| [20] | Armées : GenIAl.intradef et accord Mistral | [Acteurs Publics](https://acteurspublics.fr/articles/le-ministere-des-armees-deploie-son-chatgpt-maison/) ; [CBNews](https://www.cbnews.fr/node/89962) | 2025-02 ; 2026-01 | 2026-10-15 | haute ; moyenne |

## Carte de péremption

Calculée avec `recon_kit.py staleness` (capacités, prix ≤ 3 mois ; gouvernance ≤ 6 ; réglementation et benchmarks ≤ 12). **À revérifier déjà :** [13] SitaWare (2025-09, à revoir depuis 2025-12-01), [7] ArcGIS Pro en bêta (depuis 2026-08-01), [2] AtQuery (depuis 2026-08-17). **À revérifier ensuite :** [4] OpenGeoAgent le 2026-11-09 ; [1], [3], [10], [11] le 2027-01 ; [12] QEP428 le 2027-04-07. **Prochaine date : immédiate.**
