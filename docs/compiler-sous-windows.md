# Compiler et lancer Cassini sous Windows

Cette page est une marche à suivre. Elle ne suppose aucune connaissance du code.
Chaque étape dit **quoi taper**, **ce que tu dois voir**, et **quoi faire si ce
n'est pas ça**. Suis-les dans l'ordre : chacune vérifie avant d'installer.

Tout se tape dans **PowerShell** (menu Démarrer → « PowerShell »).

**Commence par dire à PowerShell où est le projet.** Remplace le chemin ci-dessous
par le tien, puis garde la même fenêtre PowerShell ouverte jusqu'à la fin — toutes
les commandes qui suivent s'en servent. Les guillemets ne sont pas décoratifs : si
ton chemin contient une espace, les commandes échouent sans eux.

```powershell
$projet = "C:\chemin\vers\CASSINI"
```

> **Ce que cette page prouve, et ce qu'elle ne prouve pas.** L'étape 5 lance
> l'application en mode développement : l'interface est alors servie par un petit
> serveur local, donc débrancher le réseau ne prouve rien. C'est l'étape 6, avec
> l'exécutable construit, qui constitue la vraie preuve du hors ligne. Les deux
> sont demandées, dans cet ordre, parce que l'étape 5 donne le retour le plus
> rapide si quelque chose cloche.

---

## Étape 1 — Voir ce qui est déjà installé

Quatre choses sont nécessaires. Cette étape dit laquelle manque, **y compris la
plus lourde** — le compilateur C++, plusieurs gigaoctets.

**Rust et Node :**

```powershell
rustc -V ; cargo -V ; node -v ; npm -v ; rustup show
```

Attendu : quatre numéros de version, puis un bloc où la ligne `default host` se
termine par **`x86_64-pc-windows-msvc`**. Si elle se termine par `gnu`, arrête-toi
et dis-le-moi : il faut changer de chaîne d'outils.

**Le compilateur C++ :**

```powershell
& "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
```

Attendu : un chemin de dossier. Si la commande répond qu'elle ne trouve pas le
fichier, ou n'affiche rien du tout, le compilateur C++ manque.

**Le moteur d'affichage WebView2 :**

```powershell
(Get-ItemProperty 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue).pv
(Get-ItemProperty 'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}' -ErrorAction SilentlyContinue).pv
```

Attendu : **au moins une** des deux lignes affiche un numéro de version. Si les
deux restent muettes, WebView2 manque.

> Windows 11 l'embarque toujours. Sous Windows 10 il est *souvent* présent, parce
> qu'il arrive avec Edge — mais ce n'est pas garanti, et comme cette marche à
> suivre ne fabrique pas d'installeur, rien ne le réparera à ta place. D'où la
> vérification plutôt que la supposition.

Et pour savoir quel Windows tu as : `winver`.

---

## Étape 2 — Installer ce qui manque

N'installe que ce que l'étape 1 a signalé absent, puis **ferme et réouvre
PowerShell** — sinon les nouvelles commandes restent invisibles. Pense à refaire
`$projet = "…"` dans la nouvelle fenêtre. Enfin, refais l'étape 1 : tout doit
répondre.

| Ce qui manque | Quoi faire |
|---|---|
| **Le compilateur C++** (nécessaire même pour du Rust) | Télécharge **Microsoft C++ Build Tools** sur <https://visualstudio.microsoft.com/visual-cpp-build-tools/>, lance l'installeur, et **coche la case « Développement Desktop en C++ »** (« Desktop development with C++ »). C'est la seule case nécessaire. Compte plusieurs gigaoctets. |
| **Rust** | Dans PowerShell : `winget install --id Rustlang.Rustup`. Sans `winget`, prends `rustup-init.exe` sur <https://www.rust-lang.org/tools/install>. |
| **Node.js** | Version **LTS** sur <https://nodejs.org>. |
| **WebView2** | Section « Download » de <https://developer.microsoft.com/en-us/microsoft-edge/webview2/>, prends l'**Evergreen Bootstrapper**. |

---

## Étape 3 — Nettoyer ce qui vient de Linux

Tout le développement s'est fait jusqu'ici dans WSL, c'est-à-dire sous Linux, sur
ce même dossier. Deux dossiers contiennent donc des programmes compilés pour
Linux, inutilisables sous Windows. **Les deux sont régénérables et ignorés par
git : il n'y a rien d'important dedans, rien ne sera perdu.**

```powershell
Remove-Item -Recurse -Force "$projet\app\node_modules"
Remove-Item -Recurse -Force "$projet\target"
cd "$projet\app"
npm install
```

Attendu : quelques dizaines de secondes de téléchargement, puis une ligne du genre
`added NNN packages`. Le fichier `package-lock.json` n'est pas modifié : il
contient déjà les variantes Windows.

> **Les deux systèmes se gênent, et ça recommencera.** `cargo` ne sépare pas ses
> objets compilés par système : Linux et Windows écrivent dans le même dossier
> `target`. La première compilation Windows sera longue — dix à vingt minutes — et
> **chaque fois que je relancerai les tests depuis WSL, ce dossier se remplira à
> nouveau d'objets Linux**, donc ta compilation Windows suivante sera longue elle
> aussi. Symétriquement, le `npm install` que tu viens de faire a remplacé les
> binaires Linux : de mon côté, `npm run build` et `npm run tauri` ne marcheront
> plus jusqu'à une réinstallation, même si `npm run check` continue de passer.
> Ce n'est pas une panne, c'est le prix d'un dossier partagé entre deux systèmes.
> Si ça devient pénible, dis-le-moi : on peut séparer les dossiers de compilation.

---

## Étape 4 — Compiler sans lancer

C'est l'étape qui attrape la plupart des problèmes, et la plus rapide à relancer.

```powershell
cd "$projet"
cargo check -p cassini-app
```

Attendu : beaucoup de lignes `Compiling …`, puis `Finished`. Des lignes `warning`
sont acceptables ; seul `error` compte.

**Ce fichier n'a jamais été compilé nulle part.** Il a été relu ligne à ligne
contre le code source de Tauri, mais c'est la première fois qu'un compilateur le
juge. Si une erreur apparaît ici, c'est normal et attendu : copie-la-moi entière.

---

## Étape 5 — Lancer en mode développement

```powershell
cd "$projet\app"
npm run tauri dev
```

Attendu : une fenêtre « Cassini » s'ouvre, et dedans :

1. un **damier gris** qui couvre la zone de Villeneuve-le-Roi ;
2. un rectangle bleu translucide par-dessus — le repère ;
3. en bas, la ligne d'état qui indique **« Carte prête — hors ligne »** ;
4. les boutons de zoom en haut à droite, l'échelle en bas à droite.

Lis d'abord la ligne d'état, **puis** zoome et déplace la carte : le damier doit
suivre, et la ligne d'état se remplacer par des coordonnées. C'est voulu — elle
sert aussi d'affichage de position. Hors de la zone couverte, c'est normal qu'il
n'y ait rien : le fond de carte de recette ne couvre qu'une petite emprise, aux
niveaux de zoom 10 à 14.

**La ligne d'état est un diagnostic.** Si elle affiche **« Fond de carte
indisponible : … »**, le reste du message nomme exactement le problème et les
emplacements cherchés. Recopie-la-moi telle quelle.

Si Windows demande d'autoriser `node.exe` sur le réseau, tu peux refuser : tout se
passe en local.

Pour arrêter : `Ctrl + C` dans PowerShell, ou ferme la fenêtre.

---

## Étape 6 — Construire l'exécutable et prouver le hors ligne

```powershell
cd "$projet\app"
npm run tauri build -- --no-bundle
```

`--no-bundle` produit l'application sans fabriquer d'installeur. C'est voulu :
construire un installeur téléchargerait deux outils tiers depuis Internet, et
cette entrée ne demande pas de paquet distribuable.

Attendu : une compilation plus longue que l'étape 4, puis `Finished`. L'application
est ici :

```powershell
Get-ChildItem "$projet\target\release\*.exe"
Get-ChildItem "$projet\target\release\fixtures"
```

Le fichier s'appelle **`Cassini.exe`** : vérifié sur un poste Windows le
2026-10-10, le renommage s'applique bien même en sautant l'installeur. En mode
développement (étape 5), en revanche, la console affiche `cassini-app.exe` — le
nom technique du paquet, c'est normal. La seconde commande doit montrer
`basemap-villeneuve-le-roi.pmtiles` : c'est le fond de carte, recopié
automatiquement à côté de l'exécutable.

**La vraie preuve, maintenant :**

1. **Coupe le réseau** — débranche le câble Ethernet, et désactive le Wi-Fi.
2. Double-clique sur l'exécutable.
3. La fenêtre doit s'ouvrir, afficher le damier, et la ligne d'état dire
   **« Carte prête — hors ligne »**.
4. **Ensuite** seulement, zoome et déplace : l'image doit suivre, et la ligne
   d'état passer aux coordonnées.

Si ça marche réseau coupé, l'exigence E3 de l'epic est tenue et l'entrée est
terminée.

---

## Ce que j'ai besoin de savoir en retour

Peu importe que ça marche ou non, dis-moi :

- **à quelle étape tu t'es arrêté**, et le texte **entier** de l'erreur s'il y en a
  une — pas un résumé, le texte brut : la ligne qui compte est souvent la
  première, pas la dernière ;
- **ce que dit la ligne d'état**, mot pour mot, **avant** d'avoir touché à la
  carte ;
- si la carte s'affiche : **est-ce que le zoom et le déplacement suivent**, ou
  est-ce que l'image reste figée ;
- le **nom exact** du fichier trouvé à l'étape 6 ;
- une **capture d'écran** si quelque chose s'affiche de travers.

---

## Pannes connues

| Message | Cause | Remède |
|---|---|---|
| `linker 'link.exe' not found` | La case « Développement Desktop en C++ » n'a pas été cochée | Relance l'installeur des Build Tools et coche-la |
| `icons\icon.ico not found` | Ne devrait plus arriver : le fichier est désormais dans le dépôt | Dis-le-moi. Ne lance pas `tauri icon` directement sur le dossier `icons` : il écraserait les icônes versionnées. Le fichier a été produit dans un dossier temporaire, puis seul `icon.ico` a été recopié |
| une erreur citant `esbuild`, `rollup` ou une plateforme | `node_modules` vient encore de Linux | Refais l'étape 3 en entier |
| la fenêtre s'ouvre **blanche** | WebView2 absent | Refais la vérification WebView2 de l'étape 1, puis installe l'Evergreen Bootstrapper |
| `failed to run light.exe` | Tu as construit un installeur | Garde `--no-bundle`, étape 6 |
| la carte est **vide**, et la ligne d'état dit « Carte prête » **avant** tout déplacement | Le fond de carte n'est pas là où l'application le cherche | Lance la seconde commande de l'étape 6 et dis-moi ce qu'elle affiche |

Si la compilation est anormalement lente, c'est souvent l'antivirus qui inspecte
chaque fichier produit. Exclure le dossier `target` de l'analyse en temps réel
change tout — à toi de juger si tu le veux.
