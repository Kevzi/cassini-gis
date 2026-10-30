# Mesure de la durée par phrase sur processeur (ticket 11)

Point de départ chiffré de l'exigence « une longue phrase résolue en moins de 30 s sur un
poste sans carte graphique ». Mesure du **binaire réel** : chaque phrase passe par
`compose_plan`, comme dans le produit (modèle rechargé à chaque appel, redemande comprise,
aucun cache). Relevé du 2026-10-17 sur un Ryzen 7 5800X (8 cœurs, 16 fils), Windows,
modèle `Qwen3.5-9B-Q4_K_M.gguf`, llama.cpp compilé **sans CUDA** (AVX2, `--release`).

C'est une mesure de référence : aucune durée n'est vérifiée par un test. La justesse des
plans reste portée par `plan_reel`, sur GPU.

## Mode opératoire

L'outil est l'essai `ai/tests/duree_reel.rs`, ignoré par défaut. Variables :

- `CASSINI_TEST_MODEL` : chemin du modèle (sans elle, l'essai ne fait rien) ;
- `CASSINI_TEST_THREADS` : nombre de fils du moteur, lecture et écriture (sans elle : défaut de llama.cpp) ;
- `CASSINI_TEST_INVENTORY` : un inventaire JSON ; l'essai passe alors les phrases A à E du
  scénario de démo au lieu des 25 phrases de l'épreuve.

Dans PowerShell (voir `modele-local-sous-windows.md` pour `LIBCLANG_PATH`) :

```powershell
$env:CARGO_TARGET_DIR = "C:\ct"
cargo test --release -p cassini-ai --test duree_reel --no-run   # jamais --features cuda
$exe = Get-ChildItem C:\ct\release\deps\duree_reel-*.exe | Sort-Object LastWriteTime -Descending | Select-Object -First 1
$env:CASSINI_TEST_MODEL = "$PWD\ai\models\Qwen3.5-9B-Q4_K_M.gguf"
$env:CASSINI_TEST_THREADS = "8"
$p = Start-Process $exe.FullName -ArgumentList "--ignored","--nocapture","duree_reel" `
       -NoNewWindow -PassThru -RedirectStandardOutput mesure-8.txt
$p.ProcessorAffinity = [IntPtr]0x5555   # 8 cœurs physiques ; 0x55 = 4 cœurs physiques
$p.WaitForExit()
```

Sur le 5800X, les deux fils d'un même cœur portent des numéros voisins (0-1, 2-3…) : le
masque `0x5555` donne un fil par cœur sur les 8 cœurs, `0x55` un fil par cœur sur 4 cœurs.
Le tableau s'imprime en Markdown, suivi d'une ligne de moyennes. Une passe des 25 phrases
dure 48 min à 8 fils, 73 min à 4 fils.

Colonnes : durées en secondes, temps réel. **Chargement** = lecture du fichier du modèle,
création du contexte et découpage de l'invite ; **lecture du prompt** = décodage de
l'invite d'un bloc ; **écriture** = réponse jeton par jeton, grammaire comprise. Une phrase
qui demande une redemande additionne ses deux appels (colonne `appels`). Un appel qui
échoue (par exemple `PromptTooLong` à la redemande, que `compose_plan` absorbe alors que le
modèle est déjà chargé) n'est ni compté ni chronométré. Le chargement compte aussi le
démarrage du moteur llama.cpp (une fois, à la première phrase) et l'attente de son verrou,
et la libération du modèle en fin d'appel n'entre dans aucune colonne : le total est donc un
peu inférieur à la durée complète de l'appel.

## 8 fils, 8 cœurs physiques (`0x5555`)

| id | appels | chargement (s) | lecture du prompt (s) | jetons lus | écriture (s) | jetons écrits | total (s) | issue |
|---|---|---|---|---|---|---|---|---|
| S01 | 1 | 4,1 | 91,7 | 5127 | 5,3 | 23 | 101,1 | plan, 1 étape(s) |
| S02 | 1 | 3,9 | 93,1 | 5129 | 13,7 | 57 | 110,7 | plan, 1 étape(s) |
| S03 | 1 | 4,0 | 93,9 | 5131 | 3,6 | 17 | 101,5 | plan, 1 étape(s) |
| S04 | 1 | 3,9 | 94,7 | 5129 | 7,5 | 24 | 106,1 | refus |
| S05 | 1 | 4,2 | 100,5 | 5130 | 9,5 | 29 | 114,2 | refus |
| S06 | 1 | 4,1 | 106,7 | 5131 | 20,2 | 72 | 131,0 | plan, 1 étape(s) |
| S07 | 1 | 4,1 | 105,9 | 5137 | 17,1 | 69 | 127,2 | plan, 1 étape(s) |
| S08 | 1 | 4,1 | 99,8 | 5141 | 17,7 | 73 | 121,7 | plan, 1 étape(s) |
| S09 | 1 | 4,0 | 97,4 | 5129 | 7,2 | 28 | 108,6 | refus |
| S10 | 1 | 4,0 | 96,0 | 5128 | 8,2 | 33 | 108,2 | refus |
| S11 | 1 | 3,9 | 103,3 | 5133 | 18,3 | 74 | 125,5 | plan, 1 étape(s) |
| S12 | 1 | 3,8 | 94,6 | 5127 | 17,8 | 74 | 116,2 | plan, 1 étape(s) |
| S13 | 1 | 4,0 | 92,2 | 5133 | 25,7 | 104 | 121,9 | plan, 1 étape(s) |
| S14 | 1 | 4,0 | 90,4 | 5152 | 24,7 | 105 | 119,1 | plan, 2 étape(s) |
| S15 | 1 | 4,0 | 90,3 | 5133 | 12,4 | 52 | 106,7 | refus |
| S16 | 1 | 4,0 | 95,7 | 5169 | 23,7 | 102 | 123,4 | plan, 1 étape(s) |
| S17 | 1 | 3,9 | 95,1 | 5159 | 58,8 | 247 | 157,7 | plan, 5 étape(s) |
| S18 | 1 | 3,9 | 92,8 | 5143 | 10,2 | 43 | 107,0 | refus |
| S19 | 1 | 3,9 | 94,0 | 5138 | 40,4 | 164 | 138,2 | plan, 3 étape(s) |
| S20 | 1 | 3,9 | 94,3 | 5172 | 6,4 | 27 | 104,6 | question |
| S21 | 1 | 4,0 | 93,0 | 5179 | 10,0 | 42 | 106,9 | question |
| S22 | 1 | 4,0 | 89,3 | 5130 | 5,5 | 24 | 98,8 | refus |
| S23 | 1 | 3,9 | 89,5 | 5132 | 8,7 | 36 | 102,0 | refus |
| S24 | 1 | 3,9 | 90,5 | 5175 | 10,0 | 42 | 104,5 | refus |
| S25 | 1 | 3,9 | 92,2 | 5164 | 9,2 | 35 | 105,3 | refus |

Moyennes : chargement 4,0 s, lecture du prompt 95,1 s, écriture 15,7 s, **total 114,7 s**.

## 4 fils, 4 cœurs physiques (`0x55`)

| id | appels | chargement (s) | lecture du prompt (s) | jetons lus | écriture (s) | jetons écrits | total (s) | issue |
|---|---|---|---|---|---|---|---|---|
| S01 | 1 | 4,0 | 159,8 | 5127 | 5,1 | 23 | 168,9 | plan, 1 étape(s) |
| S02 | 1 | 4,4 | 156,1 | 5129 | 13,3 | 57 | 173,8 | plan, 1 étape(s) |
| S03 | 1 | 4,0 | 155,8 | 5131 | 3,5 | 17 | 163,3 | plan, 1 étape(s) |
| S04 | 1 | 4,0 | 155,2 | 5129 | 5,5 | 24 | 164,7 | refus |
| S05 | 1 | 4,0 | 155,3 | 5130 | 6,8 | 29 | 166,1 | refus |
| S06 | 1 | 3,9 | 154,6 | 5131 | 17,1 | 72 | 175,5 | plan, 1 étape(s) |
| S07 | 1 | 4,0 | 155,1 | 5137 | 16,6 | 69 | 175,7 | plan, 1 étape(s) |
| S08 | 1 | 4,0 | 155,6 | 5141 | 17,1 | 73 | 176,7 | plan, 1 étape(s) |
| S09 | 1 | 3,9 | 154,6 | 5129 | 6,4 | 28 | 165,0 | refus |
| S10 | 1 | 3,9 | 154,2 | 5128 | 7,6 | 33 | 165,7 | refus |
| S11 | 1 | 3,9 | 154,5 | 5133 | 17,2 | 74 | 175,6 | plan, 1 étape(s) |
| S12 | 1 | 4,0 | 154,2 | 5127 | 17,3 | 74 | 175,5 | plan, 1 étape(s) |
| S13 | 1 | 3,9 | 154,6 | 5133 | 24,6 | 104 | 183,1 | plan, 1 étape(s) |
| S14 | 1 | 3,9 | 155,1 | 5152 | 24,5 | 105 | 183,5 | plan, 2 étape(s) |
| S15 | 1 | 3,9 | 154,5 | 5133 | 9,4 | 40 | 167,8 | refus |
| S16 | 1 | 3,8 | 155,8 | 5169 | 23,6 | 102 | 183,2 | plan, 1 étape(s) |
| S17 | 1 | 4,0 | 155,8 | 5159 | 58,0 | 247 | 217,8 | plan, 5 étape(s) |
| S18 | 1 | 4,0 | 155,0 | 5143 | 6,7 | 29 | 165,7 | refus |
| S19 | 1 | 3,9 | 154,7 | 5138 | 38,8 | 164 | 197,4 | plan, 3 étape(s) |
| S20 | 1 | 4,0 | 155,8 | 5172 | 6,3 | 27 | 166,0 | question |
| S21 | 1 | 4,0 | 155,9 | 5179 | 8,6 | 37 | 168,5 | refus |
| S22 | 1 | 4,0 | 156,6 | 5130 | 5,5 | 24 | 166,0 | refus |
| S23 | 1 | 4,0 | 153,8 | 5132 | 6,5 | 29 | 164,3 | refus |
| S24 | 1 | 3,9 | 155,5 | 5175 | 9,7 | 42 | 169,1 | refus |
| S25 | 1 | 4,0 | 155,0 | 5164 | 8,2 | 35 | 167,2 | refus |

Moyennes : chargement 4,0 s, lecture du prompt 155,3 s, écriture 14,6 s, **total 173,8 s**.

## Lecture

- **On est loin des 30 s** : environ 4 fois au-dessus à 8 fils (115 s), 6 fois à 4 fils
  (174 s). Aucune phrase ne passe sous 98 s.
- **La lecture du prompt fait l'essentiel** : 83 % du total à 8 fils, 89 % à 4 fils. Elle
  est presque la même pour toutes les phrases (5 130 à 5 180 jetons, dont la phrase ne
  représente que quelques dizaines) : c'est le catalogue des commandes et l'inventaire qui
  coûtent, relus à chaque appel. À elle seule, elle dépasse trois fois le budget de 30 s.
  Débit : environ 54 jetons/s à 8 fils, 33 jetons/s à 4 fils.
- **L'écriture ne dépend presque pas du nombre de fils** (15,7 s contre 14,6 s, environ
  4 jetons/s dans les deux cas) : elle est bornée par la mémoire, pas par le calcul. Elle
  croît avec la longueur du plan : 58 s pour les 247 jetons de S17 (cinq étapes).
- **Le chargement est stable**, 4 s par appel, payé à chaque phrase puisque le modèle
  n'est pas gardé.
- **Aucune redemande** dans ces passes : 25 appels pour 25 phrases à chaque réglage.
- Les issues ne sont pas tout à fait identiques d'un réglage à l'autre (S21 : question à
  8 fils, refus à 4 fils) : changer le nombre de fils change l'ordre des sommes en virgule
  flottante. Ce n'est pas une mesure de justesse.

**Constat sur les fils par défaut.** Le produit ne règle pas le nombre de fils : llama.cpp
prend alors `GGML_DEFAULT_N_THREADS`, qui vaut **4**, quelle que soit la machine. Le
produit tourne donc aujourd'hui à peu près comme la colonne « 4 fils », sans l'épinglage
sur 4 cœurs physiques : passer au nombre de cœurs physiques gagnerait, estimation, environ
un tiers sur un poste qui en a 8. Ce réglage n'est pas changé
ici ; il est reporté au ticket 5.12, avec le chargement unique du modèle et la mise en
cache de la partie fixe de l'invite, qui visent directement les deux postes les plus
lourds.

**Comparaison avec la mesure 8.6** (`tests/epreuve-modele-local/resultats-8-6-cpu.json`,
8 fils, `llama-completion.exe`) : chargement 1,9 s, prompt 6,0 s pour 4 873 jetons,
écriture 18,6 s pour 105 jetons. Ce n'était **pas une mesure CPU pure** : ce llama.cpp
était compilé avec CUDA et, même sans couche déportée (`ngl 0`), il confie au GPU la
lecture des grands lots (pic de mémoire vidéo de 1,5 Go dans le relevé). Son chiffre de
lecture du prompt n'est donc pas comparable à celui-ci. Seule son écriture, faite sur le
processeur, l'est à peu près (5,6 jetons/s contre environ 4 ici, avec la grammaire en plus).

**Limite du substitut.** Brider à 4 fils sur 4 cœurs physiques d'un 5800X ne reproduit ni
la mémoire ni la fréquence d'un portable : c'est un substitut, sans doute optimiste pour
l'écriture, qui dépend surtout de la bande passante mémoire.

Le relevé des phrases A à E du scénario Villeneuve-le-Roi, sur l'inventaire du projet de
test (hors dépôt), est noté dans le plan du ticket 11.

## Après le cache (ticket 12)

Depuis le ticket 12, le modèle est **chargé une fois** et gardé dans le processus du noyau
(il n'est rechargé que si le fichier change : chemin, taille ou date), et l'état du moteur
est gardé en mémoire à deux points de l'invite : après la **partie fixe** (consignes,
catalogue, exemples — l'inventaire vient désormais après eux) et après le **tour système**
(partie fixe + inventaire). Une demande reprend le plus profond de ces instantanés dont
les jetons sont un début des siens et ne lit que la suite. Rien n'est écrit sur disque, et
rien d'une demande n'est gardé pour la suivante. Le nombre de fils par défaut suit
désormais le nombre de cœurs physiques de la machine (crate `num_cpus`).

Relevé du 2026-10-18, même poste, même modèle, même binaire sans CUDA, 4 fils sur 4 cœurs
physiques (`0x55`), les 25 phrases dans **un même processus**. La colonne « jetons
repris » compte les jetons de l'invite repris d'un instantané, donc non relus ; la
« lecture du prompt » comprend la reprise de l'instantané et la prise des nouveaux.

| id | appels | chargement (s) | lecture du prompt (s) | jetons de l'invite | jetons repris | écriture (s) | jetons écrits | total (s) | issue |
|---|---|---|---|---|---|---|---|---|---|
| S01 | 1 | 4,1 | 163,7 | 5128 | 0 | 5,6 | 23 | 173,4 | plan, 1 étape(s) |
| S02 | 1 | 0,1 | 1,3 | 5130 | 5109 | 14,2 | 57 | 15,5 | plan, 1 étape(s) |
| S03 | 1 | 0,1 | 1,3 | 5132 | 5109 | 3,8 | 17 | 5,1 | plan, 1 étape(s) |
| S04 | 1 | 0,1 | 1,1 | 5130 | 5109 | 5,9 | 24 | 7,1 | refus |
| S05 | 1 | 0,0 | 1,2 | 5131 | 5109 | 15,1 | 61 | 16,3 | refus |
| S06 | 1 | 0,1 | 1,2 | 5132 | 5109 | 17,4 | 72 | 18,7 | plan, 1 étape(s) |
| S07 | 1 | 0,1 | 1,3 | 5138 | 5109 | 16,8 | 69 | 18,2 | plan, 1 étape(s) |
| S08 | 1 | 0,1 | 1,5 | 5142 | 5109 | 17,9 | 73 | 19,5 | plan, 1 étape(s) |
| S09 | 1 | 0,1 | 1,1 | 5130 | 5109 | 7,5 | 31 | 8,6 | refus |
| S10 | 1 | 0,1 | 1,0 | 5129 | 5109 | 5,1 | 23 | 6,2 | plan, 1 étape(s) |
| S11 | 1 | 0,1 | 1,2 | 5134 | 5109 | 17,9 | 74 | 19,2 | plan, 1 étape(s) |
| S12 | 1 | 0,1 | 1,0 | 5128 | 5109 | 18,0 | 74 | 19,1 | plan, 1 étape(s) |
| S13 | 1 | 0,1 | 1,2 | 5134 | 5109 | 25,9 | 104 | 27,2 | plan, 1 étape(s) |
| S14 | 1 | 0,1 | 1,9 | 5153 | 5109 | 25,6 | 105 | 27,6 | plan, 2 étape(s) |
| S15 | 1 | 0,1 | 1,2 | 5134 | 5109 | 10,9 | 45 | 12,2 | refus |
| S16 | 1 | 0,1 | 2,5 | 5170 | 5109 | 23,8 | 102 | 26,4 | plan, 1 étape(s) |
| S17 | 1 | 0,1 | 2,2 | 5160 | 5109 | 59,1 | 247 | 61,4 | plan, 5 étape(s) |
| S18 | 1 | 0,1 | 1,6 | 5144 | 5109 | 6,8 | 29 | 8,5 | refus |
| S19 | 1 | 0,1 | 1,4 | 5139 | 5109 | 39,4 | 164 | 40,8 | plan, 3 étape(s) |
| S20 | 1 | 0,1 | 2,1 | 5173 | 5109 | 7,0 | 30 | 9,2 | refus |
| S21 | 1 | 0,1 | 2,6 | 5180 | 5109 | 8,3 | 35 | 11,0 | refus |
| S22 | 1 | 0,1 | 1,1 | 5131 | 5109 | 5,8 | 25 | 7,0 | refus |
| S23 | 1 | 0,1 | 1,2 | 5133 | 5109 | 8,6 | 36 | 9,8 | refus |
| S24 | 1 | 0,1 | 2,5 | 5176 | 5109 | 9,6 | 40 | 12,1 | refus |
| S25 | 1 | 0,1 | 2,4 | 5165 | 5109 | 8,6 | 34 | 11,0 | refus |

**Premier appel, à part.** S01 paie seul le chargement (4,1 s) et la lecture complète de
l'invite (163,7 s, contre 159,8 s au ticket 11 : la lecture est découpée en trois segments
et deux instantanés d'environ 150 Mo sont pris au passage). Ce coût n'est payé qu'une fois
par session dans un processus qui dure, `cassini serve` (le canal de l'extension). La
commande ponctuelle `cassini plan` démarre un processus par demande : elle paie le
chargement et la lecture complète à chaque appel, et ne gagne que sur sa redemande, qui
reprend l'instantané du tour système.

**Appels suivants (S02 à S25).** Moyennes : chargement 0,1 s, lecture du prompt 1,5 s,
écriture 15,8 s, **total 17,4 s** (173,8 s au ticket 11). 5 109 jetons sur 5 130 environ
sont repris : l'inventaire étant le même d'une phrase à l'autre, il ne reste à lire que la
phrase et l'ouverture du tour assistant. 22 des 24 phrases passent sous 30 s. Quand
l'inventaire change (une couche ouverte ou fermée), seule la partie fixe est reprise : sur
l'inventaire de l'épreuve, environ 936 jetons (5 109 − 4 173, relevé de `cache_reel`) sont
relus, soit environ 28 s au débit de lecture à 4 fils (environ 33 jetons/s). Estimation,
non mesurée sur CPU.

**Verdict S17 : 61,4 s, au-dessus des 30 s.** La lecture de son invite ne coûte plus que
2,2 s ; il reste l'écriture des 247 jetons de son plan à cinq étapes, 59,1 s à environ
4 jetons/s. Ce débit est borné par la mémoire, pas par le nombre de fils, et le cache n'y
peut rien : il gagne la lecture, pas l'écriture. S19 (164 jetons écrits, 40,8 s) est dans
le même cas. L'écart est porté par l'entrée 10 ; c'est la longueur du plan écrit qu'il
faut réduire, ou le débit d'écriture qu'il faut augmenter.

**Justesse.** Sur GPU (`--features cuda`), `plan_reel` donne 23/25 plans conformes avec
l'ordre nouveau du message système (échecs S10 et S21). `cache_reel` vérifie sur le vrai
modèle qu'à invite égale la sortie est identique avec ou sans instantané, que le 2ᵉ appel
reprend 5 109 jetons sans recharger le modèle, et qu'un inventaire changé ne reprend que
la partie fixe (4 173 jetons). Sur CPU, S05 et S10 n'ont pas la même issue qu'au ticket 11
(S10 : un plan au lieu d'un refus) : l'invite a changé d'ordre (l'inventaire vient après
les exemples) et la partie fixe est désormais découpée en jetons à part de l'inventaire, ce
qui peut changer les jetons à la jonction. Ce n'est pas une mesure de justesse.

## Modèle ajusté (ticket 5.10, tâche 4b)

`CASSINI_TEST_MODEL` prend aussi une déclaration `*.modele.json` (base + adaptateur ou modèle
fusionné, prompt court). Avec `CASSINI_TEST_INVENTORY`, le scénario commence par `A0`, la phrase
de démo exacte de `CLAUDE.md`, précédée d'une amorce exclue des moyennes. Relevé du 2026-10-23
(4 fils, `0x55`) : A0 en 41,4 s avec le modèle fusionné, invite de 1 737 jetons ; détail dans
`entrainement-du-modele.md`, « Tâche 4 : mesures ».

## Session enchaînée dans QGIS (ticket 5.19)

Les mesures ci-dessus passent les phrases une à une sur un inventaire fixe. Dans QGIS, chaque
plan exécuté ajoute ses couches de résultat à l'inventaire, qui change donc d'une phrase à
l'autre : avant 5.19, il était relu en entier à chaque phrase (les nouveaux résultats arrivent en
tête, ordre de QGIS), seules les consignes étaient reprises.

**Mode opératoire.** Sonde jetable hors dépôt, sur le modèle de `qgis-plugin/tests/probe_demo.py` :
dix phrases enchaînées dans une même session, chaque plan exécuté, QGIS 3.44.15,
`projet-test-villeneuve.qgz`, modèle livré (`cassini-composition-4b.modele.json`), vrai noyau
construit **sans CUDA**. La sonde se borne à 4 cœurs physiques du 5800X (`0x55`, hérité par le
noyau) et lance le noyau avec deux variables de mesure, sans effet sans elles :
`CASSINI_THREADS=4` (sous Windows, le nombre de fils par défaut ne voit pas le masque
d'affinité : un noyau bridé à 4 cœurs en prendrait 8) et `CASSINI_TIMINGS=1` (une ligne
`cassini-timings` par appel au modèle sur la sortie d'erreur du noyau, sans rien de la demande).
La première phrase d'une session (chargement, première lecture) est hors critère, comme en 5.12.

Phrases : 1. démo (`CLAUDE.md`) ; 2. « Garde les zones U du PLU. » ; 3. « Trouve les parcelles
dans ces zones à moins de 300 m d'une gare. » ; 4. « Fais une zone tampon de 200 m autour des
gares. » ; 5. « Sélectionne les routes limitées à 30 km/h. » ; 6. « Trouve les bâtiments à moins
de 50 m de ces routes. » ; 7. « Calcule l'altitude moyenne du MNT dans chaque zone du PLU. » ;
8. « Garde les zones N du PLU. » ; 9. démo de nouveau ; 10. « Sors-moi la carte en PDF. »

**Avant** (2026-10-29, noyau de 4558c3f, plus la seule ligne de mesure) :

| n | issue | composition (s) | lecture (s) | jetons | repris | écriture (s) | écrits |
|---|---|---|---|---|---|---|---|
| 1 | plan | 76,4 | 30,9 | 1 778 | 0 | 43,0 | 248 |
| 2 | plan | 105,9 | 92,5 | 5 230 | 258 | 13,2 | 71 |
| 3 | plan | 141,7 | 109,1 | 6 332 | 258 | 32,3 | 180 |
| 4 | plan | 107,8 | 97,0 | 5 796 | 258 | 10,5 | 63 |
| 5 | plan | 112,8 | 100,8 | 5 956 | 258 | 11,6 | 69 |
| 6 | plan | 103,2 | 81,8 | 4 948 | 258 | 21,0 | 124 |
| 7 | refus | 97,7 | 91,2 | 5 460 | 258 | 6,0 | 36 |
| 8 | plan | 13,1 | 0,7 | 5 453 | 5 432 | 11,9 | 71 |
| 9 | question | 92,0 | 86,1 | 5 232 | 258 | 5,4 | 32 |
| 10 | plan | 4,5 | 0,7 | 5 204 | 5 183 | 3,4 | 23 |

Lecture d'environ 57 jetons/s, écriture d'environ 5,7 jetons/s. Seules les phrases qui suivent un
refus ou une question (8 et 10 : inventaire inchangé) reprennent le tour système ; toutes les
autres ne reprennent que les consignes (258 jetons) et relisent l'inventaire entier.

**Après** (même jour, même poste, noyau de 5.19) : ordre stable de l'inventaire (couches de
l'utilisateur, puis résultats du plus ancien au plus récent), lecture coupée à chaque frontière de
couches avec des instantanés à la fin des consignes, à l'ancrage, à la dernière frontière et à la
fin du tour système, identifiants courts `nom_<4 hex>` vus par le modèle. L'ancrage et la dernière
frontière sont des coupes juste avant la dernière clé (`source`) d'une couche : la dernière couche
de l'utilisateur pour l'ancrage, la dernière couche de l'inventaire pour la dernière frontière. Dans
les tableaux, « fin de l'utilisateur » désigne la reprise à l'ancrage.

| n | issue | composition (s) | lecture (s) | jetons | repris | écriture (s) | écrits | reprise |
|---|---|---|---|---|---|---|---|---|
| 1 | plan | 55,7 | 27,1 | 1 603 | 0 | 26,0 | 165 | — (1ʳᵉ phrase) |
| 2 | plan | 69,6 | 61,9 | 4 946 | 1 510 | 7,5 | 42 | fin de l'utilisateur |
| 3 | plan | 41,0 | 23,4 | 6 023 | 4 886 | 17,3 | 101 | dernière frontière |
| 4 | plan | 90,6 | 84,2 | 6 137 | 1 510 | 6,0 | 38 | fin de l'utilisateur (budget) |
| 5 | plan | 71,2 | 64,7 | 5 073 | 1 510 | 6,2 | 39 | fin de l'utilisateur (budget) |
| 6 | plan | 15,9 | 5,5 | 5 259 | 5 009 | 10,0 | 63 | dernière frontière |
| 7 | refus | 17,6 | 11,2 | 5 716 | 5 192 | 6,0 | 36 | dernière frontière |
| 8 | plan | 7,9 | 0,7 | 5 709 | 5 688 | 6,7 | 42 | tour système |
| 9 | plan | 99,4 | 72,2 | 5 483 | 1 510 | 26,7 | 165 | fin de l'utilisateur (budget) |
| 10 | plan | 86,1 | 82,0 | 6 014 | 1 510 | 3,4 | 23 | fin de l'utilisateur (budget) |

**Lecture.** Les identifiants courts réduisent l'écriture : la phrase de démo écrit 165 jetons au
lieu de 248 (26,0 s au lieu de 43,0 s), à plan identique. Quand une phrase ne fait qu'ajouter des
couches à la fin de l'inventaire, la reprise à la dernière frontière fonctionne : phrases 3, 6 et 7
en 41,0, 15,9 et 17,6 s (la phrase 3 relit encore la dernière couche d'avant et une intersection
nouvelle, 1 137 jetons). Deux cas restent au-dessus de 30 s, et ce sont eux qui portent l'écart :

- **le budget dépassé** (phrases 4, 5, 9, 10) : le budget de 5.18 retire les résultats les plus
  anciens, qui suivent maintenant les couches de l'utilisateur ; la reprise retombe à la fin des
  couches de l'utilisateur (1 510 jetons) et tous les résultats gardés sont relus (3 500 à
  4 600 jetons, 65 à 84 s) ;
- **beaucoup de résultats nouveaux d'un coup** (phrase 2) : le plan de démo produit quatre couches
  (sélection, tampon, deux intersections à beaucoup de champs), soit 3 436 jetons à lire une fois
  (62 s), sans aucun retrait.

Ces deux cas ont été soumis au propriétaire avec leurs chiffres (2026-10-29). Décisions : retrait
par paquets pour le budget ; la phrase qui suit un plan à plusieurs résultats part dans un ticket
suivant (`deferred-work.md`).

**Après le retrait par paquets** (même jour, même poste, noyau final de 5.19). Quand la vue dépasse
le budget de 14 000 caractères, les résultats les plus anciens sortent jusqu'à descendre sous
7 000 caractères (`INVENTORY_LOW_CHARS`) ; le noyau ne gardant rien d'une phrase à l'autre, la
décision est rejouée sur les résultats présents, du plus ancien au plus récent, si bien que les
résultats gardés ne changent pas jusqu'au dépassement suivant (`within_budget_by_batches`).

| n | issue | composition (s) | lecture (s) | jetons | repris | écriture (s) | écrits | reprise |
|---|---|---|---|---|---|---|---|---|
| 1 | plan | 58,7 | 28,8 | 1 603 | 0 | 27,0 | 165 | — (1ʳᵉ phrase) |
| 2 | plan | 72,5 | 64,7 | 4 942 | 1 510 | 7,5 | 42 | fin de l'utilisateur (4 résultats nouveaux) |
| 3 | plan | 36,6 | 24,6 | 6 020 | 4 882 | 11,7 | 69 | dernière frontière |
| 4 | plan | 15,0 | 8,6 | 1 946 | 1 510 | 6,1 | 38 | fin de l'utilisateur (paquet retiré) |
| 5 | plan | 10,4 | 4,1 | 2 078 | 1 878 | 6,0 | 39 | dernière frontière |
| 6 | plan | 15,5 | 4,9 | 2 262 | 2 014 | 10,4 | 63 | dernière frontière |
| 7 | refus | 16,6 | 10,2 | 2 718 | 2 195 | 6,1 | 36 | dernière frontière |
| 8 | plan | 7,7 | 0,6 | 2 711 | 2 690 | 6,8 | 42 | tour système |
| 9 | plan | 44,6 | 17,2 | 3 552 | 2 651 | 27,0 | 165 | dernière frontière |
| 10 | plan | 45,2 | 41,4 | 3 878 | 1 510 | 3,3 | 23 | fin de l'utilisateur (paquet retiré) |

**Verdict au 2026-10-29 : critère pas encore atteint pour chaque phrase.** Hors première phrase,
5 phrases sur 9 passent sous 30 s (avant 5.19 : 2 sur 9), et la durée totale des phrases 2 à 10
passe de 779 s à 264 s ; aucune `ai.prompt_too_long`. Restent au-dessus, consignés tels quels :

- phrase 2 (72,5 s) : elle lit une fois les quatre couches produites par le plan de démo
  (3 432 jetons) ; ticket suivant (couche finale seule vue par le modèle, ou champs allégés) ;
- phrase 3 (36,6 s) : même cause, en plus petit : la sélection des zones U de la phrase 2 porte
  tous les champs du PLU (environ 1 100 jetons à lire une fois) ;
- phrase 9 (44,6 s), la phrase de démo en cours de session : la lecture ne coûte plus que 17 s,
  mais l'écriture de son plan à cinq étapes reste à 27 s (165 jetons à environ 6 jetons/s). Son
  issue a changé : en 5.18 (et dans la mesure « avant »), elle rendait une question ; en 5.19, elle
  rend le plan à cinq étapes, exécuté jusqu'au PDF ;
- phrase 10 (45,2 s) : la phrase du retrait par paquets, qui relit les résultats gardés ; admise
  par la décision du propriétaire, les phrases suivantes reprenant à la dernière frontière.

La phrase de démo seule, en tête de session, est hors critère (chargement et première lecture) ;
sa composition passe de 76,4 à 58,7 s.

**Justesse sur GPU** (`--features cuda`, modèle livré, essais passés par les identifiants courts
comme dans le produit ; `CASSINI_TEST_FULL_IDS=1` pour les identifiants complets) :

| essai | 5.18 | 5.19, identifiants complets | 5.19, identifiants courts |
|---|---|---|---|
| `plan_reel` | 25 / 25 | 25 / 25 | 25 / 25 |
| `question_reel` | 12 / 12 | — | 12 / 12 |
| `ecart_reel` | 33 / 40 | 33 / 40 | 32 / 40 |

Les coupes aux frontières de couches ne changent rien (mêmes scores). Les identifiants courts
coûtent une phrase du jeu tenu à l'écart : H11 (isochrone de 15 minutes depuis un point, par la
couche de voirie) rend un refus au lieu du plan.

## Résultats allégés dans la vue du modèle (ticket 5.22)

Deux leviers, décidés par le propriétaire le 2026-10-29, ne changent que l'inventaire **vu par le
modèle** (`model_inventory`) ; QGIS, le journal, le plan gardé et l'extension gardent toutes les
couches et tous les champs :

- **couches intermédiaires cachées** : la sortie d'une étape qu'une étape **exécutée** du même plan
  consomme (`step:k`) sort de la vue. Le noyau retient ces identifiants en mémoire, le temps de son
  processus ; à la relance (QGIS rouvert), tout résultat est relu une fois. Une sortie reprise par
  une phrase suivante (« ces zones ») reste vue, et une couche de l'utilisateur n'est jamais cachée ;
- **résultats sans valeurs** : les champs des couches de résultat perdent leur clé `values` (forme
  apprise d'un champ texte trop varié) ; les couches de l'utilisateur gardent les leurs.

**Mesure** (2026-10-29, même poste et même mode opératoire qu'en 5.19 : 5800X bridé à `0x55`,
`CASSINI_THREADS=4`, noyau construit sans CUDA et installé dans `%LOCALAPPDATA%\Cassini\bin`, modèle
livré), les dix phrases de 5.19 puis une onzième, « Montre le tampon de 500 m. », qui cite le
tampon de la phrase de démo, caché au modèle :

| n | issue | composition 5.19 (s) | composition 5.22 (s) | lecture (s) | jetons | repris | écriture (s) | écrits |
|---|---|---|---|---|---|---|---|---|
| 1 | plan | 58,7 | 56,3 | 27,0 | 1 603 | 0 | 26,6 | 165 |
| 2 | plan | 72,5 | **15,1** | 7,9 | 1 970 | 1 510 | 6,9 | 42 |
| 3 | plan | 36,6 | **21,8** | 6,3 | 2 264 | 1 910 | 15,3 | 97 |
| 4 | plan | 15,0 | 14,3 | 8,2 | 2 654 | 2 192 | 5,8 | 38 |
| 5 | plan | 10,4 | 9,4 | 3,0 | 2 731 | 2 586 | 6,0 | 39 |
| 6 | plan | 15,5 | 14,3 | 4,3 | 2 887 | 2 667 | 9,6 | 63 |
| 7 | refus | 16,6 | 12,2 | 5,5 | 3 092 | 2 820 | 6,3 | 36 |
| 8 | plan | 7,7 | 8,8 | 0,7 | 3 085 | 3 064 | 7,6 | 42 |
| 9 | plan | 44,6 | 38,1 | 7,7 | 3 393 | 3 025 | 30,0 | 165 |
| 10 | plan | 45,2 | 13,5 | 9,2 | 3 760 | 3 304 | 3,6 | 23 |
| 11 | question | — | 6,6 | 0,7 | 3 763 | 3 739 | 5,3 | 28 |

**Lecture.** Les phrases 2 et 3 passent sous 30 s (72,5 → 15,1 s et 36,6 → 21,8 s) : après le plan
de démo, la phrase 2 ne lit plus que la dernière intersection, sans valeurs (460 jetons nouveaux au
lieu de 3 432) ; la phrase 3, la sélection des zones U sans valeurs (354 au lieu d'environ 1 100).
La vue reste sous le budget sur toute la session : aucun retrait par paquets, donc plus de phrase
qui relit tous les résultats gardés (la phrase 10 passe de 45,2 à 13,5 s). Aucune
`ai.prompt_too_long`. Sur les phrases 2 à 10 (même base qu'en 5.19), 8 phrases sur 9 passent sous
30 s (5.19 : 5 sur 9), et leur durée totale passe de 264 s à 148 s ; la onzième, nouvelle, passe
aussi (6,6 s).

Reste au-dessus, consigné tel quel : la **phrase 9** (38,1 s), la phrase de démo en cours de
session. Sa lecture ne coûte plus que 7,7 s ; c'est l'écriture de son plan à cinq étapes (165
jetons, 30 s) qui dépasse, hors de ce que l'inventaire peut alléger.

**Issues à surveiller.**

- **Phrase 3** (« Trouve les parcelles dans ces zones à moins de 300 m d'une gare ») : le plan prend
  la couche entière du PLU (`plu_zones_…`) pour « ces zones », et non la sélection des zones U de
  la phrase 2, pourtant vue. En 5.19, il prenait le tampon de 500 m de la phrase 1, qui est
  maintenant caché ; la sélection n'a plus ses valeurs (dont `typezone` = `U`). La cause n'est pas
  séparée entre les deux leviers (vue sans intermédiaires, résultats sans valeurs) : aucune mesure
  ne les isole. À traiter par l'entraînement (5.10), pas par une règle.
- **Phrase 11** (« Montre le tampon de 500 m. ») cite une couche cachée : le modèle rend une
  **question**, sans rien exécuter — mais une question hors de propos (« Depuis quel point faut-il
  partir : quelles sont ses coordonnées X et Y ? »). Aucun plan faux n'est exécuté ; la couche reste
  dans QGIS, où l'utilisateur la voit. Coût accepté par la décision du propriétaire.
- **Valeur retirée** : courte sonde séparée, même poste et même mode opératoire (démo, « Garde les
  zones U du PLU. », puis une phrase qui cite une valeur de la sélection). La valeur citée, `0300` du
  champ `formdomi`, n'était visible que par la sélection des zones U (dix valeurs, sous le plafond),
  donc retirée en 5.22 ; le PLU de l'utilisateur en a onze, au-delà du plafond, donc aucune.
  « Dans ces zones, garde celles dont le formdomi vaut 0300. » : durées 53,9, 14,5 et **10,8 s** ; la
  troisième rend une **question**, « Aucune couche de zonage du PLU dans le projet : laquelle faut-il
  utiliser ? », sans rien exécuter, alors que la sélection et le PLU sont vus. Le modèle ne relie
  plus la phrase à la sélection : l'utilisateur doit préciser la couche. Coût accepté par la
  décision du propriétaire ; la cause (valeur absente, ou phrase mal reliée) n'est pas séparée.
  Un premier essai citant le libellé `UCAAAAAA` (question « Quelle couche faut-il prendre ? »,
  8,7 s) ne compte pas : `libelle` a onze valeurs dans les deux couches, il n'en montrait déjà
  aucune avant 5.22.

**Justesse sur GPU** (`--features cuda`, modèle livré, identifiants courts) : `plan_reel` 25 / 25,
`question_reel` 12 / 12, `ecart_reel` 32 / 40 (plans 18 / 26, questions 8 / 8, refus 6 / 6) — les
scores de 5.19, aucun ne baisse. Ces essais composent sur des inventaires fixes, sans couche de
résultat : les deux leviers n'y changent rien, ils vérifient seulement que rien d'autre n'a bougé.
La justesse sur une session enchaînée reste portée par la recette ci-dessus.

## Fils par défaut : cœurs physiques autorisés (ticket 5.20)

Le nombre de fils par défaut du moteur (`default_threads`) compte désormais les **cœurs physiques
dont au moins un processeur logique est autorisé au processus** par son masque d'affinité, sous
Windows (`GetProcessAffinityMask`, `GetLogicalProcessorInformation`) et sous Linux
(`/proc/self/status`, un cœur par masque distinct de
`/sys/devices/system/cpu/cpuN/topology/thread_siblings_list`), sans dépasser
`available_parallelism` ; si la lecture échoue, ou au-delà de 64 processeurs logiques, il retombe
sur l'ancienne formule (cœurs physiques de la machine). `CASSINI_THREADS` garde la priorité. La
ligne `cassini-timings` donne maintenant les fils pris (`fils=`).

**Fils pris** (5800X, lus dans `cassini-timings` ou par un petit programme lancé avec
`start /affinity`) : sans masque **8** ; `0x55` **4** ; `0x3` **1** ; `0x7` **2**. Sous Linux
(WSL, `taskset`), mêmes chiffres.

**Correction des mesures 5.19 et 5.22.** Leur sonde posait le masque par `ctypes` sans déclarer
les types des arguments : le pseudo-handle de `GetCurrentProcess` était tronqué et
`SetProcessAffinityMask` échouait sans bruit. Le premier essai de 5.20, avec la même sonde, a pris
8 fils sous un `0x55` prétendu : ces deux tableaux ont donc été mesurés **sans masque, à 4 fils**,
et non sur 4 cœurs. La sonde de 5.20 déclare les types, vérifie le retour et affiche le masque
effectif (`0x55`).

**Mesure** (2026-10-30, mode opératoire de 5.22 : les onze phrases enchaînées, QGIS 3.44.15, modèle
livré, noyau construit sans CUDA ; noyau lancé depuis `C:\ct\release` copié dans la sonde, QGIS du
propriétaire ouvert, inactif, d'où une copie du projet de test pour son journal). Composition en
secondes ; la phrase 1 (chargement, première lecture) est hors critère :

| n | `0x55`, défaut (4 fils) | `0x55`, `CASSINI_THREADS=4` | sans masque, défaut (8 fils) | sans masque, 16 fils |
|---|---|---|---|---|
| 1 | 54,7 | 54,3 | 50,0 | 58,9 |
| 2 | 15,1 | 15,0 | 13,3 | 15,4 |
| 3 | 26,1 | 21,7 | 21,5 | 25,2 |
| 4 | 15,5 | 15,0 | 12,7 | 13,0 |
| 5 | 10,0 | 11,8 | 8,6 | 11,1 |
| 6 | 15,1 | 15,5 | 13,3 | 15,8 |
| 7 | 12,7 | 12,3 | 10,3 | 11,6 |
| 8 | 8,4 | 8,3 | 7,9 | 9,4 |
| 9 | 34,8 | 36,8 | **32,0** | 42,7 |
| 10 | 12,6 | 13,1 | 10,1 | 12,8 |
| 11 | 12,7 | 6,3 | 11,9 | 7,1 |
| total 2 à 10 | 150,3 | 149,5 | **129,7** | 157,0 |

**Lecture.**

- **Sous `0x55`**, le défaut prend 4 fils, comme `CASSINI_THREADS=4` : mêmes durées à l'écart de
  mesure près (150,3 contre 149,5 s sur les phrases 2 à 10). L'écart de la phrase 3 (26,1 contre
  21,7 s) tient à l'écriture (18,6 contre 15,1 s pour les mêmes 97 jetons), celui de la phrase 11 à
  l'issue : deux appels et un échec `layer.not_vector` dans un cas, une question dans l'autre —
  le modèle ne répond pas toujours pareil à cette phrase, ce n'est pas un effet du nombre de fils.
- **Sans masque, 8 fils (cœurs physiques) battent 16 (fils logiques)** : la lecture du prompt
  gagne un peu à 16 fils (phrase 1 : 17,8 contre 19,6 s), mais l'écriture jeton par jeton y perd
  nettement (phrase 9 : 37,4 contre 26,4 s), et le total des phrases 2 à 10 passe de 129,7 à
  157,0 s. Le défaut reste aux cœurs physiques ; aucune décision n'est à soumettre.
- Sur les phrases 2 à 10, 8 sur 9 passent sous 30 s dans les quatre réglages ; la phrase 9 (démo
  en cours de session) reste au-dessus, à 32,0 s au mieux (8 fils sans masque).

## Carte graphique dans le paquet livré (ticket 5.21)

Le paquet livré (`--features gpu`, voir `paquet-du-noyau.md`) charge ses moteurs de calcul depuis
le dossier de `cassini.exe` : le processeur toujours, Vulkan s'il se charge et trouve une carte
dédiée. Poste : RTX 3070 Ti 8 Go, pilote 616.92 ; modèle livré (`cassini-composition-4b.modele.json`).

**Taille du paquet** : 60,3 Mo (`cassini.exe` 1,8 Mo, `llama.dll`, `ggml.dll`, `ggml-base.dll`,
neuf variantes `ggml-cpu-*.dll` d'environ 1 Mo, `ggml-vulkan.dll` 43,1 Mo) ; 15,1 Mo sans le
moteur carte. L'ancien noyau processeur seul faisait 5,7 Mo d'un seul fichier.

**Vulkan contre CUDA** (2026-10-30, `duree_reel`, phrases A0 à E sur l'inventaire traduit, sans
masque ; A0 est la phrase de démo) :

| construction | amorce, lecture (s) | A0 (s) | A (s) | moyenne A0 à E (s) | écriture moyenne (s) |
|---|---|---|---|---|---|
| paquet, Vulkan | 18,3 | 13,9 | 13,9 | 7,1 | 6,9 |
| `--features cuda` (mesure, non livré) | 0,6 | 14,8 | 16,5 | 7,6 | 7,6 |
| paquet, carte masquée, `0x55` (4 fils) | 26,0 | 27,9 | 27,3 | 13,8 | 13,1 |
| noyau processeur seul de 5.20, `0x55` | 28,5 | 29,8 | 29,5 | 15,1 | 14,3 |

Vulkan n'est pas plus lent que CUDA : aucune décision à soumettre. La lecture de l'amorce (18,3 s
contre 0,6 s) paie la compilation des shaders Vulkan par le pilote au premier usage ; le pilote la
garde ensuite en cache (dans la session QGIS ci-dessous, première lecture en 4,0 s).

**Session enchaînée dans QGIS** (même jour, mode opératoire de 5.20 : les onze phrases, QGIS
3.44.15, chaque plan exécuté, noyau lancé depuis une copie du paquet, masque vérifié par la sonde).
Composition en secondes ; la phrase 1 est hors critère :

| n | carte (8 fils) | carte masquée, `0x55` | moteur carte retiré, `0x55` | 5.20, `0x55` |
|---|---|---|---|---|
| 1 | 20,3 | 55,0 | 55,0 | 54,7 |
| 2 | 3,9 | 14,7 | 14,8 | 15,1 |
| 3 | 8,4 | 22,1 | 21,5 | 26,1 |
| 4 | 3,5 | 15,4 | 14,6 | 15,5 |
| 5 | 3,8 | 9,8 | 9,5 | 10,0 |
| 6 | 5,9 | 15,0 | 14,9 | 15,1 |
| 7 | 3,7 | 12,7 | 11,6 | 12,7 |
| 8 | 3,9 | 8,3 | 7,6 | 8,4 |
| 9 | **14,2** | 35,3 | 33,5 | 34,8 |
| 10 | 2,5 | 12,6 | 12,5 | 12,6 |
| 11 | 5,1 | 12,5 | 12,4 | 12,7 |
| total 2 à 10 | **49,9** | 145,9 | 140,5 | 150,3 |

Carte masquée par `GGML_VK_VISIBLE_DEVICES=aucune` (Vulkan ne voit plus aucune carte) ; moteur
carte retiré : copie du paquet sans `ggml-vulkan.dll`. Les deux composent sans erreur, pas plus
lentement qu'en 5.20. Les issues sont les mêmes dans les trois colonnes (phrase 7 : refus ; phrase
11 : plan dont l'exécution échoue sur `layer.not_vector`, comme en 5.20).

**Lecture.**

- Sur la carte, les phrases 2 à 10 passent de 150,3 à 49,9 s, et la démo en cours de session
  (phrase 9) de 34,8 à **14,2 s** : 9 phrases sur 9 sous 30 s.
- L'écriture ne gagne qu'un facteur deux environ (écriture moyenne 6,9 s contre 13,1 s ; A0, pour
  165 jetons : 13,9 s contre 27,9 s, soit environ 12 jetons/s), et CUDA ne fait pas mieux : le coût n'est plus le modèle, qui tient entièrement sur
  la carte, mais ce qui reste sur le processeur à chaque jeton, pour l'essentiel la grammaire, qui
  examine à chaque pas tout le vocabulaire (248 000 jetons). Piste hors de ce ticket, sans changer
  ni la grammaire ni l'issue (décodage glouton) : proposer d'abord le jeton le plus probable seul à
  la grammaire, et ne filtrer tout le vocabulaire que s'il est refusé.

**Justesse sur la carte** (paquet, `--features gpu`) : `plan_reel` 25/25, `question_reel` 12/12,
`ecart_reel` 32/40 (plans 18/26, questions 8/8, refus 6/6) — les seuils du ticket.
