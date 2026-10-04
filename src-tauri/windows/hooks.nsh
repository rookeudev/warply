; Preserve an existing binary before replacement. No config/key backup is made.
!macro NSIS_HOOK_PREINSTALL
  !insertmacro CheckIfAppIsRunning "$INSTDIR\${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
  IfFileExists "$INSTDIR\${MAINBINARYNAME}.exe" 0 warply_backup_done
  IfFileExists "$INSTDIR\${MAINBINARYNAME}.previous.exe" warply_backup_done 0
  System::Call 'kernel32::CopyFileW(w "$INSTDIR\${MAINBINARYNAME}.exe", w "$INSTDIR\${MAINBINARYNAME}.previous.exe", i 1) i.r0'
  ${If} $0 == 0
    MessageBox MB_ICONSTOP "Could not preserve the current Warply version. Close Warply and try installing again."
    Abort
  ${EndIf}
  warply_backup_done:
!macroend

Function .onInstFailed
  IfFileExists "$INSTDIR\${MAINBINARYNAME}.previous.exe" 0 warply_restore_done
  System::Call 'kernel32::CopyFileW(w "$INSTDIR\${MAINBINARYNAME}.previous.exe", w "$INSTDIR\${MAINBINARYNAME}.exe", i 0) i.r0'
  ${If} $0 != 0
    Delete "$INSTDIR\${MAINBINARYNAME}.previous.exe"
  ${EndIf}
  warply_restore_done:
FunctionEnd

; Restore the searchable Start menu entry on fresh installs and updates.
; The default Tauri updater skips shortcut creation in update mode.
!macro NSIS_HOOK_POSTINSTALL
  ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --migrate-startup' $0
  SetShellVarContext all
  CreateShortCut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe" "" "$INSTDIR\${MAINBINARYNAME}.exe" 0 SW_SHOWNORMAL "" "Warply - Cloudflare WARP"
  !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  Delete "$INSTDIR\${MAINBINARYNAME}.previous.exe"
  System::Call 'shell32::SHChangeNotify(i 0x00000002, i 0x00000005, w "$SMPROGRAMS\${PRODUCTNAME}.lnk", p 0)'
!macroend

; Clear only Warply protection before removing its recovery executable.
!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro CheckIfAppIsRunning "$INSTDIR\${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
  ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --restore-internet' $0
  ${If} $0 != 0
    MessageBox MB_ICONSTOP "Warply could not remove its protection. Open Warply, select Restore internet, then uninstall again."
    Abort
  ${EndIf}
!macroend
