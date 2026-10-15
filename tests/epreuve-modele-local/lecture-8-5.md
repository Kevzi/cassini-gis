# Lecture des mesures de 8.5

Écrite à la main d'après `resultats-8-5.md` et `resultats-8-5.json` (le premier est généré, celle-ci ne l'est pas). Mesure du 2026-10-15 : Qwen3.5-9B Q4_K_M, carte RTX 3070 Ti de 8 Go (relevé de l'entrée 8.1), 25 phrases, température 0, graine fixe. Environ 7 secondes par phrase, chargement du modèle (environ 2,5 s par appel) compris.

## Ce que les chiffres disent

1. **Les exemples sont le seul levier net, mais plus petit qu'il n'y paraît.** Trois exemples tirés d'autres phrases font gagner **+4 phrases en sortie libre** (16 à 20, avec ou sans filtre) et **+3 phrases sous schéma** (17 à 20 en catalogue complet, 16 à 19 en filtré), soit 12 à 16 points. Ils corrigent S11, S16 et S23 dans les quatre comparaisons ; S21 aussi en libre à catalogue complet, S20 et S24 selon le cas. Avec le filtre de catalogue, ils font **perdre S18** (✓ à ✗) : le gain net est un solde.
2. **La sortie contrainte par schéma ne montre aucun effet net sur le total** : 17 contre 16 sans exemples, 20 contre 20 avec, 19 contre 20 en filtré avec exemples. Elle change pourtant des phrases une à une (par exemple S20, juste en F4 et fausse en F8). Elle garantit la forme, pas le sens, comme le disent deux préimpressions récentes à auteur unique sur des modèles plus petits (voir la recherche `research-sig-ia-hors-ligne`). Elle reste nécessaire pour la sûreté (AD-23), pas pour le score.
3. **Le filtre lexical de catalogue n'aide pas.** Il ne retient les bonnes commandes que pour 20 phrases sur 25. Il réduit l'invite de 21 à 23 % (3 434 à 3 850 jetons contre 4 462 à 4 874) mais le temps total augmente (194 s contre 180 s avec exemples sous schéma) : le modèle génère plus de jetons en catalogue filtré (156 en moyenne contre 109), probablement parce qu'il compense ce qu'on ne lui a pas montré. À ne pas retenir en l'état.
4. **La relance ne rattrape rien :** dans les huit formes, le taux après relance égale le taux du premier essai, alors que 1 à 5 phrases par forme ont été redemandées. Les relances restent utiles à la sûreté, mais ne sont pas une marge de qualité avec ce modèle.
5. **Les échecs ne sont pas des sorties illisibles.** Sur les huit formes, 19 réponses finissent non conformes (une commande absente du catalogue, ou une référence `step:k` vers une étape qui ne produit pas de couche), le reste des échecs sont des plans conformes mais différents de l'attendu. Trois phrases échouent dans les huit formes : **S15, S17, S19** ; S18 et S24 dans six. Les phrases S01 à S10, qui n'ont qu'une commande, et le refus de coordonnée WGS84 (S25) sont justes partout. Dans la forme retenue (F6), les erreurs observées sont : S15 un champ de vision que personne n'a demandé, S17 un filtre sur la mauvaise couche, S18 un zoom en trop, S19 un filtre en trop, S24 le raster et les zones intervertis. D'autres formes se trompent autrement (par exemple S15 appelle une commande inventée en F2).
6. **Le comparateur est strict :** une étape inoffensive en trop (S18) compte comme fausse. À rediscuter dans le verdict (8.6).

## Forme retenue pour 8.4

**Sortie contrainte par schéma, catalogue complet, avec exemples (F6) : 20 phrases sur 25.** Quatre formes sont à moins de deux phrases d'elle (F2, F4, F6 elle-même, F8), donc **indiscernables** : le choix de F6 repose sur le critère du plan (parmi les formes contraintes, le taux d'emblée le plus haut), et sur le fait que c'est ce que le produit utilisera (AD-23), pas sur un écart prouvé.

## Pistes non essayées

Plus d'exemples centrés sur les chaînes à plusieurs étapes ; une consigne explicite « ne fais que les étapes demandées » ; un modèle plus gros. Elles appartiennent à 8.4 ou à une suite.
