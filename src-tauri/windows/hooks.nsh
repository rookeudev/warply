; Restore the searchable Start menu entry on fresh installs and updates.
; The default Tauri updater skips shortcut creation in update mode.
!macro NSIS_HOOK_POSTINSTALL
  SetShellVarContext all
  CreateShortCut "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe" "" "$INSTDIR\${MAINBINARYNAME}.exe" 0 SW_SHOWNORMAL "" "Warply - Cloudflare WARP"
  !insertmacro SetLnkAppUserModelId "$SMPROGRAMS\${PRODUCTNAME}.lnk"
  System::Call 'shell32::SHChangeNotify(i 0x00000002, i 0x00000005, w "$SMPROGRAMS\${PRODUCTNAME}.lnk", p 0)'
!macroend
