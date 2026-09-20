#define MyAppName "Mesh"
#define MyAppPublisher "odevsa"
#define MyAppURL "https://github.com/odevsa/mesh"
#define MyAppExeName "mesh.exe"

#ifndef MyAppVersion
#define MyAppVersion "0.1.6"
#endif

#ifndef TargetArch
#define TargetArch "x86_64"
#endif

#ifndef TargetTriplet
#define TargetTriplet "x86_64-pc-windows-msvc"
#endif

[Setup]
AppId={{C6B8D142-9D09-4C57-8E82-9B5A81775E91}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
SourceDir=..\..
SetupIconFile=assets\mesh.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ChangesAssociations=yes
OutputBaseFilename=mesh-{#MyAppVersion}-windows-{#TargetArch}-setup
OutputDir=.

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "fileassoc_stl"; Description: "Associate STL 3D models (.stl)"; GroupDescription: "File Associations:"
Name: "fileassoc_3mf"; Description: "Associate 3MF 3D models (.3mf)"; GroupDescription: "File Associations:"
Name: "fileassoc_obj"; Description: "Associate Wavefront OBJ files (.obj)"; GroupDescription: "File Associations:"
Name: "fileassoc_gltf"; Description: "Associate glTF 3D models (.gltf)"; GroupDescription: "File Associations:"
Name: "fileassoc_glb"; Description: "Associate GLB 3D models (.glb)"; GroupDescription: "File Associations:"
Name: "fileassoc_fbx"; Description: "Associate Autodesk FBX models (.fbx)"; GroupDescription: "File Associations:"

[Files]
Source: "target\{#TargetTriplet}\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
; .stl
Root: HKA; Subkey: "Software\Classes\.stl"; ValueType: string; ValueData: "Mesh.stl"; Flags: uninsdeletevalue; Tasks: fileassoc_stl
Root: HKA; Subkey: "Software\Classes\Mesh.stl"; ValueType: string; ValueData: "3D STL Model"; Flags: uninsdeletekey; Tasks: fileassoc_stl
Root: HKA; Subkey: "Software\Classes\Mesh.stl\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: fileassoc_stl
Root: HKA; Subkey: "Software\Classes\Mesh.stl\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: fileassoc_stl

; .3mf
Root: HKA; Subkey: "Software\Classes\.3mf"; ValueType: string; ValueData: "Mesh.3mf"; Flags: uninsdeletevalue; Tasks: fileassoc_3mf
Root: HKA; Subkey: "Software\Classes\Mesh.3mf"; ValueType: string; ValueData: "3D 3MF Model"; Flags: uninsdeletekey; Tasks: fileassoc_3mf
Root: HKA; Subkey: "Software\Classes\Mesh.3mf\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: fileassoc_3mf
Root: HKA; Subkey: "Software\Classes\Mesh.3mf\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: fileassoc_3mf

; .obj
Root: HKA; Subkey: "Software\Classes\.obj"; ValueType: string; ValueData: "Mesh.obj"; Flags: uninsdeletevalue; Tasks: fileassoc_obj
Root: HKA; Subkey: "Software\Classes\Mesh.obj"; ValueType: string; ValueData: "Wavefront 3D Object"; Flags: uninsdeletekey; Tasks: fileassoc_obj
Root: HKA; Subkey: "Software\Classes\Mesh.obj\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: fileassoc_obj
Root: HKA; Subkey: "Software\Classes\Mesh.obj\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: fileassoc_obj

; .gltf
Root: HKA; Subkey: "Software\Classes\.gltf"; ValueType: string; ValueData: "Mesh.gltf"; Flags: uninsdeletevalue; Tasks: fileassoc_gltf
Root: HKA; Subkey: "Software\Classes\Mesh.gltf"; ValueType: string; ValueData: "glTF 3D Model"; Flags: uninsdeletekey; Tasks: fileassoc_gltf
Root: HKA; Subkey: "Software\Classes\Mesh.gltf\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: fileassoc_gltf
Root: HKA; Subkey: "Software\Classes\Mesh.gltf\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: fileassoc_gltf

; .glb
Root: HKA; Subkey: "Software\Classes\.glb"; ValueType: string; ValueData: "Mesh.glb"; Flags: uninsdeletevalue; Tasks: fileassoc_glb
Root: HKA; Subkey: "Software\Classes\Mesh.glb"; ValueType: string; ValueData: "Binary glTF 3D Model"; Flags: uninsdeletekey; Tasks: fileassoc_glb
Root: HKA; Subkey: "Software\Classes\Mesh.glb\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: fileassoc_glb
Root: HKA; Subkey: "Software\Classes\Mesh.glb\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: fileassoc_glb

; .fbx
Root: HKA; Subkey: "Software\Classes\.fbx"; ValueType: string; ValueData: "Mesh.fbx"; Flags: uninsdeletevalue; Tasks: fileassoc_fbx
Root: HKA; Subkey: "Software\Classes\Mesh.fbx"; ValueType: string; ValueData: "Autodesk FBX Model"; Flags: uninsdeletekey; Tasks: fileassoc_fbx
Root: HKA; Subkey: "Software\Classes\Mesh.fbx\DefaultIcon"; ValueType: string; ValueData: "{app}\{#MyAppExeName},0"; Tasks: fileassoc_fbx
Root: HKA; Subkey: "Software\Classes\Mesh.fbx\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: fileassoc_fbx
