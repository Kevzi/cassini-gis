# Amendement d'architecture : commandes d'un module extérieur (2026-11-10)

Statut : **proposé, en attente du propriétaire.** Décision structurante : rien n'est appliqué
au contrat d'architecture ni aux tickets. La party du 2026-11-10 donne son avis au §8.

## 1. Le problème

- **Déclencheur :** demande du propriétaire du 2026-11-10.
- **Besoin :** un **intégrateur** — éditeur de logiciel, bureau d'études, service d'une
  collectivité — veut ajouter à Cassini ses **commandes métier** (par exemple un calcul propre à
  son réseau d'assainissement, ou un contrôle de conformité à un règlement local) sans forker
  le dépôt et sans demander leur entrée au catalogue public.
- **Aujourd'hui :** le registre est compilé dans le binaire `cassini` (AD-13) ; le catalogue
  public refuse « tout paramètre propre à un usage particulier » (catalogue §1) ; la seule voie
  est le fork, qui coupe l'intégrateur des corrections du noyau.

## 2. Proposition

1. **Un programme noyau composé, pas un greffon.** `cassini-core` et la crate du binaire
   (`/cli`) exposent une **API de bibliothèque publique et versionnée** (Apache-2.0) : un
   intégrateur écrit son propre programme, qui reprend les quatre points d'entrée (`serve`,
   `mcp`, `run`, `replay`) et le registre officiel, et y **enregistre ses commandes**. Rien
   n'est chargé dynamiquement à l'exécution : pas de bibliothèque partagée découverte dans un
   dossier, pas de script. AD-13 tient pour chaque programme : un binaire, un registre, quatre
   points d'entrée identiques.
2. **Commandes ajoutées : `runs_in: core` seulement.** La table de l'extension est fermée et
   sous GPL (AD-24) : une commande déléguée à QGIS n'existe que si l'extension la connaît. Une
   commande ajoutée calcule dans le programme de l'intégrateur et rend ses sorties par les
   conventions existantes (raster → `layer.open_raster` dicté ; entités → GeoJSON et
   `layer.open_vector` dicté, AD-30 ; style par `layer.set_style`). Un intégrateur qui veut un
   geste QGIS nouveau modifie l'extension sous GPL : hors de cet amendement.
3. **Espace de noms.** Une commande ajoutée garde la forme `domaine.verbe` (AD-6), mais son
   domaine ne peut être aucune famille du catalogue (`layer`, `project`, `analysis`, `network`,
   `raster`, `data`, `layout`, `geocode`, `terrain`, `coord`, `geo`, `map`, `system`, `model`) :
   le registre refuse au démarrage un identifiant en collision, avec un message qui le nomme.
4. **Catalogue figé.** Le catalogue public décrit le registre officiel et sa recette
   (`core/tests/catalogue.rs`) ne change pas. Le programme de l'intégrateur tient son propre
   catalogue de ses commandes ; la bibliothèque fournit la même recette, paramétrée, pour qu'il
   la lance chez lui.
5. **Annonce.** `system.about` et la réponse de `session.hello` disent le nom et la version du
   programme, la version du noyau dont il dérive, et la liste des commandes ajoutées. Les
   commandes ajoutées apparaissent au catalogue de la session **comme les autres** : formulaire
   du panneau engendré depuis leur schéma JSON (AD-6), outils MCP (AD-13, modes relais et
   autonome), journal et rejeu (AD-11 : un rejeu par un autre programme nomme la commande
   manquante au lieu de la sauter).
6. **Extension.** Réglage générique « Programme noyau à utiliser » : c'est le réglage
   `cassini/core_path` qui existe déjà (AD-17), rendu visible dans les réglages du panneau avec
   son explication. L'extension **ne connaît aucune commande particulière** : elle affiche ce
   que le noyau annonce, comme aujourd'hui.

## 3. Frontière GPL et Apache-2.0 (§2.3, AD-8, AD-16)

- L'extension reste générique : aucun nom de commande ajoutée, aucune branche propre à un
  intégrateur. Le canal ne porte que les messages publics et documentés du protocole ; une
  commande ajoutée y passe comme une commande officielle (identifiant, paramètres à schéma
  JSON, résultat typé). L'argument de distance de bras est inchangé.
- Le programme de l'intégrateur est construit sur des crates Apache-2.0 ; la licence de ses
  propres commandes est son choix. Pour que l'argument de distance de bras vaille aussi pour
  lui, il doit tenir AD-1 (ne lier ni QGIS, ni GDAL, ni GEOS, ni PROJ) : la documentation de
  l'API le dit ; Cassini ne peut pas le vérifier chez lui.

## 4. Le modèle local ne connaît pas les commandes ajoutées

Un modèle ajusté n'a appris que le catalogue de son entraînement. Proposition de la party :

1. La **grammaire** du modèle local n'inclut une commande ajoutée que si la déclaration du
   modèle (`*.modele.json`) la nomme parmi les commandes qu'il connaît ; sinon elle est exclue
   de la grammaire et le modèle ne peut pas l'inventer.
2. Pour qu'un modèle la connaisse, l'intégrateur entraîne **son propre adaptateur** sur la même
   base par la chaîne reproductible, qui accepte déjà un fichier d'exemples externe
   (ticket 5.23) ; sa déclaration nomme ses commandes.
3. Sans adaptateur, la commande reste offerte par le **formulaire** du panneau et par le
   **MCP** (une IA externe lit son schéma et sa description), jamais par le modèle local.
4. Le mode « long prompt » (GGUF seul, catalogue montré dans le prompt) reste possible pour
   des essais, sans promesse de justesse ni de durée.

## 5. Sécurité : un programme noyau choisi par l'utilisateur

- Le risque existe déjà : `cassini/core_path` et `CASSINI_CORE` permettent de désigner
  n'importe quel programme (AD-17). L'amendement le rend visible au lieu de le créer.
- Le panneau affiche en permanence le nom et la version du programme branché, et la mention
  « programme dérivé de Cassini » quand ce n'est pas le programme officiel.
- **Confirmation au premier branchement** d'un programme non officiel : l'extension retient
  l'empreinte SHA-256 du programme accepté par l'utilisateur (calcul en Python, côté GPL) et
  redemande si elle change.
- La preuve hors ligne d'AD-14 (recette débranchée) ne porte que sur le programme officiel ;
  le panneau le dit pour un programme dérivé, et la documentation de l'API demande à
  l'intégrateur de lancer la même recette.

## 6. Décisions d'architecture pesées

| AD | Effet |
|---|---|
| AD-1 | Inchangée pour le noyau ; exigée de l'intégrateur par la documentation. |
| AD-6 | Amendée : « le registre » devient le registre du programme branché, officiel plus commandes ajoutées ; le catalogue public fait foi sur le registre officiel seul ; collisions refusées. |
| AD-13 | Précisée : un programme dérivé garde un binaire, un registre et les quatre points d'entrée ; aucune commande propre à un point d'entrée. |
| AD-16, §2.3 | Inchangées : l'extension reste générique, le canal public. |
| AD-17 | Le réglage `cassini/core_path` devient visible dans le panneau ; l'ordre de recherche ne change pas. |
| AD-23 | Précisée : la grammaire du modèle ne contient que les commandes que sa déclaration connaît. |
| AD-24 | Inchangée : aucune commande déléguée ajoutée sans modifier l'extension. |
| AD-11 | Inchangée : la version du registre journalisée distingue déjà deux programmes. |

## 7. Texte proposé pour AD-6 (non appliqué)

> - **Amendement (proposé le 2026-11-10, changement de cap `change-commandes-d-un-module-exterieur`) —
>   commandes ajoutées.** Un programme construit sur la bibliothèque du noyau peut enregistrer
>   des commandes `runs_in: core` hors des familles du catalogue ; le registre refuse toute
>   collision au démarrage. Ces commandes sont annoncées, offertes, journalisées et rejouées
>   comme les autres ; le catalogue public fait foi sur le registre officiel seul. Le modèle
>   local ne les voit que si sa déclaration les nomme. L'extension n'en connaît aucune.

## 8. Avis de la party du 2026-11-10

> 🏛️ **Architecte :** Pour le principe, avec une ligne rouge : pas de chargement dynamique. Un
> programme composé à la compilation, c'est AD-13 qui tient. Le vrai coût, c'est l'API publique :
> une fois promise, chaque refonte du registre devient un changement de version majeure.
>
> 📐 **Géomètre :** J'ai des calculs de cubature que personne ne mettra jamais au catalogue
> public, et c'est normal. Je veux les lancer depuis le même panneau, pas depuis un fork qui
> prend six mois de retard.
>
> 🗺️ **Cartographe :** Et moi je ne veux pas qu'un collègue branche un exécutable trouvé sur une
> clé sans que le panneau le crie. Le nom du programme affiché en permanence, et une
> confirmation au premier lancement, sinon je vote contre.
>
> 🏛️ **Architecte :** Accordé. Et le modèle : s'il n'a pas appris la commande, elle sort de la
> grammaire. Un modèle qui devine une commande métier, c'est pire qu'un refus.
>
> 📐 **Géomètre :** Tant que mon formulaire et le MCP la voient, ça me va.

**Recommandation de la party :** **approuver le principe**, ne rien construire avant la
publication (epic 7) et la stabilisation du catalogue étendu (epic 10), et commencer par
figer l'API publique du registre ; les trois garde-fous du §5 et la règle de grammaire du §4
font partie de l'approbation.

## 9. Entrées prévues (non créées)

Epic à choisir par le propriétaire (proposition : une entrée par epic concerné, toutes après
la publication) :

- **Epic 2** (`epic-registre-projet-import`) — « API publique du registre et programme noyau
  composé » : enregistrement de commandes ajoutées, refus des collisions, recette du catalogue
  paramétrée, programme d'exemple dans `/tests` avec une commande ajoutée fictive ; `risk`
  high ; `covers` CAP-6.
- **Epic 3** (`epic-facade-qgis`) — « Programme noyau choisi dans les réglages, annoncé et
  confirmé » : réglage visible, nom et version affichés, mention de programme dérivé,
  confirmation par empreinte au premier branchement ; `risk` medium ; `covers` CAP-13.
- **Epic 5** (`epic-agent-et-mcp`) — « Grammaire limitée aux commandes connues du modèle » :
  la déclaration nomme les commandes connues ; les autres sont exclues de la grammaire et
  restent offertes au formulaire et au MCP ; `risk` high ; `covers` CAP-4, CAP-8.

À créer par `bmad-ticket` seulement après l'accord du propriétaire.
