$ErrorActionPreference = 'Stop'
$scriptPath = Join-Path $PSScriptRoot '../loader/src-tauri/src/patch/shadows_install.ps1'
$source = [IO.File]::ReadAllText((Resolve-Path -LiteralPath $scriptPath))
$start = $source.IndexOf('function Get-ForgeBodyBlockStart(')
$end = $source.IndexOf('function HashForgePair(', $start)
if ($start -lt 0 -or $end -lt 0) { throw '64-bit FORGE helper missing' }
# Execute only the pure function; never run game installation in CI.
. ([scriptblock]::Create($source.Substring($start, $end - $start)))
$cases = @(
  @{prefix=512; offset=0; length=128; expected=128},
  @{prefix=512; offset=500; length=128; expected=12},
  @{prefix=512; offset=512; length=128; expected=0},
  @{prefix=512; offset=700; length=128; expected=0},
  @{prefix=512; offset=2148561062L; length=4096; expected=0},
  @{prefix=512; offset=5000000000L; length=4096; expected=0},
  @{prefix=5000000200L; offset=5000000000L; length=4096; expected=200},
  @{prefix=5000000200L; offset=0; length=4096; expected=4096}
)
foreach ($case in $cases) {
  $actual = Get-ForgeBodyBlockStart -Prefix $case.prefix -Offset $case.offset -BytesRead $case.length
  if ($actual -ne $case.expected) {
    throw "Unexpected start for prefix=$($case.prefix) offset=$($case.offset): $actual != $($case.expected)"
  }
}
Write-Host "8 FORGE Int64 offset regression tests passed"
