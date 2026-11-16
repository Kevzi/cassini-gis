# Lancer la démonstration en une commande

La suite de démo joue la phrase de démo de `CLAUDE.md` §1 dans un vrai QGIS, avec le vrai
noyau et le vrai modèle local, puis une seconde phrase (pente du MNT) et le rejeu du journal
après fermeture et réouverture du projet. Elle rend un rapport par critère et un résultat global.
Elle ne tourne pas dans la CI, qui n'a ni QGIS ni modèle.

## Prérequis

- QGIS 3.44 installé (le script prend `C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat` ;
  `CASSINI_QGIS_PYTHON` en désigne un autre).
- Le noyau installé (`%LOCALAPPDATA%\Cassini\bin\cassini.exe` ; `CASSINI_CORE` en désigne un autre)
  et le modèle local installé (`%LOCALAPPDATA%\Cassini\bin\models`, ou le dossier de `-Modeles`,
  qui ne doit contenir que le modèle à montrer).

## Lancer

Depuis la racine du dépôt :

```powershell
qgis-plugin\tests\lancer_demo.ps1
```

Le script joue la sonde `probe_demo.py` d'abord sur les **fixtures** synthétiques de
`fixtures/demo/` (copiées : jamais modifiées), puis sur le **projet réel** (par défaut celui
d'Annecy dans `..\donnees-test-cassini\annecy\`, hors dépôt). Le rapport complet est écrit dans un
fichier daté de `%TEMP%` (option `-Sortie`).

| Option | Effet |
|---|---|
| `-SansProjetReel` | ne joue que les fixtures ; le rapport dit « projet réel : non joué » |
| `-Projet <fichier.qgz>` | joue **vos propres données** : un projet QGIS qui a une couche de parcelles (nom contenant « parcelle »), un zonage avec un champ `typezone`, des gares (points, nom contenant « gare ») et un raster dont le nom commence par « MNT » |
| `-Modeles <dossier>` | dossier du modèle à montrer |
| `-Sortie <fichier>` | emplacement du rapport |

Le projet donné n'est pas modifié ; les sorties (dont le PDF) vont dans `<projet>.cassini\`,
à côté du projet. Ouvrez ce PDF et regardez-le : un verdict OK ne remplace pas le rendu.

## Lire le rapport

Chaque entrée jouée imprime un bloc, avant sa ligne `VERDICT` :

```
RAPPORT fixtures
  D1 phrase de démo, hors ligne   OK (aucune connexion externe du noyau)
  D2 rejeu identique              OK (9 étapes)
  D3 PDF : titre, légende, échelle, nord, sources   OK
  D4 commande de terrain du noyau OK (terrain.slope)
```

- **D1** : le plan composé par le modèle donne la bonne réponse (zone U, 500 m d'une gare, PDF
  lisible) et le noyau n'a ouvert aucune connexion vers l'extérieur.
- **D2** : le journal rejoué rend un résultat identique, avec autant d'étapes.
- **D3** : le PDF porte titre, légende, échelle, flèche du nord et sources, sur une page.
- **D4** : au moins une commande `terrain.*` a été exécutée par le noyau.

Chaque critère est `OK`, `ECHEC` (avec la raison) ou `non jugé` (la sonde s'est arrêtée avant de
pouvoir le juger : jamais un faux OK). Un échec de mise en page, de pente ou de rejeu
n'empêche pas de juger les critères suivants.

La dernière ligne est `RESULTAT GLOBAL : OK` ou `RESULTAT GLOBAL : ECHEC`.

## Code de sortie

| Code | Sens |
|---|---|
| 0 | tout est OK |
| 1 | un verdict en ECHEC |
| 2 | sonde non partie : Python de QGIS, noyau, dossier de modèles ou projet introuvable (un projet absent n'est jamais un OK silencieux) |

## Voir la sonde échouer

Les variables `CASSINI_DEMO_*` de `probe_demo.py` passent telles quelles. Par exemple
`$env:CASSINI_DEMO_DROP_ITEM = "legend"` retire la légende de ce que la sonde relève : le rapport
nomme D3 en échec et le code est 1.
