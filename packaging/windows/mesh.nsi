; NSIS Script for Mesh Windows Installer
; Supports x86_64 and arm64 builds

Unicode True

!ifndef VERSION
  !define VERSION "0.1.6"
!endif

!ifndef ARCH
  !define ARCH "x86_64"
!endif

!ifndef BIN_DIR
  !define BIN_DIR "target\release"
!endif

!ifndef ASSETS_DIR
  !define ASSETS_DIR "assets"
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

  ; Start Menu Shortcuts
  CreateDirectory "$SMPROGRAMS\Mesh"
  CreateShortcut "$SMPROGRAMS\Mesh\Mesh.lnk" "$INSTDIR\mesh.exe" "" "$INSTDIR\mesh.ico" 0
  CreateShortcut "$SMPROGRAMS\Mesh\Uninstall Mesh.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\uninstall.exe" 0

  ; Clean up any obsolete autostart if needed
  DeleteRegValue HKLM "Software\Microsoft\Windows\CurrentVersion\Run" "Mesh"

  ; Desktop Shortcut
  CreateShortcut "$DESKTOP\Mesh.lnk" "$INSTDIR\mesh.exe" "" "$INSTDIR\mesh.ico" 0
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

  ; Remove registry keys
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Mesh"
  DeleteRegKey HKLM "Software\Mesh"
SectionEnd
