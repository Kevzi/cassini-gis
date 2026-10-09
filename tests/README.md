# Tests d'intégration et scénario de démo

Les tests unitaires vivent à côté du code qu'ils couvrent (`core/src/**`), comme
le veut l'usage Rust. Ce dossier est réservé à ce qui traverse plusieurs briques :

- le scénario de démo de bout en bout (CLAUDE.md §1), une fois le registre de
  commandes et l'agent en place ;
- la vérification que la même commande donne le même résultat depuis l'interface,
  la CLI, l'agent local et MCP (CAP-6).

Pour l'instant, la recette tient dans `cargo test -p cassini-core` et
`npm run check` côté interface.
