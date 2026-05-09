; NSIS installer generation script for SongRec
 
; Cf. https://nsis.sourceforge.io/Sample_installation_script_for_an_application
; Cf. https://nsis.sourceforge.io/Docs/Modern%20UI%202/Readme.html
; Cf. https://nsis.sourceforge.io/Examples/Modern%20UI/Basic.nsi
; Cf. https://nsis.sourceforge.io/Examples/Modern%20UI/StartMenu.nsi
; Cf. https://stackoverflow.com/questions/11014013/copy-a-directory-using-nsis
; Cf. https://fr.wikipedia.org/wiki/Nullsoft_Scriptable_Install_System
; Cf. https://sources.debian.org/src/gentle/1.9%2Bcvs20100605%2Bdfsg1-7/InstallerScript.nsi
; Cf. https://github.com/uroni/urbackup_frontend_wx/blob/dev/urbackup_x64.nsi
; Cf. https://github.com/ib/gucharmap/blob/master/gucharmap.nsi.in ?
 
; -------------------------------
; Start
 
  !define MUI_PRODUCT "SongRec"
  !define MUI_FILE "songrec"
  !define MUI_VERSION "0.7.0"
  !define MUI_BRANDINGTEXT "${MUI_PRODUCT} ${MUI_VERSION}"
  !define WIN_DIST_DIR "/home/marin/win32/songrec-0.7.0"
  !define SOURCE_DIR "/home/marin/rust-shazam"
  CRCCheck On
 
  ; We should test if we must use an absolute path 
  !include "${NSISDIR}/Contrib/Modern UI 2/MUI2.nsh"
 
 
;---------------------------------
; General

  Name "${MUI_PRODUCT}"
  OutFile "/home/marin/win32/${MUI_PRODUCT}-${MUI_VERSION}-installer.exe"
  Unicode True

  ShowInstDetails "nevershow"
  ShowUninstDetails "nevershow"
  SetCompressor "lzma"

  !define MUI_ICON "songrec.ico"
  !define MUI_UNICON "songrec.ico"
 
 
;--------------------------------
; Folder selection page
 
  InstallDir "$PROGRAMFILES64\${MUI_PRODUCT}"
  
  ; Get installation folder from registry if available
  InstallDirRegKey HKCU "Software\${MUI_PRODUCT}" ""

  ; Request application privileges for Windows Vista
  RequestExecutionLevel admin

 
;--------------------------------
; Interface Settings

  ; !define MUI_ABORTWARNING

;--------------------------------
; Pages

  !insertmacro MUI_PAGE_LICENSE "${SOURCE_DIR}/LICENSE"
  !insertmacro MUI_PAGE_COMPONENTS
  !insertmacro MUI_PAGE_DIRECTORY

  !insertmacro MUI_PAGE_INSTFILES
  
  !insertmacro MUI_UNPAGE_CONFIRM
  !insertmacro MUI_UNPAGE_INSTFILES
 
 
;--------------------------------
;Language
 
  !insertmacro MUI_LANGUAGE "English"
 
 
;-------------------------------- 
;Installer Sections     
Section "${MUI_PRODUCT} ${MUI_VERSION}"
 
  ; Add files
  SetOutPath "$INSTDIR"
 
  File /r "${WIN_DIST_DIR}/*"
  File "${SOURCE_DIR}/LICENSE"
  File "${SOURCE_DIR}/README.md"
  
  ; Store installation folder
  WriteRegStr HKCU "Software\${MUI_PRODUCT}" "" $INSTDIR
 
  ; Create desktop shortcut
  CreateShortCut "$DESKTOP\${MUI_PRODUCT}.lnk" "$INSTDIR\${MUI_FILE}.exe" ""
 
  ; Create shortcuts
  CreateDirectory "$SMPROGRAMS\${MUI_PRODUCT}"
  CreateShortcut "$SMPROGRAMS\${MUI_PRODUCT}\Uninstall SongRec.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortCut "$SMPROGRAMS\${MUI_PRODUCT}\${MUI_PRODUCT}.lnk" "$INSTDIR\${MUI_FILE}.exe"

  ; Write uninstall information to the registry
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${MUI_PRODUCT}" "DisplayName" "${MUI_PRODUCT} (uninstall)"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${MUI_PRODUCT}" "UninstallString" "$INSTDIR\Uninstall.exe"
 
  WriteUninstaller "$INSTDIR\Uninstall.exe"
 
SectionEnd
 
 
;--------------------------------    
;Uninstaller Section  
Section "Uninstall"
 
  ; Remove the installation directory
  RMDir /r "$INSTDIR"
 
  ; Delete Start Menu Shortcuts
  Delete "$DESKTOP\${MUI_PRODUCT}.lnk"

  RMDir /r "$SMPROGRAMS\${MUI_PRODUCT}"
 
  ; Delete Uninstaller And Unistall Registry Entries
  DeleteRegKey HKEY_LOCAL_MACHINE "SOFTWARE\${MUI_PRODUCT}"
  DeleteRegKey HKEY_LOCAL_MACHINE "SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\${MUI_PRODUCT}"  
 
SectionEnd
 
 
;--------------------------------    
;MessageBox Section
 
 
;Function that calls a messagebox when installation finished correctly
Function .onInstSuccess
  MessageBox MB_OK "You have successfully installed ${MUI_PRODUCT}. Use the desktop icon to start the program."
FunctionEnd
 
Function un.onUninstSuccess
  MessageBox MB_OK "You have successfully uninstalled ${MUI_PRODUCT}."
FunctionEnd
 
 
;eof
