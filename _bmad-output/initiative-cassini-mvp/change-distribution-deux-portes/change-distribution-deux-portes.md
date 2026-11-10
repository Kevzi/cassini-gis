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
| Initiative, Unknown « chemin de livraison du modèle » | Tranché de fait par ce document ; la ligne n'est pas modifiée ici (voir §6). |
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

### Epic 5 — `epic-agent-et-mcp/epic-agent-et-mcp.md`

Nouvelle Decision datée sous la ligne « Tranché » (gardée) : le modèle n'est jamais téléchargé
par le noyau ni par l'extension, ni au premier lancement ; il vient de l'installeur en ligne
(une fois, Hugging Face, empreinte vérifiée) ou de l'installeur complet hors ligne.

### Epic de l'épreuve du modèle local — `epic-epreuve-du-modele-local/epic-epreuve-du-modele-local.md`

Nouvelle Decision datée sous celle du 2026-10-14 sur AD-17 (gardée), qui la précise par les
deux portes.

## 6. Questions ouvertes (non tranchées par le propriétaire)

1. **Où héberger l'installeur complet hors ligne ?** Il contient le modèle (2,65 Go) et dépasse
   donc la limite de 2 Gio d'un fichier de release GitHub. Hugging Face, autre hébergement, ou
   découpage : à décider.
2. **L'installeur complet hors ligne porte-t-il aussi l'archive de l'extension ?** Sur un poste
   isolé du réseau, le gestionnaire d'extensions de QGIS ne peut pas la télécharger.
3. **Le modèle reste-t-il optionnel ?** AD-17 disait « en option, un modèle » ; les décisions du
   2026-11-10 font télécharger ou embarquer le modèle sans dire si l'utilisateur peut s'en
   passer (par exemple pour n'utiliser qu'une IA externe par MCP).
4. **D'où l'installeur en ligne tient-il les empreintes de référence ?** Écrites dans
   l'installeur à sa construction, ou lues ailleurs : à fixer à l'inception.
5. **Technologie, format et licence des installeurs** (Windows : `.exe` ou `.msi` ; Linux :
   archive, paquet ou script). L'installeur est hors du noyau : sa licence n'est pas encore
   dite.
6. **Mise à jour du modèle** : quand un nouveau modèle ajusté est publié, faut-il un nouvel
   installeur, et que fait une installation existante ? (Le noyau ne se met jamais à jour seul.)
7. **Accélération sous Linux** : le noyau Vulkan n'est mesuré que sous Windows.
8. **Ligne Unknown de l'initiative** (« chemin de livraison du modèle GGUF sur un poste
   isolé ») : à marquer tranchée par ce document, si le propriétaire le souhaite.

## 7. Passage à la réalisation

- **Portée : mineure** (ajustement direct des documents, fait dans ce changement de cap).
- **À appliquer avec `bmad-ticket` :** rien maintenant ; l'epic publication sera incepté plus
  tard, en reprenant les Done when 2 et 6 et les questions ouvertes ci-dessus.
- **Réussite :** à l'inception de l'epic publication, chaque porte a son ticket ; la recette
  continue de prouver que le noyau et l'extension n'ouvrent aucun accès réseau ; une personne
  extérieure installe Cassini par l'une ou l'autre porte et refait la démo (Done when 4).
