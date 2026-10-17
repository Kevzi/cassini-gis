# Documentation

- `../README.md` — la présentation du dépôt : ce qui marche aujourd'hui,
  l'architecture en schémas, le rôle de chaque dossier, le démarrage rapide et la
  recette. À lire en premier quand on découvre le projet.
- `../CLAUDE.md` — source de vérité du projet (vision, principes, architecture).
- `../_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/` — le contrat machine
  (capacités CAP-n, contraintes, non-objectifs) dont tout développement découle.
- `../_bmad-output/initiative-cassini-mvp/research-.../research-briefing.html` —
  le dossier de recherche : contrôle à l'export, standards de défense, acteurs,
  licences des briques libres.

- `compiler-sous-windows.md` — marche à suivre pour compiler et lancer
  l'application sur un poste Windows, prérequis compris, et preuve du
  fonctionnement réseau coupé.

- `modele-local-sous-windows.md` — compiler llama.cpp, lancer `model-check`, variables
  et chemin court.

- `regle-de-redaction-des-commandes.md` — comment écrire la description et les
  schémas d'une commande, et ce que le registre contrôle lui-même.

- `canal-local.md` — le contrat des messages du canal local : ordres du noyau
  vers l'extension (`order.execute`), annulation (`request.cancel`) et erreurs.

À remplir au fil des étapes : parité fonctionnelle visée et décisions
d'architecture courtes (ADR).
