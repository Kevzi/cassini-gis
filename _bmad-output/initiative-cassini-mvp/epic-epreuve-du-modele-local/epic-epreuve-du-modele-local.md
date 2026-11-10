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
- spine — `../architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`, AD-8, AD-9, AD-15, AD-19, AD-22, AD-23, Consistency Conventions et Deferred (forme de l'invite et du plan rendu)

## Notes

- Decision: cet epic passe avant le registre et l'agent, parce que la promesse du produit était planifiée en dernier et conditionne tout le reste (propriétaire, 2026-10-14).
- Decision: pas de passe de nettoyage de clôture dans cet epic, le code étant jetable et hors du produit (2026-10-14).
- Unknown: quel modèle (Mistral ou Qwen), quelle quantification, et quelle configuration minimale de RAM et de CPU — c'est précisément ce que cet epic mesure. Question ouverte de la spec.
- Decision: la livraison du modèle sur poste isolé est tranchée par le contrat d'architecture, AD-17 : un paquet Cassini installé à part le porte, jamais l'archive de l'extension (2026-10-14).
- Decision: la décision précédente est précisée le 2026-11-10 (changement de cap `change-distribution-deux-portes`, AD-17 amendé) : le paquet Cassini a deux portes, un installeur léger en ligne qui télécharge une fois le modèle depuis Hugging Face pendant l'installation, empreinte vérifiée, et un installeur complet hors ligne, un seul fichier, pour les postes isolés du réseau ; jamais l'archive de l'extension, jamais le noyau ni l'extension (propriétaire, 2026-11-10).
- Decision: l'épreuve appelle la version compilée de llama.cpp (MIT) en ligne de commande, comme programme séparé, sans mode serveur ni port ouvert ; le harnais est en Python, bibliothèque standard seule. Écart assumé avec AD-15, qui charge le modèle dans le processus du noyau : limité à ce code jetable, la compilation de `llama-cpp-2` sous Windows restant un risque de l'epic 5 (propriétaire, 2026-10-14).
- Decision: le code de l'épreuve vit dans `tests/epreuve-modele-local/`, hors des crates et de `qgis-plugin` ; les fichiers de modèle GGUF ne sont jamais versés au dépôt (2026-10-14).
- Decision: re-découpage du 2026-10-14 pour aligner les entrées sur le contrat d'architecture, aucune n'étant commencée ; identifiants conservés. Traceur : l'entrée 1, un modèle qui répond hors ligne. Séquencement : 1 et 2 en parallèle, puis 3, puis 4 et 5 en parallèle, puis 6 (propriétaire, 2026-10-14).
- Assumption: la forme de `layer.describe` et de l'inventaire de projet est inventée par l'entrée 2 pour les besoins de la mesure ; le vrai contrat de ces commandes appartient à l'epic 2.
- Assumption: un catalogue de commandes typées suffit au modèle, qui n'a jamais besoin de voir les données elles-mêmes. C'est l'hypothèse que l'épreuve valide ou casse.
- Waits on rien : l'épreuve ne dépend que d'un poste et d'un modèle téléchargé une fois.
