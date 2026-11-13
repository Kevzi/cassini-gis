; Installeur en ligne minimal de Cassini (ticket 7.1, AD-17).
;
; Programme SEPARE du noyau et de l'extension : seul à toucher au réseau, une seule
; fois, pendant l'installation. Il lit le manifeste de la release, télécharge le
; paquet du noyau, VERIFIE le SHA-256 de chaque fichier et ne pose rien si l'une diffère.
;
; Compilation :  ISCC.exe /DVersion=0.1.1 installer\cassini-en-ligne.iss
; Test local  :  cassini-installeur-en-ligne.exe /VERYSILENT /MANIFESTE=http://127.0.0.1:8000/manifeste.json
;
; Le manifeste a la mise en forme fixée par installer/manifeste.py (un champ par ligne) :
; Inno Setup n'a pas d'analyseur JSON, on y cherche du texte.

#ifndef Version
  #define Version "0.1.1"
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
; Tout le paquet du noyau (cassini.exe et les DLL de llama.cpp), posé depuis le dossier
; temporaire après téléchargement et vérification de chaque empreinte.
Source: "{tmp}\noyau\*"; DestDir: "{app}\bin"; Flags: external ignoreversion

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

{ Un nom de fichier du manifeste : lettres, chiffres, point, tiret, souligné, rien d'autre
  (jamais de séparateur de dossier). }
function NomSain(const Nom: String): Boolean;
var
  I: Integer;
  C: Char;
begin
  Result := (Nom <> '') and (Nom <> '.') and (Nom <> '..');
  for I := 1 to Length(Nom) do
  begin
    C := Nom[I];
    if not (((C >= 'a') and (C <= 'z')) or ((C >= 'A') and (C <= 'Z')) or
            ((C >= '0') and (C <= '9')) or (C = '.') or (C = '-') or (C = '_')) then
      Result := False;
  end;
end;

procedure NettoyerNoyau;
begin
  DelTree(ExpandConstant('{tmp}\noyau'), True, True, True);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Manifeste, Json, Reste, Nom, Url, Sha, ShaBrut, Dest, Source: String;
  Brut: AnsiString;
  Nombre, P: Integer;
  AvecNoyau: Boolean;
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

  VersionInstallee := ValeurApres(Json, 'version', 1);
  if VersionInstallee = '' then
  begin
    Result := 'La liste des versions de Cassini est incomplète. Aucun fichier n''a été installé.';
    Exit;
  end;

  NettoyerNoyau;
  ForceDirectories(ExpandConstant('{tmp}\noyau'));
  Dest := ExpandConstant('{tmp}\noyau\');
  Nombre := 0;
  AvecNoyau := False;
  Reste := Json;
  { Une passe par entrée du manifeste : chaque fichier est téléchargé et son SHA-256 vérifié ;
    la pose ne vient qu'après PrepareToInstall, donc rien n'est posé si l'un échoue. }
  while True do
  begin
    Nom := ValeurApres(Reste, 'nom', 1);
    if Nom = '' then
      Break;
    Url := ValeurApres(Reste, 'url', 1);
    ShaBrut := ValeurApres(Reste, 'sha256', 1);
    Sha := Lowercase(ShaBrut);
    if (not NomSain(Nom)) or (Url = '') or (Length(Sha) <> 64) then
    begin
      NettoyerNoyau;
      Result := 'La liste des versions de Cassini est incomplète. Aucun fichier n''a été installé.';
      Exit;
    end;
    try
      DownloadTemporaryFile(Url, Nom, Sha, nil);
    except
      Result := 'Le téléchargement de ' + Nom + ' a échoué ou son empreinte SHA-256 ' +
        'ne correspond pas à celle attendue.' + #13#10 +
        'Par sécurité, rien n''a été installé.' + #13#10 + GetExceptionMessage;
      DeleteFile(ExpandConstant('{tmp}\') + Nom);
      NettoyerNoyau;
      Exit;
    end;
    if not FileCopy(ExpandConstant('{tmp}\') + Nom, Dest + Nom, False) then
    begin
      NettoyerNoyau;
      Result := 'Cassini n''a pas pu préparer ' + Nom + '. Aucun fichier n''a été installé.';
      Exit;
    end;
    DeleteFile(ExpandConstant('{tmp}\') + Nom);
    if Nom = 'cassini.exe' then
      AvecNoyau := True;
    Nombre := Nombre + 1;
    { Passe à l'entrée suivante : après le sha256, dernier champ de l'entrée (ordre nom, url,
      sha256) ; s'arrêter après le seul nom laisserait l'url et le sha256 de cette entrée
      devant la suivante. }
    P := Pos('"sha256": "' + ShaBrut + '"', Reste);
    if P = 0 then
    begin
      NettoyerNoyau;
      Result := 'La liste des versions de Cassini est incomplète. Aucun fichier n''a été installé.';
      Exit;
    end;
    Reste := Copy(Reste, P + 11 + Length(ShaBrut), Length(Reste));
  end;

  if (Nombre = 0) or (not AvecNoyau) then
  begin
    NettoyerNoyau;
    Result := 'La liste des versions de Cassini est incomplète. Aucun fichier n''a été installé.';
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    SaveStringToFile(ExpandConstant('{app}\bin\installation.json'),
      '{"version": "' + VersionInstallee + '", "installe_le": "' + DateIsoUtc + '"}' + #10, False);
end;
