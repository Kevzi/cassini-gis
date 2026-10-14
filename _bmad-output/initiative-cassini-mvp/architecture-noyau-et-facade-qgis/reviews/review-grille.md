# Revue — grille du bon contrat

Marche sur la liste de contrôle du gate : points de divergence couverts, règles
exécutables, rien de dangereux sous *Deferred*, technologies vérifiées,
ratification du code existant, couverture de la spec, et aucune dimension muette.
Conduite le 2026-10-14.

## Verdict

Solide sur la frontière, la licence, le journal et l'enveloppe d'exploitation.
Une dimension est muette et deux règles ne sont pas exécutables telles quelles.

## G1 — haute — dimension muette : ce que le modèle a le droit de produire

Le contrat décrit où vit l'agent, pas ce qu'il peut émettre. Un plan composé par
un modèle est une entrée non fiable : s'il peut rendre du code, une expression ou
un chemin absolu, il contourne AD-8, AD-9 et AD-22. Aucun AD ne le borne. C'est
la seule dimension de sécurité réelle du produit, et elle est silencieuse.

## G2 — moyenne — AD-8 est exécutable, AD-14 à moitié

« Aucun schéma ne déclare une collection de géométries non bornée » est
vérifiable mécaniquement — bien. En revanche « la recette échoue si l'arbre de
dépendances introduit un client réseau » ne dit pas comment : sans liste nommée
ni outil nommé, deux recettes vérifieront deux choses différentes.

## G3 — basse — couverture de la spec

CAP-1, 3 à 11 et 13 sont toutes portées par la carte des capacités. CAP-2 et
CAP-12 sont hors périmètre à juste titre. Les contraintes de la spec se
retrouvent dans AD-1, 8, 14, 16, 19, 20.

## G4 — basse — ratification du code existant

Le contrat ratifie les conventions de `cassini-core` (erreurs typées par module,
refus à la construction, aucune troncature silencieuse) au lieu d'en inventer
d'autres : AD-21 est marqué `[ADOPTED]` à raison. La tension relevée dans
`basemap.rs` — « l'interface ne connaît jamais le chemin du fichier » — est bien
tranchée par AD-20 pour le terrain et par AD-9 pour le vecteur.

## G5 — basse — *Deferred* ne cache rien de divergent

Le report de la socket locale est sûr parce qu'AD-3 ne nomme pas son transport.
Le report du dessin du panneau est sûr parce que le contrat fixe qui décide, pas
ce qui s'affiche. Le report du rejeu sans QGIS est couvert par l'échec nommé
d'AD-11.
