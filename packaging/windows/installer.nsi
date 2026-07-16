; Davenstein Windows installer
; CI supplies VERSION as an NSIS preprocessor definition
; Expects a payload directory next to this script containing
;   Davenstein.exe, assets.pak, README.md
; CI stages these files before calling makensis
;
; The installer deliberately omits portable.flag
; Installed copies use the platform data directory rather than data beside the executable

!ifndef VERSION
  !define VERSION "0.0.0-dev"
!endif

!define APP_NAME "Davenstein"
!define APP_PUBLISHER "David Petnick"
!define APP_EXE "Davenstein.exe"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"

Name "${APP_NAME}"
OutFile "Davenstein-Setup-${VERSION}.exe"
InstallDir "$PROGRAMFILES64\${APP_NAME}"
RequestExecutionLevel admin
ShowInstDetails show
ShowUnInstDetails show

Function .onInit
  SetRegView 64
  ReadRegStr $0 HKLM "Software\${APP_NAME}" "InstallDir"
  StrCmp $0 "" no_previous_install
  StrCpy $INSTDIR $0
no_previous_install:
FunctionEnd

Function un.onInit
  SetRegView 64
FunctionEnd

Page components
Page directory
Page instfiles

UninstPage uninstConfirm
UninstPage instfiles

Section "Davenstein (required)" SEC_MAIN
  SectionIn RO
  SetShellVarContext all
  SetOutPath "$INSTDIR"
  File "payload\${APP_EXE}"
  File "payload\assets.pak"
  File "payload\README.md"

  WriteRegStr HKLM "Software\${APP_NAME}" "InstallDir" "$INSTDIR"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "UninstallString" "$\"$INSTDIR\Uninstall.exe$\""
  WriteRegStr HKLM "${UNINSTALL_KEY}" "QuietUninstallString" "$\"$INSTDIR\Uninstall.exe$\" /S"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\${APP_EXE}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "Publisher" "${APP_PUBLISHER}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoRepair" 1

  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
SectionEnd

Section /o "Desktop Shortcut" SEC_DESKTOP
  CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"
SectionEnd

Section "Uninstall"
  SetShellVarContext all
  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\assets.pak"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\Uninstall.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"
  Delete "$DESKTOP\${APP_NAME}.lnk"

  DeleteRegKey HKLM "${UNINSTALL_KEY}"
  DeleteRegKey HKLM "Software\${APP_NAME}"
SectionEnd
