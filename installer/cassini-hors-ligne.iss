; Installeur complet hors ligne de Cassini, porte 2 (ticket 7.2, AD-17).
;
; UN SEUL fichier qui contient le noyau, le modele et l'archive de l'extension QGIS.
; Aucun acces au reseau, ni a l'installation ni apres : ce script n'a aucune fonction
; reseau (une garde de installer/tests le verifie). Memes emplacements que l'installeur
; en ligne : le noyau dans {app}\bin, le modele dans {app}\bin\models (le noyau le cherche
; a cote de cassini.exe), installation.json a cote du noyau.
;
; Taille : un seul fichier tient jusqu'a 4,2 Go compresses (documentation d'Inno Setup,
; DiskSpanning=no) ; le modele (~2,78 Go) ne se comprime presque pas, donc Compression=none.
; installer/verifier_taille.py fait echouer la recette si le fichier approche la limite.
;
; Compilation (depuis la racine du depot) :
;   ISCC.exe /DVersion=0.1.0 installer\cassini-hors-ligne.iss
; avec dist\cassini.exe, modele\cassini-composition-4b-Q4_K_M.gguf et
; dist\cassini_qgis-0.1.0.zip deja produits par la recette.

#ifndef Version
  #define Version "0.1.0"
#endif
#ifndef Noyau
  #define Noyau "..\dist\cassini.exe"
#endif
#ifndef Modele
  #define Modele "..\modele\cassini-composition-4b-Q4_K_M.gguf"
#endif
#ifndef Archive
  #define Archive "..\dist\cassini_qgis-" + Version + ".zip"
#endif

#define NomModele "cassini-composition-4b-Q4_K_M.gguf"
#define Declaration "cassini-composition-4b.modele.json"

[Setup]
; AppId identique a celui de l'installeur en ligne, voulu : un seul Cassini par utilisateur,
; chaque porte met a jour l'autre, et le desinstalleur ne retire que ce que le dernier
; installeur a pose.
AppId={{6F1C2B7A-3D0E-4C55-9B1A-CA55141B0001}
AppName=Cassini
AppVersion={#Version}
AppPublisher=Cassini
DefaultDirName={localappdata}\Cassini
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
; Un seul fichier, sans DiskSpanning ; le modele est deja quantifie.
Compression=none
DiskSpanning=no
WizardStyle=modern
OutputDir=..\installer-sortie
OutputBaseFilename=cassini-installeur-complet
UninstallDisplayName=Cassini

[Languages]
Name: "french"; MessagesFile: "compiler:Languages\French.isl"

[Files]
Source: "{#Noyau}"; DestDir: "{app}\bin"; DestName: "cassini.exe"; Flags: ignoreversion
Source: "{#Modele}"; DestDir: "{app}\bin\models"; DestName: "{#NomModele}"; Flags: ignoreversion
; Archive de l'extension, a installer a la main dans QGIS (Extensions, Installer depuis un ZIP).
Source: "{#Archive}"; DestDir: "{app}\extension"; Flags: ignoreversion

[UninstallDelete]
; Les fichiers installes se desinstallent seuls ; ne restent que ceux ecrits par le code.
Type: files; Name: "{app}\bin\installation.json"
Type: files; Name: "{app}\bin\models\{#Declaration}"

[Code]
type
  TSystemTime = record
    wYear, wMonth, wDayOfWeek, wDay, wHour, wMinute, wSecond, wMilliseconds: Word;
  end;

procedure GetSystemTime(var T: TSystemTime); external 'GetSystemTime@kernel32.dll stdcall';

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

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    { Declaration minimale du modele, comme sur le poste du proprietaire ; les empreintes
      viennent de 7.4. }
    SaveStringToFile(ExpandConstant('{app}\bin\models\{#Declaration}'),
      '{"model": "{#NomModele}", "prompt": "short"}' + #10, False);
    { Meme format que l'installeur en ligne (7.1). }
    SaveStringToFile(ExpandConstant('{app}\bin\installation.json'),
      '{"version": "{#Version}", "installe_le": "' + DateIsoUtc + '"}' + #10, False);
  end;
end;
