# RETIRED — this former single-window 4K acceptance entrypoint is intentionally
# no longer a native test runner.
#
# The final show contract is:
#   editor    \\.\DISPLAY2  1920x1080
#   LED       \\.\DISPLAY5  1920x1080
#   projector \\.\DISPLAY3  3840x2160 at 150%
#
# The old implementation moved the main editor to DISPLAY3.  That is unsafe:
# DISPLAY3 is now the projector output target.  It was removed rather than
# retained as a compatibility path.  Use run-syndocal-three-display-show-
# acceptance.ps1 after its app-owned exact output-ID observation is available.
#
# This one-way retired entrypoint is deliberately non-mutating.  It exists only
# to make a copied historical command fail visibly with its replacement.

[CmdletBinding()]
param(
  # Catch copied legacy flags (for example -Apply) so the operator receives
  # the replacement instruction rather than PowerShell's generic binder text.
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]]$RetiredArguments = @()
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-RetiredNative4kAcceptanceMessage {
  return (
    "Fail closed: run-syndocal-native-4k-acceptance.ps1 is retired. " +
    "It cannot move the editor to DISPLAY3 because DISPLAY3 is the projector output target. " +
    "Use qa\\harnesses\\run-syndocal-three-display-show-acceptance.ps1 for the " +
    "DISPLAY2 editor / DISPLAY5 LED / DISPLAY3 projector acceptance after the " +
    "app-owned exact output ID-to-HWND observation provider is installed."
  )
}

function Invoke-RetiredNative4kAcceptance {
  param([string[]]$Arguments = @())
  $message = Get-RetiredNative4kAcceptanceMessage
  if ($Arguments.Count -gt 0) {
    $message += " Retired arguments are not accepted: $($Arguments -join ' ')."
  }
  throw $message
}

# Dot-sourcing stays side-effect-free for the focused retirement test.
if ($MyInvocation.InvocationName -ne ".") {
  try {
    Invoke-RetiredNative4kAcceptance -Arguments $RetiredArguments
  } catch {
    [Console]::Error.WriteLine($_.Exception.Message)
  }
  exit 2
}
