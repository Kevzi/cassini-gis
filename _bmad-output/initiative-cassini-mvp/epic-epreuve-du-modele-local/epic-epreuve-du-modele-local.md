---
type: epic
title: "Epreuve du modele local sur registre factice"
parent: initiative-cassini-mvp
covers: []
after: []
assignee: ""
risk: high
---

# Épreuve du modèle local sur registre factice

## Description

La promesse unique du produit — une phrase en français devient une suite de
commandes correcte, calculée par un modèle qui tourne sur le poste et sans
réseau — n'a jamais été vérifiée. Elle était planifiée en dernier. Cet epic la
vérifie en premier, sur un **registre factice** et avec du code jetable : si la
réponse est non, tout le reste du projet est un SIG de plus et le plan change
avant que du travail n'ait été engagé dessus.

## Outcome

Le propriétaire sait, chiffres en main, si un modèle local quantifié sait
composer des suites de commandes typées, avec quel modèle et à quelle taille ; le
signal est un verdict écrit qui autorise ou interdit la suite du plan.

## Requirements

- E1 : un modèle open source quantifié répond sur le poste du propriétaire, hors
  ligne, servi par llama.cpp. (spec, Capabilities, CAP-4 ; spec, Constraints,
  hors ligne total)
- E2 : un catalogue de commandes typées et un jeu de phrases françaises de
  référence existent, avec le plan attendu pour chaque phrase, pour que la mesure
  soit une mesure et non une impression. (spec, Capabilities, CAP-4)
- E3 : le taux de plans corrects est mesuré par modèle, par quantification et par
  forme de sollicitation. (spec, Open Questions, modèle et configuration
  minimale)
- E4 : la forme de sollicitation retenue est écrite, parce que la rédaction des
  descriptions et des schémas de paramètres du vrai registre en dépend. (spec,
  Capabilities, CAP-6)

## Done when

1. E1 à E4 tiennent, mesurés sur au moins vingt phrases de référence et au moins
   deux combinaisons de modèle et de quantification.
2. Un verdict écrit tranche : oui, non, ou oui sous conditions — en nommant le
   modèle, la quantification et la configuration minimale de RAM et de CPU
   retenus, ou en documentant l'échec et ce qui a été essayé.
3. La forme de sollicitation retenue est écrite assez précisément pour que
   l'epic 2 rédige ses descriptions de commandes d'après elle.
4. Tout a tourné sans aucun accès réseau, modèle local compris.
5. Rien de ce qui est écrit ici n'entre dans le produit : l'épreuve reste hors du
   chemin de compilation du noyau et de l'extension.

## Boundaries

Frontière par question à trancher : une épreuve, pas une fonctionnalité. Pas de
vrai registre — il appartient à l'epic 2. Pas d'extension QGIS, pas d'agent
livrable, aucune intégration au produit. Le code de l'épreuve est jetable et
assumé comme tel.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, CAP-4, CAP-6 et section Open Questions
- change — `../change-facade-qgis-avant-app-autonome/change-facade-qgis-avant-app-autonome.md`, sections 2.2 et 4 : pourquoi cet epic passe en premier
- constraint — `CLAUDE.md` §2.5 hors ligne total, §2.8 indépendance des fournisseurs d'IA

## Notes

- Decision: cet epic passe avant le registre et l'agent, parce que la promesse du produit était planifiée en dernier et conditionne tout le reste (propriétaire, 2026-10-14).
- Decision: pas de passe de nettoyage de clôture dans cet epic, le code étant jetable et hors du produit (2026-10-14).
- Unknown: quel modèle (Mistral ou Qwen), quelle quantification, et quelle configuration minimale de RAM et de CPU — c'est précisément ce que cet epic mesure. Question ouverte de la spec.
- Unknown: comment le modèle GGUF, qui pèse plusieurs Go, est livré sur un poste isolé. L'épreuve ne le tranche pas mais nourrit la réponse. Question ouverte de la spec.
- Assumption: un catalogue de commandes typées suffit au modèle, qui n'a jamais besoin de voir les données elles-mêmes. C'est l'hypothèse que l'épreuve valide ou casse.
- Waits on rien : l'épreuve ne dépend que d'un poste et d'un modèle téléchargé une fois.
