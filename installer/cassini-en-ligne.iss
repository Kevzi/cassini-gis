; Installeur en ligne minimal de Cassini (ticket 7.1, AD-17).
;
; Programme SEPARE du noyau et de l'extension : seul à toucher au réseau, une seule
; fois, pendant l'installation. Il lit le manifeste de la release, télécharge le
; paquet du noyau, VERIFIE le SHA-256 de chaque fichier et ne pose rien si l'une diffère.
;
; Compilation :  ISCC.exe /DVersion=0.1.1 installer\cassini-en-ligne.iss
; Test local  :  cassini-installeur-en-ligne.exe /VERYSILENT /MANIFESTE=http://127.0.0.1:8000/manifeste.json
;
; Mise à jour (ticket 5 de l'epic publication) : le MEME programme, lancé avec /MISEAJOUR (raccourci
; « Mettre à jour Cassini » du menu Démarrer, qui lance la copie {app}\mettre-a-jour-cassini.exe).
; Il lit le manifeste, compare à {app}\bin\installation.json, demande confirmation, pose le noyau
; et un nouveau modèle sous son propre nom (jamais à la place d'un ancien) ; installé à jour : rien
; n'est téléchargé ni réécrit, code 0. Tout se lance à la main : aucune vérification automatique.
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
; Le modèle, facultatif (porte 1 complète, 7.4) : téléchargé et vérifié dans PrepareToInstall,
; posé seulement si la case est cochée et que le manifeste le propose. `uninsneveruninstall` :
; un modèle posé n'est jamais retiré par la désinstallation ni remplacé par une mise à jour d'un
; autre nom (un ancien modèle reste à côté).
; `onlyifdoesntexist` : un fichier de `models` n'est jamais écrasé.
Source: "{tmp}\modele\*"; DestDir: "{app}\bin\models"; Flags: external ignoreversion uninsneveruninstall onlyifdoesntexist; Check: ModelePret
; Copie de cet installeur, que lance le raccourci « Mettre à jour Cassini » (/MISEAJOUR). Pas de
; copie quand on tourne déjà depuis elle (un exécutable en cours ne se remplace pas) : elle reste
; celle de la dernière installation par l'installeur d'origine.
Source: "{srcexe}"; DestDir: "{app}"; DestName: "mettre-a-jour-cassini.exe"; Flags: external ignoreversion; Check: CopieInstalleurUtile

[Icons]
Name: "{autoprograms}\Mettre à jour Cassini"; Filename: "{app}\mettre-a-jour-cassini.exe"; Parameters: "/MISEAJOUR"; Comment: "Met à jour le noyau et le modèle de Cassini"

[UninstallDelete]
; Les fichiers du noyau seulement (jamais le dossier `models` : le modèle est à l'utilisateur
; et ne se supprime jamais à la désinstallation, ni sa déclaration).
Type: files; Name: "{app}\bin\*"

[Code]
type
  TSystemTime = record
    wYear, wMonth, wDayOfWeek, wDay, wHour, wMinute, wSecond, wMilliseconds: Word;
  end;

procedure GetSystemTime(var T: TSystemTime); external 'GetSystemTime@kernel32.dll stdcall';

const
  { Même nom que la déclaration de l'installeur complet (porte 2) : une porte met l'autre à jour. }
  DeclarationModele = 'cassini-composition-4b.modele.json';

procedure ExitProcess(Code: Cardinal); external 'ExitProcess@kernel32.dll stdcall';

var
  VersionAPoser, VersionPresente: String;
  PageModele: TInputOptionWizardPage;
  NomModele, ShaModele: String;
  ModeleTelecharge: Boolean;

{ Lancé avec /MISEAJOUR ? (interrupteur sans valeur : on parcourt la ligne de commande.) }
function ModeMiseAJour: Boolean;
var
  I: Integer;
begin
  Result := False;
  for I := 1 to ParamCount do
    if CompareText(ParamStr(I), '/MISEAJOUR') = 0 then
      Result := True;
end;

{ Au moins un modèle (.gguf) déjà posé dans models ? }
function ModeleDejaPresent: Boolean;
var
  R: TFindRec;
begin
  Result := False;
  if FindFirst(ExpandConstant('{app}\bin\models\*.gguf'), R) then
  begin
    Result := True;
    FindClose(R);
  end;
end;

{ Pas de copie de l'installeur sur lui-même (mise à jour lancée depuis le raccourci). }
function CopieInstalleurUtile: Boolean;
begin
  Result := CompareText(ExpandConstant('{srcexe}'), ExpandConstant('{app}\mettre-a-jour-cassini.exe')) <> 0;
end;

{ Une partie (0 = majeure, 1, 2) d'une version « a.b.c » ; une partie absente vaut 0, une partie
  vide ou non numérique rend -1. }
function PartieVersion(V: String; N: Integer): Integer;
var
  I, P: Integer;
  Part: String;
begin
  Part := '0';
  for I := 0 to N do
  begin
    if V = '' then
    begin
      Part := '0';
      Break;
    end;
    P := Pos('.', V);
    if P = 0 then
    begin
      Part := V;
      V := '';
    end
    else
    begin
      Part := Copy(V, 1, P - 1);
      V := Copy(V, P + 1, Length(V));
    end;
  end;
  Result := StrToIntDef(Part, -1);
end;

{ Compare deux versions : -1 si A < B, 0 si égales, 1 si A > B, -2 si l'une est illisible. }
function ComparerVersions(const A, B: String): Integer;
var
  I, X, Y: Integer;
begin
  Result := 0;
  if (A = '') or (B = '') then
  begin
    Result := -2;
    Exit;
  end;
  for I := 0 to 2 do
  begin
    X := PartieVersion(A, I);
    Y := PartieVersion(B, I);
    if (X < 0) or (Y < 0) then
    begin
      Result := -2;
      Exit;
    end;
    if X < Y then
    begin
      Result := -1;
      Exit;
    end;
    if X > Y then
    begin
      Result := 1;
      Exit;
    end;
  end;
end;

{ Case « modèle » cochée par défaut ; /MODELE=0 ou /MODELE=1 pour l'installation silencieuse. }
procedure InitializeWizard;
begin
  PageModele := CreateInputOptionPage(wpWelcome, 'Modèle de langage',
    'Installer aussi le modèle local ?',
    'Le modèle (environ 2,8 Go) permet à Cassini de comprendre vos demandes en français, ' +
    'sans réseau. Sans lui, seul le noyau est installé. Il est téléchargé une seule fois, ' +
    'pendant l''installation, si cette version de Cassini le propose.',
    False, False);
  PageModele.Add('Télécharger et installer le modèle');
  PageModele.Values[0] := ExpandConstant('{param:MODELE|1}') <> '0';
  { En mise à jour, le modèle n'est proposé par défaut que si l'utilisateur en avait déjà un
    (/MODELE=0 ou /MODELE=1 reste prioritaire). }
  if ModeMiseAJour and (ExpandConstant('{param:MODELE|}') = '') then
    PageModele.Values[0] := ModeleDejaPresent;
end;

function ModelePret: Boolean;
begin
  Result := ModeleTelecharge;
end;

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

{ La section « modele » du manifeste (vide si absente), coupée avant « porte2 ». }
function SectionModele(const Json: String): String;
var
  P: Integer;
begin
  Result := '';
  P := Pos('"modele"', Json);
  if P = 0 then
    Exit;
  Result := Copy(Json, P, Length(Json));
  P := Pos('"porte2"', Result);
  if P > 0 then
    Result := Copy(Result, 1, P - 1);
end;

{ La version écrite dans {app}\bin\installation.json ; vide si le fichier manque ou est illisible. }
function LireVersionPresente: String;
var
  Brut: AnsiString;
begin
  Result := '';
  if LoadStringFromFile(ExpandConstant('{app}\bin\installation.json'), Brut) then
    Result := ValeurApres(String(Brut), 'version', 1);
end;

{ Le dossier bin est-il en cours d'usage (QGIS ouvert : cassini.exe ou une DLL chargée) ?
  Un dossier qui contient un fichier ouvert ne se renomme pas : on le renomme puis on le remet. }
function NoyauEnCoursDUsage: Boolean;
var
  Bin, Essai: String;
begin
  Result := False;
  Bin := ExpandConstant('{app}\bin');
  Essai := ExpandConstant('{app}\bin-verif');
  if not DirExists(Bin) then
    Exit;
  if RenameFile(Bin, Essai) then
  begin
    { Impossible de remettre le dossier : on refuse (rien n'est installé) plutôt que de continuer sans bin. }
    if not RenameFile(Essai, Bin) then
    begin
      Result := True;
      MsgBox('Cassini : le dossier ' + Essai + ' doit être renommé en bin à la main, puis la mise à jour relancée. ' +
        'Aucun fichier n''a été posé.', mbError, MB_OK);
    end;
  end
  else
    Result := True;
end;

procedure NettoyerNoyau;
begin
  DelTree(ExpandConstant('{tmp}\noyau'), True, True, True);
  DelTree(ExpandConstant('{tmp}\modele'), True, True, True);
  ModeleTelecharge := False;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Manifeste, Json, Reste, Nom, Url, Sha, ShaBrut, Dest, Source: String;
  Brut: AnsiString;
  Nombre, P, DebutModele, DebutPorte2, Cmp: Integer;
  AvecNoyau, ModelePresent, ModeleVoulu: Boolean;
  Section, Fichiers, NomManifeste, Question: String;
begin
  Result := '';
  ModeleTelecharge := False;
  Source := ExpandConstant('{param:MANIFESTE|{#ManifesteParDefaut}}');

  { Mise à jour d'un Cassini qui n'est pas installé : rien à faire, code non nul en silencieux. }
  if ModeMiseAJour and (not FileExists(ExpandConstant('{app}\bin\installation.json'))) and
     (not FileExists(ExpandConstant('{app}\bin\cassini.exe'))) then
  begin
    Result := 'Cassini n''est pas installé. Lancez d''abord l''installeur de Cassini.' + #13#10 +
      'Aucun fichier n''a été posé.';
    Exit;
  end;

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

  VersionAPoser := ValeurApres(Json, 'version', 1);
  if VersionAPoser = '' then
  begin
    Result := 'La liste des versions de Cassini est incomplète. Aucun fichier n''a été installé.';
    Exit;
  end;

  if ModeMiseAJour then
  begin
    VersionPresente := LireVersionPresente;
    ModeleVoulu := PageModele.Values[0];
    NomManifeste := ValeurApres(SectionModele(Json), 'nom', 1);
    { Le nom versionné du modèle dit s'il est déjà là : pas de calcul d'empreinte de 2,8 Go. }
    ModelePresent := (not ModeleVoulu) or (NomManifeste = '') or
      FileExists(ExpandConstant('{app}\bin\models\') + NomManifeste);
    Cmp := ComparerVersions(VersionPresente, VersionAPoser);
    { Installée plus récente : à jour. Égale : à jour si le modèle du manifeste est là.
      Plus ancienne ou illisible : mise à jour proposée. }
    if (Cmp = 1) or ((Cmp = 0) and ModelePresent) then
    begin
      if not WizardSilent then
        MsgBox('Cassini est à jour (version ' + VersionPresente + ').', mbInformation, MB_OK);
      ExitProcess(0);
    end;
    if Cmp = -2 then
      Question := 'Mettre à jour Cassini vers la version ' + VersionAPoser + ' ?'
    else if Cmp = 0 then
      Question := 'Cassini ' + VersionPresente + ' est installé ; le nouveau modèle sera ajouté à côté de l''ancien. Continuer ?'
    else
      Question := 'Mettre à jour Cassini de la version ' + VersionPresente + ' vers la version ' +
        VersionAPoser + ' ?';
    if not ModelePresent then
      Question := Question + #13#10#13#10 + 'Le nouveau modèle (' + NomManifeste +
        ') sera téléchargé et ajouté ; l''ancien modèle reste.';
    { En silencieux, pas de dialogue : la confirmation vaut « oui ». }
    if (not WizardSilent) and (MsgBox(Question, mbConfirmation, MB_YESNO) = IDNO) then
      ExitProcess(0);
    if NoyauEnCoursDUsage then
    begin
      Result := 'Cassini est en cours d''utilisation. Fermez QGIS et toute fenêtre ouverte sur le dossier de Cassini, puis relancez la mise à jour.' +
        #13#10 + 'Aucun fichier n''a été posé.';
      Exit;
    end;
  end;

  NettoyerNoyau;
  ForceDirectories(ExpandConstant('{tmp}\noyau'));
  Dest := ExpandConstant('{tmp}\noyau\');
  Nombre := 0;
  AvecNoyau := False;
  { La boucle des fichiers s'arrête avant la section « modele » (et « porte2 », qui la suit) :
    ni l'un ni l'autre ne se pose dans le dossier du noyau. }
  Fichiers := Json;
  DebutModele := Pos('"modele"', Json);
  DebutPorte2 := Pos('"porte2"', Json);
  if (DebutPorte2 > 0) and ((DebutModele = 0) or (DebutPorte2 < DebutModele)) then
    DebutModele := DebutPorte2;
  if DebutModele > 0 then
    Fichiers := Copy(Json, 1, DebutModele - 1);
  Reste := Fichiers;
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
    Exit;
  end;

  { Le modèle, facultatif : seulement si la case est cochée et si le manifeste a une section
    « modele » (un ancien manifeste n'en a pas : noyau seul, sans erreur). Même règle que pour
    le noyau : une empreinte fausse, une adresse morte ou un octet changé refusent TOUTE
    l'installation (le noyau compris), rien n'est posé. }
  if (not PageModele.Values[0]) or (Pos('"modele"', Json) = 0) then
    Exit;
  Section := SectionModele(Json);
  NomModele := ValeurApres(Section, 'nom', 1);
  Url := ValeurApres(Section, 'url', 1);
  ShaModele := Lowercase(ValeurApres(Section, 'sha256', 1));
  if (not NomSain(NomModele)) or (Url = '') or (Length(ShaModele) <> 64) or
     (Lowercase(Copy(NomModele, Length(NomModele) - 4, 5)) <> '.gguf') then
  begin
    NettoyerNoyau;
    Result := 'La liste des versions de Cassini est incomplète (modèle). Aucun fichier n''a été installé.';
    Exit;
  end;
  { Ce modèle est déjà là (même nom) : ni téléchargé ni touché, un fichier de models n'est jamais écrasé. }
  if FileExists(ExpandConstant('{app}\bin\models\') + NomModele) then
    Exit;
  try
    DownloadTemporaryFile(Url, NomModele, ShaModele, nil);
  except
    Result := 'Le téléchargement du modèle ' + NomModele + ' a échoué ou son empreinte SHA-256 ' +
      'ne correspond pas à celle attendue.' + #13#10 +
      'Par sécurité, rien n''a été installé (ni le noyau, ni le modèle).' + #13#10 + GetExceptionMessage;
    DeleteFile(ExpandConstant('{tmp}\') + NomModele);
    NettoyerNoyau;
    Exit;
  end;
  ForceDirectories(ExpandConstant('{tmp}\modele'));
  { Renommer plutôt que copier : le fichier fait plusieurs Go. }
  if not RenameFile(ExpandConstant('{tmp}\') + NomModele, ExpandConstant('{tmp}\modele\') + NomModele) then
  begin
    DeleteFile(ExpandConstant('{tmp}\') + NomModele);
    NettoyerNoyau;
    Result := 'Cassini n''a pas pu préparer ' + NomModele + '. Aucun fichier n''a été installé.';
    Exit;
  end;
  ModeleTelecharge := True;
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  NomDecl: String;
  Existant: AnsiString;
begin
  if CurStep = ssPostInstall then
  begin
    { Déclaration du modèle avec son empreinte (même forme que celle de la porte 2). Nom fixe,
      sauf si une déclaration de ce nom existe déjà et ne nomme pas ce modèle (mise à jour avec
      un autre modèle) : elle est alors laissée intacte et la nouvelle prend le nom du modèle. }
    if ModeleTelecharge then
    begin
      NomDecl := DeclarationModele;
      if FileExists(ExpandConstant('{app}\bin\models\') + NomDecl) and
         LoadStringFromFile(ExpandConstant('{app}\bin\models\') + NomDecl, Existant) and
         (Pos(NomModele, String(Existant)) = 0) then
        NomDecl := Copy(NomModele, 1, Length(NomModele) - 5) + '.modele.json';
      SaveStringToFile(ExpandConstant('{app}\bin\models\') + NomDecl,
        '{"model": "' + NomModele + '", "prompt": "short", "model_sha256": "' + ShaModele + '"}' + #10, False);
    end;
    SaveStringToFile(ExpandConstant('{app}\bin\installation.json'),
      '{"version": "' + VersionAPoser + '", "installe_le": "' + DateIsoUtc + '"}' + #10, False);
  end;
end;
