!include TextFunc.nsh
; Bundled extension choices; schema independent of project data.
Var SyndocalDjLinkCheckbox
Var SyndocalDjLinkChoice
Var SyndocalExtensionDir

!macro SYNDOCAL_EXTENSIONS_PAGE
Page custom SyndocalExtensionsPage SyndocalExtensionsLeave
Function SyndocalExtensionsPage
  Call SkipIfPassive
  nsDialogs::Create 1018
  Pop $R0
  ${If} $R0 == error
    Abort
  ${EndIf}
  !insertmacro MUI_HEADER_TEXT "Extensions / 拡張機能" "Choose integrations / 使用する拡張を選択"
  ${NSD_CreateLabel} 0 0 100% 35u "DMX, video and MIDI are included.$\r$\nDMX・映像・MIDIは本体に含まれます。"
  Pop $R1
  ${NSD_CreateCheckbox} 0 45u 100% 20u "DJ Link (DJ computer integration / DJ用PCとの連携)"
  Pop $SyndocalDjLinkCheckbox
  ${NSD_SetState} $SyndocalDjLinkCheckbox $SyndocalDjLinkChoice
  nsDialogs::Show
FunctionEnd
Function SyndocalExtensionsLeave
  ${NSD_GetState} $SyndocalDjLinkCheckbox $SyndocalDjLinkChoice
FunctionEnd
Function SyndocalExtensionsInit
  ; The choice is per user even when installer shell context is all users.
  ReadEnvStr $SyndocalExtensionDir "LOCALAPPDATA"
  StrCmp $SyndocalExtensionDir "" SyndocalExtensionsInvalid
  StrCpy $SyndocalExtensionDir "$SyndocalExtensionDir\${BUNDLEID}"
  StrCpy $SyndocalDjLinkChoice 0
  IfFileExists "$SyndocalExtensionDir\extensions.ini" 0 SyndocalExtensionsInitDone
  ClearErrors
  FileOpen $R7 "$SyndocalExtensionDir\extensions.ini" r
  IfErrors SyndocalExtensionsInvalid
  FileRead $R7 $R8
  ${TrimNewLines} $R8 $R8
  StrCmp $R8 "[SyndocalExtensions]" 0 SyndocalExtensionsInvalidClose
  FileRead $R7 $R8
  ${TrimNewLines} $R8 $R8
  StrCmp $R8 "Schema=1" 0 SyndocalExtensionsInvalidClose
  FileRead $R7 $R8
  ${TrimNewLines} $R8 $R8
  StrCmp $R8 "DJLink=0" SyndocalExtensionsCore
  StrCmp $R8 "DJLink=1" SyndocalExtensionsDjLink SyndocalExtensionsInvalidClose
  SyndocalExtensionsCore:
  StrCpy $SyndocalDjLinkChoice 0
  Goto SyndocalExtensionsEndCheck
  SyndocalExtensionsDjLink:
  StrCpy $SyndocalDjLinkChoice 1
  SyndocalExtensionsEndCheck:
  ClearErrors
  FileRead $R7 $R8
  IfErrors SyndocalExtensionsValidClose
  ${TrimNewLines} $R8 $R8
  StrCmp $R8 "" SyndocalExtensionsEndCheck SyndocalExtensionsInvalidClose
  SyndocalExtensionsValidClose:
  FileClose $R7
  Goto SyndocalExtensionsInitDone
  SyndocalExtensionsInvalidClose:
  FileClose $R7
  SyndocalExtensionsInvalid:
  MessageBox MB_ICONSTOP|MB_OK "Invalid or unsupported extension settings. Preserve extensions.ini and install a compatible version." /SD IDOK
  SetErrorLevel 1
  Abort
  SyndocalExtensionsInitDone:

  ClearErrors
  ${GetOptions} $CMDLINE "/DJLINK=" $R8
  ${IfNot} ${Errors}
    ${If} $R8 != "0"
    ${AndIf} $R8 != "1"
      MessageBox MB_ICONSTOP|MB_OK "Use /DJLINK=0 or /DJLINK=1." /SD IDOK
      SetErrorLevel 1
      Abort
    ${EndIf}
    StrCpy $SyndocalDjLinkChoice $R8
  ${EndIf}
FunctionEnd
!macroend

!macro NSIS_HOOK_POSTINSTALL
  CreateDirectory "$SyndocalExtensionDir"
  ClearErrors
  FileOpen $R8 "$SyndocalExtensionDir\extensions.ini.pending" w
  ${If} ${Errors}
    MessageBox MB_ICONSTOP|MB_OK "Extension settings could not be written." /SD IDOK
    SetErrorLevel 1
    Abort
  ${EndIf}
  FileWrite $R8 "[SyndocalExtensions]$\r$\nSchema=1$\r$\nDJLink=$SyndocalDjLinkChoice$\r$\n"
  ${If} ${Errors}
    FileClose $R8
    Delete "$SyndocalExtensionDir\extensions.ini.pending"
    MessageBox MB_ICONSTOP|MB_OK "Extension settings could not be written." /SD IDOK
    SetErrorLevel 1
    Abort
  ${EndIf}
  FileClose $R8
  System::Call 'kernel32::MoveFileExW(w "$SyndocalExtensionDir\extensions.ini.pending", w "$SyndocalExtensionDir\extensions.ini", i 1)i.rR8'
  ${If} $R8 == 0
    MessageBox MB_ICONSTOP|MB_OK "Extension settings could not be saved. Installation cannot continue." /SD IDOK
    SetErrorLevel 1
    Abort
  ${EndIf}
!macroend
