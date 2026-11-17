# Frontière entre l'extension (GPL) et le noyau (Apache-2.0)

Cassini est livré en deux programmes qui restent séparés.

- **Le noyau**, en Rust, sous licence Apache-2.0 : le binaire `cassini`, le registre de
  commandes, le journal, l'agent et le moteur de terrain. Il ne contient ni GPL ni LGPL, y
  compris par liaison statique et de façon transitive : ni GEOS, ni GDAL. La liste de ce qu'il
  embarque est dans `LICENCES-TIERCES.txt`.
- **L'extension QGIS**, en Python, sous licence GPL, comme QGIS. Elle est livrée à part, par le
  dépôt d'extensions de QGIS, et ne contient aucun binaire.

## Pourquoi deux programmes

QGIS est sous GPL et l'extension s'exécute dans son processus : elle est donc elle aussi sous GPL.
Le noyau, lui, n'est pas lié à QGIS. Les deux programmes sont des **processus séparés** : l'extension
lance le binaire du noyau comme processus enfant, et ils dialoguent par l'entrée et la sortie
standard, sans socket réseau, sans port et sans fichier d'échange.

## Ce que le canal transporte

Le canal est du JSON-RPC 2.0, un message par ligne, dans les deux sens. Il ne porte que des
messages publics et documentés : identifiants de commandes, chemins, paramètres décrits par un
schéma JSON, résultats typés. Il ne porte jamais une structure de données interne d'un des deux
programmes. L'échange reste ainsi à distance de bras : ce n'est pas un échange assez intime pour
que les deux programmes n'en fassent qu'un.

## GDAL et GEOS

QGIS embarque GDAL et GEOS de son côté. L'extension les utilise dans le processus de QGIS, sous
leurs licences et celles de QGIS. Le noyau ne les lie jamais : passer par QGIS ne lève pas
l'interdiction de GPL et de LGPL dans le noyau.

## Le noyau tourne sans QGIS

Le noyau se lance, s'exécute et se teste seul. Toute dépendance du noyau envers QGIS serait un
défaut. C'est ce qui permet à une application autonome de reprendre le noyau sans l'extension.

## Ce que cela donne à l'utilisateur

- Le noyau, sous Apache-2.0, peut être utilisé, y compris commercialement, sous les conditions de
  cette licence.
- L'extension, sous GPL, se redistribue sous GPL, avec son code source.
- Les deux se téléchargent séparément, et aucun fichier de l'un n'est livré dans l'archive de l'autre.

Ce texte décrit l'organisation du projet ; il ne remplace pas les textes des licences.
