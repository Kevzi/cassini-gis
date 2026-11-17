#!/bin/sh
# Installeur de Cassini pour Linux : les deux portes et la mise a jour, dans un seul script
# POSIX (ticket 7.7, AD-17). Sans droits root : tout va dans ~/.local/share/cassini, ou
# l'extension QGIS cherche le noyau (bin/cassini).
#
#   Porte 1 (reseau)  : sh cassini.sh
#       lit le manifeste des versions de la release, telecharge le noyau et, sauf
#       --sans-modele, le modele, verifie le SHA-256 de chaque fichier, puis les pose.
#   Porte 2 (hors ligne) : sh cassini.sh, depuis l'archive complete depliee
#       le script trouve noyau/ et modele/ a cote de lui : aucun appel reseau.
#   Mise a jour (a la main) : sh cassini.sh --mise-a-jour
#       ou ~/.local/share/cassini/mettre-a-jour-cassini --mise-a-jour (copie posee a l'installation).
#
# Options : --mise-a-jour  --manifeste URL_OU_CHEMIN  --sans-modele  --avec-modele  --oui  --aide
#
# Regles (les memes que l'installeur Windows, ticket 7.5) : chaque fichier est verifie par
# sha256sum avant toute pose, et rien n'est pose si l'un echoue ; le noyau est pose par mv d'un
# fichier temporaire du meme dossier (QGIS peut etre ouvert) ; aucun fichier de models n'est
# supprime ni ecrase ; le modele garde son nom versionne ; a jour = rien telecharge ni reecrit.
# Dependances : sh, tar, sha256sum, et curl ou wget pour la porte 1 seulement.

set -u

MANIFESTE_DEFAUT="https://github.com/Kevzi/cassini-gis/releases/latest/download/manifeste.json"
DECLARATION="cassini-composition-4b.modele.json"

MODE_MAJ=0
AVEC_MODELE=""
OUI=0
MANIFESTE="${CASSINI_MANIFESTE:-$MANIFESTE_DEFAUT}"

dire() { printf '%s\n' "$*"; }
erreur() { printf 'Cassini : %s\n' "$*" >&2; exit 1; }

while [ $# -gt 0 ]; do
  case "$1" in
    --mise-a-jour) MODE_MAJ=1 ;;
    --sans-modele) AVEC_MODELE=0 ;;
    --avec-modele) AVEC_MODELE=1 ;;
    --oui) OUI=1 ;;
    --manifeste)
      [ $# -ge 2 ] || erreur "--manifeste demande une adresse ou un chemin"
      MANIFESTE="$2"; shift ;;
    --aide|-h|--help)
      sed -n '2,19p' "$0" 2>/dev/null | sed 's/^# \{0,1\}//'
      exit 0 ;;
    *) erreur "option inconnue : $1 (voir --aide)" ;;
  esac
  shift
done

[ -n "${HOME:-}" ] || erreur "la variable HOME n'est pas definie"
RACINE="$HOME/.local/share/cassini"
BIN="$RACINE/bin"
MODELES="$BIN/models"

command -v sha256sum >/dev/null 2>&1 || erreur "sha256sum est introuvable"
command -v tar >/dev/null 2>&1 || erreur "tar est introuvable"

# --- Outils -----------------------------------------------------------------------------------

sha_de() { sha256sum "$1" | cut -d' ' -f1; }

# Nom de fichier sain : ni dossier ni remontee, caracteres simples seulement.
nom_sain() { printf '%s\n' "$1" | grep -Eq '^[A-Za-z0-9][A-Za-z0-9._-]*$'; }

# Une partie (1 a 3) d'une version « a.b.c » ; une partie absente vaut 0, les zeros de tete sautent.
partie() {
  p=$(printf '%s.0.0.0\n' "$1" | cut -d. -f"$2" | sed 's/^0*\(.\)/\1/')
  printf '%s' "$p"
}

# Compare deux versions a trois nombres : -1 si $1 < $2, 0 si egales, 1 si $1 > $2, ? si illisible.
comparer() {
  for v in "$1" "$2"; do
    printf '%s\n' "$v" | grep -Eq '^[0-9]+(\.[0-9]+){0,2}$' || { printf '?'; return; }
  done
  i=1
  while [ "$i" -le 3 ]; do
    a=$(partie "$1" "$i"); b=$(partie "$2" "$i")
    if [ "$a" -lt "$b" ]; then printf -- '-1'; return; fi
    if [ "$a" -gt "$b" ]; then printf '1'; return; fi
    i=$((i + 1))
  done
  printf '0'
}

# Lecture du manifeste sans analyseur JSON : sa mise en forme est fixee (installer/manifeste.py),
# une valeur par ligne. `bloc NOM RETRAIT` lit l'entree standard et rend le bloc « "NOM": { ... } ».
bloc() { deb="^$2\"$1\": {\$"; fin="^$2},\\{0,1\\}\$"; sed -n "/$deb/,/$fin/p"; }
champ() { sed -n "s/^ *\"$1\": \"\\(.*\\)\",\\{0,1\\}\$/\\1/p" | head -n 1; }

telecharger() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL --retry 2 -o "$2" "$1"
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$2" "$1"
  else
    erreur "curl ou wget est necessaire pour la porte 1 (ou utilisez l'archive complete, porte 2)"
  fi
}

recuperer_manifeste() {
  case "$MANIFESTE" in
    http://*|https://*|file://*) telecharger "$MANIFESTE" "$1" ;;
    *) [ -f "$MANIFESTE" ] && cp "$MANIFESTE" "$1" ;;
  esac
}

version_installee() {
  [ -f "$BIN/installation.json" ] || return 0
  sed -n 's/.*"version": *"\([^"]*\)".*/\1/p' "$BIN/installation.json" | head -n 1
}

un_modele_present() {
  for f in "$MODELES"/*.gguf; do [ -e "$f" ] && return 0; done
  return 1
}

# --- Quelle porte ? ---------------------------------------------------------------------------

DIR=""
if [ -f "$0" ]; then DIR=$(cd "$(dirname -- "$0")" && pwd); fi
PORTE2=0
if [ -n "$DIR" ] && [ -f "$DIR/noyau/cassini" ] && [ -d "$DIR/modele" ]; then PORTE2=1; fi

INSTALLEE=""
if [ "$MODE_MAJ" = 1 ]; then
  if [ ! -f "$BIN/cassini" ] || [ ! -f "$BIN/installation.json" ]; then
    erreur "Cassini n'est pas installe (rien dans $BIN). Lancez d'abord l'installation, sans --mise-a-jour. Rien n'a ete pose."
  fi
  INSTALLEE=$(version_installee)
fi

# Modele voulu : facultatif ; a l'installation il est pris par defaut, a la mise a jour seulement
# si un .gguf est deja la (comme l'installeur Windows).
if [ -z "$AVEC_MODELE" ]; then
  AVEC_MODELE=1
  if [ "$MODE_MAJ" = 1 ] && ! un_modele_present; then AVEC_MODELE=0; fi
fi

# --- Espace de travail : dans RACINE, pour tenir le modele (2,8 Go) hors d'un /tmp trop petit ---

mkdir -p "$RACINE" || erreur "impossible de creer $RACINE"
TMP=$(mktemp -d "$RACINE/.cassini-tmp.XXXXXX") || erreur "impossible de creer un dossier temporaire dans $RACINE"
nettoyer() { rm -rf "$TMP"; }
trap nettoyer EXIT
trap 'nettoyer; exit 130' INT TERM

# --- Ce qu'on va poser -----------------------------------------------------------------------

NOYAU_SRC=""; NOYAU_SHA=""; NOYAU_NOM="cassini"
MODELE_SRC=""; MODELE_SHA=""; MODELE_NOM=""
NOYAU_URL=""; MODELE_URL=""

if [ "$PORTE2" = 1 ]; then
  [ -f "$DIR/VERSION" ] && [ -f "$DIR/empreintes.txt" ] || erreur "l'archive complete est incomplete (VERSION ou empreintes.txt absent)"
  VERSION=$(head -n 1 "$DIR/VERSION" | tr -d '\r ')
  NOYAU_SRC="$DIR/noyau/cassini"
  NOYAU_SHA=$(sed -n 's/^\([0-9a-f]\{64\}\)  noyau\/cassini\r\{0,1\}$/\1/p' "$DIR/empreintes.txt" | head -n 1)
  MODELE_NOM=$(sed -n 's/^[0-9a-f]\{64\}  modele\/\(.*\.gguf\)\r\{0,1\}$/\1/p' "$DIR/empreintes.txt" | head -n 1)
  [ -n "$NOYAU_SHA" ] || erreur "empreintes.txt ne donne pas l'empreinte de noyau/cassini"
  if [ "$AVEC_MODELE" = 1 ]; then
    [ -n "$MODELE_NOM" ] && nom_sain "$MODELE_NOM" || erreur "empreintes.txt ne donne pas le modele"
    MODELE_SRC="$DIR/modele/$MODELE_NOM"
    MODELE_SHA=$(grep -F "  modele/$MODELE_NOM" "$DIR/empreintes.txt" | head -n 1 | cut -c1-64)
  fi
else
  recuperer_manifeste "$TMP/manifeste.json" ||
    erreur "la liste des versions est introuvable ($MANIFESTE). Aucun fichier n'a ete installe."
  M="$TMP/manifeste.json"
  VERSION=$(champ version < "$M")
  [ -n "$VERSION" ] || erreur "la liste des versions est incomplete (version absente). Aucun fichier n'a ete installe."
  LIN=$(bloc linux '  ' < "$M")
  [ -n "$LIN" ] || erreur "la liste des versions ne contient pas de noyau Linux. Aucun fichier n'a ete installe."
  NOYAU_BLOC=$(printf '%s\n' "$LIN" | bloc noyau '    ')
  NOYAU_NOM_TAR=$(printf '%s\n' "$NOYAU_BLOC" | champ nom)
  NOYAU_URL=$(printf '%s\n' "$NOYAU_BLOC" | champ url)
  NOYAU_SHA=$(printf '%s\n' "$NOYAU_BLOC" | champ sha256)
  nom_sain "$NOYAU_NOM_TAR" && [ -n "$NOYAU_URL" ] && [ "${#NOYAU_SHA}" = 64 ] ||
    erreur "la liste des versions est incomplete (noyau Linux). Aucun fichier n'a ete installe."
  NOYAU_SRC="$TMP/$NOYAU_NOM_TAR"
  MODELE_BLOC=$(bloc modele '  ' < "$M")
  if [ -n "$MODELE_BLOC" ]; then
    MODELE_NOM=$(printf '%s\n' "$MODELE_BLOC" | champ nom)
    MODELE_URL=$(printf '%s\n' "$MODELE_BLOC" | champ url)
    MODELE_SHA=$(printf '%s\n' "$MODELE_BLOC" | champ sha256)
  fi
  if [ "$AVEC_MODELE" = 1 ]; then
    # Un ancien manifeste sans section modele : noyau seul, sans erreur.
    if [ -z "$MODELE_BLOC" ]; then
      AVEC_MODELE=0
    else
      nom_sain "$MODELE_NOM" && [ -n "$MODELE_URL" ] && [ "${#MODELE_SHA}" = 64 ] &&
        [ "${MODELE_NOM%.gguf}" != "$MODELE_NOM" ] ||
        erreur "la liste des versions est incomplete (modele). Aucun fichier n'a ete installe."
      MODELE_SRC="$TMP/$MODELE_NOM"
    fi
  fi
fi
[ "$AVEC_MODELE" = 1 ] && { [ "${#MODELE_SHA}" = 64 ] || erreur "empreinte du modele illisible. Aucun fichier n'a ete installe."; }
printf '%s\n' "$VERSION" | grep -Eq '^[0-9]+(\.[0-9]+){0,2}$' || erreur "version illisible : $VERSION. Aucun fichier n'a ete installe."

# --- Mise a jour : a jour ? confirmation -----------------------------------------------------

if [ "$MODE_MAJ" = 1 ]; then
  CMP=$(comparer "$INSTALLEE" "$VERSION")
  MODELE_PRESENT=1
  if [ "$AVEC_MODELE" = 1 ] && [ ! -e "$MODELES/$MODELE_NOM" ]; then MODELE_PRESENT=0; fi
  if [ "$CMP" = 1 ] || { [ "$CMP" = 0 ] && [ "$MODELE_PRESENT" = 1 ]; }; then
    dire "Cassini est a jour (version $INSTALLEE). Rien n'a ete telecharge ni reecrit."
    exit 0
  fi
  if [ "$CMP" = 0 ]; then
    Q="Cassini $INSTALLEE est installe ; le nouveau modele sera ajoute a cote de l'ancien."
  elif [ "$CMP" = "?" ]; then
    Q="Mettre a jour Cassini vers la version $VERSION ?"
  else
    Q="Mettre a jour Cassini de la version $INSTALLEE vers la version $VERSION ?"
  fi
  [ "$MODELE_PRESENT" = 0 ] && Q="$Q Le modele $MODELE_NOM sera ajoute, l'ancien reste."
  if [ "$OUI" = 0 ] && [ -t 0 ]; then
    printf '%s [O/n] ' "$Q"
    read -r REP
    case "$REP" in ''|o|O|oui|Oui|y|Y|yes) : ;; *) dire "Mise a jour annulee. Rien n'a ete pose."; exit 1 ;; esac
  else
    dire "$Q"
  fi
fi

# --- Recuperation et verification de TOUT avant la moindre pose -------------------------------

verifier() { # fichier empreinte-attendue nom-affiche
  reel=$(sha_de "$1")
  if [ "$reel" != "$2" ]; then
    erreur "l'empreinte SHA-256 de $3 est fausse (attendue $2, obtenue $reel). Installation refusee, aucun fichier n'a ete pose."
  fi
}

if [ "$PORTE2" = 1 ]; then
  dire "Archive complete : verification des fichiers (aucun acces au reseau)."
  verifier "$NOYAU_SRC" "$NOYAU_SHA" "noyau/cassini"
  [ "$AVEC_MODELE" = 1 ] && { [ -f "$MODELE_SRC" ] || erreur "modele/$MODELE_NOM est absent de l'archive."; verifier "$MODELE_SRC" "$MODELE_SHA" "modele/$MODELE_NOM"; }
  NOYAU_PRET="$NOYAU_SRC"
else
  dire "Telechargement du noyau Cassini $VERSION..."
  telecharger "$NOYAU_URL" "$NOYAU_SRC" || erreur "le telechargement de $NOYAU_NOM_TAR a echoue. Aucun fichier n'a ete installe."
  verifier "$NOYAU_SRC" "$NOYAU_SHA" "$NOYAU_NOM_TAR"
  if [ "$AVEC_MODELE" = 1 ]; then
    if [ -e "$MODELES/$MODELE_NOM" ]; then
      dire "Le modele $MODELE_NOM est deja installe : il est conserve."
      AVEC_MODELE=0
    else
      dire "Telechargement du modele $MODELE_NOM (environ 2,8 Go)..."
      telecharger "$MODELE_URL" "$MODELE_SRC" || erreur "le telechargement de $MODELE_NOM a echoue. Aucun fichier n'a ete installe."
      verifier "$MODELE_SRC" "$MODELE_SHA" "$MODELE_NOM"
    fi
  fi
  # Le tar du noyau ne contient que `cassini` et les deux textes de licences (ou `cassini` seul).
  LISTE=$(tar -tzf "$NOYAU_SRC" 2>/dev/null | sed 's|^\./||' | tr '
' ' ') || LISTE=""
  [ "$LISTE" = "cassini " ] || [ "$LISTE" = "cassini LICENCES-TIERCES.txt FRONTIERE-GPL.txt " ] ||
    erreur "$NOYAU_NOM_TAR ne contient pas exactement le fichier cassini et les textes de licences. Aucun fichier n'a ete installe."
  mkdir -p "$TMP/noyau" && tar -xzf "$NOYAU_SRC" -C "$TMP/noyau" || erreur "impossible de deplier $NOYAU_NOM_TAR."
  NOYAU_PRET="$TMP/noyau/cassini"
  [ -f "$NOYAU_PRET" ] || erreur "$NOYAU_NOM_TAR ne contient pas cassini."
fi

# --- Pose --------------------------------------------------------------------------------------

mkdir -p "$MODELES" || erreur "impossible de creer $MODELES"

MODELE_POSE=0
if [ "$AVEC_MODELE" = 1 ] && [ ! -e "$MODELES/$MODELE_NOM" ]; then
  # Fichier temporaire du meme dossier, puis mv : jamais un modele a moitie ecrit sous son nom.
  cp "$MODELE_SRC" "$MODELES/.$MODELE_NOM.part" && mv -f "$MODELES/.$MODELE_NOM.part" "$MODELES/$MODELE_NOM" ||
    { rm -f "$MODELES/.$MODELE_NOM.part"; erreur "impossible de poser $MODELE_NOM."; }
  MODELE_POSE=1
  # Declaration lue par le noyau (meme forme que la porte 2). Si une declaration existe deja pour
  # un autre modele, celle du nouveau prend son nom : aucune n'est ecrasee.
  DECL="$MODELES/$DECLARATION"
  if [ -f "$DECL" ] && ! grep -qF "$MODELE_NOM" "$DECL"; then DECL="$MODELES/${MODELE_NOM%.gguf}.modele.json"; fi
  printf '{"model": "%s", "prompt": "short", "model_sha256": "%s"}\n' "$MODELE_NOM" "$MODELE_SHA" > "$DECL.part" &&
    mv -f "$DECL.part" "$DECL" || erreur "impossible d'ecrire la declaration du modele."
fi

# Le noyau : pose par mv d'un fichier temporaire du meme dossier, QGIS peut etre ouvert.
cp "$NOYAU_PRET" "$BIN/.cassini.nouveau" && chmod 755 "$BIN/.cassini.nouveau" && mv -f "$BIN/.cassini.nouveau" "$BIN/cassini" ||
  { rm -f "$BIN/.cassini.nouveau"; erreur "impossible de poser le noyau dans $BIN (est-il en cours d'usage ?)."; }

# Textes de licences (a cote du noyau, comme sous Windows) : porte 2 dans l'archive depliee, porte 1
# dans le tar du noyau. Facultatifs pour une ancienne archive qui n'en contient pas.
if [ "$PORTE2" = 1 ]; then SRC_TXT="$DIR"; else SRC_TXT="$TMP/noyau"; fi
for t in LICENCES-TIERCES.txt FRONTIERE-GPL.txt; do
  if [ -f "$SRC_TXT/$t" ]; then
    cp "$SRC_TXT/$t" "$BIN/.$t.part" && mv -f "$BIN/.$t.part" "$BIN/$t" || { rm -f "$BIN/.$t.part"; erreur "impossible de poser $t."; }
  fi
done

DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ)
printf '{"version": "%s", "installe_le": "%s"}\n' "$VERSION" "$DATE" > "$BIN/installation.json.part" &&
  mv -f "$BIN/installation.json.part" "$BIN/installation.json" || erreur "impossible d'ecrire installation.json."

# Copie du script : c'est le programme de « Mettre a jour Cassini ».
if [ -f "$0" ]; then
  cp "$0" "$RACINE/.mettre-a-jour-cassini.part" && chmod 755 "$RACINE/.mettre-a-jour-cassini.part" &&
    mv -f "$RACINE/.mettre-a-jour-cassini.part" "$RACINE/mettre-a-jour-cassini" || rm -f "$RACINE/.mettre-a-jour-cassini.part"
fi

if [ "$MODE_MAJ" = 1 ]; then
  dire "Cassini est a jour : version $VERSION (installee le $DATE)."
else
  dire "Cassini $VERSION est installe dans $RACINE."
fi
[ "$MODELE_POSE" = 1 ] && dire "Modele pose : $MODELE_NOM (les anciens modeles de $MODELES sont gardes)."
dire "Pour mettre a jour plus tard : $RACINE/mettre-a-jour-cassini --mise-a-jour"
exit 0
