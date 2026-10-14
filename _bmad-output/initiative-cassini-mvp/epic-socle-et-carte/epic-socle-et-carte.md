---
type: epic
title: "Socle technique, verrou hors ligne et recette"
parent: initiative-cassini-mvp
covers: [CAP-1]
after: []
assignee: ""
risk: medium
---

# Socle technique, verrou hors ligne et recette

## Description

Le socle sur lequel tout le reste se pose : un workspace Rust à deux étages, une
coquille de bureau qui n'ouvre qu'une fenêtre, une carte qui s'affiche depuis un
fichier local, et une recette qui tourne toute seule. Cet epic livre la partie de
CAP-1 qui ne dépend pas du projet : afficher un fond de carte lu sur le disque et
le naviguer, sans réseau.

## Outcome

Le propriétaire installe l'application sur son poste Windows, l'ouvre, et navigue
sur un fond de carte alors que la machine est débranchée du réseau ; le signal est
que la recette passe au vert sans intervention à chaque poussée.

## Requirements

- E1 (CAP-1) : afficher un fond de carte lu depuis un fichier local et le naviguer
  au zoom et au déplacement. (spec, Capabilities, CAP-1)
- E2 : aucune sortie réseau possible, ni par configuration ni par code, vérifiée
  autrement que par relecture. (spec, Constraints, hors ligne total)
- E3 : la chaîne de compilation produit une application qui s'ouvre sous Windows.
  (spec, Constraints, application de bureau)
- E5 : le dépôt explique son architecture à qui le découvre, schémas compris.
  (demande du propriétaire, 2026-10-09)
- E4 : la recette — tests, `clippy`, `fmt`, typage strict, garde hors ligne —
  s'exécute automatiquement. (CLAUDE.md §8, §9)

## Done when

1. E1 à E4 tiennent sur une application lancée depuis un poste Windows, fond de
   carte lu dans `/fixtures`, machine sans réseau.
2. La recette complète passe au vert sur GitHub Actions à chaque poussée, et
   échoue si une adresse réseau apparaît dans le style du fond de carte.
3. Le dépôt porte un README qui explique l'architecture avec des schémas, lisible
   par quelqu'un qui découvre le projet.
4. Aucune dépendance ajoutée n'est sous GPL ou LGPL, et la licence de chacune est
   écrite dans le message de commit qui l'ajoute.

## Boundaries

Frontière par couche technique : le workspace, le verrou hors ligne, la carte de
départ et la recette. Pas de registre de commandes — c'est l'epic 2. Pas
d'extension QGIS — c'est l'epic 9. Pas de terrain — c'est l'epic 4.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, CAP-1 et section Constraints
- constraint — `CLAUDE.md` §2.2 licences, §2.3 hors ligne, §3 architecture, §9 conventions
- research — `../research-sig-francais-hors-ligne-pour-la-defense/research-sig-francais-hors-ligne-pour-la-defense.md`, section D4 pour les licences des briques

## Notes

- Decision: le fond de carte PMTiles de recette est généré par un script, jamais téléchargé (2026-10-09).
- Decision: le README détaillé avec schémas Mermaid est avancé dans cet epic au lieu de l'étape 7, le dépôt étant déjà public (2026-10-09).
- Decision: l'epic 2 reste entier plutôt que coupé en deux, choix du propriétaire (2026-10-09).
- Unknown: d'où viendra le fond de carte en production — apporté par l'utilisateur, livré avec l'app, ou aucun par défaut. L'entrée 3 travaille sur le fond de recette d'ici là.
- Le traceur est l'entrée 1 : le chemin le plus mince qui traverse le moteur Rust, la coquille Tauri, la webview et MapLibre. Elle est faite.
- Les entrées 2, 5 et 6 ne partagent aucun fichier et peuvent avancer en parallèle.
- Decision: le virage du 2026-10-14 sort la carte MapLibre et la coquille Tauri du chemin
  critique du MVP, mais cet epic reste livre tel quel : son travail est fait et son verrou
  hors ligne sert aux deux scenarios. `/app` reste au depot comme fondation de V2.
- Decision: l'entree 7, passe de nettoyage, ne doit **pas** supprimer `/app` ni `/fixtures` :
  tous deux sont conserves, le premier comme fondation de V2, le second parce qu'il prouve
  le verrou hors ligne (2026-10-14).
