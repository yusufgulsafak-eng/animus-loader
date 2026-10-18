# ANIMUS | Assassin's Creed Shadows | Dusuk alanli geri alma
param([Parameter(Mandatory=$true)][string]$GameRoot,[Parameter(Mandatory=$true)][string]$LoaderTransaction)
Set-StrictMode -Version Latest
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$OutputEncoding=[Console]::OutputEncoding
$ErrorActionPreference='Stop'
$Paket=Split-Path -Parent $MyInvocation.MyCommand.Path
$PaketAdi='ORIJINAL'
$script:RemovalTimer=[Diagnostics.Stopwatch]::StartNew()
$script:RemovalStage=''
$script:RemovalStarted=0.0
function ShowRemoval([string]$Phase,[int]$Done,[int]$Total,[int]$Start,[int]$End){
 if($script:RemovalStage -cne $Phase){$script:RemovalStage=$Phase;$script:RemovalStarted=$script:RemovalTimer.Elapsed.TotalSeconds}
 $pc=[int][Math]::Min(100,[Math]::Floor($Start+($End-$Start)*[double]$Done/[Math]::Max(1,$Total)))
 $elapsed=$script:RemovalTimer.Elapsed.TotalSeconds
 $remaining='hesaplaniyor'
 if($Done -gt 0){$seconds=($elapsed-$script:RemovalStarted)*[Math]::Max(0,$Total-$Done)/$Done;$remaining=[TimeSpan]::FromSeconds([Math]::Max(0,$seconds)).ToString('hh\:mm\:ss')}
 $status=('{0} | {1}/{2} | %{3} | gecen {4} | bu asama kalan ~{5}' -f $Phase,$Done,$Total,$pc,([TimeSpan]::FromSeconds($elapsed).ToString('hh\:mm\:ss')),$remaining)
 Write-Output ('[ANIMUS_PROGRESS]{0}|{1}' -f $pc,$status)
 Write-Progress -Id 1 -Activity 'ANIMUS SHADOWS - YAMAYI KALDIR' -Status $status -PercentComplete $pc
 if($Done -gt 0){Write-Host ('[ILERLEME] {0} | {1}/{2} | %{3} | kalan ~{4}' -f $Phase,$Done,$Total,$pc,$remaining)}
}

function AssertGameStopped(){
  if(Get-Process -Name 'ACShadows' -ErrorAction SilentlyContinue){Die "Assassin's Creed Shadows acik. Oyunu kapatip yeniden deneyin."}
}
function Die([string]$s){throw $s}
function Rel([string]$r,[string]$p){Join-Path $r ($p.Replace('/',[IO.Path]::DirectorySeparatorChar))}
function Hash([string]$p){(Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash.ToLowerInvariant()}
function SafeFile([string]$p){if(-not(Test-Path -LiteralPath $p -PathType Leaf)){Die "Dosya bulunamadi: $p"};$f=Get-Item -LiteralPath $p -Force;if(($f.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0){Die "Baglanti dosyasi reddedildi: $p"}}
function SaveJson([string]$Target,$Value){
  $temporary=$Target+'.tmp'
  $bytes=[Text.UTF8Encoding]::new($false).GetBytes(($Value|ConvertTo-Json -Depth 10))
  $stream=[IO.File]::Open($temporary,[IO.FileMode]::Create,[IO.FileAccess]::Write,[IO.FileShare]::None)
  try{$stream.Write($bytes,0,$bytes.Length);$stream.Flush($true)}finally{$stream.Dispose()}
  if([IO.File]::Exists($Target)){[IO.File]::Replace($temporary,$Target,($Target+'.previous'))}
  else{[IO.File]::Move($temporary,$Target)}
}
function HasBackup([string]$p){$p -and (Test-Path -LiteralPath (Join-Path $p '.animus-shadows-backup/install.json') -PathType Leaf)}
# Bos OYUN_YOLU.txt dosyasinda Get-Content hicbir satir uretmeyebilir.
# .Trim() yalnizca dogrulanmis, null-olmayan string uzerinde cagrilir.
function FindGame([string]$Explicit){
  if(-not(HasBackup $Explicit)){Die 'Loader oyun klasorunde geri alma kaydi yok.'}
  return (Resolve-Path -LiteralPath $Explicit).Path
}
function HashTail([string]$File,[long]$Start,[long]$Length){
 $src=[IO.File]::Open($File,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
 $sha=[Security.Cryptography.SHA256]::Create()
 try{
  if($src.Length -lt $Length -or $Start -gt $Length){Die "FORGE boyutu degismis: $File"}
  $src.Seek($Start,[IO.SeekOrigin]::Begin)|Out-Null
  $buf=New-Object byte[] (4MB);[long]$left=$Length-$Start
  while($left -gt 0){$n=$src.Read($buf,0,[int][Math]::Min([long]$buf.Length,$left));if($n -le 0){Die "FORGE veri okunamadi: $File"};[void]$sha.TransformBlock($buf,0,$n,$buf,0);$left-=$n}
  [void]$sha.TransformFinalBlock([byte[]]@(),0,0)
  return ([BitConverter]::ToString($sha.Hash)).Replace('-','').ToLowerInvariant()
 }finally{$sha.Dispose();$src.Dispose()}
}
function RestorePrefix([string]$live,[string]$saved,[long]$original,[long]$count){
 $source=[IO.File]::OpenRead($saved)
 try{
  if($source.Length -ne $count){Die 'Indeks yedegi boyut hatasi'}
  $dest=[IO.File]::Open($live,[IO.FileMode]::Open,[IO.FileAccess]::ReadWrite,[IO.FileShare]::None)
  try{
   if($dest.Length -lt $original){Die "Beklenmeyen kisa FORGE: $live"}
   $buf=New-Object byte[] (4MB);[long]$rem=$count
   while($rem -gt 0){$want=[int][Math]::Min([long]$buf.Length,$rem);$n=$source.Read($buf,0,$want);if($n -le 0){Die 'Yedek okuma hatasi'};$dest.Write($buf,0,$n);$rem-=$n}
   $dest.SetLength($original);$dest.Flush($true)
  }finally{$dest.Dispose()}
 }finally{$source.Dispose()}
}
try{
 Write-Host "ANIMUS SHADOWS $PaketAdi | YAMAYI KALDIR"
 $GameRoot=FindGame $GameRoot
 $backup=Join-Path $GameRoot '.animus-shadows-backup'
 $meta=Join-Path $backup 'install.json'
 $j=Get-Content -LiteralPath $meta -Raw -Encoding UTF8|ConvertFrom-Json
 if($j.loader_transaction -cne $LoaderTransaction){Die 'Loader kurulum kimligi uyusmuyor.'}
 if($j.schema_version -ne 2 -or $j.edition -cne $PaketAdi -or $j.engine -cne 'bounded-index-append-v1'){Die 'Kurulum yedegi uyumsuz. Bu KALDIR eski tam yedekli pakete uygulanamaz.'}
 if($j.status -notin @('installed','applying','prepared','rolled_back','uninstalling','uninstalled')){Die 'Gecersiz kurulum durumu.'}
 $checkedBackup=0
 ShowRemoval 'Yedekleri dogrula' 0 $j.files.Count 0 10
 foreach($x in $j.files){
   $save=Rel $backup ([string]$x.backup);SafeFile $save
   if((Hash $save) -ne $x.prefix_sha256){Die "Geri alma indeksi bozuk: $save"}
   $checkedBackup++
   ShowRemoval 'Yedekleri dogrula' $checkedBackup $j.files.Count 0 10
 }
 $checkedForge=0
 ShowRemoval 'FORGE dogrulama' 0 $j.files.Count 10 55
 foreach($x in $j.files){
  $live=Rel $GameRoot ([string]$x.path);SafeFile $live
  $current=Hash $live
  if($current -ne $x.original_sha256){
    if($j.status -eq 'installed' -and $current -ne $x.patched_sha256){Die "Oyunun FORGE dosyasi disaridan degistirilmis: $live. Guvenlik icin islem durdu."}
    if((HashTail $live ([long]$x.prefix_length) ([long]$x.original_length)) -ne $x.original_suffix_sha256){Die "Orijinal FORGE govdesi degismis: $live. Guvenli geri alma mumkun degil."}
  }
  $checkedForge++
  ShowRemoval 'FORGE dogrulama' $checkedForge $j.files.Count 10 55
 }
 AssertGameStopped
 $j.status='uninstalling';SaveJson $meta $j
 $restored=0
 ShowRemoval 'Orijinali geri yukle' 0 $j.files.Count 55 99
 foreach($x in $j.files){
  $live=Rel $GameRoot ([string]$x.path)
  if((Hash $live) -ne $x.original_sha256){
    RestorePrefix $live (Rel $backup ([string]$x.backup)) ([long]$x.original_length) ([long]$x.prefix_length)
    if((Hash $live) -ne $x.original_sha256){Die "Dosya tamamen eski haline donmedi: $live. Yedegi silme."}
    Write-Host "Geri alindi: $($x.path)"
  }
  $restored++
  ShowRemoval 'Orijinali geri yukle' $restored $j.files.Count 55 99
 }
 $j.status='uninstalled';SaveJson $meta $j
 $history=Join-Path $GameRoot ('.animus-shadows-history-'+[DateTime]::UtcNow.ToString('yyyyMMddHHmmss')+'-'+[Guid]::NewGuid().ToString('N').Substring(0,5))
 Move-Item -LiteralPath $backup -Destination $history
 ShowRemoval 'Tamamlandi' 1 1 99 100
 Write-Progress -Id 1 -Activity 'ANIMUS SHADOWS - YAMAYI KALDIR' -Completed
 Write-Host '[BASARILI] Orijinal dosya hashleri geri geldi. Yedek raporu arsivlendi.'
 exit 0
}catch{
 Write-Progress -Id 1 -Activity 'ANIMUS SHADOWS - YAMAYI KALDIR' -Completed
 $failure=$_
 Write-Host ('[HATA] '+$failure.Exception.Message)
 if($failure.InvocationInfo -and $failure.InvocationInfo.ScriptLineNumber -gt 0){
  Write-Host ('[HATA SATIRI] KALDIR.ps1 satir '+$failure.InvocationInfo.ScriptLineNumber)
 }
 if($failure.ScriptStackTrace){Write-Host ('[HATA IZ] '+$failure.ScriptStackTrace)}
 Write-Host 'Yedekleri silme. Gerekirse oyun dosyalarini platformdan dogrula.'
 exit 3
}
