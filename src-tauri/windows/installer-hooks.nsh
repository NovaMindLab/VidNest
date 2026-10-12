!macro NSIS_HOOK_PREINSTALL
  ; Kill running process so files are never locked and overwrite succeeds seamlessly
  nsExec::Exec 'taskkill /F /IM "Local Video Intelligence.exe"'
  nsExec::Exec 'taskkill /F /IM "local-video-intelligence.exe"'
  Sleep 500
!macroend
