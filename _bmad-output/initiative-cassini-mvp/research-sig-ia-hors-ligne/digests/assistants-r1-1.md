# Digest — assistants IA des SIG du marché, tour 1, assistant 1

Consultation : 2026-10-15. Budget respecté : 5 pages lues, 6 recherches, 5 pages en 403/404. Plusieurs lignes viennent de résumés de moteur de recherche, signalées « faible ».

| Affirmation | Source (URL) | Éditeur | Date publ. | Consultation | Confiance | Classe |
|---|---|---|---|---|---|---|
| ArcGIS Pro 3.7 « assistant » en bêta. Cinq modes : aide/documentation (défaut), Actions (« actions assistées par IA dans Pro »), génération de code ArcPy, SQL de couches de requête, requêtes OpenCypher. La page ne dit rien sur cloud, licence ou prix. | https://earlyadopter.esri.com/ArcGISProAssistant | Esri (Early Adopter Community) | non indiquée (©2026) | 2026-10-15 | haute (existence, modes) ; inconnu (cloud, prix) | capacité |
| Accès : bêta sur invitation, compte Esri Early Access obligatoire. Pas de disponibilité générale constatée. | même page ; recherche web (résumé) | Esri | non indiquée | 2026-10-15 | moyenne | trajectoire |
| Esri indique des assistants IA dans ArcGIS en bêta/aperçu avec entrée en langage naturel (« Winter 2026 »). Le blog « What's new in AI assistants, Oct 2025 » est en 403, non lu. | https://www.esri.com/arcgis-blog/products/arcgis-online/geoai/whats-new-in-ai-assistants-october-2025 | Esri | 2025-10 (d'après l'URL) | 2026-10-15 | faible (titre/extrait seulement) | trajectoire |
| Add-in tiers non officiel Knight60/ArcGIS-Pro-MCP : expose à un assistant externe via MCP 112 outils et des commandes arcpy arbitraires. | https://enterprisedna.co/directories/mcp/knight60-arcgis-pro-mcp | Enterprise DNA (annuaire) | inconnue | 2026-10-15 | faible (agrégateur) | capacité |
| QGIS, extension AtQuery v1.1 : requêtes SIG en langage naturel, « exécution entièrement locale via Ollama, sans API cloud ». Un routage choisit un domaine d'outils puis ne charge que ceux-là. Étiquettes « tool calling » et « code generation ». Licence non indiquée. | https://plugins.qgis.org/plugins/AtQuery/ | plugins.qgis.org | 2026-05-17 (v1.1) | 2026-10-15 | haute (local, date) ; moyenne (mode d'exécution) | capacité |
| QGIS, extension GeoAgent (MIT) : le LLM choisit des algorithmes Processing (natifs, GDAL, GRASS), remplit leurs paramètres, les chaîne, et peut sortir un modèle .model3. Backends : Ollama (local), OpenAI, Gemini, Claude. Un onglet Logs journalise étapes, paramètres et jetons. Aucune confirmation décrite. Pas de release, 191 commits. | https://github.com/iamtekson/GeoAgent | dépôt GitHub (iamtekson) | non indiquée | 2026-10-15 | haute (page lue) | capacité |
| Autres extensions QGIS : QGISGPT (OpenAI, donc cloud), QGIS Agent (DeepSeek/Gemini/GLM, cloud), IntelliGeo (local non précisé). Un mémoire OST 2024 : les premiers essais avec Llama open source étaient « sous-optimaux ». | https://plugins.qgis.org/plugins/qgisgpt_plugin/ ; https://plugins.qgis.org/plugins/qgis_agent/ ; https://eprints.ost.ch/id/eprint/1261/ | plugins.qgis.org ; OST | 2024-2026 | 2026-10-15 | faible (résumé de moteur) | capacité |
| Projet QGIS : un QEP (« QEP428 »), approuvé le 2026-10-07 selon des sources secondaires, interdit toute contribution assistée par IA au dépôt principal. Texte primaire non lu. Cela concerne le code de QGIS, pas les extensions tierces. | https://lists.osgeo.org/pipermail/qgis-developer/2026-June/068387.html (débat) ; https://aiweekly.co/fr/alerts/qgis-adopte-qep428-et-bannit-les-contributions-assistes-par-ia | liste qgis-developer ; aiweekly.co | 2026-06 ; 2026-10 | 2026-10-15 | faible à moyenne (secondaire) | trajectoire |
| Felt AI : édite la carte en direct, fait des analyses spatiales (tampons, jointures), écrit et exécute du SQL en lecture seule (visible en dépliant les étapes), style, export PNG/JPG/PDF. N'attend pas d'approbation, sauf requête ambiguë. Modifications non annulables. Modèles de pointe via Amazon Bedrock, donc cloud obligatoire, aucun mode hors ligne. | https://help.felt.com/felt-ai/ai-assistant | Felt | non indiquée | 2026-10-15 | haute | capacité |
| Felt, prix annoncé : IA incluse dans tous les plans payants. Team 250 crédits IA/mois (199 $/mois annuel, 249 $ mensuel, 3 sièges), Business 1 000, Enterprise 2 500 ; MCP sur Business et Enterprise ; 50 crédits en plus = 5 $/mois. Lu seulement via un résumé de recherche (blog « Felt's new pricing »), page non ouverte. | https://felt.com/blog/felts-new-pricing | Felt | ~2026-10 (selon résumé) | 2026-10-15 | faible à moyenne | prix |
| CARTO : agents IA dans Builder, désactivés par défaut. Modèles Gemini gérés par CARTO, ou clé Google/OpenAI apportée. Outils typés (géocodage, zoom, filtre, couches stylées) plus outils MCP issus de Workflows. Serveur MCP : le calcul reste dans l'entrepôt client (BigQuery, Snowflake). Compte CARTO requis, donc cloud. Prix non trouvé. | https://docs.carto.com/carto-user-manual/settings/carto-ai ; https://carto.com/blog/carto-mcp-server-turn-your-ai-agents-into-geospatial-experts/ | CARTO | non indiquées | 2026-10-15 | moyenne (résumé de recherche) | capacité |

## Réponses synthétiques

- Mode de fonctionnement : les outils du marché mélangent les deux approches. Esri génère du code (ArcPy, SQL, OpenCypher) en plus d'actions ; Felt génère du SQL ; CARTO appelle des outils typés et MCP ; GeoAgent appelle des algorithmes Processing paramétrés. Validation du plan avant exécution : Felt dit explicitement ne pas en demander. Pour Esri, CARTO et GeoAgent, aucune validation n'est documentée dans ce qui a été lu. Journal : seul GeoAgent a un journal des étapes, et il exporte un modèle .model3 rejouable. Aucun journal rejouable n'est documenté chez Esri, Felt ni CARTO.
- Hors ligne : seuls AtQuery et GeoAgent (QGIS, via Ollama) peuvent tourner avec un modèle local. Les offres Esri, Felt et CARTO sont cloud ou non documentées.
- Prix et conditions : Esri non publié (bêta sur invitation). QGIS et ses extensions sont gratuits (AtQuery : licence non indiquée). Felt et CARTO : voir lignes ci-dessus.

## Pistes à creuser

- Texte primaire du QEP QGIS (dépôt GitHub qgis/QGIS-Enhancement-Proposals), pour confirmer la date et la portée.
- Page tarifs officielle de Felt ; la page ArcGIS Pro 3.7 assistant hors bêta ; les blogs Esri en 403 (à lire par un autre canal) pour savoir si ou quand l'assistant devient généralement disponible et à quel prix.
- Dépôts et dates de commits d'AtQuery, GeoAgent et de l'extension QGIS Agent. Tester si une validation avant exécution existe.
- Google (Gemini dans BigQuery/Earth), Mapbox, et d'autres projets MCP pour QGIS (par ex. « Smart-QGIS »). Non cherchés faute de budget.
- Prix CARTO et Esri : non vérifiés.

## Cherché et non trouvé

- Aucune documentation officielle Esri indiquant un assistant Pro fonctionnant hors ligne ou avec un modèle local.
- Aucun journal rejouable ni validation de plan documentés chez Esri, Felt ou CARTO.
- Aucune déclaration officielle du projet QGIS sur l'IA lue directement : seulement des relais secondaires.
- Aucune information sur Google et Mapbox (non cherchés).
- Date de dernière version de GeoAgent : non affichée.
