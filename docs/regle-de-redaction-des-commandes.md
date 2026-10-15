# Règle de rédaction des commandes

Comment écrire la description et les schémas d'une commande du registre pour que
l'IA locale en tire des plans corrects. Les règles viennent des essais de l'epic 8
(`_bmad-output/initiative-cassini-mvp/epic-epreuve-du-modele-local/verdict-epreuve-du-modele-local.md`,
section « Ce que l'epic 2 en retient »).

Les règles 1 et 2 sont **contrôlées** par `Registry::check_contract()`
(`core/src/contract.rs`) ; les règles 3 à 10 sont **écrites, pas vérifiées**.

## Contrôlé par le registre

1. **Contrat complet.** Chaque commande déclare : un id `domaine.verbe` en anglais
   snake_case, une description française et une anglaise, où elle s'exécute
   (`core` ou `qgis`), si elle est annulable, au moins un test (par son nom), et des
   schémas engendrés en JSON Schema 2020-12. Rien n'a de valeur par défaut.
2. **Pas de collection de géométries sans borne** (AD-8) : tout tableau de
   géométries dans un schéma porte un `maxItems`.

## Écrit, non vérifié

3. **Des exemples, centrés sur les chaînes à plusieurs étapes.** Les exemples
   phrase-plan valent plus que tout le reste (3 à 4 phrases gagnées en 8.5). On
   garde un petit jeu d'exemples avec le catalogue.
4. **Les champs de catégorie portent des valeurs d'exemple** (`values`, 12 au
   plus). Sans elles, le modèle invente la valeur du filtre (`constructible` au
   lieu de `U` ou `AUc`).
5. **Aucune mesure nue.** Une distance, une hauteur ou un rayon est
   `{value, unit}` ; une coordonnée est `{x, y, epsg}`. Une coordonnée WGS84 pour
   le terrain se refuse.
6. **Aucun chemin ni nom de sortie dans les paramètres** (AD-10). Seule
   `layer.add` reçoit un chemin d'entrée relatif.
7. **Une option facultative n'est posée que si la phrase la demande.** Les étapes
   et filtres en trop sont la première cause d'erreur.
8. **Descriptions en français et en anglais.** La mesure porte sur des phrases
   françaises : l'anglais n'est pas établi.
9. **La référence entre étapes `step:k` est provisoire.** Elle sert pour démarrer ;
   prévoir de la remplacer (par exemple un nom donné à chaque sortie) et de
   re-mesurer.
10. **Unités.** 500 m et 0,5 km ne donnent pas le même plan pour un comparateur
    strict : la normalisation des unités est reportée.
