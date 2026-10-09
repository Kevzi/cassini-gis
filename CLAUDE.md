# CLAUDE.md — Cassini

> Ce fichier est la source de vérité du projet. Lis-le en entier avant chaque session.
> Si une demande contredit ce fichier, signale-le avant d'agir.

## 1. Vision

Cassini est un SIG (système d'information géographique) open source, de bureau,
qui fonctionne **100 % hors ligne** et se pilote **en langage naturel** grâce à
un agent IA intégré. Il vise à remplacer ArcGIS Pro pour les usages courants,
avec les **données publiques françaises intégrées nativement**.

Phrase de démo : « Trouve les parcelles constructibles du PLU à moins de 500 m
d'une gare à Villeneuve-le-Roi et sors-moi la carte en PDF. »

Public cible : bureaux d'études (urbanisme, environnement), collectivités,
étudiants, puis à terme défense / sécurité (usage air-gapped).

## 2. Principes non négociables

1. **Clean-room.** On travaille uniquement à partir de la documentation publique
   d'Esri et du comportement observable. Interdit : copier du code, des icônes,
   des noms de produits, des presets ou des assets d'Esri. Ne pas reprendre de
   code sous licence GPL/LGPL.
2. **Hors ligne d'abord.** Toute fonction du MVP doit marcher sans internet.
   Le réseau sert uniquement à télécharger des données ou à brancher une IA
   externe, en option.
3. **Agent-first.** Toute action utilisateur passe par une commande du registre
   (voir §4). Aucune logique métier dans l'interface.
4. **Indépendant des fournisseurs d'IA.** Le code ne dépend d'aucune IA en
   particulier. IA locale par défaut, IA externes via MCP.
5. **Petit et solide.** Mieux vaut 5 fonctions fiables que 50 à moitié finies.

## 3. Architecture

```
┌─────────────────────────────────────────────┐
│  Interface (Tauri v2 + TypeScript + MapLibre)│
└──────────────────┬──────────────────────────┘
                   │ appels de commandes
┌──────────────────▼──────────────────────────┐
│  Registre de commandes (Rust)                │
│  ← utilisé par : UI, CLI, agent local, MCP   │
└──────┬───────────────┬───────────────┬──────┘
       │               │               │
┌──────▼─────┐  ┌──────▼──────┐  ┌─────▼───────┐
│ Moteur géo │  │ Stockage    │  │ Couche IA   │
│ DuckDB     │  │ local       │  │ llama.cpp   │
│ spatial,   │  │ GeoParquet, │  │ (local) +   │
│ geo, proj  │  │ PMTiles     │  │ serveur MCP │
└────────────┘  └─────────────┘  └─────────────┘
```

- **App de bureau** : Tauri v2 (Windows, macOS, Linux).
- **Moteur** : Rust. Calculs spatiaux via DuckDB (extension spatial) et les
  crates `geo` / `proj`.
- **Carte** : MapLibre GL JS. Fonds de carte hors ligne au format PMTiles.
- **Données vectorielles** : stockées localement en GeoParquet.
- **IA locale** : modèle open source (Mistral ou Qwen, quantifié GGUF) servi par
  llama.cpp, embarqué avec l'app.
- **IA externe (optionnel)** : serveur MCP qui expose le registre de commandes,
  pour Claude, ChatGPT, Gemini, etc.

## 4. Registre de commandes

Chaque fonction est une commande unique, décrite une seule fois :

- `id` (ex. `analysis.buffer`)
- description courte en français et en anglais (lue par l'IA)
- paramètres typés avec schéma JSON
- résultat typé
- implémentation en Rust
- au moins un test

L'interface, la CLI, l'agent local et le serveur MCP appellent **les mêmes
commandes**. Aucune fonction ne doit exister ailleurs que dans le registre.

Chaque exécution est journalisée dans l'historique du projet pour pouvoir être
rejouée (reproductibilité).

## 5. Données françaises (MVP)

À télécharger une fois puis utiliser en local :

| Donnée | Source |
|---|---|
| Cadastre (parcelles, bâtiments) | cadastre.data.gouv.fr (Etalab) |
| Plan local d'urbanisme (zonage) | Géoportail de l'Urbanisme |
| Adresses | Base Adresse Nationale (BAN) |
| Routes, gares, bâti | IGN BD TOPO |
| Limites administratives | IGN ADMIN EXPRESS |
| Fonds de carte | IGN Plan / OpenStreetMap, convertis en PMTiles |

Une commande `data.download_commune` télécharge et prépare tout ce qu'il faut
pour une commune donnée (code INSEE).

## 6. Périmètre du MVP

1. **Carte** : afficher des couches vectorielles et un fond de carte hors ligne.
2. **Données** : charger les données françaises d'une commune en une commande.
3. **Analyses** : zone tampon, intersection, sélection par attribut,
   statistiques par zone, isochrones à pied et en voiture (graphe routier local).
4. **Agent IA** : une demande en langage naturel devient une suite de commandes,
   affichée étape par étape, que l'utilisateur peut valider et rejouer.
5. **Export** : carte PDF avec titre, légende, échelle, flèche du nord et sources.

**Hors périmètre (V2 et plus)** : édition avancée, 3D, IA sur images satellite,
formats défense (DTED, NITF, MGRS, APP-6), plugins, collaboration, version web.

## 7. Étapes

- **Étape 1** : squelette Tauri, carte MapLibre, fond PMTiles hors ligne.
- **Étape 2** : registre de commandes et chargement du cadastre et du PLU.
- **Étape 3** : analyses de base et historique rejouable.
- **Étape 4** : agent IA local (llama.cpp) et serveur MCP.
- **Étape 5** : export PDF et scénario de démo complet.
- **Étape 6** : vidéo de démo, README, publication GitHub.

Ne commence pas une étape tant que la précédente n'est pas testée et commitée.

## 8. Règles de travail pour Claude

- Le propriétaire du projet ne code pas lui-même : explique chaque changement
  en 2–3 phrases simples, en français.
- **Une tâche courte par session.** Si une tâche paraît grosse, propose de la
  découper avant de coder.
- Avant d'ajouter une dépendance, demande et donne sa licence. Licences
  autorisées : MIT, Apache-2.0, BSD. Pas de GPL/LGPL.
- Chaque nouvelle commande vient avec son test.
- Après chaque étape qui marche : lancer les tests, puis proposer un commit
  avec un message clair.
- Ne jamais inventer une API de données : vérifier la documentation officielle
  de la source.
- Ne jamais copier quoi que ce soit d'Esri (voir §2).
- Si quelque chose ne marche pas après deux essais, s'arrêter et expliquer le
  problème au lieu d'empiler des correctifs.

## 9. Conventions

- Code et noms de variables en anglais, commentaires et docs utilisateur en
  français.
- Rust : `cargo fmt` et `cargo clippy` sans avertissement.
- TypeScript : mode strict.
- Structure du dépôt :

```
/app          interface Tauri + TypeScript
/core         moteur Rust et registre de commandes
/ai           couche IA (llama.cpp, serveur MCP)
/data         scripts de téléchargement et préparation des données
/docs         documentation, parité fonctionnelle, roadmap
/tests        tests d'intégration et scénario de démo
```

## 10. Licence

Apache-2.0 (permet un usage commercial et institutionnel, y compris défense).
