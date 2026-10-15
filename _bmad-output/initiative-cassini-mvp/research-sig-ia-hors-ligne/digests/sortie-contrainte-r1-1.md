# Digest — sortie contrainte, plan validé, journal rejouable, tour 1, assistant 3

Consultation : 2026-10-15. Toutes les pages ont été lues sauf indication ; seuls les résumés (abstracts) ont été lus, pas les articles complets.

| Affirmation | Source (URL) | Éditeur | Publication | Consultation | Confiance | Classe |
|---|---|---|---|---|---|---|
| GIS Copilot (Penn State) pilote QGIS en langage naturel : le LLM génère workflows ET code ; plus de 100 tâches, 3 niveaux. Taux « élevés » pour basique/intermédiaire, difficulté sur l'avancé. Aucun chiffre ni modèle dans l'abstract. | https://arxiv.org/abs/2411.03205 | arXiv (Akinboyewa, Li, Ning, Lessani) | 2024-11 (v4 2024-11-22) | 2026-10-15 | haute (existence), faible (résultats) | capacité |
| Un article cité par un moteur de recherche donne 86 % de réussite sur 110 tâches pour GIS Copilot. Non vérifié à la source. | https://pure.psu.edu/en/publications/gis-copilot-towards-an-autonomous-gis-agent-for-spatial-analysis/ | Penn State (via agrégateur) | n.d. | 2026-10-15 | faible | benchmark |
| GISclaw : agent open source, bac à sable Python persistant (GeoPandas, rasterio…), pas d'appels typés. Benchmark GeoAnalystBench (50 tâches, 5,8 étapes en moyenne), 1 800 exécutions, 6 modèles dont un 14B local. Succès moyen 97 %, jusqu'à 100 %. Seul le 14B local profite de l'orchestration multi-agent. Le modèle 14B n'est pas nommé dans l'abstract. | https://arxiv.org/abs/2603.26845 | arXiv (Han et al.) | 2026-03-27 (v2 2026-05-03) | 2026-10-15 | haute (chiffres de l'abstract) | benchmark |
| GeoBenchX : agent à appels d'outils (23 fonctions géospatiales), 8 LLM commerciaux seulement (Claude, GPT, Gemini), pas de modèle local. Inclut des tâches volontairement insolubles. Erreurs courantes : relations géométriques mal comprises, gestion de données inefficace. Nombre de tâches non trouvé. | https://arxiv.org/abs/2503.18129 | arXiv (Krechetova, Kochedykov) | 2025-03 (v2 2025-10-22) | 2026-10-15 | haute | benchmark |
| Un article multi-agents QGIS (Taylor & Francis, 2026) traduit des requêtes en workflows structurés avec les algorithmes de traitement QGIS comme outils. Il conclut que le raisonnement spatial doit encore progresser. Page en 403, donc résumé de moteur de recherche. | https://www.tandfonline.com/doi/10.1080/17538947.2026.2633849 | Int. J. Digital Earth (T&F), via moteur de recherche | 2026 | 2026-10-15 | faible | capacité |
| « Constraint Tax » : la contrainte dure de schéma fait passer la validité de 61,5 % à 100 % mais l'exactitude de 19,7 % à 11,0 % (Qwen2.5-0.5B/1.5B, SmolLM2-1.7B ; 15 000 générations). Tâche d'appel d'outil (calendrier) avec Qwen2.5-1.5B : 91,5 % en JSON par consigne seule contre 48,0 % sous schéma dur ; les deux sont valides à 100 %, donc les erreurs sont sémantiques. Tâches synthétiques ; écart de 7 points entre vLLM et SGLang pour un même modèle ; pas de code lié. Un seul auteur, pas de relecture visible. Modèles de 0,5 à 1,7 B, très en dessous de 7 à 14 B. | https://www.alphaxiv.org/abs/2605.26128 | arXiv 2605.26128 (J. Ray), via alphaxiv | 2026-05-20 | 2026-10-15 | moyenne | benchmark |
| Outlines et XGrammar : validité du schéma de 78,6–92,9 % sans contrainte à 100 % avec ; les échecs sémantiques (appels de fonctions multi-étapes) persistent. 5 modèles de 0,6 à 4 B, 14 tâches ; code publié (tag v1.0). Auteur unique. | https://arxiv.org/abs/2609.23742 | arXiv (A. Chavan) | 2026-09-20 | 2026-10-15 | moyenne | benchmark |
| Références BFCL pour petits modèles (TinyLLM) : xLAM-2-3b-fc-r 65,74 % global, 88,22 % non-live AST, 55,62 % multi-tours ; Qwen3-1.7B (prompt) 55,49 %. Seulement relevé via moteur de recherche, non lu à la source ; aucune mesure BFCL de l'effet d'une contrainte de grammaire trouvée. | https://arxiv.org/pdf/2511.22138 | arXiv | 2025-11 | 2026-10-15 | faible | benchmark |
| Rejeu d'agents IA : pratique répandue = enregistrer les réponses du modèle et des outils (journal d'événements en ajout seul) et rejouer l'enregistrement, pas régénérer. Le rejeu n'annule pas les effets de bord réels. Presque tout est de la littérature de fournisseurs (Gravitee, Insforge, Fastio) ou de projets jeunes (hark v0.1, Linux seul) ; aucune norme trouvée. L'article « The Log is the Agent » (arXiv) propose l'event sourcing. | https://arxiv.org/pdf/2605.21997 | arXiv, avec articles de fournisseurs | 2026-05 | 2026-10-15 | moyenne (résumé de moteur de recherche) | capacité |
| Esri : l'assistant ArcGIS Pro (bêta, v3.7) propose documentation, actions dans l'application, génération ArcPy et requêtes de couches. Condition d'accès : Pro 3.5 ou plus, installateur séparé, organisation autorisant les bêtas. Aucun prix publié trouvé. | https://earlyadopter.esri.com/ArcGISProAssistant | Esri | 2025-2026 (n.d.) | 2026-10-15 | moyenne | prix/capacité |

## Lecture pour la décision

- Les travaux SIG trouvés suivent surtout la voie « génération de code » (Python, GeoPandas, bac à sable, PyQGIS). La voie « appels typés validés » est représentée par GeoBenchX (23 outils, modèles cloud). Aucun travail trouvé ne mesure une sortie contrainte par schéma avec de petits modèles locaux sur des tâches SIG.
- La contrainte garantit la forme, pas le sens. Les deux études récentes concordent là-dessus, mais sur des modèles de 0,5 à 4 B et avec un seul auteur chacune.

## Pistes à creuser

- Lire en entier GISclaw (modèle 14B local, taux par architecture), GeoBenchX (nombre de tâches, taux par catégorie) et GeoAnalystBench.
- Obtenir l'article T&F (403) et la version complète de GIS Copilot : chiffres, modèles, forme exacte (code ou outils).
- Lire BFCL sur gorilla.cs.berkeley.edu pour des chiffres de modèles 7 à 14 B (Qwen, Mistral) et la documentation de llama.cpp sur les grammaires GBNF.
- Autres travaux : GeoFlow (arXiv 2508.04719), GeoLLM-Engine (2404.15500), GeoSkill (2609.13667), cités mais non lus.
- « The Log is the Agent » (2605.21997) pour la conception du journal rejouable.

## Cherché et non trouvé

- Aucune mesure de fiabilité d'appels typés avec contrainte de grammaire pour des modèles de 7 à 14 B quantifiés, en SIG ou en général.
- Aucun produit SIG qui journalise un agent IA pour un rejeu à l'identique.
- Aucun prix publié pour l'assistant Esri ni pour d'autres produits commerciaux. Aucun produit commercial hors Esri n'a été examiné.
- Aucun chiffre BFCL v4 vérifié à la source.
