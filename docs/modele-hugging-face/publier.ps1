# Publie le modèle de Cassini sur Hugging Face (ticket 7.3).
#
# Préalables, à faire une fois par le propriétaire : un compte Hugging Face nommé « Kevzi », puis
# `hf auth login` avec un jeton à droits d'écriture (saisi dans le terminal, jamais dans un chat).
#
# Usage :  powershell -File docs\modele-hugging-face\publier.ps1
#
# Le dépôt est créé public s'il n'existe pas ; le modèle (2,65 Go) n'est jamais copié dans ce dépôt.

$ErrorActionPreference = 'Stop'

$repo = 'Kevzi/cassini-composition-4b-GGUF'
$nom = 'cassini-composition-4b-0.1.0-Q4_K_M.gguf'
$sha = 'b687313f4a49c23e13673d14d185f3ee56d3e710e8d01c08595339236b21a865'
$modele = Join-Path $env:LOCALAPPDATA 'Cassini\bin\models\cassini-composition-4b-Q4_K_M.gguf'
$ici = $PSScriptRoot
$racine = Split-Path (Split-Path $ici)

if (-not (Test-Path $modele)) { throw "Modèle introuvable : $modele" }
$reel = (Get-FileHash $modele -Algorithm SHA256).Hash.ToLower()
if ($reel -ne $sha) { throw "Le modèle installé n'est pas celui décrit par la fiche ($reel)." }

hf repos create $repo --repo-type model --no-private --exist-ok
if ($LASTEXITCODE -ne 0) { throw 'création du dépôt refusée' }

hf upload $repo $modele $nom
hf upload $repo (Join-Path $ici 'README.md') 'README.md'
hf upload $repo (Join-Path $ici 'NOTICE') 'NOTICE'
hf upload $repo (Join-Path $racine 'LICENSE') 'LICENSE'
if ($LASTEXITCODE -ne 0) { throw 'envoi refusé' }

Write-Host "Publié : https://huggingface.co/$repo"
