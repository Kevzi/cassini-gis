# Digest — extensions QGIS à modèle de langage, tour 2, assistant 1

Consultation : 2026-10-15. Budget d'appels atteint, d'où des cases incertaines.

| Extension | Hors ligne réel | Confirmation avant exécution | Code généré ou outils typés | Journal rejouable | Licence | Dernière MAJ | Source | Confiance |
|---|---|---|---|---|---|---|---|---|
| GeoAgent (iamtekson) | Incertain. Ollama est annoncé « local and free », mais d'autres fournisseurs existent (OpenAI, Gemini, Claude). Aucune preuve lue d'absence d'appel réseau. | Non lu | Algorithmes Processing en chaîne multi-étapes (outils) ; mode General en plus | Onglet « Logs » : étapes, paramètres, jetons. Export en modèle `.model3` rejouable. | MIT (le tout sous GPL une fois dans QGIS) | Date du dernier commit non lue ; 46 étoiles | https://github.com/iamtekson/GeoAgent | Moyenne |
| AtQuery (id 5217) | Oui, selon la page : « Fully local execution through Ollama (no cloud APIs required) ». Annonce de l'éditeur, non vérifiée dans le code. | Non indiqué ; seulement « transparent tool execution and result display » | Outils typés, avec routage par « toolbox » avant le modèle. Tags « tool calling » et « code generation » présents. | Non indiqué | Non lue | v1.1, 2026-05-17 ; 4 votes | https://plugins.qgis.org/plugins/AtQuery/ | Moyenne (page seule) |
| Promptly (id 3788) | Incertain. Ollama est pris en charge, mais le plugin utilise aussi OpenAI, OpenRouter, Anthropic. Il exige la bibliothèque `requests`. | Non indiqué ; la page parle d'exécution avec correction d'erreurs | Code Python généré et exécuté | Non indiqué | Non lue | v2.0 ; dernière date de version non lue (la liste du tag donne 2025-03-20 comme date de création) | https://plugins.qgis.org/plugins/promptly/ | Moyenne |
| QGIS Agent (bunkmr, id 5459) | Non démontré. Fournisseurs cloud listés (DeepSeek, Gemini, GLM, Mimo) ; aucune mention d'Ollama ou de modèle local. Le serveur MCP écoute sur 127.0.0.1 seulement. | Oui pour les opérations dangereuses (confirmation à l'écran) ; outils privilégiés du MCP masqués par défaut | Mixte : 23 outils, 679 entrées d'algorithmes Processing, et exécution PyQGIS avec analyse AST de sécurité | « Enregistrement et rejeu » d'un flux d'opérations mentionné ; pas d'historique décrit | Non indiquée | v2.4.15, 2026-10-09 ; 177 téléchargements pour cette version | https://plugins.qgis.org/plugins/qgis_agent/ (dépôt : github.com/bunkmr/qgis-agent) | Moyenne |
| AgenticGIS (id 5386) | Incertain. Clé d'API, point d'accès compatible ou agent CLI local. Rien sur Ollama ou llama.cpp. | Non indiqué | Code PyQGIS généré et exécuté (« generating and running PyQGIS code ») | Non indiqué | Non lue | v0.4.6, 2026-10-05 ; 4 501 téléchargements (colonne du tag) | https://plugins.qgis.org/plugins/AgenticGIS/ | Moyenne |
| OpenGeoAgent (id open_geoagent) | Incertain, plutôt possible. La doc liste Ollama et le fournisseur « openai-compatible » (llama.cpp, LM Studio). La reconnaissance vocale passe par OpenAI en option. Aucune preuve d'absence d'appel réseau. | Oui, par un « confirmation hook » (suppression de couches, sauvegarde, traitements, scripts PyQGIS de repli). MAIS le profil par défaut au premier lancement est « Trusted auto-approve », donc sans confirmation. | Outils typés, avec PyQGIS en repli | Non indiqué | MIT (GeoAgent) | v1.8.2 ; date non lue (doc modifiée 2026-08-09) | https://geoagent.gishub.org/qgis-plugin | Moyenne |
| AI Agent by TerraLab | Non. La page dit envoyer la requête et une description du projet à agent.terra-lab.ai (UE) ; palier gratuit sans carte. Aucune mention de modèle local. | Oui : « you see every step before it runs », changements soumis à approbation, annulation en un clic | 253 outils prédéfinis. Génération de code non précisée. | Une conversation par projet ; pas de rejeu décrit | Non lue (dépôt TerraLabAI/QGIS_AI-Agent) | v1.7.1, 2026-10-11 ; 7 640 téléchargements (colonne du tag) | https://plugins.qgis.org/plugins/AI_Agent/ | Moyenne-haute sur « non hors ligne » |
| QGIS MCP (Karasiak, id 4831) | Incertain. C'est un serveur socket TCP piloté par un client MCP externe ; la limitation à localhost n'est pas dite. Le modèle est celui du client. | Non indiqué | 118 outils MCP typés ; exécution de code non mentionnée | Non indiqué | Non lue (dépôt nkarasiak/qgis-mcp) | v0.15.1, 2026-10-07 ; 4 401 (colonne non étiquetée, probablement téléchargements) | https://plugins.qgis.org/plugins/qgis_mcp_plugin/ | Moyenne |

**Défense, postes isolés, sécurité :** aucune des huit ne cible explicitement les postes isolés, la défense ou la sécurité. Seul AtQuery met en avant le « no cloud APIs ». Le modèle de confiance de ces extensions n'a pas été examiné.

## Constats d'ensemble (faits lus)

- AtQuery est la seule dont la page annonce une exécution entièrement locale par Ollama, avec des outils routés.
- Une seule extension montre explicitement le plan avant l'exécution (TerraLab), mais elle est en nuage. OpenGeoAgent confirme les actions sensibles, mais n'impose pas la confirmation par défaut.
- Seuls GeoAgent (Logs et `.model3`) et QGIS Agent (enregistrement et rejeu) décrivent une trace des étapes.
- Aucune ne mentionne un plan validé de bout en bout, ni un journal rejouable à l'identique, ni un registre de commandes unique.

## Pistes à creuser

- Lire le README et le code d'AtQuery (dépôt non identifié) : appels réseau, confirmation, licence.
- Lire le code de GeoAgent : fonctionnement du plan, du `.model3` et de l'hôte Ollama, avec la licence et la date du dernier commit.
- Vérifier le « replay » de QGIS Agent (github.com/bunkmr/qgis-agent) et le fonctionnement hors ligne d'OpenGeoAgent avec llama.cpp.
- Examiner les autres extensions du tag, notamment « AI Agent » de mzarov (1 518 téléchargements), QGIS AI Agent (nl_qgis_agent), QGPT Agent, Q-LLM et IntelliGeo.

## Cherché et non trouvé

- Licence des pages de plugins : non lue pour AtQuery, Promptly, AgenticGIS, TerraLab, QGIS Agent et QGIS MCP. Le lien du dépôt GitHub (non lu) est le seul recours.
- Dates de dernier commit et nombre d'étoiles : seulement GeoAgent (46 étoiles) ; aucune date de commit.
- Preuve d'absence de télémétrie ou d'appel réseau dans le code : aucune, sur les huit.
- Page de confirmation avant exécution pour GeoAgent, AtQuery, AgenticGIS, Promptly et QGIS MCP : non trouvée.
- Dépôt GitHub propre à OpenGeoAgent : non trouvé ; le code semble être dans opengeos/GeoAgent.
- Le dépôt jjgarciaiap/qgis_mcp (deviné) a répondu 404. Le dépôt cité par GeoAgent est jjsantos01/qgis_mcp.
- Le total de téléchargements de la page de chaque plugin n'a pas été lu ; les chiffres viennent des listes de tags ou des tableaux de versions, avec les réserves indiquées.
