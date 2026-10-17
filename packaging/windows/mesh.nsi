; NSIS Script for Mesh Windows Installer
; Supports x86_64 and arm64 builds

Unicode True

!ifndef VERSION
  !define VERSION "0.2.1"
!endif

!ifndef ARCH
  !define ARCH "x86_64"
!endif

!ifndef BIN_DIR
  !define BIN_DIR "target\release"
!endif

!ifndef ASSETS_DIR
  !define ASSETS_DIR "packaging\windows\assets"
!endif

!ifndef OUT_DIR
  !define OUT_DIR "dist"
!endif

Name "Mesh"
OutFile "${OUT_DIR}\mesh-${VERSION}-windows-${ARCH}.exe"
InstallDir "$PROGRAMFILES64\Mesh"
InstallDirRegKey HKLM "Software\Mesh" "Install_Dir"
RequestExecutionLevel admin

; Includes
!include "MUI2.nsh"
!include "FileFunc.nsh"

; MUI Settings
!define MUI_ABORTWARNING
!define MUI_ICON "${ASSETS_DIR}\mesh.ico"
!define MUI_UNICON "${ASSETS_DIR}\mesh.ico"

; Installer Pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES

; Finish Page options
!define MUI_FINISHPAGE_RUN "$INSTDIR\mesh.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch Mesh now"
!insertmacro MUI_PAGE_FINISH

; Uninstaller Pages
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Portuguese"

; Macro to register file extension and MIME associations
!macro RegisterExtension EXT PROGID DESC MIME
  ; Associate file extension
  WriteRegStr HKLM "Software\Classes\.${EXT}" "" "${PROGID}"
  WriteRegStr HKLM "Software\Classes\.${EXT}" "Content Type" "${MIME}"
  WriteRegStr HKLM "Software\Classes\.${EXT}\OpenWithProgids" "${PROGID}" ""

  ; MIME Database mapping
  WriteRegStr HKLM "Software\Classes\MIME\Database\Content Type\${MIME}" "Extension" ".${EXT}"

  ; SystemFileAssociations for Explorer context menu
  WriteRegStr HKLM "Software\Classes\SystemFileAssociations\.${EXT}\OpenWithList\mesh.exe" "" ""

  ; ProgID definition
  WriteRegStr HKLM "Software\Classes\${PROGID}" "" "${DESC}"
  WriteRegStr HKLM "Software\Classes\${PROGID}" "FriendlyTypeName" "${DESC}"
  WriteRegStr HKLM "Software\Classes\${PROGID}\DefaultIcon" "" "$INSTDIR\mesh.ico,0"
  WriteRegStr HKLM "Software\Classes\${PROGID}\shell" "" "open"
  WriteRegStr HKLM "Software\Classes\${PROGID}\shell\open" "" "Open with Mesh"
  WriteRegStr HKLM "Software\Classes\${PROGID}\shell\open\command" "" '"$INSTDIR\mesh.exe" "%1"'
  WriteRegStr HKLM "Software\Classes\${PROGID}\SupportedMimeTypes" "${MIME}" ""

  ; Capabilities for Windows Default Programs / Apps
  WriteRegStr HKLM "Software\Mesh\Capabilities\FileAssociations" ".${EXT}" "${PROGID}"
  WriteRegStr HKLM "Software\Mesh\Capabilities\MimeAssociations" "${MIME}" "${PROGID}"

  ; Applications\mesh.exe supported types
  WriteRegStr HKLM "Software\Classes\Applications\mesh.exe\SupportedTypes" ".${EXT}" ""
!macroend

!macro UnregisterExtension EXT PROGID
  ; Remove ProgID
  DeleteRegKey HKLM "Software\Classes\${PROGID}"

  ; Remove OpenWithProgids value
  DeleteRegValue HKLM "Software\Classes\.${EXT}\OpenWithProgids" "${PROGID}"

  ; Remove SystemFileAssociations
  DeleteRegKey HKLM "Software\Classes\SystemFileAssociations\.${EXT}\OpenWithList\mesh.exe"

  ; If extension (Default) points to this ProgID, clear it
  ReadRegStr $0 HKLM "Software\Classes\.${EXT}" ""
  StrCmp $0 "${PROGID}" unreg_${EXT}_match unreg_${EXT}_skip
unreg_${EXT}_match:
  DeleteRegValue HKLM "Software\Classes\.${EXT}" ""
unreg_${EXT}_skip:
!macroend

Section "Mesh Application" SecCore
  SetShellVarContext all
  SetOutPath "$INSTDIR"
  
  ; Write installed files
  File "${BIN_DIR}\mesh.exe"
  File "${ASSETS_DIR}\mesh.ico"

  ; Create uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Save install location
  WriteRegStr HKLM "Software\Mesh" "Install_Dir" "$INSTDIR"

  ; Add to Add/Remove Programs
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Mesh" "DisplayName" "Mesh"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Mesh" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Mesh" "DisplayIcon" "$INSTDIR\mesh.ico"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Mesh" "Publisher" "Odevsa"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Mesh" "URLInfoAbout" "https://github.com/odevsa/mesh"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Mesh" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Mesh" "NoModify" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Mesh" "NoRepair" 1

  ; Calculate and write EstimatedSize
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Mesh" "EstimatedSize" "$0"

  ; App Paths (allows running "mesh" or "mesh.exe" from Run dialog or command line)
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\mesh.exe" "" "$INSTDIR\mesh.exe"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\mesh.exe" "Path" "$INSTDIR"

  ; Applications registration for "Open with..."
  WriteRegStr HKLM "Software\Classes\Applications\mesh.exe" "FriendlyAppName" "Mesh"
  WriteRegStr HKLM "Software\Classes\Applications\mesh.exe\DefaultIcon" "" "$INSTDIR\mesh.ico,0"
  WriteRegStr HKLM "Software\Classes\Applications\mesh.exe\shell\open\command" "" '"$INSTDIR\mesh.exe" "%1"'

  ; RegisteredApplications (Windows Default Apps & Settings)
  WriteRegStr HKLM "Software\RegisteredApplications" "Mesh" "Software\Mesh\Capabilities"
  WriteRegStr HKLM "Software\Mesh\Capabilities" "ApplicationName" "Mesh"
  WriteRegStr HKLM "Software\Mesh\Capabilities" "ApplicationDescription" "Mesh - A lightweight 3D Mesh Viewer"
  WriteRegStr HKLM "Software\Mesh\Capabilities" "ApplicationIcon" "$INSTDIR\mesh.ico,0"

  ; File Associations & MIME Types
  !insertmacro RegisterExtension "stl" "Mesh.stl" "3D STL Model" "model/stl"
  !insertmacro RegisterExtension "3mf" "Mesh.3mf" "3D 3MF Model" "model/3mf"
  !insertmacro RegisterExtension "obj" "Mesh.obj" "Wavefront 3D Object" "model/obj"
  !insertmacro RegisterExtension "gltf" "Mesh.gltf" "glTF 3D Model" "model/gltf+json"
  !insertmacro RegisterExtension "glb" "Mesh.glb" "Binary glTF 3D Model" "model/gltf-binary"
  !insertmacro RegisterExtension "fbx" "Mesh.fbx" "Autodesk FBX Model" "model/fbx"

  ; Start Menu Shortcuts
  CreateDirectory "$SMPROGRAMS\Mesh"
  CreateShortcut "$SMPROGRAMS\Mesh\Mesh.lnk" "$INSTDIR\mesh.exe" "" "$INSTDIR\mesh.ico" 0
  CreateShortcut "$SMPROGRAMS\Mesh\Uninstall Mesh.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\uninstall.exe" 0

  ; Clean up any obsolete autostart if needed
  DeleteRegValue HKLM "Software\Microsoft\Windows\CurrentVersion\Run" "Mesh"

  ; Desktop Shortcut
  CreateShortcut "$DESKTOP\Mesh.lnk" "$INSTDIR\mesh.exe" "" "$INSTDIR\mesh.ico" 0

  ; Notify Windows Shell of association changes
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
SectionEnd

Section "Uninstall"
  SetShellVarContext all

  ; Remove shortcuts
  Delete "$DESKTOP\Mesh.lnk"
  Delete "$SMPROGRAMS\Mesh\Mesh.lnk"
  Delete "$SMPROGRAMS\Mesh\Uninstall Mesh.lnk"
  RMDir "$SMPROGRAMS\Mesh"

  ; Remove files
  Delete "$INSTDIR\mesh.exe"
  Delete "$INSTDIR\mesh.ico"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  ; Remove File Associations
  !insertmacro UnregisterExtension "stl" "Mesh.stl"
  !insertmacro UnregisterExtension "3mf" "Mesh.3mf"
  !insertmacro UnregisterExtension "obj" "Mesh.obj"
  !insertmacro UnregisterExtension "gltf" "Mesh.gltf"
  !insertmacro UnregisterExtension "glb" "Mesh.glb"
  !insertmacro UnregisterExtension "fbx" "Mesh.fbx"

  ; Remove Default Programs and App registration
  DeleteRegValue HKLM "Software\RegisteredApplications" "Mesh"
  DeleteRegKey HKLM "Software\Classes\Applications\mesh.exe"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\App Paths\mesh.exe"

  ; Remove registry keys
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Mesh"
  DeleteRegKey HKLM "Software\Mesh"

  ; Notify Windows Shell of association changes
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
SectionEnd
