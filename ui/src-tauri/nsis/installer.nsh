!macro NSIS_HOOK_POSTINSTALL
  IfFileExists "$INSTDIR\tvpnd.exe" 0 +2
    ExecWait '"$INSTDIR\tvpnd.exe" --install'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  IfFileExists "$INSTDIR\tvpnd.exe" 0 +2
    ExecWait '"$INSTDIR\tvpnd.exe" --uninstall'
!macroend
