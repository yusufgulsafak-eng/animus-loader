# ANIMUS | Assassin's Creed Shadows | Dusuk alan gerektiren geri alinabilir kurulum
# Sadece dogrulanan modloc build'i: append payload + 12 byte indeks guncellemesi.
param([Parameter(Mandatory=$true)][string]$GameRoot,[Parameter(Mandatory=$true)][string]$PackageRoot,[Parameter(Mandatory=$true)][string]$LoaderTransaction,[switch]$Fast)
Set-StrictMode -Version Latest
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$OutputEncoding=[Console]::OutputEncoding
$ErrorActionPreference = 'Stop'
$Paket = $PackageRoot
$PaketAdi = 'ORIJINAL'
$ForgeMap = [ordered]@{
  'DataPC_boot' = 'DataPC_boot.forge'
  'DataPC_boot_patch_01' = 'DataPC_boot_patch_01.forge'
  'DataPC_boot_patch_02' = 'DataPC_boot_patch_02.forge'
  'dlc_20_boot' = 'dlc_20/DataPC_boot_20_dlc.forge'
  'dlc_20_boot_patch_01' = 'dlc_20/DataPC_boot_20_dlc_patch_01.forge'
}
$Stage = $null
$BackupRoot = $null
$JournalStarted = $false
$script:LogPath = Join-Path $Paket 'PATCH_LOG.txt'
function Msg([string]$Text){Write-Host $Text;Add-Content -LiteralPath $script:LogPath -Value $Text -Encoding UTF8 -ErrorAction SilentlyContinue}

# Progress counters are based on completed work, not promises about exact duration.
# The ETA is a moving estimate for the CURRENT stage, not the whole installation.
$script:RunTimer=[Diagnostics.Stopwatch]::StartNew()
$script:CurrentPhase=''
$script:PhaseStarted=0.0
function TimeText([double]$Seconds){
  $s=[long][Math]::Max(0,[Math]::Round($Seconds))
  return ('{0:D2}:{1:D2}:{2:D2}' -f [int][Math]::Floor($s/3600),[int][Math]::Floor(($s%3600)/60),[int]($s%60))
}
function ShowStage([string]$Phase,[int]$Done,[int]$Total,[int]$From,[int]$To){
  if($script:CurrentPhase -cne $Phase){
    $script:CurrentPhase=$Phase
    $script:PhaseStarted=$script:RunTimer.Elapsed.TotalSeconds
  }
  $totalSafe=[Math]::Max(1,$Total)
  $percent=[int][Math]::Min(100,[Math]::Floor($From+(($To-$From)*[double]$Done/$totalSafe)))
  $elapsed=TimeText $script:RunTimer.Elapsed.TotalSeconds
  $eta='hesaplaniyor'
  if($Done -gt 0){
    $stageSeconds=$script:RunTimer.Elapsed.TotalSeconds-$script:PhaseStarted
    $estimated=$stageSeconds*[Math]::Max(0,($Total-$Done))/$Done
    $eta=TimeText $estimated
  }
  $status=('{0} | {1}/{2} | %{3} | gecen {4} | bu asama kalan ~{5}' -f $Phase,$Done,$Total,$percent,$elapsed,$eta)
  Write-Output ('[ANIMUS_PROGRESS]{0}|{1}' -f $percent,$status)
  Write-Progress -Id 1 -Activity 'ANIMUS SHADOWS - YAMA KUR' -Status $status -PercentComplete $percent
  if($Done -gt 0 -and ($Done -eq $Total -or $Done % 5 -eq 0)){
    Msg ('[ILERLEME] {0} | {1}/{2} | %{3} | gecen {4} | bu asama kalan ~{5}' -f $Phase,$Done,$Total,$percent,$elapsed,$eta)
  }
}
function FinishProgress(){Write-Progress -Id 1 -Activity 'ANIMUS SHADOWS - YAMA KUR' -Completed}

# FAST: compute both the full SHA-256 and unchanged-body SHA-256 in ONE scan.
# Both values are still stored in the original v2 uninstall journal.
function HashForgePair([string]$File,[long]$Prefix,[long]$Length){
  SafeFile $File
  $src=[IO.File]::Open($File,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
  $whole=[Security.Cryptography.SHA256]::Create()
  $body=[Security.Cryptography.SHA256]::Create()
  try{
    if($src.Length -ne $Length -or $Prefix -gt $Length -or $Prefix -lt 1){Die "FORGE uzunlugu degismis: $File"}
    $buf=New-Object byte[] (4MB)
    [long]$offset=0
    $lastPct=-1
    while($offset -lt $Length){
      $wanted=[int][Math]::Min([long]$buf.Length,$Length-$offset)
      $n=$src.Read($buf,0,$wanted)
      if($n -le 0){Die "FORGE hash okunamadi: $File"}
      [void]$whole.TransformBlock($buf,0,$n,$buf,0)
      $bodyStart=[int][Math]::Max(0,[long]$Prefix-$offset)
      if($bodyStart -lt $n){
        [void]$body.TransformBlock($buf,$bodyStart,$n-$bodyStart,$buf,$bodyStart)
      }
      $offset+=$n
      $pct=[int][Math]::Floor(100*[double]$offset/$Length)
      if($pct -ge ($lastPct+5) -or $pct -eq 100){
        Write-Progress -Id 2 -ParentId 1 -Activity 'FORGE butunluk taramasi (FAST)' -Status ('{0} | %{1}' -f [IO.Path]::GetFileName($File),$pct) -PercentComplete $pct
        $lastPct=$pct
      }
    }
    [void]$whole.TransformFinalBlock([byte[]]@(),0,0)
    [void]$body.TransformFinalBlock([byte[]]@(),0,0)
    return [pscustomobject]@{
      full=([BitConverter]::ToString($whole.Hash)).Replace('-','').ToLowerInvariant()
      tail=([BitConverter]::ToString($body.Hash)).Replace('-','').ToLowerInvariant()
    }
  }finally{
    Write-Progress -Id 2 -Activity 'FORGE butunluk taramasi (FAST)' -Completed
    $whole.Dispose();$body.Dispose();$src.Dispose()
  }
}
function AssertGameStopped(){
  if(Get-Process -Name 'ACShadows' -ErrorAction SilentlyContinue){Die "Assassin's Creed Shadows acik. Oyunu kapatip yeniden deneyin."}
}
function Die([string]$Text){throw $Text}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Rel([string]$Root,[string]$Part){Join-Path $Root ($Part.Replace('/',[IO.Path]::DirectorySeparatorChar))}
function SaveJson([string]$Target,$Value){
  $temporary=$Target+'.tmp'
  $bytes=[Text.UTF8Encoding]::new($false).GetBytes(($Value|ConvertTo-Json -Depth 10))
  $stream=[IO.File]::Open($temporary,[IO.FileMode]::Create,[IO.FileAccess]::Write,[IO.FileShare]::None)
  try{$stream.Write($bytes,0,$bytes.Length);$stream.Flush($true)}finally{$stream.Dispose()}
  if([IO.File]::Exists($Target)){[IO.File]::Replace($temporary,$Target,($Target+'.previous'))}
  else{[IO.File]::Move($temporary,$Target)}
}
function SafeFile([string]$Path){
  if(-not(Test-Path -LiteralPath $Path -PathType Leaf)){Die "Dosya eksik: $Path"}
  $Info=Get-Item -LiteralPath $Path -Force
  if(($Info.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0){Die "Baglanti dosyasi reddedildi: $Path"}
}
function ValidRoot([string]$Loc){
  if(-not $Loc -or -not(Test-Path -LiteralPath $Loc -PathType Container)){return $false}
  foreach($p in $ForgeMap.Values){if(-not(Test-Path -LiteralPath (Rel $Loc $p) -PathType Leaf)){return $false}}
  return $true
}
# Bos OYUN_YOLU.txt dosyasinda Get-Content hicbir satir uretmeyebilir.
# .Trim() yalnizca dogrulanmis, null-olmayan string uzerinde cagrilir.
function FindGame([string]$Explicit){
  if(-not(ValidRoot $Explicit)){Die 'Loader oyun klasoru gecersiz.'}
  return (Resolve-Path -LiteralPath $Explicit).Path
}
function MakePrefixInfo([string]$File){
  # Validated modloc uses magic "scimitar", count at 0x41A,
  # index entries 24 bytes from 0x462, and writes only append / entry+0 / entry+16.
  $stream=[IO.File]::Open($File,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
  try {
    if($stream.Length -lt 0x480){Die "FORGE cok kisa: $File"}
    $reader=[IO.BinaryReader]::new($stream)
    $magic=([Text.Encoding]::ASCII.GetString($reader.ReadBytes(8)))
    if($magic -cne 'scimitar'){Die "Desteklenmeyen FORGE basligi: $File"}
    $stream.Seek(0x41A,[IO.SeekOrigin]::Begin)|Out-Null
    $count=[long]$reader.ReadUInt32()
    if($count -lt 1 -or $count -gt 20000000){Die "Supheli FORGE indeks sayisi: $count"}
    [long]$prefix=0x462 + ($count * 24)
    if($prefix -gt $stream.Length -or $prefix -gt 512MB){Die "FORGE indeks araligi desteklenmiyor: $File. Tam yedek gerekli."}
    return [pscustomobject]@{length=[long]$stream.Length; prefix_len=$prefix; count=$count}
  }finally{$stream.Dispose()}
}
# Read the exact index format expected by the pinned modloc build.
# Every translation target must be present exactly once before touching the game.
function CheckForgeEntries([string]$File,[long]$Count,[string[]]$Ids){
  $wanted=@{}
  foreach($id in $Ids){
    [UInt64]$n=[Convert]::ToUInt64($id,16)
    $wanted[$n.ToString('X16')]=0
  }
  $src=[IO.File]::Open($File,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
  try {
    $src.Seek(0x462,[IO.SeekOrigin]::Begin)|Out-Null
    $record=New-Object byte[] (24*2048)
    [long]$left=$Count
    while($left -gt 0){
      $records=[int][Math]::Min(2048,$left)
      $bytes=$records*24
      $pos=0
      while($pos -lt $bytes){$n=$src.Read($record,$pos,$bytes-$pos);if($n -le 0){Die "FORGE indeks okunamadi: $File"};$pos+=$n}
      for($i=0;$i -lt $records;$i++){
        $hash=[BitConverter]::ToUInt64($record,($i*24)+8).ToString('X16')
        if($wanted.ContainsKey($hash)){$wanted[$hash]=[int]$wanted[$hash]+1}
      }
      $left-=$records
    }
    foreach($entry in $wanted.GetEnumerator()){
      if($entry.Value -ne 1){Die "FORGE kaydi eksik/tekrarlanmis ($($entry.Key): $($entry.Value)): $File"}
    }
  }finally{$src.Dispose()}
}
# Preserve the untouched payload body hash as well as the index snapshot.
# On interrupted writes, refuse rollback if the tool unexpectedly changed body bytes.
function HashTail([string]$File,[long]$Start,[long]$Length){
  $src=[IO.File]::Open($File,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
  $sha=[Security.Cryptography.SHA256]::Create()
  try{
    if($src.Length -lt $Length -or $Start -gt $Length){Die "FORGE boyutu degisti: $File"}
    $src.Seek($Start,[IO.SeekOrigin]::Begin)|Out-Null
    $buf=New-Object byte[] (4MB)
    [long]$remaining=$Length-$Start
    while($remaining -gt 0){
      $n=$src.Read($buf,0,[int][Math]::Min([long]$buf.Length,$remaining))
      if($n -le 0){Die "FORGE veri bolumu okunamadi: $File"}
      [void]$sha.TransformBlock($buf,0,$n,$buf,0)
      $remaining-=$n
    }
    [void]$sha.TransformFinalBlock([byte[]]@(),0,0)
    return ([BitConverter]::ToString($sha.Hash)).Replace('-','').ToLowerInvariant()
  }finally{$sha.Dispose();$src.Dispose()}
}
# $input is an automatic PowerShell variable (case-insensitive).
# Never use it as a function parameter for a file path.
function CopyPrefix(
  [Parameter(Mandatory=$true)][ValidateNotNullOrEmpty()][string]$SourcePath,
  [Parameter(Mandatory=$true)][ValidateNotNullOrEmpty()][string]$DestinationPath,
  [Parameter(Mandatory=$true)][ValidateRange(1,536870912)][long]$PrefixLength
){
  if([string]::IsNullOrWhiteSpace($SourcePath) -or [string]::IsNullOrWhiteSpace($DestinationPath)){
    Die 'Indeks yedegi icin kaynak veya hedef yol bos olamaz.'
  }
  SafeFile $SourcePath
  $parent=[IO.Path]::GetDirectoryName($DestinationPath)
  if([string]::IsNullOrWhiteSpace($parent) -or -not (Test-Path -LiteralPath $parent -PathType Container)){
    Die "Indeks yedegi hedef dizini bulunamadi: $DestinationPath"
  }
  $sourceInfo=Get-Item -LiteralPath $SourcePath
  if([long]$sourceInfo.Length -lt $PrefixLength){Die "FORGE indeks yedegi uzunlugu gecersiz: $SourcePath"}
  $src=[IO.File]::Open($SourcePath,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
  try {
    $dst=[IO.File]::Open($DestinationPath,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
    try {
      $buf=New-Object byte[] (4MB)
      [long]$left=$PrefixLength
      while($left -gt 0){
        $want=[int][Math]::Min([long]$buf.Length,$left)
        $n=$src.Read($buf,0,$want)
        if($n -le 0){Die 'Indeks yedegi kopyalanirken beklenmeyen EOF'}
        $dst.Write($buf,0,$n)
        $left-=$n
      }
      $dst.Flush($true)
    }finally{$dst.Dispose()}
  }finally{$src.Dispose()}
}
function RestorePrefix([string]$Live,[string]$Saved,[long]$Length,[long]$Prefix){
  SafeFile $Live;SafeFile $Saved
  $src=[IO.File]::OpenRead($Saved)
  try{
    if($src.Length -ne $Prefix){Die "Indeks yedegi boyutu tutmuyor: $Saved"}
    $dst=[IO.File]::Open($Live,[IO.FileMode]::Open,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
    try{
      if($dst.Length -lt $Length){Die "Oyun arsivi orijinalden kisa, kurtarma durduruldu: $Live"}
      $buf=New-Object byte[] (4MB)
      [long]$left=$Prefix
      while($left -gt 0){$want=[int][Math]::Min([long]$buf.Length,$left);$n=$src.Read($buf,0,$want);if($n -le 0){Die 'Yedek okunamadi'};$dst.Write($buf,0,$n);$left-=$n}
      $dst.SetLength($Length)
      $dst.Flush($true)
    }finally{$dst.Dispose()}
  }finally{$src.Dispose()}
}
function RestoreAll([string]$Path,[string]$Root,[bool]$RequirePatched){
  $meta=Join-Path $Path 'install.json'
  $j=Get-Content -LiteralPath $meta -Raw -Encoding UTF8|ConvertFrom-Json
  if($j.schema_version -ne 2 -or $j.edition -cne $PaketAdi -or $j.engine -cne 'bounded-index-append-v1'){Die 'Yedek baska surume ait.'}
  # Verify all snapshot files BEFORE touching the game.
  foreach($x in $j.files){$backup=Join-Path $Path ([string]$x.backup);SafeFile $backup;if((Hash $backup) -ne $x.prefix_sha256){Die "Indeks yedegi bozuk: $backup"}}
  foreach($x in $j.files){
    $live=Rel $Root ([string]$x.path);SafeFile $live
    $current=Hash $live
    if($current -eq $x.original_sha256){continue}
    if($RequirePatched -and $current -ne $x.patched_sha256){Die "Oyun disaridan degistirilmis. Kaldirma durduruldu: $live"}
    if((HashTail $live ([long]$x.prefix_length) ([long]$x.original_length)) -ne $x.original_suffix_sha256){Die "Orijinal FORGE veri alani degismis: $live. Otomatik geri alma guvenli degil."}
    RestorePrefix $live (Join-Path $Path ([string]$x.backup)) ([long]$x.original_length) ([long]$x.prefix_length)
    if((Hash $live) -ne $x.original_sha256){Die "Orijinal hash geri alinamadi: $live. Yedegi silme!"}
  }
  $j.status='rolled_back';SaveJson $meta $j
}
# $Args is a PowerShell automatic variable: never use it for CLI arguments.
# aclocimport writes <input>.out, it does NOT overwrite the original .dat.
function Tool([string]$Exe,[string[]]$ToolArgs,[string]$Work,[switch]$Quiet){
  Push-Location -LiteralPath $Work
  try{
    $old=$ErrorActionPreference
    $ErrorActionPreference='Continue'
    $global:LASTEXITCODE = 0
    $tail=New-Object 'System.Collections.Generic.List[string]'
    try {
      & $Exe @ToolArgs 2>&1 | ForEach-Object {
        $line=[string]$_
        if($Quiet){
          if($tail.Count -ge 10){$tail.RemoveAt(0)}
          $tail.Add($line)
        }else{Msg $line}
      }
      $result=[int]$LASTEXITCODE
    } finally { $ErrorActionPreference=$old }
    if($result -ne 0){
      if($Quiet){foreach($line in $tail){Msg ('[ARAC] '+$line)}}
      Die "Arac hata kodu $result : $([IO.Path]::GetFileName($Exe)) ; girdi: $($ToolArgs -join ' ')"
    }
    return $result
  }finally{Pop-Location}
}
# ACLocImport updates the package size at bytes 4..7. Comparing the first
# eight bytes to the old DAT incorrectly rejects a successful conversion.
# Check the first four-byte localization class signature and a bounded size
# field instead. These outputs are produced by our hash-pinned converter.
function IsLocalizationDat([string]$Candidate,[string]$Template){
  if(-not(Test-Path -LiteralPath $Candidate -PathType Leaf)){return $false}
  if(-not(Test-Path -LiteralPath $Template -PathType Leaf)){return $false}
  $newLength=[long](Get-Item -LiteralPath $Candidate).Length
  $oldLength=[long](Get-Item -LiteralPath $Template).Length
  if($newLength -lt 88 -or $oldLength -lt 88){return $false}
  $src=[IO.File]::OpenRead($Template)
  $dst=[IO.File]::OpenRead($Candidate)
  try{
    $a=New-Object byte[] 8
    $b=New-Object byte[] 8
    if($src.Read($a,0,8) -ne 8 -or $dst.Read($b,0,8) -ne 8){return $false}
    # First four bytes identify this DAT class. The next four are its size.
    for($i=0;$i -lt 4;$i++){if($a[$i] -ne $b[$i]){return $false}}
    if([BitConverter]::ToUInt32($b,0) -ne [uint32]0x6e3c9c6f){return $false}
    [long]$declared=[BitConverter]::ToUInt32($b,4)
    [long]$overhead=$newLength-$declared
    if($declared -lt 1 -or $overhead -lt 1 -or $overhead -gt 4096){return $false}
    return $true
  }finally{$src.Dispose();$dst.Dispose()}
}
try{
  Msg "ANIMUS SHADOWS - $PaketAdi | YAMA KUR"
  if($Fast){Msg '[MOD] FAST - dogrulamalar korunur, cift tam FORGE okumalari teke indirilir.'}
  else{Msg '[MOD] STANDART - tum asamalar, ayrintili arac ciktisi.'}
  ShowStage 'Paket SHA256' 0 1314 0 5
  $manifest=Join-Path $Paket 'MANIFEST_SHA256.txt'
  $lines=Get-Content -LiteralPath $manifest -Encoding UTF8
  $count=0
  foreach($line in $lines){
    if($line -notmatch '^([a-f0-9]{64})  (data/[a-zA-Z0-9_./-]+)$'){Die 'Bozuk paket manifesti.'}
    $expected=$Matches[1];$f=Rel $Paket $Matches[2];SafeFile $f
    if((Hash $f) -ne $expected){Die "Paket butunlugu bozuk: $f"}
    $count++
    if($count % 50 -eq 0 -or $count -eq 1314){ShowStage 'Paket SHA256' $count 1314 0 5}
  }
  if($count -ne 1314){Die "Eksik yama kaynak manifesti: $count / 1314"}
  Msg "Paket SHA-256 kontrolu: $count dosya dogrulandi."
  $Tools=Join-Path $Paket 'data/tools'
  $modloc=Join-Path $Tools 'modloc.exe';$acloc=Join-Path $Tools 'aclocimport.exe';$oodle=Join-Path $Tools 'oo2core_7_win64.dll'
  if((Hash $modloc) -cne '1ce63bd5a0936d6e01d3f61d9b5ecd6491998ee187bdd0f795bbb852f0bf7d71'){Die 'Modloc surumu desteklenmiyor; in-place islem yasak.'}
  if((Hash $acloc) -cne '011c2362e2f96431e99199f52d60cb066ec48f89af76f541c865e6b5a0488d85'){Die 'aclocimport surumu desteklenmiyor.'}
  if((Hash $oodle) -cne 'c968942fecda4e94ba95a8f17572884f8cc6a6351ceefaf5131e0b46dc149c55'){Die 'Oodle v7 dogrulanamadi.'}
  $GameRoot=FindGame $GameRoot
  $BackupRoot=Join-Path $GameRoot '.animus-shadows-backup'
  if(Test-Path -LiteralPath $BackupRoot){Die 'Mevcut ANIMUS yedegi var. Loader icinden Yamayi Kaldir ile once geri yukle; yedekleri silme.'}
  $data=Join-Path $Paket 'data/_Modding_WorkDir'
  if(-not(Test-Path -LiteralPath $data -PathType Container)){Die 'Ceviri kaynaklari eksik.'}
  $st=New-Object System.Collections.ArrayList
  [long]$prefixTotal=0
  foreach($group in $ForgeMap.Keys){
    $rel=[string]$ForgeMap[$group];$forge=Rel $GameRoot $rel;SafeFile $forge
    $info=MakePrefixInfo $forge
    $prefixTotal+=$info.prefix_len
    [void]$st.Add([pscustomobject]@{path=$rel;group=$group;original_length=$info.length;prefix_length=$info.prefix_len;entry_count=$info.count;prefix_sha256='';original_sha256='';original_suffix_sha256='';patched_sha256='';backup=('index/'+$group+'.bin')})
  }
  # Maximum backup is 512 MB per forge; allowed if enough free space.
  $payloadFiles=Get-ChildItem -LiteralPath $data -Recurse -File
  [long]$payloadBytes=0
  foreach($p in $payloadFiles){$payloadBytes+=$p.Length}
  $disk=[IO.DriveInfo]::new([IO.Path]::GetPathRoot($GameRoot))
  [long]$require=2GB + $prefixTotal + ($payloadBytes * 4)
  if($disk.AvailableFreeSpace -lt $require){Die "Dusuk alan modunda en az $([Math]::Ceiling($require/1GB)) GB bosluk gerekli; mevcut $([Math]::Round($disk.AvailableFreeSpace/1GB,1)) GB."}
  Msg "Yedeklenecek indeks: $([Math]::Round($prefixTotal/1MB,1)) MB; gerekli tahmini minimum bos alan $([Math]::Ceiling($require/1GB)) GB."
  # Preparation and conversions are done without touching the game.
  $Stage=Join-Path $Paket ('.animus-work-'+[Guid]::NewGuid().ToString('N'))
  $StagePayload=Join-Path $Stage 'payload';$StageTools=Join-Path $Stage 'tools'
  New-Item -ItemType Directory -Path $StageTools -Force|Out-Null
  Copy-Item -LiteralPath $data -Destination $StagePayload -Recurse -Force
  Copy-Item -LiteralPath $modloc -Destination $StageTools
  Copy-Item -LiteralPath $acloc -Destination $StageTools
  Copy-Item -LiteralPath $oodle -Destination $StageTools
  $imports=Join-Path $StageTools 'aclocimport.exe'
  $inject=Join-Path $StageTools 'modloc.exe'
  $operations=New-Object System.Collections.ArrayList
  ShowStage 'Ceviri donusturme' 0 82 10 35
  foreach($group in $ForgeMap.Keys){
    $dir=Join-Path (Join-Path $StagePayload $group) 'dat'
    $entries=@(Get-ChildItem -LiteralPath $dir -File -Filter '*.dat.txt')
    if($entries.Count -lt 1){Die "DAT bulunamadi: $group"}
    foreach($e in $entries){
      $dat=$e.FullName.Substring(0,$e.FullName.Length-4)
      SafeFile $dat;SafeFile ($e.FullName+'.header')
      $id=[IO.Path]::GetFileNameWithoutExtension($dat)
      if($id -cnotmatch '^0x[0-9A-Fa-f]{11}$'){Die "DAT kimligi gecersiz: $id"}
      # For each input, wait for a newly generated .out before altering .dat.
      # The original installer also consumes this .out output.
      $outFile=$e.FullName+'.out'
      if(Test-Path -LiteralPath $outFile){Remove-Item -LiteralPath $outFile -Force}
      # Confirmed by Windows CMD: aclocimport.exe <input.dat.txt> exits 0,
      # and produces exactly <input.dat.txt>.out. Fail closed on nonzero.
      [void](Tool -Exe $imports -ToolArgs @([string]$e.FullName) -Work $StageTools -Quiet:$Fast)
      if(-not(Test-Path -LiteralPath $outFile -PathType Leaf)){
        Die "aclocimport cikti olusturmadi: $id. Girdi: $($e.FullName)"
      }
      if(-not(IsLocalizationDat $outFile $dat)){
        $outputLen=(Get-Item -LiteralPath $outFile).Length
        Die "aclocimport cikti DAT bicimi dogrulanamadi: $id ; boyut=$outputLen bayt. Oyun dosyalarina dokunulmadi."
      }
      if((Hash $outFile) -eq (Hash $dat)){
        Die "aclocimport ceviriyi isleyemedi (cikti orijinalle ayni): $id"
      }
      # Only a validated converted output is allowed to replace the source DAT.
      Move-Item -LiteralPath $outFile -Destination $dat -Force
      [void]$operations.Add([pscustomobject]@{group=$group;dat=$dat;id=$id.Substring(2)})
      ShowStage 'Ceviri donusturme' $operations.Count 82 10 35
    }
  }
  if($operations.Count -ne 82){Die "Eksik veri: $($operations.Count) / 82"}
  # Match the original injector's case-insensitive 64-bit IDs and reject
  # incompatible game builds before the first backup or FORGE modification.
  $validated=0
  ShowStage 'FORGE indeks kontrolu' 0 $st.Count 35 40
  foreach($x in $st){
    $ids=@($operations | Where-Object {$_.group -eq $x.group} | ForEach-Object {$_.id})
    CheckForgeEntries (Rel $GameRoot $x.path) ([long]$x.entry_count) $ids
    $validated++
    ShowStage 'FORGE indeks kontrolu' $validated $st.Count 35 40
  }
  Msg '[KONTROL] 82 hedef FORGE kaydi mevcut ve benzersiz.'

  AssertGameStopped
  # Create durable index snapshots first. Each snapshot holds the original index,
  # while original forge body is NOT duplicated.
  New-Item -ItemType Directory -Path (Join-Path $BackupRoot 'index') -Force | Out-Null
  SaveJson (Join-Path $BackupRoot 'loader-owner.json') ([pscustomobject]@{transaction=$LoaderTransaction})
  $backed=0
  ShowStage 'Yedek ve hash' 0 $st.Count 40 55
  foreach($x in $st){
    $forge=Rel $GameRoot $x.path
    $backup=Rel $BackupRoot $x.backup
    CopyPrefix -SourcePath $forge -DestinationPath $backup -PrefixLength ([long]$x.prefix_length)
    $x.prefix_sha256=Hash $backup
    if($Fast){
      $pair=HashForgePair -File $forge -Prefix ([long]$x.prefix_length) -Length ([long]$x.original_length)
      $x.original_sha256=$pair.full
      $x.original_suffix_sha256=$pair.tail
    }else{
      $x.original_sha256=Hash $forge
      $x.original_suffix_sha256=HashTail $forge ([long]$x.prefix_length) ([long]$x.original_length)
    }
    $backed++
    ShowStage 'Yedek ve hash' $backed $st.Count 40 55
  }
  $journal=[pscustomobject]@{schema_version=2;loader_transaction=$LoaderTransaction;edition=$PaketAdi;engine='bounded-index-append-v1';status='prepared';created_utc=[DateTime]::UtcNow.ToString('o');files=@($st.ToArray())}
  SaveJson (Join-Path $BackupRoot 'install.json') $journal
  $JournalStarted=$true
  AssertGameStopped
  $journal.status='applying';SaveJson (Join-Path $BackupRoot 'install.json') $journal
  Msg '[KURULUM] FORGE dosyalari yerinde isleniyor. Tam arsiv kopyasi yapilmiyor.'
  $n=0
  ShowStage 'Yamayi uygulama' 0 $operations.Count 55 93
  foreach($op in $operations){
    $live=Rel $GameRoot ([string]$ForgeMap[[string]$op.group])
    [long]$beforeInject=(Get-Item -LiteralPath $live).Length
    [void](Tool -Exe $inject -ToolArgs @('inject',[string]$op.dat,$live,[string]$op.id) -Work $StageTools -Quiet:$Fast)
    if((Get-Item -LiteralPath $live).Length -le $beforeInject){Die "Enjeksiyon sonrasi FORGE buyumedi: $live"}
    $n++
    ShowStage 'Yamayi uygulama' $n $operations.Count 55 93
    if([IO.DriveInfo]::new([IO.Path]::GetPathRoot($GameRoot)).AvailableFreeSpace -lt 1GB){Die 'Disk guvenlik sinirina yaklasti. Kurulum durduruldu.'}
  }
  $checked=0
  ShowStage 'Son FORGE kontrolu' 0 $st.Count 93 99
  foreach($x in $st){
    $live=Rel $GameRoot $x.path
    if((Get-Item -LiteralPath $live).Length -le $x.original_length){Die "FORGE buyumedi: $($x.path)"}
    if((HashTail $live ([long]$x.prefix_length) ([long]$x.original_length)) -ne $x.original_suffix_sha256){Die "Orijinal FORGE govdesi degisti: $live"}
    $ids=@($operations | Where-Object {$_.group -eq $x.group} | ForEach-Object {$_.id})
    CheckForgeEntries $live ([long]$x.entry_count) $ids
    $x.patched_sha256=Hash $live
    if($x.patched_sha256 -eq $x.original_sha256){Die "FORGE degismedi: $($x.path)"}
    $checked++
    ShowStage 'Son FORGE kontrolu' $checked $st.Count 93 99
  }
  $journal.status='installed';SaveJson (Join-Path $BackupRoot 'install.json') $journal
  ShowStage 'Tamamlandi' 1 1 99 100
  FinishProgress
  Msg '[BASARILI] Yama dosyalara islendi. Geri alma icin Loader icinden Yamayi Kaldir kullan.'
  exit 0
}catch{
  FinishProgress
  $failure=$_
  Msg ('[HATA] '+$failure.Exception.Message)
  if($failure.InvocationInfo -and $failure.InvocationInfo.ScriptLineNumber -gt 0){
    Msg ('[HATA SATIRI] KURULUM.ps1 satir '+$failure.InvocationInfo.ScriptLineNumber)
  }
  if($failure.ScriptStackTrace){Msg ('[HATA IZ] '+$failure.ScriptStackTrace)}
  if($JournalStarted){
    try{RestoreAll $BackupRoot $GameRoot $false;Msg '[GERI ALINDI] Orijinal indeksler geri yazildi, eklenen veriler kesildi.'}
    catch{Msg ('[KRITIK] Geri alma tamamlanamadi: '+$_.Exception.Message);Msg "Yedegi silme: $BackupRoot"}
  }else{
    Msg 'Oyun arsivlerine yazilmadi.'
    Write-Output '[ANIMUS_NO_GAME_WRITES]'
    # Loader retains/archives any owned, unfinished index snapshots.
  }
  exit 3
}finally{
  if($Stage -and(Test-Path -LiteralPath $Stage)){Remove-Item -LiteralPath $Stage -Recurse -Force -ErrorAction SilentlyContinue}
}
