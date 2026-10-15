# Verdict de l'épreuve du modèle local (entrée 8.6)

**État : recommandation, non signée.** Le propriétaire la signe ou la change (section « Décision »). Sources : `tests/epreuve-modele-local/resultats-8-4.md`, `resultats-8-5.md`, `lecture-8-4.md`, `lecture-8-5.md`, et la mesure sur processeur seul de cette entrée, `resultats-8-6-cpu.json`.

## Réponse

**Oui, sous conditions.** Un modèle quantifié local sait transformer une phrase française en suite de commandes typées correcte **quatre fois sur cinq** (20 phrases sur 25), sur un catalogue factice de 16 commandes, sans réseau. Les 5 autres erreurs sont des plans **conformes mais différents** de l'attendu (4 cas) ou un plan **non conforme** rejeté par la validation (1 cas) : aucune n'est une sortie illisible. Cela justifie d'engager les epics 2 et 5, à condition que **l'utilisateur valide chaque plan avant exécution** (CLAUDE.md §6) et que **la validation contre les schémas reste sans réparation** (AD-23). Sans cette validation humaine, 80 % ne suffit pas.

Il n'y a pas de seuil chiffré du « oui » (décision du 2026-10-15) : le verdict lit la nature des erreurs.

## Ce qui est recommandé

| Point | Recommandation | Preuve |
|---|---|---|
| Modèle | **Qwen3.5-9B**, licence **Apache-2.0** (fiche du modèle) | 20/25, résultats 8.4 |
| Quantification | **Q4_K_M** (fichier de 5,68 Go) | 20/25 ; Ministral 3 8B Q4_K_M 17/25 ; Ministral Q6_K 20/25 mais apparemment saturé (voir plus bas) |
| Forme de sollicitation | **Sortie contrainte par schéma, catalogue complet, avec trois exemples** (F6), mode réflexion coupé | 20/25 ; sans exemples 17/25 (8.5) |
| Format du plan | liste d'objets `{"command", "params"}`, ou `{"refusal": "<raison>"}` ; validée contre les schémas avant tout affichage, jamais réparée | AD-23 ; 24 sorties sur 25 conformes d'emblée |

Pourquoi Qwen et pas Ministral Q6_K : les deux font 20/25 (écart nul), mais Ministral Q6_K semble saturer la carte de 8 Go (7,4 Go de VRAM, lecture de l'invite 11,4 s contre 1,6 s ; cause non vérifiée, hypothèse de `lecture-8-4.md`), alors que Qwen Q4_K_M tient dans 5,8 Go. Le choix repose sur la mémoire, pas sur une différence de qualité. Ministral Q4_K_M (17/25) est écarté : trois phrases de moins, écart mince mais au-dessus du seuil de deux phrases.

Le choix de la forme F6 n'est **pas** un écart prouvé : trois autres formes (F2, F4, F8) sont à moins de deux phrases d'elle (8.5). Il repose sur le critère du plan et sur ce que le produit utilisera (sortie contrainte, AD-23).

## Configuration minimale mesurée

Une seule machine : Ryzen 7 5800X (8 cœurs, 16 fils), RTX 3070 Ti de 8 Go. Contexte de 8192 jetons, prompt d'environ 4 900 jetons.

| Cas | Mémoire | Temps par demande | Taux |
|---|---|---|---|
| Carte graphique (`--ngl 99`) | 5,3 Go de RAM, 5,8 Go de VRAM | environ 5,5 s (chargement 2,7 s, lecture 1,6 s, génération 1,3 s) | 20/25 |
| **Processeur seul** (`--ngl 0`) | 5,7 Go de RAM au pic | environ **26,5 s** (chargement 1,9 s, lecture 6,0 s, génération 18,6 s pour 105 jetons) | 20/25 |

Lecture : sur processeur seul le taux ne bouge pas (20/25, mêmes phrases fausses que sur carte : S15, S17, S18, S19, S24 ; S15 y est non conforme au lieu de conforme-mais-différent). Le temps est cinq fois plus long. Le délai du harnais a été relevé de 300 s à 900 s par précaution ; aucun appel ne l'a approché (le plus long : S17, 65,4 s avec sa relance). Moyenne par phrase, relance comprise : 29,2 s. L'attente est celle d'une demande qu'on valide ensuite à la main. « Minimum » veut donc dire : **au moins 5,7 Go de RAM libre mesurés (8 Go est une marge de ma part, non mesurée), un processeur du niveau d'un Ryzen 7 5800X pour rester autour d'une demi-minute, une carte de 8 Go pour passer à quelques secondes**. Un processeur plus faible, une machine à 8 Go de RAM au total ou un portable n'ont **pas été mesurés**.

Réserve sur la mesure processeur : le harnais a relevé 1,6 Go de VRAM malgré `--ngl 0` ; je ne l'ai pas expliqué (probablement l'affichage et d'autres programmes, non vérifié). La RAM de 5,7 Go est le chiffre qui compte.

## Ce que les erreurs montrent

Erreurs de la forme retenue (8.5, reproduites en 8.4 et sur processeur) :

- **S15** : un champ de vision que personne n'a demandé ; sur processeur, la première réponse invente un identifiant de couche (`mnt_zone_nord_0000…`) et la relance, `layer.add` du `.tif` puis `zonal_statistics` avec `raster: "step:1"`, est rejetée (« step:1 ne l'est pas » un raster).
- **S17** : un filtre posé sur la mauvaise couche.
- **S18** : un zoom en trop (étape inoffensive, comparée comme fausse).
- **S19** : un filtre en trop.
- **S24** : le raster et les zones intervertis.

Deux phrases (S17, S19) échouent dans les huit formes de 8.5 **et** pour les trois combinaisons de 8.4 ; S15 échoue dans les huit formes avec Qwen mais est juste avec les deux Ministral ; S24 échoue dans six formes sur huit (justes en F4 et F8) et pour les trois combinaisons de 8.4. Pour S17, S19 et S24, la cause est peut-être la phrase ou le catalogue plutôt que le modèle (hypothèse non vérifiée, déjà notée en 8.4). Je ne les ai pas retouchées pour améliorer un score (interdit par le plan).

Ce que ça veut dire pour la validation humaine : les erreurs sont des plans **lisibles et plausibles** (couche inversée, étape en trop), pas des sorties absurdes. Un utilisateur qui ne lit pas le plan peut donc les laisser passer. S17 (filtre sur la mauvaise couche) et S24 (entrées interverties) sont les plus à craindre, par inférence : l'un ou l'autre peut rendre un résultat faux sans être rejeté (S24 pourrait aussi être rejeté par le contrôle de type ; je ne l'ai pas vérifié). S17 est la phrase de démo de CLAUDE.md §1 et elle est fausse dans la configuration recommandée. Le produit doit **montrer le plan étape par étape** et ne rien exécuter avant validation. Ni la relance (taux inchangé dans les huit formes de 8.5) ni la contrainte de schéma (aucun effet net sur le total) ne rattrapent ces erreurs : elles garantissent la forme, pas le sens.

## Ce que l'epic 2 en retient

Pour rédiger les descriptions de commandes et les schémas de paramètres, d'après les essais :

1. **Les exemples valent plus que tout le reste** : trois exemples phrase-plan tirés hors des phrases mesurées ont fait gagner 3 à 4 phrases (8.5). Prévoir un petit jeu d'exemples maintenu avec le catalogue, **centré sur les chaînes à plusieurs étapes**.
2. **Les champs de catégorie portent des valeurs d'exemple** (`values`, 12 au plus : règle du catalogue, voir le README de l'épreuve) : sans elles, le modèle invente la valeur du filtre (`typezone contains "constructible"` au lieu de `U` ou `AUc`), constaté à la main le 2026-10-15.
3. **Aucune mesure nue** : toute distance, hauteur ou rayon est `{value, unit}`, toute coordonnée `{x, y, epsg}` ; la bonne réponse à une coordonnée WGS84 pour le terrain est un refus (S25, juste dans toutes les formes).
4. **Aucun chemin ni nom de sortie dans les paramètres** (AD-10) ; seul `layer.add` reçoit un chemin d'entrée relatif.
5. **Une option facultative n'est posée que si la phrase la demande** : les erreurs S15, S18 et S19 sont des étapes ou filtres en trop. Une consigne explicite « ne fais que les étapes demandées » n'a **pas** été essayée.
6. **Descriptions fr et en** : le catalogue porte les deux langues ; la mesure porte sur des phrases françaises. Je n'ai pas lu de résultat sur les phrases anglaises dans les fichiers de 8.4 et 8.5 : l'anglais n'est pas établi.
7. **La référence entre étapes `step:k` n'est pas acquise.** Une confusion de numérotation a été observée à la main (README de l'épreuve, hors des chiffres mesurés). S15 sur processeur échoue sur un `step:1` que la validation refuse comme raster : je ne sais pas si c'est le modèle ou la validation qui ne type pas la sortie de `layer.add` d'un `.tif` (non vérifié). La mesure a gardé `step:k` partout : **aucune autre forme n'a été comparée**. L'epic 2 peut l'adopter pour démarrer, mais doit prévoir de la remplacer (par exemple un nom donné à chaque sortie) et le re-mesurer.
8. **Unités** : 500 m et 0,5 km donnent deux plans « différents » au comparateur. Normalisation reportée (`deferred-work.md`).

## Risques

- **Taille du catalogue.** 16 commandes donnent déjà 4 900 jetons d'invite. Le catalogue réel sera plus grand : le temps de lecture croît avec lui (6 s sur processeur à 16 commandes). Le filtre lexical à 8 commandes **n'a pas aidé** (il retient les bonnes commandes pour 20 phrases sur 25, et allonge le temps total) : il n'est pas une réponse prête. Non mesuré au-delà de 16.
- **Pas de gagnant net.** Qwen Q4_K_M et Ministral Q6_K sont à égalité (20/25) ; Ministral Q4_K_M est à 17/25. La marge sur 25 phrases est mince et chaque mesure est unique (température 0, graine fixe) ; le choix de Qwen se fait sur la mémoire.
- **Comparateur strict.** Un zoom en trop compte comme faux ; le taux vrai de « plans utilisables » est peut-être plus haut, et peut aussi être plus bas si l'on compte les erreurs silencieuses (S17, S24).
- **Données inventées.** Les couches et les phrases sont fabriquées pour la mesure ; des vrais projets, noms de champs et fautes de frappe ne sont pas couverts.

## Ce qui n'a pas été essayé

Plus d'exemples ou une consigne « seulement les étapes demandées » ; un modèle plus gros ; un modèle plus petit pour les postes faibles ; une autre forme de référence entre étapes ; un catalogue de plus de 16 commandes ; un processeur plus faible ou moins de RAM ; plusieurs mesures par forme (la variation d'une graine à l'autre) ; l'anglais ; des phrases rédigées par d'autres que moi ; le mode réflexion du modèle (coupé partout).

## Ce que le verdict change

- **Epic 2 (registre)** : peut partir. Il reprend les règles ci-dessus pour ses descriptions et schémas, et le format de plan du harnais.
- **Epic 5 (agent)** : peut partir, **avec** le plan montré étape par étape, la validation sans réparation (AD-23), et une attente de l'ordre de 30 s en moyenne sur un poste sans carte (jusqu'à plus d'une minute pour les plans à plusieurs étapes, relance comprise). Le dernier point est à confirmer sur une machine plus modeste avant de promettre quoi que ce soit.
- Le plan du projet n'est pas modifié par ce document.

## Décision

Signée par le propriétaire le : ____________  Verdict retenu : ☐ oui sous conditions ☐ oui ☐ non  Remarques : ____________
