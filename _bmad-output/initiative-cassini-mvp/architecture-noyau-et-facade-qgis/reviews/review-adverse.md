# Revue adverse — deux moitiés obéissantes peuvent-elles encore divergir ?

Lentille configurée dans `finalize_reviewers` : construire deux unités qui
respectent chaque AD à la lettre et se construisent pourtant incompatibles.
Conduite le 2026-10-14. Chaque paire trouvée est un trou à fermer.

## Verdict

Huit trous trouvés, dont trois provoquent un blocage ou une perte de données en
exploitation. Tous fermables par un AD nouveau ou resserré.

## A1 — critique — qui parle le premier, et comment le noyau apprend-il le projet

AD-7 dit « le premier échange est une annonce » sans dire qui l'émet ni sous
quelle forme. Implémentation 1 : l'extension envoie une requête et attend.
Implémentation 2 : le noyau attend une notification puis émet la sienne. Les deux
obéissent, et rien ne se branche.

Pire : AD-9, AD-10 et AD-11 résolvent tous les chemins « relatifs au fichier de
projet », mais **aucun AD ne dit comment le noyau apprend ce chemin**. Deux
implémentations plausibles — argument de ligne de commande, ou champ de l'annonce
— et le journal atterrit à deux endroits différents.

## A2 — critique — deux fenêtres QGIS, un projet, deux écrivains

AD-11 garantit un auteur unique du journal. Mais AD-2 donne **un noyau par
instance de QGIS**. Deux fenêtres ouvertes sur le même projet font donc deux
noyaux, donc deux écrivains en ajout sur le même fichier. L'invariant est violé
sans qu'aucune règle ne soit enfreinte.

## A3 — critique — la sortie d'erreur non drainée bloque tout

AD-2 réserve la sortie d'erreur aux traces. Si l'extension ne la lit pas, le
tampon du système se remplit et le noyau se bloque en écriture, définitivement.
AD-4 interdit les lectures bloquantes mais n'oblige personne à drainer les traces.
Implémentation 1 draine, implémentation 2 ne connecte que la sortie standard :
la seconde fonctionne en démonstration et se bloque sur un travail bavard.

## A4 — haute — qui traduit en langage de QGIS

`analysis.select_by_attribute` a besoin d'un filtre. Si le noyau compose une
expression QGIS, il connaît la syntaxe d'expression de QGIS — AD-1 est contournée
en esprit, et le module défense privé héritera d'un noyau de forme QGIS. Si c'est
l'extension, il faut que les paramètres du registre soient neutres. Rien ne le
dit : deux implémentations divergeront sur le type même du paramètre. Même trou
pour les noms d'algorithmes de traitement : rien n'interdit au noyau d'écrire
`native:buffer`.

## A5 — haute — le rejeu et les identifiants de couche

Le journal enregistre des identifiants de couche QGIS en sortie d'étape. Au
rejeu, QGIS attribue de **nouveaux** identifiants. L'étape 3 qui consomme la
sortie de l'étape 2 doit donc passer par une table de correspondance. AD-9 et
AD-11 ne la nomment pas : une implémentation résout par l'identifiant enregistré
et échoue, l'autre par correspondance et marche.

## A6 — haute — supprimer un fichier que QGIS tient ouvert

AD-12 charge le noyau de nettoyer ses sorties partielles. Sous Windows, un
fichier qu'une couche QGIS tient ouvert ne peut pas être supprimé. Le noyau
obéit, la suppression échoue, et l'utilisateur garde un fichier à moitié écrit
que rien ne retire.

## A7 — moyenne — qui valide les paramètres

AD-6 engendre les schémas mais ne dit pas qui valide. Si le noyau suppose que
l'exécutant valide et l'exécutant suppose le contraire, un paramètre aberrant
traverse et casse au fond de QGIS, loin de sa cause.

## A8 — moyenne — annulation impossible et unités implicites

AD-12 exige que toute étape soit annulable, mais un traitement QGIS n'est
annulable que si l'extension lui passe un objet de retour annulable ; certaines
opérations ne le sont pas du tout. Rien ne permet de déclarer une commande non
annulable, donc une moitié promet ce que l'autre ne peut pas tenir. De même,
« tampon de 500 » sans unité : une couche en degrés et une couche en mètres ne
donnent pas le même résultat, et aucun AD n'oblige à porter l'unité.
