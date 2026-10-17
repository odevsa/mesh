; NSIS Script for Mesh Windows Installer
; Supports x86_64 and arm64 builds

Unicode True

!ifndef VERSION
  !define VERSION "0.2.2"
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
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES

; Finish Page options
!define MUI_FINISHPAGE_RUN "$INSTDIR\mesh.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch Mesh now"
!insertmacro MUI_PAGE_FINISH

; Uninstaller Pages
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "PortugueseBR"
!insertmacro MUI_LANGUAGE "Portuguese"
!insertmacro MUI_LANGUAGE "Spanish"
!insertmacro MUI_LANGUAGE "French"
!insertmacro MUI_LANGUAGE "German"
!insertmacro MUI_LANGUAGE "Italian"
!insertmacro MUI_LANGUAGE "Japanese"
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "TradChinese"
!insertmacro MUI_LANGUAGE "Russian"
!insertmacro MUI_LANGUAGE "Korean"
!insertmacro MUI_LANGUAGE "Dutch"
!insertmacro MUI_LANGUAGE "Polish"
!insertmacro MUI_LANGUAGE "Turkish"

; Language Strings (English)
LangString SEC_CORE ${LANG_ENGLISH} "Mesh Application (required)"
LangString SEC_GROUP_SHORTCUTS ${LANG_ENGLISH} "Shortcuts"
LangString SEC_STARTMENU ${LANG_ENGLISH} "Start Menu Shortcut"
LangString SEC_DESKTOP ${LANG_ENGLISH} "Desktop Shortcut"
LangString SEC_GROUP_ASSOC ${LANG_ENGLISH} "File Associations"
LangString SEC_ASSOC_STL ${LANG_ENGLISH} "STL 3D Model (.stl)"
LangString SEC_ASSOC_3MF ${LANG_ENGLISH} "3MF 3D Model (.3mf)"
LangString SEC_ASSOC_OBJ ${LANG_ENGLISH} "Wavefront OBJ (.obj)"
LangString SEC_ASSOC_GLTF ${LANG_ENGLISH} "glTF 3D Model (.gltf)"
LangString SEC_ASSOC_GLB ${LANG_ENGLISH} "Binary glTF (.glb)"
LangString SEC_ASSOC_FBX ${LANG_ENGLISH} "Autodesk FBX (.fbx)"
LangString DESC_SecCore ${LANG_ENGLISH} "Core application binaries and required files."
LangString DESC_SecGroupShortcuts ${LANG_ENGLISH} "Create shortcuts to easily launch Mesh."
LangString DESC_SecStartMenu ${LANG_ENGLISH} "Add Mesh shortcuts to the Windows Start Menu."
LangString DESC_SecDesktop ${LANG_ENGLISH} "Add a shortcut to launch Mesh directly from your Desktop."
LangString DESC_SecGroupAssoc ${LANG_ENGLISH} "Configure Windows to open 3D model formats with Mesh by default."
LangString DESC_SecAssocSTL ${LANG_ENGLISH} "Associate Stereolithography (.stl) 3D models with Mesh."
LangString DESC_SecAssoc3MF ${LANG_ENGLISH} "Associate 3D Manufacturing Format (.3mf) models with Mesh."
LangString DESC_SecAssocOBJ ${LANG_ENGLISH} "Associate Wavefront OBJ (.obj) 3D models with Mesh."
LangString DESC_SecAssocGLTF ${LANG_ENGLISH} "Associate glTF JSON (.gltf) 3D models with Mesh."
LangString DESC_SecAssocGLB ${LANG_ENGLISH} "Associate Binary glTF (.glb) 3D models with Mesh."
LangString DESC_SecAssocFBX ${LANG_ENGLISH} "Associate Autodesk FBX (.fbx) 3D models with Mesh."

; Language Strings (PortugueseBR)
LangString SEC_CORE ${LANG_PORTUGUESEBR} "Aplicativo Mesh (obrigatório)"
LangString SEC_GROUP_SHORTCUTS ${LANG_PORTUGUESEBR} "Atalhos"
LangString SEC_STARTMENU ${LANG_PORTUGUESEBR} "Atalho no Menu Iniciar"
LangString SEC_DESKTOP ${LANG_PORTUGUESEBR} "Atalho na Área de Trabalho"
LangString SEC_GROUP_ASSOC ${LANG_PORTUGUESEBR} "Associações de Arquivos"
LangString SEC_ASSOC_STL ${LANG_PORTUGUESEBR} "Modelo 3D STL (.stl)"
LangString SEC_ASSOC_3MF ${LANG_PORTUGUESEBR} "Modelo 3D 3MF (.3mf)"
LangString SEC_ASSOC_OBJ ${LANG_PORTUGUESEBR} "Modelo 3D Wavefront OBJ (.obj)"
LangString SEC_ASSOC_GLTF ${LANG_PORTUGUESEBR} "Modelo 3D glTF (.gltf)"
LangString SEC_ASSOC_GLB ${LANG_PORTUGUESEBR} "Modelo 3D Binary glTF (.glb)"
LangString SEC_ASSOC_FBX ${LANG_PORTUGUESEBR} "Modelo 3D Autodesk FBX (.fbx)"
LangString DESC_SecCore ${LANG_PORTUGUESEBR} "Binários principais do aplicativo e arquivos necessários."
LangString DESC_SecGroupShortcuts ${LANG_PORTUGUESEBR} "Criar atalhos para iniciar o Mesh facilmente."
LangString DESC_SecStartMenu ${LANG_PORTUGUESEBR} "Adicionar atalhos do Mesh ao Menu Iniciar do Windows."
LangString DESC_SecDesktop ${LANG_PORTUGUESEBR} "Adicionar um atalho para abrir o Mesh diretamente da Área de Trabalho."
LangString DESC_SecGroupAssoc ${LANG_PORTUGUESEBR} "Configurar o Windows para abrir arquivos de modelos 3D com o Mesh por padrão."
LangString DESC_SecAssocSTL ${LANG_PORTUGUESEBR} "Associar modelos 3D Stereolithography (.stl) ao Mesh."
LangString DESC_SecAssoc3MF ${LANG_PORTUGUESEBR} "Associar modelos 3D Manufacturing Format (.3mf) ao Mesh."
LangString DESC_SecAssocOBJ ${LANG_PORTUGUESEBR} "Associar modelos 3D Wavefront OBJ (.obj) ao Mesh."
LangString DESC_SecAssocGLTF ${LANG_PORTUGUESEBR} "Associar modelos 3D glTF JSON (.gltf) ao Mesh."
LangString DESC_SecAssocGLB ${LANG_PORTUGUESEBR} "Associar modelos 3D Binary glTF (.glb) ao Mesh."
LangString DESC_SecAssocFBX ${LANG_PORTUGUESEBR} "Associar modelos 3D Autodesk FBX (.fbx) ao Mesh."

; Language Strings (Portuguese)
LangString SEC_CORE ${LANG_PORTUGUESE} "Aplicação Mesh (obrigatório)"
LangString SEC_GROUP_SHORTCUTS ${LANG_PORTUGUESE} "Atalhos"
LangString SEC_STARTMENU ${LANG_PORTUGUESE} "Atalho no Menu Iniciar"
LangString SEC_DESKTOP ${LANG_PORTUGUESE} "Atalho no Ambiente de Trabalho"
LangString SEC_GROUP_ASSOC ${LANG_PORTUGUESE} "Associações de Ficheiros"
LangString SEC_ASSOC_STL ${LANG_PORTUGUESE} "Modelo 3D STL (.stl)"
LangString SEC_ASSOC_3MF ${LANG_PORTUGUESE} "Modelo 3D 3MF (.3mf)"
LangString SEC_ASSOC_OBJ ${LANG_PORTUGUESE} "Modelo 3D Wavefront OBJ (.obj)"
LangString SEC_ASSOC_GLTF ${LANG_PORTUGUESE} "Modelo 3D glTF (.gltf)"
LangString SEC_ASSOC_GLB ${LANG_PORTUGUESE} "Modelo 3D Binary glTF (.glb)"
LangString SEC_ASSOC_FBX ${LANG_PORTUGUESE} "Modelo 3D Autodesk FBX (.fbx)"
LangString DESC_SecCore ${LANG_PORTUGUESE} "Binários principais da aplicação e ficheiros necessários."
LangString DESC_SecGroupShortcuts ${LANG_PORTUGUESE} "Criar atalhos para iniciar o Mesh facilmente."
LangString DESC_SecStartMenu ${LANG_PORTUGUESE} "Adicionar atalhos do Mesh ao Menu Iniciar do Windows."
LangString DESC_SecDesktop ${LANG_PORTUGUESE} "Adicionar um atalho para abrir o Mesh diretamente do Ambiente de Trabalho."
LangString DESC_SecGroupAssoc ${LANG_PORTUGUESE} "Configurar o Windows para abrir ficheiros de modelos 3D com o Mesh por predefinição."
LangString DESC_SecAssocSTL ${LANG_PORTUGUESE} "Associar modelos 3D Stereolithography (.stl) ao Mesh."
LangString DESC_SecAssoc3MF ${LANG_PORTUGUESE} "Associar modelos 3D Manufacturing Format (.3mf) ao Mesh."
LangString DESC_SecAssocOBJ ${LANG_PORTUGUESE} "Associar modelos 3D Wavefront OBJ (.obj) ao Mesh."
LangString DESC_SecAssocGLTF ${LANG_PORTUGUESE} "Associar modelos 3D glTF JSON (.gltf) ao Mesh."
LangString DESC_SecAssocGLB ${LANG_PORTUGUESE} "Associar modelos 3D Binary glTF (.glb) ao Mesh."
LangString DESC_SecAssocFBX ${LANG_PORTUGUESE} "Associar modelos 3D Autodesk FBX (.fbx) ao Mesh."

; Language Strings (Spanish)
LangString SEC_CORE ${LANG_SPANISH} "Aplicación Mesh (requerido)"
LangString SEC_GROUP_SHORTCUTS ${LANG_SPANISH} "Accesos directos"
LangString SEC_STARTMENU ${LANG_SPANISH} "Acceso directo en el Menú Inicio"
LangString SEC_DESKTOP ${LANG_SPANISH} "Acceso directo en el Escritorio"
LangString SEC_GROUP_ASSOC ${LANG_SPANISH} "Asociaciones de archivos"
LangString SEC_ASSOC_STL ${LANG_SPANISH} "Modelo 3D STL (.stl)"
LangString SEC_ASSOC_3MF ${LANG_SPANISH} "Modelo 3D 3MF (.3mf)"
LangString SEC_ASSOC_OBJ ${LANG_SPANISH} "Modelo 3D Wavefront OBJ (.obj)"
LangString SEC_ASSOC_GLTF ${LANG_SPANISH} "Modelo 3D glTF (.gltf)"
LangString SEC_ASSOC_GLB ${LANG_SPANISH} "Modelo 3D Binary glTF (.glb)"
LangString SEC_ASSOC_FBX ${LANG_SPANISH} "Modelo 3D Autodesk FBX (.fbx)"
LangString DESC_SecCore ${LANG_SPANISH} "Binarios principales de la aplicación y archivos necesarios."
LangString DESC_SecGroupShortcuts ${LANG_SPANISH} "Crear accesos directos para iniciar Mesh fácilmente."
LangString DESC_SecStartMenu ${LANG_SPANISH} "Agregar accesos directos de Mesh al Menú Inicio de Windows."
LangString DESC_SecDesktop ${LANG_SPANISH} "Agregar un acceso directo para iniciar Mesh directamente desde el Escritorio."
LangString DESC_SecGroupAssoc ${LANG_SPANISH} "Configurar Windows para abrir modelos 3D con Mesh de forma predeterminada."
LangString DESC_SecAssocSTL ${LANG_SPANISH} "Asociar modelos 3D Stereolithography (.stl) con Mesh."
LangString DESC_SecAssoc3MF ${LANG_SPANISH} "Asociar modelos 3D Manufacturing Format (.3mf) con Mesh."
LangString DESC_SecAssocOBJ ${LANG_SPANISH} "Asociar modelos 3D Wavefront OBJ (.obj) con Mesh."
LangString DESC_SecAssocGLTF ${LANG_SPANISH} "Asociar modelos 3D glTF JSON (.gltf) con Mesh."
LangString DESC_SecAssocGLB ${LANG_SPANISH} "Asociar modelos 3D Binary glTF (.glb) con Mesh."
LangString DESC_SecAssocFBX ${LANG_SPANISH} "Asociar modelos 3D Autodesk FBX (.fbx) con Mesh."

; Language Strings (French)
LangString SEC_CORE ${LANG_FRENCH} "Application Mesh (requis)"
LangString SEC_GROUP_SHORTCUTS ${LANG_FRENCH} "Raccourcis"
LangString SEC_STARTMENU ${LANG_FRENCH} "Raccourci dans le Menu Démarrer"
LangString SEC_DESKTOP ${LANG_FRENCH} "Raccourci sur le Bureau"
LangString SEC_GROUP_ASSOC ${LANG_FRENCH} "Associations de fichiers"
LangString SEC_ASSOC_STL ${LANG_FRENCH} "Modèle 3D STL (.stl)"
LangString SEC_ASSOC_3MF ${LANG_FRENCH} "Modèle 3D 3MF (.3mf)"
LangString SEC_ASSOC_OBJ ${LANG_FRENCH} "Modèle 3D Wavefront OBJ (.obj)"
LangString SEC_ASSOC_GLTF ${LANG_FRENCH} "Modèle 3D glTF (.gltf)"
LangString SEC_ASSOC_GLB ${LANG_FRENCH} "Modèle 3D Binary glTF (.glb)"
LangString SEC_ASSOC_FBX ${LANG_FRENCH} "Modèle 3D Autodesk FBX (.fbx)"
LangString DESC_SecCore ${LANG_FRENCH} "Fichiers binaires principaux de l'application et composants requis."
LangString DESC_SecGroupShortcuts ${LANG_FRENCH} "Créer des raccourcis pour lancer facilement Mesh."
LangString DESC_SecStartMenu ${LANG_FRENCH} "Ajouter des raccourcis Mesh au Menu Démarrer de Windows."
LangString DESC_SecDesktop ${LANG_FRENCH} "Ajouter un raccourci pour lancer Mesh directement depuis le Bureau."
LangString DESC_SecGroupAssoc ${LANG_FRENCH} "Configurer Windows pour ouvrir les formats 3D avec Mesh par défaut."
LangString DESC_SecAssocSTL ${LANG_FRENCH} "Associer les modèles 3D Stereolithography (.stl) à Mesh."
LangString DESC_SecAssoc3MF ${LANG_FRENCH} "Associer les modèles 3D Manufacturing Format (.3mf) à Mesh."
LangString DESC_SecAssocOBJ ${LANG_FRENCH} "Associer les modèles 3D Wavefront OBJ (.obj) à Mesh."
LangString DESC_SecAssocGLTF ${LANG_FRENCH} "Associer les modèles 3D glTF JSON (.gltf) à Mesh."
LangString DESC_SecAssocGLB ${LANG_FRENCH} "Associer les modèles 3D Binary glTF (.glb) à Mesh."
LangString DESC_SecAssocFBX ${LANG_FRENCH} "Associer les modèles 3D Autodesk FBX (.fbx) à Mesh."

; Language Strings (German)
LangString SEC_CORE ${LANG_GERMAN} "Mesh-Anwendung (erforderlich)"
LangString SEC_GROUP_SHORTCUTS ${LANG_GERMAN} "Verknüpfungen"
LangString SEC_STARTMENU ${LANG_GERMAN} "Verknüpfung im Startmenü"
LangString SEC_DESKTOP ${LANG_GERMAN} "Desktop-Verknüpfung"
LangString SEC_GROUP_ASSOC ${LANG_GERMAN} "Dateiverknüpfungen"
LangString SEC_ASSOC_STL ${LANG_GERMAN} "STL 3D-Modell (.stl)"
LangString SEC_ASSOC_3MF ${LANG_GERMAN} "3MF 3D-Modell (.3mf)"
LangString SEC_ASSOC_OBJ ${LANG_GERMAN} "Wavefront OBJ 3D-Modell (.obj)"
LangString SEC_ASSOC_GLTF ${LANG_GERMAN} "glTF 3D-Modell (.gltf)"
LangString SEC_ASSOC_GLB ${LANG_GERMAN} "Binäres glTF 3D-Modell (.glb)"
LangString SEC_ASSOC_FBX ${LANG_GERMAN} "Autodesk FBX 3D-Modell (.fbx)"
LangString DESC_SecCore ${LANG_GERMAN} "Hauptprogrammdateien und erforderliche Komponenten."
LangString DESC_SecGroupShortcuts ${LANG_GERMAN} "Verknüpfungen erstellen, um Mesh einfach zu starten."
LangString DESC_SecStartMenu ${LANG_GERMAN} "Mesh-Verknüpfungen zum Windows-Startmenü hinzufügen."
LangString DESC_SecDesktop ${LANG_GERMAN} "Eine Verknüpfung auf dem Desktop hinzufügen, um Mesh direkt zu starten."
LangString DESC_SecGroupAssoc ${LANG_GERMAN} "Windows so konfigurieren, dass 3D-Modelle standardmäßig mit Mesh geöffnet werden."
LangString DESC_SecAssocSTL ${LANG_GERMAN} "Stereolithography (.stl) 3D-Modelle mit Mesh verknüpfen."
LangString DESC_SecAssoc3MF ${LANG_GERMAN} "3D Manufacturing Format (.3mf) Modelle mit Mesh verknüpfen."
LangString DESC_SecAssocOBJ ${LANG_GERMAN} "Wavefront OBJ (.obj) 3D-Modelle mit Mesh verknüpfen."
LangString DESC_SecAssocGLTF ${LANG_GERMAN} "glTF JSON (.gltf) 3D-Modelle mit Mesh verknüpfen."
LangString DESC_SecAssocGLB ${LANG_GERMAN} "Binäre glTF (.glb) 3D-Modelle mit Mesh verknüpfen."
LangString DESC_SecAssocFBX ${LANG_GERMAN} "Autodesk FBX (.fbx) 3D-Modelle mit Mesh verknüpfen."

; Language Strings (Italian)
LangString SEC_CORE ${LANG_ITALIAN} "Applicazione Mesh (obbligatorio)"
LangString SEC_GROUP_SHORTCUTS ${LANG_ITALIAN} "Collegamenti"
LangString SEC_STARTMENU ${LANG_ITALIAN} "Collegamento nel Menu Start"
LangString SEC_DESKTOP ${LANG_ITALIAN} "Collegamento sul Desktop"
LangString SEC_GROUP_ASSOC ${LANG_ITALIAN} "Associazioni file"
LangString SEC_ASSOC_STL ${LANG_ITALIAN} "Modello 3D STL (.stl)"
LangString SEC_ASSOC_3MF ${LANG_ITALIAN} "Modello 3D 3MF (.3mf)"
LangString SEC_ASSOC_OBJ ${LANG_ITALIAN} "Modello 3D Wavefront OBJ (.obj)"
LangString SEC_ASSOC_GLTF ${LANG_ITALIAN} "Modello 3D glTF (.gltf)"
LangString SEC_ASSOC_GLB ${LANG_ITALIAN} "Modello 3D Binary glTF (.glb)"
LangString SEC_ASSOC_FBX ${LANG_ITALIAN} "Modello 3D Autodesk FBX (.fbx)"
LangString DESC_SecCore ${LANG_ITALIAN} "File binari principali dell'applicazione e componenti necessari."
LangString DESC_SecGroupShortcuts ${LANG_ITALIAN} "Crea collegamenti per avviare facilmente Mesh."
LangString DESC_SecStartMenu ${LANG_ITALIAN} "Aggiungi i collegamenti di Mesh al Menu Start di Windows."
LangString DESC_SecDesktop ${LANG_ITALIAN} "Aggiungi un collegamento sul Desktop per avviare direttamente Mesh."
LangString DESC_SecGroupAssoc ${LANG_ITALIAN} "Configura Windows per aprire i formati 3D con Mesh come predefinito."
LangString DESC_SecAssocSTL ${LANG_ITALIAN} "Associa modelli 3D Stereolithography (.stl) a Mesh."
LangString DESC_SecAssoc3MF ${LANG_ITALIAN} "Associa modelli 3D Manufacturing Format (.3mf) a Mesh."
LangString DESC_SecAssocOBJ ${LANG_ITALIAN} "Associa modelli 3D Wavefront OBJ (.obj) a Mesh."
LangString DESC_SecAssocGLTF ${LANG_ITALIAN} "Associa modelli 3D glTF JSON (.gltf) a Mesh."
LangString DESC_SecAssocGLB ${LANG_ITALIAN} "Associa modelli 3D Binary glTF (.glb) a Mesh."
LangString DESC_SecAssocFBX ${LANG_ITALIAN} "Associa modelli 3D Autodesk FBX (.fbx) a Mesh."

; Language Strings (Japanese)
LangString SEC_CORE ${LANG_JAPANESE} "Mesh アプリケーション (必須)"
LangString SEC_GROUP_SHORTCUTS ${LANG_JAPANESE} "ショートカット"
LangString SEC_STARTMENU ${LANG_JAPANESE} "スタートメニューのショートカット"
LangString SEC_DESKTOP ${LANG_JAPANESE} "デスクトップのショートカット"
LangString SEC_GROUP_ASSOC ${LANG_JAPANESE} "ファイルの関連付け"
LangString SEC_ASSOC_STL ${LANG_JAPANESE} "STL 3Dモデル (.stl)"
LangString SEC_ASSOC_3MF ${LANG_JAPANESE} "3MF 3Dモデル (.3mf)"
LangString SEC_ASSOC_OBJ ${LANG_JAPANESE} "Wavefront OBJ 3Dモデル (.obj)"
LangString SEC_ASSOC_GLTF ${LANG_JAPANESE} "glTF 3Dモデル (.gltf)"
LangString SEC_ASSOC_GLB ${LANG_JAPANESE} "バイナリ glTF 3Dモデル (.glb)"
LangString SEC_ASSOC_FBX ${LANG_JAPANESE} "Autodesk FBX 3Dモデル (.fbx)"
LangString DESC_SecCore ${LANG_JAPANESE} "アプリケーション本体および必要なファイル。"
LangString DESC_SecGroupShortcuts ${LANG_JAPANESE} "Mesh を簡単に起動するためのショートカットを作成します。"
LangString DESC_SecStartMenu ${LANG_JAPANESE} "Windows スタートメニューに Mesh のショートカットを追加します。"
LangString DESC_SecDesktop ${LANG_JAPANESE} "デスクトップに Mesh を直接起動するショートカットを追加します。"
LangString DESC_SecGroupAssoc ${LANG_JAPANESE} "Windows で 3Dモデルファイルを標準で Mesh で開くように設定します。"
LangString DESC_SecAssocSTL ${LANG_JAPANESE} "Stereolithography (.stl) 3Dモデルを Mesh に関連付けます。"
LangString DESC_SecAssoc3MF ${LANG_JAPANESE} "3D Manufacturing Format (.3mf) モデルを Mesh に関連付けます。"
LangString DESC_SecAssocOBJ ${LANG_JAPANESE} "Wavefront OBJ (.obj) 3Dモデルを Mesh に関連付けます。"
LangString DESC_SecAssocGLTF ${LANG_JAPANESE} "glTF JSON (.gltf) 3Dモデルを Mesh に関連付けます。"
LangString DESC_SecAssocGLB ${LANG_JAPANESE} "バイナリ glTF (.glb) 3Dモデルを Mesh に関連付けます。"
LangString DESC_SecAssocFBX ${LANG_JAPANESE} "Autodesk FBX (.fbx) 3Dモデルを Mesh に関連付けます。"

; Language Strings (SimpChinese)
LangString SEC_CORE ${LANG_SIMPCHINESE} "Mesh 应用程序 (必选)"
LangString SEC_GROUP_SHORTCUTS ${LANG_SIMPCHINESE} "快捷方式"
LangString SEC_STARTMENU ${LANG_SIMPCHINESE} "开始菜单快捷方式"
LangString SEC_DESKTOP ${LANG_SIMPCHINESE} "桌面快捷方式"
LangString SEC_GROUP_ASSOC ${LANG_SIMPCHINESE} "文件关联"
LangString SEC_ASSOC_STL ${LANG_SIMPCHINESE} "STL 3D模型 (.stl)"
LangString SEC_ASSOC_3MF ${LANG_SIMPCHINESE} "3MF 3D模型 (.3mf)"
LangString SEC_ASSOC_OBJ ${LANG_SIMPCHINESE} "Wavefront OBJ 3D模型 (.obj)"
LangString SEC_ASSOC_GLTF ${LANG_SIMPCHINESE} "glTF 3D模型 (.gltf)"
LangString SEC_ASSOC_GLB ${LANG_SIMPCHINESE} "二进制 glTF 3D模型 (.glb)"
LangString SEC_ASSOC_FBX ${LANG_SIMPCHINESE} "Autodesk FBX 3D模型 (.fbx)"
LangString DESC_SecCore ${LANG_SIMPCHINESE} "核心应用程序二进制文件及必需组件。"
LangString DESC_SecGroupShortcuts ${LANG_SIMPCHINESE} "创建快捷方式以便快速启动 Mesh。"
LangString DESC_SecStartMenu ${LANG_SIMPCHINESE} "将 Mesh 快捷方式添加到 Windows 开始菜单。"
LangString DESC_SecDesktop ${LANG_SIMPCHINESE} "在桌面添加直接启动 Mesh 的快捷方式。"
LangString DESC_SecGroupAssoc ${LANG_SIMPCHINESE} "配置 Windows 默认使用 Mesh 打开 3D 模型格式。"
LangString DESC_SecAssocSTL ${LANG_SIMPCHINESE} "将 Stereolithography (.stl) 3D 模型与 Mesh 关联。"
LangString DESC_SecAssoc3MF ${LANG_SIMPCHINESE} "将 3D Manufacturing Format (.3mf) 模型与 Mesh 关联。"
LangString DESC_SecAssocOBJ ${LANG_SIMPCHINESE} "将 Wavefront OBJ (.obj) 3D 模型与 Mesh 关联。"
LangString DESC_SecAssocGLTF ${LANG_SIMPCHINESE} "将 glTF JSON (.gltf) 3D 模型与 Mesh 关联。"
LangString DESC_SecAssocGLB ${LANG_SIMPCHINESE} "将二进制 glTF (.glb) 3D 模型与 Mesh 关联。"
LangString DESC_SecAssocFBX ${LANG_SIMPCHINESE} "将 Autodesk FBX (.fbx) 3D 模型与 Mesh 关联。"

; Language Strings (TradChinese)
LangString SEC_CORE ${LANG_TRADCHINESE} "Mesh 應用程式 (必選)"
LangString SEC_GROUP_SHORTCUTS ${LANG_TRADCHINESE} "捷徑"
LangString SEC_STARTMENU ${LANG_TRADCHINESE} "開始功能表捷徑"
LangString SEC_DESKTOP ${LANG_TRADCHINESE} "桌面捷徑"
LangString SEC_GROUP_ASSOC ${LANG_TRADCHINESE} "檔案關聯"
LangString SEC_ASSOC_STL ${LANG_TRADCHINESE} "STL 3D模型 (.stl)"
LangString SEC_ASSOC_3MF ${LANG_TRADCHINESE} "3MF 3D模型 (.3mf)"
LangString SEC_ASSOC_OBJ ${LANG_TRADCHINESE} "Wavefront OBJ 3D模型 (.obj)"
LangString SEC_ASSOC_GLTF ${LANG_TRADCHINESE} "glTF 3D模型 (.gltf)"
LangString SEC_ASSOC_GLB ${LANG_TRADCHINESE} "二進位 glTF 3D模型 (.glb)"
LangString SEC_ASSOC_FBX ${LANG_TRADCHINESE} "Autodesk FBX 3D模型 (.fbx)"
LangString DESC_SecCore ${LANG_TRADCHINESE} "核心應用程式二進位檔案及必要元件。"
LangString DESC_SecGroupShortcuts ${LANG_TRADCHINESE} "建立捷徑以輕鬆啟動 Mesh。"
LangString DESC_SecStartMenu ${LANG_TRADCHINESE} "將 Mesh 捷徑新增至 Windows 開始功能表。"
LangString DESC_SecDesktop ${LANG_TRADCHINESE} "在桌面上新增直接啟動 Mesh 的捷徑。"
LangString DESC_SecGroupAssoc ${LANG_TRADCHINESE} "設定 Windows 預設使用 Mesh 開啟 3D 模型檔案。"
LangString DESC_SecAssocSTL ${LANG_TRADCHINESE} "將 Stereolithography (.stl) 3D 模型與 Mesh 關聯。"
LangString DESC_SecAssoc3MF ${LANG_TRADCHINESE} "將 3D Manufacturing Format (.3mf) 模型與 Mesh 關聯。"
LangString DESC_SecAssocOBJ ${LANG_TRADCHINESE} "將 Wavefront OBJ (.obj) 3D 模型與 Mesh 關聯。"
LangString DESC_SecAssocGLTF ${LANG_TRADCHINESE} "將 glTF JSON (.gltf) 3D 模型與 Mesh 關聯。"
LangString DESC_SecAssocGLB ${LANG_TRADCHINESE} "將二進位 glTF (.glb) 3D 模型與 Mesh 關聯。"
LangString DESC_SecAssocFBX ${LANG_TRADCHINESE} "將 Autodesk FBX (.fbx) 3D 模型與 Mesh 關聯。"

; Language Strings (Russian)
LangString SEC_CORE ${LANG_RUSSIAN} "Приложение Mesh (обязательно)"
LangString SEC_GROUP_SHORTCUTS ${LANG_RUSSIAN} "Ярлыки"
LangString SEC_STARTMENU ${LANG_RUSSIAN} "Ярлык в меню «Пуск»"
LangString SEC_DESKTOP ${LANG_RUSSIAN} "Ярлык на Рабочем столе"
LangString SEC_GROUP_ASSOC ${LANG_RUSSIAN} "Ассоциации файлов"
LangString SEC_ASSOC_STL ${LANG_RUSSIAN} "3D-модель STL (.stl)"
LangString SEC_ASSOC_3MF ${LANG_RUSSIAN} "3D-модель 3MF (.3mf)"
LangString SEC_ASSOC_OBJ ${LANG_RUSSIAN} "3D-модель Wavefront OBJ (.obj)"
LangString SEC_ASSOC_GLTF ${LANG_RUSSIAN} "3D-модель glTF (.gltf)"
LangString SEC_ASSOC_GLB ${LANG_RUSSIAN} "Двоичная 3D-модель glTF (.glb)"
LangString SEC_ASSOC_FBX ${LANG_RUSSIAN} "3D-модель Autodesk FBX (.fbx)"
LangString DESC_SecCore ${LANG_RUSSIAN} "Основные исполняемые файлы приложения и необходимые компоненты."
LangString DESC_SecGroupShortcuts ${LANG_RUSSIAN} "Создать ярлыки для быстрого запуска Mesh."
LangString DESC_SecStartMenu ${LANG_RUSSIAN} "Добавить ярлыки Mesh в меню «Пуск» Windows."
LangString DESC_SecDesktop ${LANG_RUSSIAN} "Добавить ярлык на Рабочий стол для быстрого запуска Mesh."
LangString DESC_SecGroupAssoc ${LANG_RUSSIAN} "Настроить Windows для открытия файлов 3D-моделей в Mesh по умолчанию."
LangString DESC_SecAssocSTL ${LANG_RUSSIAN} "Ассоциировать 3D-модели Stereolithography (.stl) с Mesh."
LangString DESC_SecAssoc3MF ${LANG_RUSSIAN} "Ассоциировать 3D-модели 3D Manufacturing Format (.3mf) с Mesh."
LangString DESC_SecAssocOBJ ${LANG_RUSSIAN} "Ассоциировать 3D-модели Wavefront OBJ (.obj) с Mesh."
LangString DESC_SecAssocGLTF ${LANG_RUSSIAN} "Ассоциировать 3D-модели glTF JSON (.gltf) с Mesh."
LangString DESC_SecAssocGLB ${LANG_RUSSIAN} "Ассоциировать двоичные 3D-модели glTF (.glb) с Mesh."
LangString DESC_SecAssocFBX ${LANG_RUSSIAN} "Ассоциировать 3D-модели Autodesk FBX (.fbx) с Mesh."

; Language Strings (Korean)
LangString SEC_CORE ${LANG_KOREAN} "Mesh 응용 프로그램 (필수)"
LangString SEC_GROUP_SHORTCUTS ${LANG_KOREAN} "바로가기"
LangString SEC_STARTMENU ${LANG_KOREAN} "시작 메뉴 바로가기"
LangString SEC_DESKTOP ${LANG_KOREAN} "바탕화면 바로가기"
LangString SEC_GROUP_ASSOC ${LANG_KOREAN} "파일 연결"
LangString SEC_ASSOC_STL ${LANG_KOREAN} "STL 3D 모델 (.stl)"
LangString SEC_ASSOC_3MF ${LANG_KOREAN} "3MF 3D 모델 (.3mf)"
LangString SEC_ASSOC_OBJ ${LANG_KOREAN} "Wavefront OBJ 3D 모델 (.obj)"
LangString SEC_ASSOC_GLTF ${LANG_KOREAN} "glTF 3D 모델 (.gltf)"
LangString SEC_ASSOC_GLB ${LANG_KOREAN} "바이너리 glTF 3D 모델 (.glb)"
LangString SEC_ASSOC_FBX ${LANG_KOREAN} "Autodesk FBX 3D 모델 (.fbx)"
LangString DESC_SecCore ${LANG_KOREAN} "핵심 애플리케이션 실행 파일 및 필수 구성 요소."
LangString DESC_SecGroupShortcuts ${LANG_KOREAN} "Mesh를 쉽게 실행할 수 있도록 바로가기를 생성합니다."
LangString DESC_SecStartMenu ${LANG_KOREAN} "Windows 시작 메뉴에 Mesh 바로가기를 추가합니다."
LangString DESC_SecDesktop ${LANG_KOREAN} "바탕화면에서 Mesh를 바로 실행할 수 있도록 바로가기를 추가합니다."
LangString DESC_SecGroupAssoc ${LANG_KOREAN} "Windows에서 3D 모델 파일을 기본적으로 Mesh로 열도록 설정합니다."
LangString DESC_SecAssocSTL ${LANG_KOREAN} "Stereolithography (.stl) 3D 모델을 Mesh와 연결합니다."
LangString DESC_SecAssoc3MF ${LANG_KOREAN} "3D Manufacturing Format (.3mf) 모델을 Mesh와 연결합니다."
LangString DESC_SecAssocOBJ ${LANG_KOREAN} "Wavefront OBJ (.obj) 3D 모델을 Mesh와 연결합니다."
LangString DESC_SecAssocGLTF ${LANG_KOREAN} "glTF JSON (.gltf) 3D 모델을 Mesh와 연결합니다."
LangString DESC_SecAssocGLB ${LANG_KOREAN} "바이너리 glTF (.glb) 3D 모델을 Mesh와 연결합니다."
LangString DESC_SecAssocFBX ${LANG_KOREAN} "Autodesk FBX (.fbx) 3D 모델을 Mesh와 연결합니다."

; Language Strings (Dutch)
LangString SEC_CORE ${LANG_DUTCH} "Mesh Toepassing (vereist)"
LangString SEC_GROUP_SHORTCUTS ${LANG_DUTCH} "Snelkoppelingen"
LangString SEC_STARTMENU ${LANG_DUTCH} "Startmenu snelkoppeling"
LangString SEC_DESKTOP ${LANG_DUTCH} "Bureaublad snelkoppeling"
LangString SEC_GROUP_ASSOC ${LANG_DUTCH} "Bestandskoppelingen"
LangString SEC_ASSOC_STL ${LANG_DUTCH} "STL 3D-model (.stl)"
LangString SEC_ASSOC_3MF ${LANG_DUTCH} "3MF 3D-model (.3mf)"
LangString SEC_ASSOC_OBJ ${LANG_DUTCH} "Wavefront OBJ 3D-model (.obj)"
LangString SEC_ASSOC_GLTF ${LANG_DUTCH} "glTF 3D-model (.gltf)"
LangString SEC_ASSOC_GLB ${LANG_DUTCH} "Binair glTF 3D-model (.glb)"
LangString SEC_ASSOC_FBX ${LANG_DUTCH} "Autodesk FBX 3D-model (.fbx)"
LangString DESC_SecCore ${LANG_DUTCH} "Belangrijkste programmabestanden en vereiste onderdelen."
LangString DESC_SecGroupShortcuts ${LANG_DUTCH} "Maak snelkoppelingen om Mesh eenvoudig te starten."
LangString DESC_SecStartMenu ${LANG_DUTCH} "Mesh-snelkoppelingen toevoegen aan het Windows-startmenu."
LangString DESC_SecDesktop ${LANG_DUTCH} "Een snelkoppeling toevoegen op het bureaublad om Mesh direct te openen."
LangString DESC_SecGroupAssoc ${LANG_DUTCH} "Windows configureren om 3D-modellen standaard met Mesh te openen."
LangString DESC_SecAssocSTL ${LANG_DUTCH} "Stereolithography (.stl) 3D-modellen koppelen aan Mesh."
LangString DESC_SecAssoc3MF ${LANG_DUTCH} "3D Manufacturing Format (.3mf) modellen koppelen aan Mesh."
LangString DESC_SecAssocOBJ ${LANG_DUTCH} "Wavefront OBJ (.obj) 3D-modellen koppelen aan Mesh."
LangString DESC_SecAssocGLTF ${LANG_DUTCH} "glTF JSON (.gltf) 3D-modellen koppelen aan Mesh."
LangString DESC_SecAssocGLB ${LANG_DUTCH} "Binaire glTF (.glb) 3D-modellen koppelen aan Mesh."
LangString DESC_SecAssocFBX ${LANG_DUTCH} "Autodesk FBX (.fbx) 3D-modellen koppelen aan Mesh."

; Language Strings (Polish)
LangString SEC_CORE ${LANG_POLISH} "Aplikacja Mesh (wymagane)"
LangString SEC_GROUP_SHORTCUTS ${LANG_POLISH} "Skróty"
LangString SEC_STARTMENU ${LANG_POLISH} "Skrót w Menu Start"
LangString SEC_DESKTOP ${LANG_POLISH} "Skrót na Pulpicie"
LangString SEC_GROUP_ASSOC ${LANG_POLISH} "Skojarzenia plików"
LangString SEC_ASSOC_STL ${LANG_POLISH} "Model 3D STL (.stl)"
LangString SEC_ASSOC_3MF ${LANG_POLISH} "Model 3D 3MF (.3mf)"
LangString SEC_ASSOC_OBJ ${LANG_POLISH} "Model 3D Wavefront OBJ (.obj)"
LangString SEC_ASSOC_GLTF ${LANG_POLISH} "Model 3D glTF (.gltf)"
LangString SEC_ASSOC_GLB ${LANG_POLISH} "Model 3D binarny glTF (.glb)"
LangString SEC_ASSOC_FBX ${LANG_POLISH} "Model 3D Autodesk FBX (.fbx)"
LangString DESC_SecCore ${LANG_POLISH} "Główne pliki aplikacji i wymagane składniki."
LangString DESC_SecGroupShortcuts ${LANG_POLISH} "Utwórz skróty, aby łatwo uruchamiać Mesh."
LangString DESC_SecStartMenu ${LANG_POLISH} "Dodaj skróty Mesh do Menu Start systemu Windows."
LangString DESC_SecDesktop ${LANG_POLISH} "Dodaj skrót na Pulpicie, aby bezpośrednio otwierać Mesh."
LangString DESC_SecGroupAssoc ${LANG_POLISH} "Skonfiguruj system Windows, aby domyślnie otwierał pliki 3D za pomocą programu Mesh."
LangString DESC_SecAssocSTL ${LANG_POLISH} "Skojarz modele 3D Stereolithography (.stl) z programem Mesh."
LangString DESC_SecAssoc3MF ${LANG_POLISH} "Skojarz modele 3D Manufacturing Format (.3mf) z programem Mesh."
LangString DESC_SecAssocOBJ ${LANG_POLISH} "Skojarz modele 3D Wavefront OBJ (.obj) z programem Mesh."
LangString DESC_SecAssocGLTF ${LANG_POLISH} "Skojarz modele 3D glTF JSON (.gltf) z programem Mesh."
LangString DESC_SecAssocGLB ${LANG_POLISH} "Skojarz modele 3D binarne glTF (.glb) z programem Mesh."
LangString DESC_SecAssocFBX ${LANG_POLISH} "Skojarz modele 3D Autodesk FBX (.fbx) z programem Mesh."

; Language Strings (Turkish)
LangString SEC_CORE ${LANG_TURKISH} "Mesh Uygulaması (gerekli)"
LangString SEC_GROUP_SHORTCUTS ${LANG_TURKISH} "Kısayollar"
LangString SEC_STARTMENU ${LANG_TURKISH} "Başlat Menüsü Kısayolu"
LangString SEC_DESKTOP ${LANG_TURKISH} "Masaüstü Kısayolu"
LangString SEC_GROUP_ASSOC ${LANG_TURKISH} "Dosya İlişkilendirmeleri"
LangString SEC_ASSOC_STL ${LANG_TURKISH} "STL 3D Modeli (.stl)"
LangString SEC_ASSOC_3MF ${LANG_TURKISH} "3MF 3D Modeli (.3mf)"
LangString SEC_ASSOC_OBJ ${LANG_TURKISH} "Wavefront OBJ 3D Modeli (.obj)"
LangString SEC_ASSOC_GLTF ${LANG_TURKISH} "glTF 3D Modeli (.gltf)"
LangString SEC_ASSOC_GLB ${LANG_TURKISH} "İkili glTF 3D Modeli (.glb)"
LangString SEC_ASSOC_FBX ${LANG_TURKISH} "Autodesk FBX 3D Modeli (.fbx)"
LangString DESC_SecCore ${LANG_TURKISH} "Ana uygulama ikili dosyaları ve gerekli bileşenler."
LangString DESC_SecGroupShortcuts ${LANG_TURKISH} "Mesh uygulamasını kolayca başlatmak için kısayollar oluşturun."
LangString DESC_SecStartMenu ${LANG_TURKISH} "Windows Başlat Menüsüne Mesh kısayollarını ekleyin."
LangString DESC_SecDesktop ${LANG_TURKISH} "Mesh uygulamasını doğrudan Masaüstünden açmak için kısayol ekleyin."
LangString DESC_SecGroupAssoc ${LANG_TURKISH} "Windows'u 3D model dosyalarını varsayılan olarak Mesh ile açacak şekilde yapılandırın."
LangString DESC_SecAssocSTL ${LANG_TURKISH} "Stereolithography (.stl) 3D modellerini Mesh ile ilişkilendirin."
LangString DESC_SecAssoc3MF ${LANG_TURKISH} "3D Manufacturing Format (.3mf) modellerini Mesh ile ilişkilendirin."
LangString DESC_SecAssocOBJ ${LANG_TURKISH} "Wavefront OBJ (.obj) 3D modellerini Mesh ile ilişkilendirin."
LangString DESC_SecAssocGLTF ${LANG_TURKISH} "glTF JSON (.gltf) 3D modellerini Mesh ile ilişkilendirin."
LangString DESC_SecAssocGLB ${LANG_TURKISH} "İkili glTF (.glb) 3D modellerini Mesh ile ilişkilendirin."
LangString DESC_SecAssocFBX ${LANG_TURKISH} "Autodesk FBX (.fbx) 3D modellerini Mesh ile ilişkilendirin."
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

Section $(SEC_CORE) SecCore
  SectionIn RO
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

  ; Clean up any obsolete autostart if needed
  DeleteRegValue HKLM "Software\Microsoft\Windows\CurrentVersion\Run" "Mesh"
SectionEnd

SectionGroup /e $(SEC_GROUP_SHORTCUTS) SecGroupShortcuts
  Section $(SEC_STARTMENU) SecStartMenu
    SetShellVarContext all
    CreateDirectory "$SMPROGRAMS\Mesh"
    CreateShortcut "$SMPROGRAMS\Mesh\Mesh.lnk" "$INSTDIR\mesh.exe" "" "$INSTDIR\mesh.ico" 0
    CreateShortcut "$SMPROGRAMS\Mesh\Uninstall Mesh.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\uninstall.exe" 0
  SectionEnd

  Section $(SEC_DESKTOP) SecDesktop
    SetShellVarContext all
    CreateShortcut "$DESKTOP\Mesh.lnk" "$INSTDIR\mesh.exe" "" "$INSTDIR\mesh.ico" 0
  SectionEnd
SectionGroupEnd

SectionGroup /e $(SEC_GROUP_ASSOC) SecGroupAssoc
  Section $(SEC_ASSOC_STL) SecAssocSTL
    !insertmacro RegisterExtension "stl" "Mesh.stl" "3D STL Model" "model/stl"
  SectionEnd

  Section $(SEC_ASSOC_3MF) SecAssoc3MF
    !insertmacro RegisterExtension "3mf" "Mesh.3mf" "3D 3MF Model" "model/3mf"
  SectionEnd

  Section $(SEC_ASSOC_OBJ) SecAssocOBJ
    !insertmacro RegisterExtension "obj" "Mesh.obj" "Wavefront 3D Object" "model/obj"
  SectionEnd

  Section $(SEC_ASSOC_GLTF) SecAssocGLTF
    !insertmacro RegisterExtension "gltf" "Mesh.gltf" "glTF 3D Model" "model/gltf+json"
  SectionEnd

  Section $(SEC_ASSOC_GLB) SecAssocGLB
    !insertmacro RegisterExtension "glb" "Mesh.glb" "Binary glTF 3D Model" "model/gltf-binary"
  SectionEnd

  Section $(SEC_ASSOC_FBX) SecAssocFBX
    !insertmacro RegisterExtension "fbx" "Mesh.fbx" "Autodesk FBX Model" "model/fbx"
  SectionEnd
SectionGroupEnd

Section "-Post"
  ; Calculate and write EstimatedSize
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Mesh" "EstimatedSize" "$0"

  ; Notify Windows Shell of association changes
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecCore} $(DESC_SecCore)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecGroupShortcuts} $(DESC_SecGroupShortcuts)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecStartMenu} $(DESC_SecStartMenu)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktop} $(DESC_SecDesktop)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecGroupAssoc} $(DESC_SecGroupAssoc)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecAssocSTL} $(DESC_SecAssocSTL)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecAssoc3MF} $(DESC_SecAssoc3MF)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecAssocOBJ} $(DESC_SecAssocOBJ)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecAssocGLTF} $(DESC_SecAssocGLTF)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecAssocGLB} $(DESC_SecAssocGLB)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecAssocFBX} $(DESC_SecAssocFBX)
!insertmacro MUI_FUNCTION_DESCRIPTION_END

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
