; Repair upgrades from 0.2.1 without invoking its broken uninstaller.
; MUI invokes this after .onInit and before displaying its pages.
!define MUI_CUSTOMFUNCTION_GUIINIT WarplyRepairUpgrade
Function WarplyRepairUpgrade
  Push $0
  Push $1
  ClearErrors
  ${GetOptions} $CMDLINE "/UPDATE" $0
  ${IfNot} ${Errors}
    Goto warply_repair_init_done
  ${EndIf}
  ReadRegStr $0 HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Warply" "DisplayVersion"
  ${If} $0 == "0.2.1"
    ClearErrors
    ${GetOptions} $CMDLINE "/P" $1
    ${IfNot} ${Errors}
      Exec '$\"$EXEPATH$\" /UPDATE /P'
    ${Else}
      Exec '$\"$EXEPATH$\" /UPDATE'
    ${EndIf}
    ${If} ${Errors}
      MessageBox MB_ICONSTOP "Could not start repair upgrade. Run this installer with /UPDATE to replace the previous version while keeping your profile."
    ${EndIf}
    Quit
  ${EndIf}
  warply_repair_init_done:
  Pop $1
  Pop $0
FunctionEnd

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
  IfFileExists "$INSTDIR\${MAINBINARYNAME}.exe" 0 warply_cleanup_missing
  ClearErrors
  ExecWait '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --cleanup-for-uninstall' $0
  IfErrors warply_cleanup_missing
  ${Switch} $0
    ${Case} 0
      Goto warply_cleanup_done
    ${Case} 20
      MessageBox MB_ICONSTOP "Windows could not stop the Warply tunnel service. Restore internet in Warply or restart Windows, then retry uninstall."
      ${Break}
    ${Case} 21
      MessageBox MB_ICONSTOP "Windows could not verify or remove Warply's firewall protection. Reinstall the latest Warply and use Restore internet, then retry."
      ${Break}
    ${Case} 22
      MessageBox MB_ICONSTOP "The tunnel and protection are stopped, but Windows could not remove the encrypted service copy. Reinstall the latest Warply and retry uninstall. Your account profile is preserved."
      ${Break}
    ${Case} 23
      MessageBox MB_ICONSTOP "Uninstall needs administrator rights for network cleanup. Approve the Windows prompt and retry."
      ${Break}
    ${Default}
      MessageBox MB_ICONSTOP "Network cleanup did not complete. Reinstall the latest Warply to repair its uninstaller, then retry."
  ${EndSwitch}
  Abort
  warply_cleanup_missing:
  MessageBox MB_ICONSTOP "The Warply cleanup executable is missing or could not start. Reinstall the latest Warply before uninstalling."
  Abort
  warply_cleanup_done:
!macroend
