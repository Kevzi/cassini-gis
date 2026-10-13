# Jeux de données de test

Cassini ne distribue aucune donnée géographique (CLAUDE.md §2.4). Ce dossier ne
contient donc **que** des jeux de test légers, fabriqués pour la recette :

- assez petits pour être versionnés sans alourdir le dépôt ;
- soit synthétiques, soit issus d'une source sous licence ouverte explicitement
  citée dans ce fichier ;
- jamais une donnée réelle de production, et jamais un livrable d'utilisateur.

Le `.gitignore` du dépôt ignore les formats de données lourds partout ailleurs,
avec une exception pour ce dossier.

| Fichier | Nature | Origine | Utilisé par |
|---|---|---|---|
| `basemap-villeneuve-le-roi.pmtiles` | Archive PMTiles v3, tuiles PNG en niveaux de gris, 35 tuiles, 25,7 Ko | **Généré**, aucun téléchargement : `app/scripts/generate-basemap.mjs` | Le fond de carte de l'application, et la recette |

## Le fond de carte de recette

C'est un damier, pas une carte. Il sert à prouver trois choses : que l'archive
est conforme à la spécification PMTiles v3, que MapLibre sait l'afficher depuis
un fichier local, et qu'aucune requête réseau n'est nécessaire pour voir une
carte à l'écran.

- **Emprise** : `2.37, 48.71` → `2.46, 48.76`, soit la commune de
  Villeneuve-le-Roi, la même boîte que le repère de `app/src/basemap.style.json`.
- **Zooms** : 10 à 14. En dehors, l'archive est vide — c'est assumé. La carte
  s'ouvre au zoom 12 sur `2.415, 48.735`.
- **Hors de l'emprise** : aucune tuile. Un lecteur doit répondre « pas de
  tuile », pas planter.

### D'où l'application le lit

L'archive est **embarquée dans l'installeur** : la carte affiche un fond dès la
première ouverture, sans aucune manipulation. C'est un damier fabriqué par le
dépôt, sans la moindre donnée géographique tierce, donc rien n'est redistribué
(CLAUDE.md §2.4).

1. `app/src-tauri/tauri.conf.json`, clé `bundle.resources`, copie ce fichier dans
   l'installeur sous `fixtures/basemap-villeneuve-le-roi.pmtiles`.
2. `app/src-tauri/src/lib.rs` le cherche d'abord dans le répertoire de ressources
   de l'application installée, puis — en développement, où `cargo tauri dev` ne
   passe pas par l'installeur — dans le fichier de ce dossier.
3. Le chemin est validé par `BasemapSource::from_local_path`, puis lu par
   plages d'octets avec `BasemapSource::read_range`. **L'interface ne connaît
   jamais le chemin** : elle demande des octets par l'IPC Tauri, et la carte les
   reçoit par le protocole `pmtiles://cassini-basemap-fixture`. Aucune requête
   réseau, aucune origine HTTP ajoutée au CSP de la fenêtre.

`app/scripts/verify-protocol.mjs` vérifie que ces endroits s'accordent : la clé
du style est celle du code, le chemin du moteur est celui de l'installeur, et la
commande appelée par l'interface est celle qu'enregistre le moteur.

### Régénérer

```bash
cd app
npm run fixtures          # réécrit le .pmtiles
npm run verify:fixtures   # le relit avec la bibliothèque pmtiles et le valide
```

La génération est **déterministe** : deux exécutions produisent le même fichier,
octet pour octet, ce que la vérification contrôle. `npm run check` lance la
vérification, donc une archive cassée fait échouer la recette.

### Ce que la vérification contrôle

30 contrôles, dont : la magie et la version de l'en-tête, le type de tuile, les
compressions, les bornes de zoom et l'emprise annoncées, la relecture des
métadonnées, la concordance de notre calcul d'identifiant de tuile avec celui de
la bibliothèque sur les 35 tuiles, la valeur de référence donnée par la
spécification (`z12/3423/1763 → 19078479`), la validité PNG de chaque tuile avec
ses CRC, l'absence de tuile hors emprise et hors zooms, le déterminisme, la
concordance de l'emprise avec le repère lu dans `app/src/basemap.style.json`, le
centre et le zoom d'ouverture annoncés, et enfin **quatre contrôles qui mettent
le validateur en échec exprès** — octet corrompu, tuile tronquée, signature
absente — parce qu'un garde-fou qu'on n'a jamais vu refuser quelque chose ne
garde rien.

`npm run verify:protocol` ajoute 24 contrôles sur le chemin de chargement de la
carte : le TileJSON construit depuis l'en-tête, une tuile connue servie en PNG,
une tuile hors emprise ou hors zooms qui revient vide sans exception, et la
preuve qu'une clé non enregistrée part bien sur le réseau — c'est ce que la garde
de `check-offline.mjs` existe pour empêcher. Le refus d'une plage hors fichier,
lui, est contrôlé là où il est écrit : les tests de `cassini-core`.
