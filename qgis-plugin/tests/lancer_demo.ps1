# Suite de demo complete en une commande : la sonde `probe_demo.py` jouee par le Python de QGIS,
# d'abord sur les fixtures synthetiques, puis sur le projet reel (Annecy). N'installe rien et ne
# modifie ni les fixtures ni le projet reel (la sonde travaille sur une copie ou sans l'ecrire).
#   -Projet          projet QGIS (.qgz) reel ; par defaut, celui d'Annecy de donnees-test-cassini
#   -SansProjetReel  ne joue que les fixtures (" projet reel : non joue ")
#   -Modeles         dossier qui ne contient que le modele a montrer (CASSINI_MODELS) ; par defaut,
#                    le dossier des modeles installes
#   -Sortie          fichier du rapport complet ; par defaut, un fichier date dans %TEMP%
# Variables lues si elles existent : CASSINI_CORE (noyau ; sinon celui de l'installation),
# CASSINI_QGIS_PYTHON (python-qgis-ltr.bat). Les variables CASSINI_DEMO_* de la sonde passent telles quelles.
# Codes de sortie : 0 tout est OK, 1 un verdict ECHEC, 2 sonde non partie (noyau, modele, QGIS ou
# projet introuvable).
param(
  [string]$Projet = "",
  [switch]$SansProjetReel,
  [string]$Modeles = (Join-Path $env:LOCALAPPDATA "Cassini\bin\models"),
  [string]$Sortie = (Join-Path $env:TEMP ("demo-" + (Get-Date -Format "yyyyMMdd-HHmmss") + ".txt"))
)

$Racine = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if (-not $Projet) {
  $Projet = Join-Path $Racine "..\donnees-test-cassini\annecy\projet-test-annecy.qgz"
}
# Chemins relatifs resolus ici, depuis le dossier de l'appelant, avant tout Set-Location.
function Absolu([string]$Chemin) {
  if ([System.IO.Path]::IsPathRooted($Chemin)) { return [System.IO.Path]::GetFullPath($Chemin) }
  [System.IO.Path]::GetFullPath((Join-Path (Get-Location).ProviderPath $Chemin))
}
$Projet = Absolu $Projet
$Modeles = Absolu $Modeles
$Sortie = Absolu $Sortie

# Ecrit a l'ecran et dans le fichier du rapport (compatible Windows PowerShell 5.1 et 7).
function Say {
  process { Write-Host $_; Add-Content -Encoding UTF8 -LiteralPath $Sortie -Value $_ }
}

function Stop-Demo([string]$Pourquoi) {
  "SONDE NON PARTIE : $Pourquoi" | Out-File -Encoding utf8 $Sortie
  "RESULTAT GLOBAL : ECHEC (sonde non partie) - code 2" | Out-File -Append -Encoding utf8 $Sortie
  Write-Host "SONDE NON PARTIE : $Pourquoi"
  exit 2
}

if (-not $env:CASSINI_CORE) { $env:CASSINI_CORE = Join-Path $env:LOCALAPPDATA "Cassini\bin\cassini.exe" }
$Python = if ($env:CASSINI_QGIS_PYTHON) { $env:CASSINI_QGIS_PYTHON } else { "C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat" }
if (-not (Test-Path -LiteralPath $Python)) { Stop-Demo "Python de QGIS introuvable : $Python (poser CASSINI_QGIS_PYTHON)" }
if (-not (Test-Path -LiteralPath $env:CASSINI_CORE)) { Stop-Demo "noyau introuvable : $env:CASSINI_CORE (poser CASSINI_CORE)" }
if (-not (Test-Path -LiteralPath $Modeles)) { Stop-Demo "dossier de modeles introuvable : $Modeles (option -Modeles)" }
if (-not $SansProjetReel -and -not (Test-Path -LiteralPath $Projet)) {
  Stop-Demo "projet reel introuvable : $Projet (option -Projet, ou -SansProjetReel pour les fixtures seules)"
}

$env:CASSINI_MODELS = $Modeles
$env:QT_QPA_PLATFORM = "windows"
$env:PYTHONIOENCODING = "utf-8"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Set-Location $Racine
"" | Out-File -Encoding utf8 $Sortie

# Joue la sonde une fois ; rend son code. Le rapport complet va a l'ecran et dans le fichier.
function Invoke-Sonde([string]$Intitule, [string]$ProjetDonne) {
  $env:CASSINI_DEMO_LABEL = $Intitule
  "=== $Intitule ===" | Say
  $lignes = @()
  if ($ProjetDonne) {
    & $Python "qgis-plugin\tests\probe_demo.py" $ProjetDonne 2>&1 |
      ForEach-Object { "$_" } | Tee-Object -Variable lignes | Say
  } else {
    & $Python "qgis-plugin\tests\probe_demo.py" 2>&1 |
      ForEach-Object { "$_" } | Tee-Object -Variable lignes | Say
  }
  $c = $LASTEXITCODE
  if ($null -eq $c) { $c = 2 }
  # Une sonde sortie sans ligne VERDICT n'a rien juge : sonde non partie, pas un ECHEC.
  if (-not ($lignes | Where-Object { $_ -match "VERDICT : (OK|ECHEC)" })) {
    "SONDE NON PARTIE : aucune ligne VERDICT dans la sortie ($Intitule)" | Say
    $c = 2
  }
  return $c
}

$codes = @()
$codes += Invoke-Sonde "fixtures" ""
if ($SansProjetReel) {
  "projet reel : non joue" | Say
} else {
  $codes += Invoke-Sonde "projet reel" (Resolve-Path -LiteralPath $Projet).Path
}

# Un code 2 (sonde sortie sans ligne VERDICT : fixture ou projet illisible, noyau absent) l'emporte sur un ECHEC.
if ($codes | Where-Object { $_ -ne 0 -and $_ -ne 1 }) { $final = 2 }
elseif ($codes | Where-Object { $_ -ne 0 }) { $final = 1 }
else { $final = 0 }
$texte = if ($final -eq 0) { "OK" } else { "ECHEC" }
"RESULTAT GLOBAL : $texte" | Say
Write-Host "Rapport complet : $Sortie (code $final)"
exit $final
