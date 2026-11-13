# Changement de cap : distribution du MVP, deux portes (2026-11-10)

Statut : **approuvé** par le propriétaire le 2026-11-10. Portée mineure, ajustement direct
des documents ; aucun ticket créé (l'epic publication sera incepté plus tard avec
`bmad-ticket`).

## 1. Le problème

- **Déclencheur :** préparation de l'epic publication. AD-17 disait « un paquet Cassini par
  système porte le binaire et, en option, un modèle », et l'epic 5 avait tranché « livré dans
  le paquet Cassini, jamais téléchargé ». Restait à dire comment ce paquet arrive chez
  l'utilisateur, alors que le modèle pèse plusieurs Go.
- **Nature :** précision d'une exigence existante (livraison), pas un virage de produit.
- **Faits mesurés :**
  - noyau Windows avec accélération Vulkan ≈ 60 Mo (`cassini.exe` et DLL ggml/llama, aucune
    DLL NVIDIA) ;
  - modèle GGUF Q4_K_M : 2,65 Go ;
  - une release GitHub refuse les fichiers de plus de 2 Gio : le modèle n'y tient pas ;
  - base Qwen sous Apache-2.0, redistribuable avec sa notice.
- **Pourquoi un changement de cap :** AD-17 est un invariant d'architecture (CLAUDE.md §8 :
  il se cite, il ne se re-décide pas en passant), et son *Prevents* écarte « toute tentation de
  réparer une installation par le réseau ». Un installeur qui télécharge doit donc être borné
  par écrit.

## 2. Décisions du propriétaire (2026-11-10)

1. **Deux portes d'installation.**
   - **Porte 1, installeur léger en ligne** (quelques Mo) : télécharge **une seule fois,
     pendant l'installation**, le noyau depuis une release GitHub et le modèle depuis Hugging
     Face, depuis des **adresses fixes**, et vérifie l'**empreinte SHA-256** de chaque fichier
     avant de l'installer. Après l'installation, Cassini (noyau et extension) ne touche plus
     jamais au réseau. L'installeur est un programme séparé du noyau ; le noyau et
     l'extension ne changent pas.
   - **Porte 2, installeur complet hors ligne** : un seul fichier qui contient tout (noyau et
     modèle), pour les postes isolés du réseau. Pas de « pack USB ».
2. **Le modèle est hébergé sur Hugging Face**, dépôt public, avec sa licence.
3. **Signature des installeurs : plus tard**, avant les premiers vrais clients. Au début,
   installeurs non signés, avec une page qui explique l'avertissement de Windows.
4. **Systèmes : Windows et Linux d'abord** ; macOS plus tard, quand un Mac est disponible.

### Compléments du 2026-11-10 (réponse du propriétaire aux questions ouvertes)

5. **Hébergement de la porte 2** : l'installeur hors ligne, qui dépasse 2 Gio, est hébergé sur
   Hugging Face, à côté du modèle.
6. **La porte 2 contient aussi l'extension QGIS** (archive installable à la main), un poste
   isolé du réseau ne pouvant pas la prendre au dépôt d'extensions.
7. **Le modèle reste facultatif** dans l'installeur en ligne (utilisateur d'une IA externe
   seulement).
8. **Manifeste des versions** : les empreintes de référence viennent d'un manifeste publié dans
   la release GitHub, à adresse fixe (version et SHA-256 de chaque fichier) ; le même manifeste
   sert aux mises à jour.
9. **Mises à jour** :
   - « Mettre à jour Cassini », lancé à la main par l'utilisateur (entrée du menu Démarrer sous
     Windows, commande sous Linux), est le même programme que l'installeur en ligne : il lit le
     manifeste, compare à l'installé, propose, télécharge, vérifie l'empreinte, installe ;
   - un nouveau modèle se télécharge à côté des autres ; l'ancien est toujours gardé, jamais
     supprimé ; le sélecteur de modèle du panneau permet de revenir en arrière ;
   - l'extension se met à jour par le gestionnaire d'extensions de QGIS, rien à faire ;
   - le panneau affiche la version installée et, passé un délai (par exemple trois mois depuis
     l'installation ou la dernière mise à jour, calcul de date local, sans réseau), un rappel
     « pensez à lancer Mettre à jour Cassini » ;
   - aucune vérification automatique en arrière-plan. Piste, non décidée : une option
     désactivée par défaut pourra venir plus tard si des clients la demandent.
10. **Initiative** : la question du chemin de livraison du modèle est marquée tranchée.

## 3. Analyse d'impact

| Endroit | Effet |
|---|---|
| CLAUDE.md §2.5 (hors ligne total) | Inchangé sur le fond : « Aucune fonction du MVP n'utilise le réseau » reste vrai pour le noyau et l'extension. Une phrase précise que l'installeur en ligne n'est pas une fonction du produit en fonctionnement. |
| CLAUDE.md §2.6 | Inchangé : il porte sur les données de l'utilisateur, que Cassini ne télécharge toujours pas. |
| CLAUDE.md §3, ligne « Distribution » | Complétée : deux portes, adresses fixes, empreintes, Windows et Linux d'abord ; seul l'installeur télécharge. |
| Architecture, AD-17 | Amendement daté : la règle de l'extension qui ne télécharge jamais rien est gardée ; la frontière de l'installeur est écrite. Ordre de recherche du noyau inchangé. |
| Architecture, AD-14 | Inchangé : le noyau et l'extension n'ouvrent toujours aucun accès réseau ; la recette « zéro requête réseau » de l'extension tient. L'installeur est hors de ce périmètre. |
| Architecture, enveloppe d'exploitation | Inchangée. « Une clé USB portant les deux artefacts suffit » reste vrai : la clé porte l'archive de l'extension et l'installeur complet hors ligne. Le schéma cite encore `pkg` (macOS) : macOS est repoussé, pas retiré. |
| Spec (`spec-cassini-mvp.md`, Constraints) | Inchangée : « le noyau et le modèle dans un paquet Cassini installé à part » et « rien n'est jamais téléchargé pour réparer une installation » restent vrais. |
| Initiative, Unknown « chemin de livraison du modèle » | Marquée tranchée par une Decision datée. |
| Panneau (extension) | Affichera la version installée et le rappel de mise à jour, calcul de date local, sans réseau ; le sélecteur de modèle existant sert à revenir en arrière. À découper à l'inception de l'epic publication. |
| Epic publication | Done when 2 (Windows et Linux, deux portes) et 6 (Hugging Face, deux chemins) mis en accord ; Description ajustée ; une Decision datée. |
| Epic 5 (`epic-agent-et-mcp`) | La ligne « Tranché : livré dans le paquet Cassini, jamais téléchargé » est gardée ; une Decision datée la précise. L'en-tête de `tickets.toml`, commentaire historique, n'est pas touché. |
| Epic de l'épreuve du modèle local | Une Decision datée précise celle du 2026-10-14 sur AD-17, sans l'effacer. |
| Noyau, extension, canal | Aucun changement de code. |
| PRD, UX | Aucun changement. |

## 4. Voie recommandée

**Ajustement direct** des documents, sans retour en arrière ni réduction de périmètre du
produit. Le seul périmètre qui recule est macOS, repoussé dans l'epic publication. Effort :
documentaire ici ; le travail réel (deux installeurs par système, dépôt Hugging Face, page sur
l'avertissement de Windows) sera découpé à l'inception de l'epic publication. Risque : faible
pour le produit (le noyau et l'extension ne changent pas) ; le point sensible est la frontière
de l'installeur, désormais écrite dans AD-17.

## 5. Changements appliqués

### CLAUDE.md §2.5

Ajout, après « Le réseau ne sert qu'à brancher une IA externe, en option. » :

> L'installeur en ligne (§3, « Distribution ») n'est pas une fonction du produit : programme
> séparé, il télécharge une seule fois, pendant l'installation, puis le noyau et l'extension
> ne touchent plus jamais au réseau.

### CLAUDE.md §3, ligne « Distribution »

Ajout, après « sans jamais rien télécharger. » :

> Ce paquet passe par **deux portes** (2026-11-10) : un installeur léger en ligne, qui
> télécharge une seule fois, pendant l'installation, le noyau depuis une release GitHub et le
> modèle depuis Hugging Face, à des adresses fixes, en vérifiant l'empreinte SHA-256 de chaque
> fichier avant de l'installer ; et un installeur complet hors ligne, un seul fichier qui
> contient tout, pour les postes isolés du réseau. Windows et Linux d'abord, macOS plus tard.
> Seul l'installeur, programme séparé, télécharge : jamais le noyau ni l'extension (AD-17).

### Architecture — AD-17

Amendement ajouté sous la règle, qui reste telle quelle :

> **Amendement (2026-11-10, changement de cap `change-distribution-deux-portes`) — deux portes
> d'installation.** La règle ci-dessus ne change pas : l'extension ne télécharge jamais rien,
> le noyau non plus. Le paquet Cassini existe sous deux formes, l'une et l'autre programmes
> séparés du noyau et de l'extension : porte 1, installeur léger en ligne (adresses fixes,
> release GitHub pour le noyau, dépôt public Hugging Face pour le modèle, empreinte SHA-256
> vérifiée avant installation) ; porte 2, installeur complet hors ligne, un seul fichier.
>
> Frontière : seul un installeur peut télécharger, une fois, pendant l'installation, à des
> adresses fixes, empreintes vérifiées. Jamais le noyau ni l'extension, jamais à l'exécution,
> et jamais pour « réparer » une installation : un noyau ou un modèle introuvable se signale
> comme avant, il ne déclenche aucun téléchargement. Après l'installation, Cassini ne touche
> plus au réseau (AD-14 inchangé).

### Epic publication — `epic-publication/epic-publication.md`

Done when 2, avant :

> Le noyau est empaqueté pour Windows, macOS et Linux, produit automatiquement et
> téléchargeable depuis une release du dépôt.

Après :

> Le noyau est empaqueté pour Windows et Linux (macOS plus tard, quand un Mac est disponible),
> produit automatiquement et téléchargeable depuis une release du dépôt ; chaque système a ses
> deux portes : un installeur léger en ligne et un installeur complet hors ligne.

Done when 6, avant :

> Le modèle local est trouvable et installable sur un poste isolé, par un chemin documenté qui
> ne passe pas par le dépôt d'extensions.

Après :

> Le modèle local est publié sur un dépôt public Hugging Face avec sa licence et sa notice ; il
> s'installe par l'installeur en ligne, qui vérifie son empreinte SHA-256, ou, sur un poste
> isolé du réseau, par l'installeur complet hors ligne, un seul fichier ; aucun des deux
> chemins ne passe par le dépôt d'extensions.

Description : « le paquet du noyau pour les trois systèmes » devient « le paquet du noyau, par
deux portes d'installation, pour Windows et Linux d'abord et macOS plus tard ».

Notes, nouvelle Decision datée du 2026-11-10 : deux portes, Hugging Face, signature plus tard
avec page sur l'avertissement de Windows, Windows et Linux d'abord ; elle remplace pour ces
points la décision du 2026-10-14, qui reste écrite.

Notes, compléments du même jour : une Decision datée (hébergement de la porte 2, extension dans
la porte 2, modèle facultatif, manifeste des versions, « Mettre à jour Cassini », modèles gardés
côte à côte, rappel du panneau), une ligne Parked (vérification automatique désactivée par
défaut, piste) et une Open question (format des installeurs, Vulkan sous Linux).

### Compléments du 2026-11-10

- **AD-17** : un second paragraphe dans l'amendement — porte 2 sur Hugging Face avec
  l'extension, modèle facultatif, manifeste des versions, « Mettre à jour Cassini » soumis à la
  même frontière que l'installeur (séparé du noyau et de l'extension, lancé par l'utilisateur,
  adresses fixes, empreintes vérifiées), rappel du panneau sans réseau.
- **CLAUDE.md §3** : une phrase sur les mises à jour (même programme, manifeste, modèles côte
  à côte).
- **Initiative** (`initiative-cassini-mvp.md`) : une Decision datée sous l'Unknown du chemin
  de livraison du modèle, qui la marque tranchée.

### Epic 5 — `epic-agent-et-mcp/epic-agent-et-mcp.md`

Nouvelle Decision datée sous la ligne « Tranché » (gardée) : le modèle n'est jamais téléchargé
par le noyau ni par l'extension, ni au premier lancement ; il vient de l'installeur en ligne
(une fois, Hugging Face, empreinte vérifiée) ou de l'installeur complet hors ligne.

### Epic de l'épreuve du modèle local — `epic-epreuve-du-modele-local/epic-epreuve-du-modele-local.md`

Nouvelle Decision datée sous celle du 2026-10-14 sur AD-17 (gardée), qui la précise par les
deux portes.

## 6. Questions ouvertes

Tranchées le 2026-11-10 (voir §2, compléments) : hébergement de la porte 2, extension dans la
porte 2, modèle facultatif, origine des empreintes, mise à jour du modèle, ligne de
l'initiative. Restent ouvertes, à trancher à l'inception de l'epic publication :

1. **Format des installeurs** (Windows : `.exe` ou `.msi` ; Linux : archive, paquet ou
   script), et leur licence, l'installeur étant hors du noyau.
2. **Accélération Vulkan sous Linux** : le noyau Vulkan n'est mesuré que sous Windows.

Piste, non décidée : une vérification automatique des mises à jour, désactivée par défaut, si
des clients la demandent.

## 7. Passage à la réalisation

- **Portée : mineure** (ajustement direct des documents, fait dans ce changement de cap).
- **À appliquer avec `bmad-ticket` :** rien maintenant ; l'epic publication sera incepté plus
  tard, en reprenant les Done when 2 et 6 et les questions ouvertes ci-dessus.
- **Réussite :** à l'inception de l'epic publication, chaque porte a son ticket ; la recette
  continue de prouver que le noyau et l'extension n'ouvrent aucun accès réseau ; une personne
  extérieure installe Cassini par l'une ou l'autre porte et refait la démo (Done when 4).
