---
id: 9
type: bug
title: "Le rejeu diverge quand un raster de terrain précède un export PDF"
parent: epic-raster-et-terrain
covers: ["CAP-7", "CAP-11"]
after: []
assignee: ""
refined: true
hitl: false
risk: high
severity: P2
---

# Le rejeu diverge quand un raster de terrain précède un export PDF

## Description

L'utilisateur calcule une pente ou un champ de vision, puis exporte la carte en PDF. Le rejeu du journal s'arrête alors sur l'export avec `replay.diverged`, dans la même session comme après fermeture et réouverture du projet. Le rejeu doit redonner `identical: true`, comme pour une chaîne sans raster de terrain.

## Reproduction

1. Poste Windows, QGIS 3.44.15, noyau `--features gpu` et extension installés le 2026-11-05, modèle de 5.41.
2. Copie du projet `projet-sonde-4-4.qgz` (projet réel de Villeneuve-le-Roi et MNT Lambert-93), sans journal.
3. Dans une session, enchaîner : la phrase de démo (plan de 5 étapes finissant par `layout.export_pdf`), « Calcule la pente du terrain en degrés. », « Calcule la pente du terrain en pourcentage. », deux lignes de vue, deux champs de vision, « Exporte la carte au 1:25 000. », une isochrone, « Sors-moi la carte au 1/10 000. ». Les rasters de terrain s'ouvrent dans le projet.
4. Rejouer le journal dans la même session. Obtenu : `{"code": "replay.diverged", "message": "Le rejeu diverge du journal à l'étape 5 : la commande layout.export_pdf ne redonne pas le même résultat ou la même erreur.", "step": 5}`.
5. Rouvrir le projet dans une session neuve et rejouer le même journal. Obtenu : `replay.diverged` à l'étape 12, le premier export qui suit un raster de terrain.
6. Attendu dans les deux cas : `{"steps": 14, "identical": true}`.

## Cause Hypothesis

À l'origine, l'export compte et cite les rasters de terrain ouverts par le panneau (`layer_count` 16 à l'étape 12). Au rejeu, deux choses changent ce compte. D'abord, l'export rejoué ne retire de la carte (ticket 5.36) que les couches dont le journal porte un `layer_id`, et les rasters du noyau n'en portent pas : dans la même session, les rasters de la première exécution restent donc sur la carte de l'étape 5. Ensuite, le rejeu ne rouvre pas les rasters qu'il recalcule : dans une session neuve, ils manquent à l'étape 12. Le défaut est probablement à la jonction des tickets 4.3 et 5.36, pas dans les commandes de terrain ni dans l'export.

## Acceptance Criteria

1. **Le rejeu d'une chaîne terrain puis export est identique**
   **Given** le projet et la chaîne de la reproduction, exécutée dans une session
   **When** le journal est rejoué dans la même session, puis dans une session neuve après réouverture du projet
   **Then** les deux rejeux rendent `identical: true`, et la carte de chaque export rejoué montre les mêmes couches que l'export d'origine
2. **Tests cover the condition found and fixed**
   **Given** les tests du noyau et de l'extension
   **When** ils tournent
   **Then** un test qui échoue sur le défaut et passe sur le correctif couvre un raster du noyau suivi d'un export, rejoué dans la même session ; un autre couvre le même rejeu dans une session neuve ; un troisième garde vert le rejeu d'une chaîne sans raster (le cas de 5.36)
3. **Or: no change is needed, with proof**
   **Given** la reproduction
   **When** elle est rejouée sur le code courant
   **Then** les deux rejeux rendent déjà `identical: true`, ou le constat était faux, preuve consignée dans Notes — ceci remplace 1 et 2

## References

- parent — `_bmad-output/initiative-cassini-mvp/epic-raster-et-terrain/epic-raster-et-terrain.md`
- spec — `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/spec-cassini-mvp.md`, CAP-7 et CAP-11
- architecture — `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`, AD-11
- plan de 4.3 — `_bmad-output/initiative-cassini-mvp/epic-raster-et-terrain/story-les-sorties-raster-du-noyau-s-ouvrent-dans-le-projet-qgis-plan.md`
- plan de 5.36 — `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-second-rejeu-dans-la-meme-session-qgis-l-export-pdf-voit-les-plan.md`
- constat — `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-re-entrainement-groupe-commandes-de-terrain-et-export-a-l-ec-plan.md`, tâche « recette QGIS » de la liste des tâches

## Notes

- Decision (2026-11-05, propriétaire) : ticket de bogue à part, à corriger avant la démo ; 5.41 se clôt sans attendre.
- Le contrat d'architecture fixe le rejeu (AD-11, rejeu conduit par le noyau) : le correctif le cite et ne le re-décide pas.
- Contrôle hors du ticket (risque high) : une sonde QGIS réelle sur la chaîne de la reproduction, lancée par Claude, puis relecture `thorough`.
- Hors dépôt : la sonde et sa sortie sont dans `%TEMP%\sonde-5-41\` sur le poste du propriétaire (`sonde.py`, `sonde-rejeu.py`, `sortie.txt`).
