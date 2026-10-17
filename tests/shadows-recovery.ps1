param([string]$Workspace = '')
Set-StrictMode -Version Latest
$ErrorActionPreference='Stop'
$ProgressPreference='SilentlyContinue'
$repo=Split-Path -Parent $PSScriptRoot
$installer=Join-Path $repo 'loader/src-tauri/src/patch/shadows_install.ps1'
$uninstaller=Join-Path $repo 'loader/src-tauri/src/patch/shadows_uninstall.ps1'
$powershell=(Get-Process -Id $PID).Path
if(-not $Workspace){$Workspace=Join-Path ([IO.Path]::GetTempPath()) ('animus-shadows-test-'+[Guid]::NewGuid().ToString('N'))}
New-Item -ItemType Directory -Force -Path $Workspace|Out-Null

# Import the actual installer's functions without executing its main body.
$tokens=$null;$errors=$null
$ast=[Management.Automation.Language.Parser]::ParseFile($installer,[ref]$tokens,[ref]$errors)
if($errors.Count){throw ($errors|Out-String)}
$defs=$ast.FindAll({param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst]},$false)
foreach($definition in $defs){Invoke-Expression $definition.Extent.Text}
function Assert([bool]$Value,[string]$Message){if(-not $Value){throw $Message}}
$map=[ordered]@{
 'DataPC_boot'='DataPC_boot.forge'
 'DataPC_boot_patch_01'='DataPC_boot_patch_01.forge'
 'DataPC_boot_patch_02'='DataPC_boot_patch_02.forge'
 'dlc_20_boot'='dlc_20/DataPC_boot_20_dlc.forge'
 'dlc_20_boot_patch_01'='dlc_20/DataPC_boot_20_dlc_patch_01.forge'
}
try{
 foreach($mode in @('installed','applying','tampered-body','tampered-backup')){
  $game=Join-Path $Workspace $mode
  $backup=Join-Path $game '.animus-shadows-backup'
  New-Item -ItemType Directory -Force -Path (Join-Path $backup 'index')|Out-Null
  $transaction=[Guid]::NewGuid().ToString()
  SaveJson (Join-Path $backup 'loader-owner.json') ([pscustomobject]@{transaction=$transaction})
  $files=New-Object System.Collections.ArrayList
  $position=0
  foreach($group in $map.Keys){
   $relative=[string]$map[$group];$live=Rel $game $relative
   New-Item -ItemType Directory -Force -Path ([IO.Path]::GetDirectoryName($live))|Out-Null
   $bytes=New-Object byte[] 4096
   for($i=0;$i -lt $bytes.Length;$i++){$bytes[$i]=[byte](($i+$position)%251)}
   [Text.Encoding]::ASCII.GetBytes('scimitar').CopyTo($bytes,0)
   [BitConverter]::GetBytes([uint32]3).CopyTo($bytes,0x41A)
   for($i=0;$i -lt 3;$i++){
    [BitConverter]::GetBytes([uint64](1250+$i*300)).CopyTo($bytes,(0x462+$i*24))
    [BitConverter]::GetBytes([uint64](0x12345678901+$i)).CopyTo($bytes,(0x462+$i*24+8))
    [BitConverter]::GetBytes([uint32]100).CopyTo($bytes,(0x462+$i*24+16))
   }
   [IO.File]::WriteAllBytes($live,$bytes)
   $info=MakePrefixInfo $live
   CheckForgeEntries $live $info.count @('12345678901','12345678902')
   $rejected=$false
   try{CheckForgeEntries $live $info.count @('12345678999')}catch{$rejected=$true}
   Assert $rejected 'Missing resource ID was accepted'
   $save='index/'+$group+'.bin'
   CopyPrefix -SourcePath $live -DestinationPath (Rel $backup $save) -PrefixLength $info.prefix_len
   $pair=HashForgePair -File $live -Prefix $info.prefix_len -Length $info.length
   Assert ($pair.full -ceq (Hash $live)) 'Combined full hash differs'
   Assert ($pair.tail -ceq (HashTail $live $info.prefix_len $info.length)) 'Combined tail hash differs'
   $entry=[pscustomobject]@{path=$relative;group=$group;original_length=$info.length;prefix_length=$info.prefix_len;entry_count=$info.count;prefix_sha256=(Hash (Rel $backup $save));original_sha256=$pair.full;original_suffix_sha256=$pair.tail;patched_sha256='';backup=$save}
   if($mode -ne 'applying' -or $position -lt 2){
    $changed=New-Object byte[] ($bytes.Length+128)
    $bytes.CopyTo($changed,0)
    [BitConverter]::GetBytes([uint64]$bytes.Length).CopyTo($changed,0x462)
    [BitConverter]::GetBytes([uint32]128).CopyTo($changed,(0x462+16))
    for($i=$bytes.Length;$i -lt $changed.Length;$i++){$changed[$i]=[byte]170}
    [IO.File]::WriteAllBytes($live,$changed)
   }
   if($mode -eq 'installed' -or $mode -like 'tampered-*'){$entry.patched_sha256=Hash $live}
   [void]$files.Add($entry)
   $position++
  }
  $status='installed';if($mode -eq 'applying' -or $mode -eq 'tampered-body'){$status='applying'}
  $journal=[pscustomobject]@{schema_version=2;loader_transaction=$transaction;edition='ORIJINAL';engine='bounded-index-append-v1';status='prepared';created_utc=[DateTime]::UtcNow.ToString('o');files=@($files.ToArray())}
  $meta=Join-Path $backup 'install.json'
  SaveJson $meta $journal
  $journal.status=$status;SaveJson $meta $journal
  $saved=Get-Content -LiteralPath $meta -Raw -Encoding UTF8|ConvertFrom-Json
  Assert ($saved.status -ceq $status) 'Durable journal replacement failed'
  $last=$files[$files.Count-1]
  if($mode -eq 'tampered-body'){
   $live=Rel $game $last.path;$bytes=[IO.File]::ReadAllBytes($live);$bytes[$last.prefix_length+17]=$bytes[$last.prefix_length+17] -bxor 255;[IO.File]::WriteAllBytes($live,$bytes)
  }
  if($mode -eq 'tampered-backup'){
   $path=Rel $backup $last.backup;$bytes=[IO.File]::ReadAllBytes($path);$bytes[9]=$bytes[9] -bxor 255;[IO.File]::WriteAllBytes($path,$bytes)
  }
  $before=@{};foreach($entry in $files){$before[$entry.path]=Hash (Rel $game $entry.path)}
  $log=@(& $powershell -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $uninstaller -GameRoot $game -LoaderTransaction $transaction 2>&1)
  $exitCode=$LASTEXITCODE
  if($mode -like 'tampered-*'){
   Assert ($exitCode -ne 0) "Invalid state accepted: $mode"
   foreach($entry in $files){Assert ((Hash (Rel $game $entry.path)) -ceq $before[$entry.path]) "Game changed before conflict rejection: $mode / $($entry.path)"}
   Assert (Test-Path -LiteralPath $meta) 'Conflict removed backup material'
  }else{
   if($exitCode -ne 0){throw ($log -join [Environment]::NewLine)}
   foreach($entry in $files){Assert ((Hash (Rel $game $entry.path)) -ceq $entry.original_sha256) "Original hash not restored: $mode / $($entry.path)"}
   Assert (-not(Test-Path -LiteralPath $backup)) 'Active backup was not archived'
   Assert (@(Get-ChildItem -LiteralPath $game -Directory -Filter '.animus-shadows-history-*' -Force).Count -eq 1) 'History missing'
  }
  Write-Output "PASS: Shadows $mode"
 }
 Write-Output 'PASS: 4 real-script recovery fixtures, resource IDs and atomic journals'
}finally{
 if(Test-Path -LiteralPath $Workspace){Remove-Item -LiteralPath $Workspace -Recurse -Force}
}

# Expected child rejections must not leak LASTEXITCODE to the Actions wrapper.
exit 0
