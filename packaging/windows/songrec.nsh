; NSIS installer generation script for SongRec
 
; Cf. https://nsis.sourceforge.io/Sample_installation_script_for_an_application
; Cf. https://stackoverflow.com/questions/11014013/copy-a-directory-using-nsis
; Cf. https://fr.wikipedia.org/wiki/Nullsoft_Scriptable_Install_System
; Cf. https://sources.debian.org/src/gentle/1.9%2Bcvs20100605%2Bdfsg1-7/InstallerScript.nsi
; Cf. https://github.com/uroni/urbackup_frontend_wx/blob/dev/urbackup_x64.nsi
; Cf. https://github.com/ib/gucharmap/blob/master/gucharmap.nsi.in ?
 
; -------------------------------
; Start
 
  !define MUI_PRODUCT "SongRec"
  !define MUI_FILE "songrec"
  !define MUI_VERSION "0.6.7"
  !define MUI_BRANDINGTEXT "${MUI_PRODUCT} ${MUI_VERSION}"
  !define IN_DIR "XX WIP"
  CRCCheck On
 
  ; We should test if we must use an absolute path 
  !include "${NSISDIR}\Contrib\Modern UI\MUI2.nsh"
 
 
;---------------------------------
;General
 
  OutFile "${MUI_PRODUCT}-${MUI_VERSION}-installer.exe"
  ShowInstDetails "nevershow"
  ShowUninstDetails "nevershow"
  SetCompressor "lzma"
 
  !define MUI_ICON "songrec.ico"
  !define MUI_UNICON "songrec.ico"
  !define MUI_SPECIALBITMAP "songrec.bmp"
 
 
;--------------------------------
;Folder selection page
 
  InstallDir "$PROGRAMFILES\${MUI_PRODUCT}"
 
 
;--------------------------------
;Modern UI Configuration
 
  !define MUI_WELCOMEPAGE  
  !define MUI_LICENSEPAGE
  !define MUI_DIRECTORYPAGE
  !define MUI_ABORTWARNING
  !define MUI_UNINSTALLER
  !define MUI_UNCONFIRMPAGE
  !define MUI_FINISHPAGE  
 
 
;--------------------------------
;Language
 
  !insertmacro MUI_LANGUAGE "English"
 
 
;-------------------------------- 
;Modern UI System
 
  !insertmacro MUI_SYSTEM 
 
 
;--------------------------------
;Data
 
  LicenseData "LICENSE"
 
 
;-------------------------------- 
;Installer Sections     
Section "install" Installation info
 
;Add files
  SetOutPath "$INSTDIR"
 
  File /r "${IN_DIR}\*"
 
;create desktop shortcut
  CreateShortCut "$DESKTOP\${MUI_PRODUCT}.lnk" "$INSTDIR\${MUI_FILE}.exe" ""
 
;create start-menu items
  CreateDirectory "$SMPROGRAMS\${MUI_PRODUCT}"
  CreateShortCut "$SMPROGRAMS\${MUI_PRODUCT}\Uninstall.lnk" "$INSTDIR\Uninstall.exe" "" "$INSTDIR\Uninstall.exe" 0
  CreateShortCut "$SMPROGRAMS\${MUI_PRODUCT}\${MUI_PRODUCT}.lnk" "$INSTDIR\${MUI_FILE}.exe" "" "$INSTDIR\${MUI_FILE}.exe" 0
 
;write uninstall information to the registry
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${MUI_PRODUCT}" "DisplayName" "${MUI_PRODUCT} (remove only)"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${MUI_PRODUCT}" "UninstallString" "$INSTDIR\Uninstall.exe"
 
  WriteUninstaller "$INSTDIR\Uninstall.exe"
 
SectionEnd
 
 
;--------------------------------    
;Uninstaller Section  
Section "Uninstall"
 
;Delete Files 
  RMDir /r "$INSTDIR\*.*"    
 
;Remove the installation directory
  RMDir "$INSTDIR"
 
;Delete Start Menu Shortcuts
  Delete "$DESKTOP\${MUI_PRODUCT}.lnk"
  Delete "$SMPROGRAMS\${MUI_PRODUCT}\*.*"
  RmDir  "$SMPROGRAMS\${MUI_PRODUCT}"
 
;Delete Uninstaller And Unistall Registry Entries
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
