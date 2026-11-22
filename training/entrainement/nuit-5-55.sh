#!/bin/bash
# 5.55 (nuit d'expérience) : ajustement continu contre départ de zéro, sur le même jeu, jugés
# par les mêmes barres. Dérivé de nuit-5-54.sh / nuit-5-59.sh (hors dépôt), écrit au dépôt par
# décision du propriétaire (2026-11-16).
#
#   bash training/entrainement/nuit-5-55.sh base
#   bash training/entrainement/nuit-5-55.sh continu famille_a,famille_b[,…]
#   bash training/entrainement/nuit-5-55.sh deux famille_a,famille_b[,…]    # base, puis continu
#
# Deux bras, une seule graine torch (1), un seul juge (non_oubli.py) et les mêmes relevés de
# référence (reference-non-oubli-5-58-1, ceux du modèle installé) :
#   - « base »   : entraînement de zéro sur le jeu complet par défaut (DEFAULT_COUNT, 8 600
#                  exemples), sans --depuis ni --exemples ;
#   - « continu » : une séance depuis l'adaptateur installé ($PARENT), sur les familles données,
#                  3 000 exemples dont une part de rappel ($PART_RAPPEL, 0,5 par défaut).
# Les deux sortent du même commit du générateur (le worktree $WT). Pour chaque bras : gardes,
# entraînement (durée écrite dans l'état), copie dans cassini-modeles/$NOM SANS installation,
# essais GPU (--features cuda), recettes d'Annecy, du gros projet et v4, juge. Rien n'est
# installé ni poussé : le propriétaire tranche sur le tableau.
#
# Le worktree entraîné ($WT) et CASSINI-nuit, où se jouent les essais, doivent être au même
# commit, propres ; GPU libre ; aucun entraînement ni cargo en cours ; script de recette
# Windows ($RECETTE_PS1) présent. Un refus des gardes s'écrit dans l'état et sort en code 1,
# sans toucher au GPU. Le dépôt principal et les dossiers de modèles installés ne sont jamais
# touchés. États : $E/nuit-$NOM_BASE.etat et $E/nuit-$NOM_CONTINU.etat.
set -u
MODE=${1:?usage : nuit-5-55.sh base | continu <familles> | deux <familles>}
case $MODE in
  base) FAMILLES="" ;;
  continu|deux) FAMILLES=${2:?usage : nuit-5-55.sh $MODE famille_a,famille_b (familles du bras continu)} ;;
  *) echo "mode inconnu : $MODE (base | continu | deux)" >&2; exit 2 ;;
esac
E=~/cassini-entrainement
WT=${WT:-~/cassini-worktrees/5-55}
PARENT=${PARENT:-$E/ajuste-continu-5-58-1}
NOM_BASE=${NOM_BASE:-base-5-55}
NOM_CONTINU=${NOM_CONTINU:-continu-5-55}
MODELES=/mnt/c/Users/kevin/cassini-modeles
REF=${REF:-$E/reference-non-oubli-5-58-1}
MES_ROOT=${MES_ROOT:-$E/5-55-mesures}
PART_RAPPEL=${PART_RAPPEL:-0.5}
TEMPW=/mnt/c/Users/kevin/AppData/Local/Temp
DW='C:\Users\kevin\Downloads\Projet code'
NUIT="/mnt/c/Users/kevin/Downloads/Projet code/CASSINI-nuit"
ESSAIS_PS1='C:\Users\kevin\Downloads\Projet code\outils-cassini\scripts\essais-cuda.ps1'
RECETTE_PS1=${RECETTE_PS1:-'C:\Users\kevin\Downloads\Projet code\outils-cassini\scripts\recette-5-54.ps1'}
TESTS="plan_reel,question_reel,ecart_reel,echelle_utm_reel,terrain_reel,epic11_reel,style_reel,par_commande_reel"

# Fixés par bras (voir bras) : l'état et les noms de sortie.
NOM=""; ETAT=""; OUT=""; M=""; MES=""
etat() { echo "$(date '+%F %T') $*" >> "$ETAT"; }
# Secondes -> « 2 h 41 min 07 s ».
hms() { printf '%d h %02d min %02d s' $(($1 / 3600)) $(($1 % 3600 / 60)) $(($1 % 60)); }

# Les gardes de nuit-5-54.sh, refaites juste avant chaque prise du GPU : tout doute arrête.
gardes() {
  local apps
  apps=$(nvidia-smi --query-compute-apps=pid --format=csv,noheader) || { echo "nvidia-smi (WSL) illisible"; return 1; }
  [ -z "$apps" ] || { echo "GPU pris (WSL) : $apps"; return 1; }
  apps=$(powershell.exe -NoProfile -Command "nvidia-smi --query-compute-apps=pid,process_name --format=csv,noheader" | tr -d '\r') \
    || { echo "nvidia-smi (Windows) illisible"; return 1; }
  apps=$(printf '%s\n' "$apps" | grep -iE 'python|\\deps\\|cassini|llama' || true)
  [ -z "$apps" ] || { echo "GPU pris (Windows) : $apps"; return 1; }
  if pgrep -f '^(/[^ ]*/)?(cargo|rustc)( |$)|python[0-9.]* .*pipeline\.py' >/dev/null; then
    echo "entraînement ou cargo en cours (WSL) :"; pgrep -af '^(/[^ ]*/)?(cargo|rustc)( |$)|python[0-9.]* .*pipeline\.py'; return 1
  fi
  apps=$(powershell.exe -NoProfile -Command "Get-Process cargo -ErrorAction SilentlyContinue | ForEach-Object { \$_.Id }; Get-CimInstance Win32_Process -Filter \"Name like 'python%'\" | Where-Object { \$_.CommandLine -like '*pipeline.py*' } | ForEach-Object { \$_.ProcessId }" | tr -d '\r') \
    || { echo "processus Windows illisibles"; return 1; }
  [ -z "$apps" ] || { echo "entraînement ou cargo en cours (Windows) : $apps"; return 1; }
  apps=$(git -C "$NUIT" status --porcelain --untracked-files=no) || { echo "CASSINI-nuit illisible"; return 1; }
  [ -z "$apps" ] || { echo "CASSINI-nuit pas propre"; return 1; }
  apps=$(git -C "$WT" status --porcelain --untracked-files=no) || { echo "worktree illisible : $WT"; return 1; }
  [ -z "$apps" ] || { echo "worktree pas propre : $WT"; return 1; }
  [ "$(git -C "$NUIT" rev-parse HEAD)" = "$(git -C "$WT" rev-parse HEAD)" ] \
    || { echo "CASSINI-nuit ($(git -C "$NUIT" rev-parse --short HEAD)) n'est pas au commit du worktree ($(git -C "$WT" rev-parse --short HEAD))"; return 1; }
}

# Ce qui doit exister avant de prendre le GPU ; rien n'est écrasé.
preparatifs() {
  [ -d "$WT" ] || { echo "worktree absent : $WT"; return 1; }
  ls "$REF"/* >/dev/null 2>&1 || { echo "relevés de référence absents : $REF"; return 1; }
  [ ! -e "$OUT" ] || { echo "sortie déjà là : $OUT (rien n'est écrasé)"; return 1; }
  [ ! -e "$M" ] || { echo "dossier de modèle déjà là : $M (rien n'est écrasé)"; return 1; }
  [ -f "$(wslpath -u "$RECETTE_PS1")" ] || { echo "script de recette absent : $RECETTE_PS1"; return 1; }
  [ -f "$(wslpath -u "$ESSAIS_PS1")" ] || { echo "script des essais absent : $ESSAIS_PS1"; return 1; }
  if [ "$1" = continu ]; then
    [ -f "$PARENT/rapport.json" ] && [ -f "$PARENT/travail/adaptateur-hf/adapter_model.safetensors" ] \
      || { echo "parent incomplet : $PARENT"; return 1; }
  fi
}

# bras base|continu : un bras de bout en bout. Code 0 si le juge a rendu un verdict, 1 sinon.
bras() {
  local kind=$1 raison t0 t1 rc L r P F CANDIDAT code verdict
  if [ "$kind" = base ]; then NOM=$NOM_BASE; else NOM=$NOM_CONTINU; fi
  ETAT=$E/nuit-$NOM.etat; OUT=$E/$NOM; M=$MODELES/$NOM; MES=$MES_ROOT/$NOM
  # Un refus des gardes s'ajoute à l'état : le verdict d'une nuit précédente reste lisible.
  if ! raison=$(preparatifs "$kind") || ! raison=$(gardes); then
    etat "ÉCHEC des gardes : $raison"
    return 1
  fi
  : > "$ETAT"
  etat "début bras $kind ($(git -C "$WT" rev-parse --short HEAD), essais sur CASSINI-nuit $(git -C "$NUIT" rev-parse --short HEAD)), sortie $NOM, graine torch 1${FAMILLES:+, familles $FAMILLES, part de rappel $PART_RAPPEL, parent $PARENT}"
  mkdir -p "$MES"
  cd "$WT" || { etat "ÉCHEC worktree"; return 1; }

  # 1. L'entraînement. Base : jeu complet par défaut. Continu : séance depuis le parent.
  t0=$(date +%s)
  if [ "$kind" = base ]; then
    etat "début entraînement de zéro, jeu complet par défaut"
    uv run --project training/entrainement python training/entrainement/pipeline.py --sortie "$OUT" \
      --graine-torch 1 > "$E/$NOM.log" 2>&1
  else
    etat "début séance continue depuis $PARENT"
    uv run --project training/entrainement python training/entrainement/pipeline.py --sortie "$OUT" \
      --depuis "$PARENT" --familles "$FAMILLES" --graine-torch 1 --exemples 3000 --part-rappel "$PART_RAPPEL" \
      > "$E/$NOM.log" 2>&1
  fi
  rc=$?
  t1=$(date +%s)
  etat "fin entraînement, code $rc"
  etat "DUREE entraînement $NOM : $((t1 - t0)) s ($(hms $((t1 - t0))))"
  if [ $rc -ne 0 ] || [ ! -f "$OUT/cassini-composition-4b-Q4_K_M.gguf" ]; then
    etat "ÉCHEC entraînement : voir $E/$NOM.log"
    return 1
  fi
  python3 -I -c "
import json, sys
r = json.load(open(sys.argv[1], encoding='utf-8'))
v = r.get('validation', {}); d = r.get('donnees', {})
print('VALIDATION %s/%s ; maillons %d ; exemples %s' % (v.get('justes', '?'), v.get('total', '?'),
      len(r.get('lignee', [])), d.get('exemples', d.get('count', '?'))))
" "$OUT/rapport.json" >> "$ETAT" 2>&1 || etat "résumé du rapport illisible : $OUT/rapport.json"

  # 2. Copie, sans installation.
  mkdir -p "$M" && cp "$OUT/cassini-composition-4b-Q4_K_M.gguf" "$OUT/rapport.json" "$M/" && \
    echo '{"model": "cassini-composition-4b-Q4_K_M.gguf", "prompt": "short"}' > "$M/cassini-composition-4b.modele.json" \
    || { etat "ÉCHEC copie dans $M"; return 1; }
  etat "modèle copié dans $M (non installé)"

  # 3. Essais GPU, puis recettes ; les gardes refaites (l'entraînement a rendu le GPU).
  if ! raison=$(gardes); then
    etat "ÉCHEC des gardes avant les essais : $raison"
    return 1
  fi
  L=essais-gpu-$NOM.log
  rm -f "$TEMPW/$L"
  powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$ESSAIS_PS1" \
    -Model "C:\\Users\\kevin\\cassini-modeles\\$NOM\\cassini-composition-4b.modele.json" \
    -Log "C:\\Users\\kevin\\AppData\\Local\\Temp\\$L" -Tests "$TESTS"
  [ -s "$TEMPW/$L" ] || { etat "ÉCHEC : relevé des essais absent ou vide ($TEMPW/$L)"; return 1; }
  cp "$TEMPW/$L" "$MES/"
  grep -a "^SCORE\|=== fin" "$MES/$L" >> "$ETAT"
  CANDIDAT=("$MES/$L")
  for r in annecy gros v4; do
    if [ $r = annecy ]; then
      P="$DW\\donnees-test-cassini\\annecy\\projet-test-annecy.qgz"; F="$DW\\CASSINI-nuit\\qgis-plugin\\tests\\recette\\annecy.json"
    elif [ $r = gros ]; then
      P="$DW\\donnees-test-cassini\\annecy\\projet-gros-annecy.qgz"; F="$DW\\donnees-test-cassini\\annecy\\essais-5-48\\phrases-gros.json"
    else
      P="$DW\\donnees-test-cassini\\annecy\\projet-gros-annecy.qgz"; F="$DW\\CASSINI-nuit\\qgis-plugin\\tests\\recette\\gros-annecy-v4.json"
    fi
    rm -f "$TEMPW/recette-$r-$NOM.txt"
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$RECETTE_PS1" \
      -Models "C:\\Users\\kevin\\cassini-modeles\\$NOM" -Project "$P" -Phrases "$F" \
      -Out "C:\\Users\\kevin\\AppData\\Local\\Temp\\recette-$r-$NOM.txt"
    if [ -s "$TEMPW/recette-$r-$NOM.txt" ]; then
      cp "$TEMPW/recette-$r-$NOM.txt" "$MES/"
      grep -a "SCORE :\|FIN :\|CODE\|SONDE NON PARTIE" "$MES/recette-$r-$NOM.txt" | sed "s/^/[$r] /" >> "$ETAT"
      # Un relevé sans ligne de score rendrait la barre de non-oubli illisible : hors du juge.
      if grep -aq "SCORE :" "$MES/recette-$r-$NOM.txt"; then
        [ $r = v4 ] || CANDIDAT+=("$MES/recette-$r-$NOM.txt")
      else
        etat "recette $r sans ligne SCORE : laissée hors du juge ($MES/recette-$r-$NOM.txt)"
      fi
    else
      etat "recette $r absente ou vide"
    fi
  done

  # 4. Le juge, contre les mêmes relevés, en mode `base` pour un modèle de zéro (qui perd et gagne
  # une vingtaine de phrases au hasard à chaque tirage) et `continu` sinon. Rien n'est installé.
  python3 -I training/entrainement/non_oubli.py --mode "$kind" --parent "$REF"/* --candidat "${CANDIDAT[@]}" \
    --json "$MES/non-oubli.json" > "$MES/non-oubli.txt" 2>&1
  code=$?
  case $code in
    0) verdict="installable" ;;
    1) verdict="refusé" ;;
    3) verdict="à décider" ;;
    *) verdict="relevé illisible ou erreur ($code)" ;;
  esac
  tail -n 1 "$MES/non-oubli.txt" >> "$ETAT"
  etat "VERDICT non-oubli : $verdict (code $code ; tableau : $MES/non-oubli.txt) — rien d'installé"
  etat "TOUT FINI ($NOM, entraînement $(hms $((t1 - t0))))"
  return 0
}

case $MODE in
  base) bras base ;;
  continu) bras continu ;;
  deux) bras base; rb=$?; bras continu; rc2=$?; [ $rb -eq 0 ] && [ $rc2 -eq 0 ] ;;
esac
