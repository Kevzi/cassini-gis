# Construire le paquet du noyau (ticket 5.21)

Le noyau livré n'est plus un seul fichier mais un **dossier** : `cassini.exe` et les
bibliothèques de llama.cpp, posées à côté de lui. Au démarrage, le moteur charge ses
« moteurs de calcul » depuis le dossier de l'exécutable, jamais depuis le dossier de
construction :

- un **moteur processeur**, en plusieurs variantes par jeu d'instructions
  (`ggml-cpu-x64.dll`, `ggml-cpu-sse42.dll`, `ggml-cpu-haswell.dll`…) : la meilleure pour le
  processeur du poste est prise seule, y compris sur un processeur sans AVX2 ;
- un **moteur carte graphique**, `ggml-vulkan.dll` (Vulkan : NVIDIA, AMD et Intel). Il est
  ignoré s'il ne se charge pas (pas de pilote Vulkan, donc pas de `vulkan-1.dll`) ou s'il ne
  trouve aucune carte.

Le modèle va entièrement sur une **carte dédiée** s'il y en a une ; une carte intégrée n'est
jamais prise. Si la carte refuse le modèle ou son contexte (mémoire trop petite), le modèle est
rechargé sur le processeur, une fois. Aucun réglage n'est demandé à l'utilisateur.

Pourquoi des bibliothèques à part : lié directement au chargeur Vulkan, `cassini.exe` refuserait
de démarrer sur un poste sans pilote Vulkan. Chargé à part, un moteur qui échoue est simplement
ignoré : c'est le repli sur le processeur.

CUDA n'est pas livré (bibliothèques propriétaires de NVIDIA) ; la feature `cuda` reste réservée
aux mesures.

## Prérequis du poste de construction

Ceux de `modele-local-sous-windows.md` (Build Tools C++, CMake, libclang, chemin court), plus le
**SDK Vulkan**, sur le poste de construction seulement — il n'est jamais livré :

```powershell
winget install KhronosGroup.VulkanSDK
```

L'installeur pose `VULKAN_SDK` au niveau de la machine (par exemple
`C:\VulkanSDK\1.4.363.0`) ; une fenêtre PowerShell ouverte avant l'installation ne la voit pas :
la rouvrir, ou la poser à la main. La construction en a besoin pour compiler les shaders.

La recette par défaut (CI Linux et Windows) n'a pas ce SDK : la feature `gpu` est hors défaut,
et `cargo build` sans elle donne le noyau processeur seul, d'un seul fichier, comme avant.

## Commande

```powershell
$env:VULKAN_SDK = "C:\VulkanSDK\1.4.363.0"
$env:LIBCLANG_PATH = "C:\chemin\vers\libclang\clang\native"
$env:CARGO_TARGET_DIR = "C:\ct"
cargo build --release -p cassini --features gpu
cargo test --release -p cassini --features gpu --test hors_ligne
```

La première construction compile llama.cpp et ses shaders Vulkan (une dizaine de minutes). Il
arrive que la toute première échoue au milieu de la compilation des shaders : la relancer
suffit.

Le second appel vérifie le dossier : ni `cassini.exe` ni aucune DLL posée à côté n'importe une
bibliothèque réseau, `cassini.exe` n'importe pas le chargeur Vulkan, et les moteurs attendus sont
là.

## Fichiers à copier

Depuis `C:\ct\release`, vers `%LOCALAPPDATA%\Cassini\bin` (où l'extension cherche le noyau), QGIS
fermé :

| Fichier | Rôle |
|---|---|
| `cassini.exe` | le noyau |
| `llama.dll`, `ggml.dll`, `ggml-base.dll` | llama.cpp, importés par `cassini.exe` |
| `ggml-cpu-*.dll` (toutes) | moteur processeur, une variante par jeu d'instructions |
| `ggml-vulkan.dll` | moteur carte graphique (facultatif : sans lui, processeur) |

Ne **jamais** copier `llama-common.dll` : le noyau ne l'importe pas et elle embarque un client
HTTP (elle importe `ws2_32`). La construction ne la pose d'ailleurs pas dans `C:\ct\release`.

Le dossier `models\` ne change pas.

## Vérifier sur le poste

- Avec la carte : une phrase dans QGIS ; la mémoire de la carte monte (`nvidia-smi`, ou le
  gestionnaire des tâches, onglet Performances).
- Sans la carte : lancer QGIS depuis une fenêtre où `GGML_VK_VISIBLE_DEVICES` vaut un texte qui
  n'est pas un numéro (par exemple `set GGML_VK_VISIBLE_DEVICES=aucune`, puis lancer QGIS depuis
  cette fenêtre) : Vulkan ne voit plus aucune carte, la phrase est composée sur le processeur.
  Sous Windows, une variable vide équivaut à une variable absente : il faut un texte. Ou bien
  retirer `ggml-vulkan.dll` d'une copie du paquet.
- Diagnostic : `CASSINI_DEBUG_LLAMA=1` rend au noyau les journaux de llama.cpp sur sa sortie
  d'erreur (moteurs chargés, périphérique choisi, couches posées sur la carte), sans rien changer
  d'autre.
