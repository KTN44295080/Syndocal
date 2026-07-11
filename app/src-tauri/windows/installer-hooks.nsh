!macro NSIS_HOOK_POSTINSTALL
  SetOutPath "$INSTDIR"
  CopyFiles /SILENT "$INSTDIR\runtime-libs\*.dll" "$INSTDIR"
  RMDir /r "$INSTDIR\runtime-libs"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Delete "$INSTDIR\avcodec-*.dll"
  Delete "$INSTDIR\avdevice-*.dll"
  Delete "$INSTDIR\avfilter-*.dll"
  Delete "$INSTDIR\avformat-*.dll"
  Delete "$INSTDIR\avutil-*.dll"
  Delete "$INSTDIR\swresample-*.dll"
  Delete "$INSTDIR\swscale-*.dll"
!macroend
