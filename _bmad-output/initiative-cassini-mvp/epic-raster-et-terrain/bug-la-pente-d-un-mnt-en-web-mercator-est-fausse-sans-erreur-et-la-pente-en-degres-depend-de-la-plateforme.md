---
id: 11
type: bug
title: "La pente d'un MNT en Web Mercator est fausse sans erreur, et la pente en degrés dépend de la plateforme"
parent: epic-raster-et-terrain
covers: ["CAP-11"]
after: []
assignee: ""
refined: true
hitl: false
risk: high
severity: P2
---

# La pente d'un MNT en Web Mercator est fausse sans erreur, et la pente en degrés dépend de la plateforme

## Description

Deux défauts trouvés par la rétrospective de l'epic, corrigés ensemble.

1. **Web Mercator.** L'utilisateur ouvre un MNT en EPSG:3857, système courant des tuiles
   téléchargées. Le noyau l'accepte comme métrique, alors que ses « mètres » sont dilatés
   d'environ 1,5 fois à la latitude de la France. La pente, les distances, la courbure et la
   borne du champ de vision sont faussées, sans aucun message. Le noyau doit refuser ce MNT par
   une erreur typée qui invite à le reprojeter dans QGIS, comme il refuse déjà un MNT en degrés.
   Le refus vaut pour tout système projeté hors de la liste fermée des systèmes métriques connus,
   pas seulement pour Web Mercator.
2. **Pente en degrés et plateforme.** La pente en degrés passe par l'arc tangente de la
   bibliothèque mathématique du système. Cette fonction n'est pas garantie identique entre
   Windows et Linux. Un journal écrit sur l'une peut donc rendre `replay.diverged` au rejeu sur
   l'autre. La pente doit donner les mêmes octets sur les deux plateformes.

## Reproduction

**Web Mercator** (fait le 2026-11-05, poste du propriétaire, `cassini.exe` installé) :

1. Reprojeter la fixture de la colline en EPSG:3857 avec l'outil de QGIS :
   `gdalwarp -t_srs EPSG:3857 -r bilinear mnt-colline-2154.tif mnt-colline-3857.tif`
   (`gdalsrsinfo` confirme `EPSG:3857`).
2. `cassini.exe run --project <dossier>\projet.qgz terrain.slope '{"dem":"mnt-colline-3857.tif","unit":"degrees"}'`
3. Constaté : succès, `"min":1.38,"max":15.82` degrés.
4. Attendu : une erreur typée qui dit que le système n'est pas en vrais mètres et invite à
   reprojeter. Pour comparaison, le même relief en Lambert-93 rend `"min":3.53,"max":23.29`.

**Pente en degrés et plateforme** : aucune divergence observée à ce jour ; le défaut est établi
par lecture. La pente en degrés appelle l'arc tangente de la plateforme. Le ticket 4.4 a retiré
`hypot` de la ligne de vue pour la même raison (relecture de 4.4, constat 6). Aucun test ne fige
les octets d'une pente : le test du plan incliné calcule son attendu avec la même fonction.

## Cause Hypothesis

- **Web Mercator** : le contrôle du système du MNT accepte tout code projeté dont la clé d'unité
  linéaire vaut le mètre. La liste fermée des systèmes métriques connus ne sert que quand cette
  clé manque. La décision du 2026-11-03 (« tout MNT en système projeté métrique ») ne tranchait
  pas le cas des pseudo-mètres.
- **Plateforme** : seule la pente en degrés est touchée. La pente en pourcentage n'utilise que
  des opérations exactes (racine carrée IEEE).

## Acceptance Criteria

1. **Un MNT hors de la liste fermée des systèmes métriques est refusé**
   **Given** un MNT en EPSG:3857, et un autre en EPSG:3395, chacun avec la clé d'unité au mètre
   **When** on lance `terrain.slope`, `terrain.line_of_sight` ou `terrain.viewshed` dessus
   **Then** la commande rend une erreur typée, de la même famille que le refus d'un MNT en
   degrés ; son message en français nomme le système et invite à reprojeter le MNT dans QGIS
   (par exemple en Lambert-93) ; aucun fichier de sortie n'est écrit
2. **Les MNT métriques acceptés aujourd'hui le restent**
   **Given** les MNT en Lambert-93, en coniques conformes 9 zones, en Lambert zones et en UTM
   **When** on lance les trois commandes
   **Then** elles répondent sans erreur, et les tests existants des trois commandes restent verts
   avec les mêmes résultats
3. **La pente en degrés est la même sur toutes les plateformes**
   **Given** la fixture synthétique de la colline (`/fixtures`)
   **When** on calcule sa pente en degrés sous Linux puis sous Windows
   **Then** les deux fichiers de sortie ont les mêmes octets, et ces octets sont ceux figés dans
   un test
4. **Tests cover the condition found and fixed**
   **Given** les tests du noyau, lancés par la CI sous Linux et sous Windows
   **When** ils tournent
   **Then** un test refuse un MNT en EPSG:3857 et un autre en EPSG:3395 ; un test fige l'empreinte
   (ou les octets) de la pente en degrés de la colline et échoue si l'arc tangente change ; les
   tests existants des trois commandes restent verts
5. **Or: no change is needed, with proof**
   **Given** la reproduction
   **When** elle est rejouée sur le code courant
   **Then** le MNT en EPSG:3857 est déjà refusé et la pente en degrés a déjà les mêmes octets sur
   les deux plateformes, preuve consignée dans Notes — ceci remplace 1 à 4

## References

- parent — `_bmad-output/initiative-cassini-mvp/epic-raster-et-terrain/epic-raster-et-terrain.md`
- source — `_bmad-output/initiative-cassini-mvp/epic-raster-et-terrain/epic-raster-et-terrain-retrospective.md`, constats F1 et F2, section « Décisions du propriétaire »
- spec — `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/spec-cassini-mvp.md`, CAP-11
- architecture — `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`, AD-11 (rejeu) et AD-19 (le noyau ne reprojette pas)
- plan de 4.4 — `_bmad-output/initiative-cassini-mvp/epic-raster-et-terrain/story-terrain-line-of-sight-ligne-de-vue-entre-deux-points-plan.md`, Review Triage Log, constat 6

## Notes

- Decision (2026-11-05, propriétaire) : refuser les MNT en pseudo-mètres (EPSG:3857 et les
  autres, couverts par la liste fermée ci-dessous), pas un simple avertissement. Le noyau ne reprojette pas (AD-19) : l'utilisateur
  reprojette dans QGIS.
- Decision (2026-11-05, propriétaire) : l'arc tangente portable vient de la crate `libm`
  (MIT OR Apache-2.0, sans dépendance). L'accord sur l'ajout de la dépendance est redemandé au
  build, avec sa licence (CLAUDE.md §8).
- Decision (2026-11-05, propriétaire) : liste fermée. Même quand la clé d'unité vaut le mètre,
  le noyau n'accepte que les systèmes métriques connus (ceux acceptés aujourd'hui sans clé
  d'unité) ; tout autre code projeté est refusé, Web Mercator compris. Un système métrique
  légitime manquant s'ajoute à la liste, ticket à part.
- Contrôle hors du ticket (risque high) : relecture `thorough`, et la CI sous Linux et sous
  Windows doit être verte sur le test des octets figés.
- Hors dépôt : le MNT reprojeté de la reproduction est dans
  `%LOCALAPPDATA%\Temp\retro-epic4\` sur le poste du propriétaire.
