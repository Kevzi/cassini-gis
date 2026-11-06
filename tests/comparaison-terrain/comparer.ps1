# Comparaison des commandes de terrain du noyau à QGIS et GRASS (ticket 7, epic raster et terrain).
#
# Lance, en chronométrant, le noyau (pente, exposition, ombrage, trois champs de vision, trente
# lignes de vue) puis QGIS (native:slope, native:aspect, native:hillshade, trois
# grass:r.viewshed), plus un appel à vide de chacun ; normalise les
# sorties de QGIS en GeoTIFF Float32 non compressé ; écrit temps.json. Tout est écrit dans le
# dossier de travail, hors du dépôt. Mode d'emploi : LISEZMOI.md, à côté.
#
# Paramètres communs aux deux outils : observateur 1,6 m, cible 0 m, courbure et réfraction
# actives, coefficient 0,13, rayon 10 km ; facteur Z 1, soleil à 315° et 45° pour l'ombrage. Les points viennent de points.json, écrit avant par
# `cargo run --release -p cassini-core --example comparer_terrain -- points <mnt>`.

param(
    [Parameter(Mandatory = $true)] [string] $Travail,
    [string] $Mnt = '',
    [string] $Cassini = 'C:\ct\release\cassini.exe',
    [string] $Qgis = 'C:\Program Files\QGIS 3.44.15'
)

$ErrorActionPreference = 'Stop'
# Windows PowerShell 5.1 ne connaît pas $PSScriptRoot dans les valeurs par défaut des paramètres.
if (-not $Mnt) {
    $Mnt = Join-Path $PSScriptRoot '..\..\..\donnees-test-cassini\dem\mnt-villeneuve-le-roi-2154.tif'
}
$Travail = (Resolve-Path $Travail).Path
$points = Get-Content -Raw -Encoding UTF8 (Join-Path $Travail 'points.json') | ConvertFrom-Json
$qgisProcess = Join-Path $Qgis 'bin\qgis_process-qgis-ltr.bat'
$gdalTranslate = Join-Path $Qgis 'bin\gdal_translate.exe'
$env:PROJ_DATA = Join-Path $Qgis 'share\proj'
$env:PROJ_LIB = $env:PROJ_DATA
$env:GDAL_DATA = Join-Path $Qgis 'apps\gdal\share\gdal'

Copy-Item -Force $Mnt (Join-Path $Travail 'mnt.tif')
$mntTif = Join-Path $Travail 'mnt.tif'
$projet = Join-Path $Travail 'projet.qgz'
Remove-Item -Recurse -Force -ErrorAction SilentlyContinue (Join-Path $Travail 'projet.cassini')
Get-ChildItem $Travail -Filter '*-*.tif' | Remove-Item -Force

$temps = [ordered]@{ noyau = [ordered]@{}; qgis = [ordered]@{} }

function Mesurer([scriptblock] $bloc) {
    $chrono = [Diagnostics.Stopwatch]::StartNew()
    & $bloc
    [math]::Round($chrono.Elapsed.TotalSeconds, 3)
}

# Windows PowerShell retire les guillemets d'un argument passé à un exécutable : on les échappe.
function Noyau([string] $id, $params, [switch] $SansProjet) {
    $json = ($params | ConvertTo-Json -Compress -Depth 6) -replace '"', '\"'
    if ($SansProjet) {
        $sortie = & $Cassini run $id $json
    } else {
        $sortie = & $Cassini run $id $json --project $projet
    }
    if ($LASTEXITCODE -ne 0) { throw "cassini run $id a échoué." }
    $sortie | ConvertFrom-Json
}

function Copier-Sortie($reponse, [string] $nom) {
    $fichier = $reponse.file
    if (-not [IO.Path]::IsPathRooted($fichier)) { $fichier = Join-Path $Travail $fichier }
    Copy-Item -Force $fichier (Join-Path $Travail $nom)
}

function Qgis([string[]] $arguments) {
    & $qgisProcess @arguments | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "qgis_process $($arguments -join ' ') a échoué." }
}

function Normaliser([string] $brut, [string] $nom) {
    & $gdalTranslate -q -of GTiff -co COMPRESS=NONE -ot Float32 -a_srs EPSG:2154 $brut (Join-Path $Travail $nom)
    if ($LASTEXITCODE -ne 0) { throw "gdal_translate $brut a échoué." }
}

function Point($p) { @{ x = $p.x; y = $p.y; epsg = 2154 } }
$oeil = @{ value = 1.6; unit = 'm' }
$cible = @{ value = 0; unit = 'm' }

# --- Noyau -------------------------------------------------------------------------------
Write-Host 'Noyau : appel à vide, pente, exposition, ombrage, champs de vision, lignes de vue'
$temps.noyau['vide'] = Mesurer { & $Cassini run system.about | Out-Null }
$script:r = $null
$temps.noyau['pente'] = Mesurer { $script:r = Noyau 'terrain.slope' @{ dem = 'mnt.tif'; unit = 'degrees' } }
Copier-Sortie $script:r 'noyau-pente.tif'
$temps.noyau['exposition'] = Mesurer { $script:r = Noyau 'terrain.aspect' @{ dem = 'mnt.tif' } }
Copier-Sortie $script:r 'noyau-exposition.tif'
$temps.noyau['ombrage'] = Mesurer { $script:r = Noyau 'terrain.hillshade' @{ dem = 'mnt.tif' } }
Copier-Sortie $script:r 'noyau-ombrage.tif'
foreach ($o in $points.observateurs) {
    $params = @{
        dem = 'mnt.tif'; observer = (Point $o); observer_height = $oeil; target_height = $cible
        radius = @{ value = 10; unit = 'km' }; curvature = $true; refraction = 0.13
    }
    $temps.noyau["vue-$($o.nom)"] = Mesurer { $script:r = Noyau 'terrain.viewshed' $params }
    Copier-Sortie $script:r "noyau-vue-$($o.nom).tif"
}
$temps.noyau['lignes-de-vue-30'] = Mesurer {
    foreach ($c in $points.cibles) {
        $o = $points.observateurs | Where-Object { $_.nom -eq $c.observateur }
        Noyau 'terrain.line_of_sight' @{
            dem = $mntTif; observer = (Point $o); target = (Point $c); observer_height = $oeil
            target_height = $cible; curvature = $true; refraction = 0.13
        } -SansProjet | Out-Null
    }
}

# --- QGIS et GRASS -----------------------------------------------------------------------
Write-Host 'QGIS : appel à vide, native:slope, native:aspect, native:hillshade, grass:r.viewshed'
$temps.qgis['vide'] = Mesurer { Qgis @('help', 'native:slope') }
$brut = Join-Path $Travail 'brut-qgis-pente.tif'
$temps.qgis['pente'] = Mesurer { Qgis @('run', 'native:slope', '--', "INPUT=$mntTif", 'Z_FACTOR=1', "OUTPUT=$brut") }
Normaliser $brut 'qgis-pente.tif'
$brut = Join-Path $Travail 'brut-qgis-exposition.tif'
$temps.qgis['exposition'] = Mesurer { Qgis @('run', 'native:aspect', '--', "INPUT=$mntTif", 'Z_FACTOR=1', "OUTPUT=$brut") }
Normaliser $brut 'qgis-exposition.tif'
$brut = Join-Path $Travail 'brut-qgis-ombrage.tif'
$temps.qgis['ombrage'] = Mesurer {
    Qgis @('run', 'native:hillshade', '--', "INPUT=$mntTif", 'Z_FACTOR=1', 'AZIMUTH=315', 'V_ANGLE=45', "OUTPUT=$brut")
}
Normaliser $brut 'qgis-ombrage.tif'
foreach ($o in $points.observateurs) {
    $brut = Join-Path $Travail "brut-qgis-vue-$($o.nom).tif"
    $arguments = @(
        'run', 'grass:r.viewshed', '--', "input=$mntTif", "coordinates=$($o.x),$($o.y)",
        'observer_elevation=1.6', 'target_elevation=0', 'max_distance=10000',
        'refraction_coeff=0.13', '-c=1', '-r=1', '-b=1', '-e=0', "output=$brut"
    )
    $temps.qgis["vue-$($o.nom)"] = Mesurer { Qgis $arguments }
    Normaliser $brut "qgis-vue-$($o.nom).tif"
}

$temps | ConvertTo-Json -Depth 4 | Out-File -Encoding utf8 (Join-Path $Travail 'temps.json')
Write-Host "Fait : $Travail\temps.json"
