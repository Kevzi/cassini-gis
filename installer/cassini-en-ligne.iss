; Installeur en ligne minimal de Cassini (ticket 7.1, AD-17).
;
; Programme SEPARE du noyau et de l'extension : seul à toucher au réseau, une seule
; fois, pendant l'installation. Il lit le manifeste de la release, télécharge le
; noyau, VERIFIE son SHA-256 et ne pose rien si l'empreinte ne correspond pas.
;
; Compilation :  ISCC.exe /DVersion=0.1.0 installer\cassini-en-ligne.iss
; Test local  :  cassini-installeur-en-ligne.exe /VERYSILENT /MANIFESTE=http://127.0.0.1:8000/manifeste.json
;
; Le manifeste a la mise en forme fixée par installer/manifeste.py (un champ par ligne) :
; Inno Setup n'a pas d'analyseur JSON, on y cherche du texte.

#ifndef Version
  #define Version "0.1.0"
#endif

#define ManifesteParDefaut "https://github.com/Kevzi/cassini-gis/releases/latest/download/manifeste.json"

[Setup]
AppId={{6F1C2B7A-3D0E-4C55-9B1A-CA55141B0001}
AppName=Cassini
AppVersion={#Version}
AppPublisher=Cassini
AppPublisherURL=https://kevzi.github.io/cassini-gis
DefaultDirName={localappdata}\Cassini
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
OutputDir=..\installer-sortie
OutputBaseFilename=cassini-installeur-en-ligne
UninstallDisplayName=Cassini

[Languages]
Name: "french"; MessagesFile: "compiler:Languages\French.isl"

[Files]
; Posé depuis le dossier temporaire, après téléchargement et vérification.
Source: "{tmp}\cassini.exe"; DestDir: "{app}\bin"; Flags: external ignoreversion

[UninstallDelete]
; Le noyau seulement : le dossier du modèle, posé à côté par la porte 1 complète (7.4), est
; à l'utilisateur et ne se supprime jamais à la désinstallation.
Type: filesandordirs; Name: "{app}\bin"

[Code]
type
  TSystemTime = record
    wYear, wMonth, wDayOfWeek, wDay, wHour, wMinute, wSecond, wMilliseconds: Word;
  end;

procedure GetSystemTime(var T: TSystemTime); external 'GetSystemTime@kernel32.dll stdcall';

var
  VersionInstallee: String;

function Pad(N, Width: Integer): String;
begin
  Result := IntToStr(N);
  while Length(Result) < Width do
    Result := '0' + Result;
end;

function DateIsoUtc: String;
var
  T: TSystemTime;
begin
  GetSystemTime(T);
  Result := Pad(T.wYear, 4) + '-' + Pad(T.wMonth, 2) + '-' + Pad(T.wDay, 2) + 'T' +
    Pad(T.wHour, 2) + ':' + Pad(T.wMinute, 2) + ':' + Pad(T.wSecond, 2) + 'Z';
end;

{ La valeur de la chaîne `"Cle": "valeur"` cherchée à partir de Depart (1 = début). }
function ValeurApres(const Json, Cle: String; Depart: Integer): String;
var
  Motif, Reste: String;
  P, Q: Integer;
begin
  Result := '';
  Motif := '"' + Cle + '": "';
  Reste := Copy(Json, Depart, Length(Json));
  P := Pos(Motif, Reste);
  if P = 0 then
    Exit;
  Reste := Copy(Reste, P + Length(Motif), Length(Reste));
  Q := Pos('"', Reste);
  if Q = 0 then
    Exit;
  Result := Copy(Reste, 1, Q - 1);
end;

{ Position de l'entrée du fichier Nom dans le manifeste, 0 si absente. }
function PositionFichier(const Json, Nom: String): Integer;
begin
  Result := Pos('"nom": "' + Nom + '"', Json);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Manifeste, Json, Url, Sha: String;
  Brut: AnsiString;
  Pos0: Integer;
  Source: String;
begin
  Result := '';
  Source := ExpandConstant('{param:MANIFESTE|{#ManifesteParDefaut}}');

  try
    DownloadTemporaryFile(Source, 'manifeste.json', '', nil);
  except
    Result := 'Cassini n''a pas pu lire la liste des versions (' + Source + ').' + #13#10 +
      'Vérifiez la connexion au réseau, puis relancez l''installation.' + #13#10 +
      'Aucun fichier n''a été installé.';
    Exit;
  end;

  Manifeste := ExpandConstant('{tmp}\manifeste.json');
  if not LoadStringFromFile(Manifeste, Brut) then
  begin
    Result := 'La liste des versions de Cassini est illisible. Aucun fichier n''a été installé.';
    Exit;
  end;
  Json := String(Brut);

  Pos0 := PositionFichier(Json, 'cassini.exe');
  Url := '';
  Sha := '';
  if Pos0 > 0 then
  begin
    Url := ValeurApres(Json, 'url', Pos0);
    Sha := Lowercase(ValeurApres(Json, 'sha256', Pos0));
  end;
  VersionInstallee := ValeurApres(Json, 'version', 1);
  if (Url = '') or (Length(Sha) <> 64) or (VersionInstallee = '') then
  begin
    Result := 'La liste des versions de Cassini est incomplète. Aucun fichier n''a été installé.';
    Exit;
  end;

  try
    DownloadTemporaryFile(Url, 'cassini.exe', Sha, nil);
  except
    Result := 'Le téléchargement du moteur de Cassini a échoué ou son empreinte SHA-256 ' +
      'ne correspond pas à celle attendue.' + #13#10 +
      'Par sécurité, rien n''a été installé.' + #13#10 + GetExceptionMessage;
    DeleteFile(ExpandConstant('{tmp}\cassini.exe'));
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    SaveStringToFile(ExpandConstant('{app}\bin\installation.json'),
      '{"version": "' + VersionInstallee + '", "installe_le": "' + DateIsoUtc + '"}' + #10, False);
end;
