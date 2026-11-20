<#
.SYNOPSIS
  Baixa os emuladores completos (ex.: PCSX2) do release oficial e confere o SHA-256.

.DESCRIPTION
  Lê emulators/emulators.lock.toml. Para cada emulador: baixa o arquivo da URL oficial, confere
  o hash e só então extrai em emulators/bin/<nome>/. Se o hash não bater, apaga o download e
  para com erro. Também cria o arquivo portable.ini, para o emulador guardar as configurações
  dele dentro da própria pasta (e não espalhar pelo Documentos do usuário).

.EXAMPLE
  ./emulators/fetch-emulators.ps1
#>
param([string]$Only)

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
$lockPath = Join-Path $root "emulators.lock.toml"
$dlDir = Join-Path $root "downloads"
$binDir = Join-Path $root "bin"
New-Item -ItemType Directory -Force $dlDir, $binDir | Out-Null

function Read-Lock([string]$path) {
  $items = [ordered]@{}; $current = $null
  foreach ($raw in Get-Content $path -Encoding UTF8) {
    $line = ($raw -replace '\s+#.*$', '').Trim()
    if (-not $line -or $line.StartsWith('#')) { continue }
    if ($line -match '^\[(.+)\]$') { $current = $Matches[1]; $items[$current] = [ordered]@{}; continue }
    if ($current -and $line -match '^([A-Za-z0-9_]+)\s*=\s*(.+)$') {
      $key = $Matches[1]; $value = $Matches[2].Trim()
      if ($value -match '^"(.*)"$') { $value = $Matches[1] }
      $items[$current][$key] = $value
    }
  }
  return $items
}

$lock = Read-Lock $lockPath
foreach ($name in $lock.Keys) {
  if ($Only -and $name -ne $Only) { continue }
  $e = $lock[$name]
  $dest = Join-Path $binDir $name
  $stamp = Join-Path $dest ".roxemu-version"
  if ((Test-Path $stamp) -and ((Get-Content $stamp -Raw).Trim() -eq $e.version)) {
    Write-Host "[$name] $($e.version) já instalado." -ForegroundColor Green
    continue
  }

  $file = Join-Path $dlDir ([IO.Path]::GetFileName($e.url))
  Write-Host "[$name] baixando $($e.version) de $($e.url)" -ForegroundColor Magenta
  $ProgressPreference = "SilentlyContinue"
  Invoke-WebRequest -UseBasicParsing -Uri $e.url -OutFile $file

  $hash = (Get-FileHash $file -Algorithm SHA256).Hash.ToLower()
  if ($hash -ne $e.sha256.ToLower()) {
    Remove-Item $file -Force
    throw "[$name] HASH NÃO CONFERE! esperado $($e.sha256), obtido $hash. Download apagado."
  }
  Write-Host "[$name] hash confere: $hash" -ForegroundColor Green

  if (Test-Path $dest) { Remove-Item $dest -Recurse -Force }
  New-Item -ItemType Directory -Force $dest | Out-Null
  # tar do Windows 11 (libarchive) extrai .7z e .zip
  tar -xf $file -C $dest
  if ($LASTEXITCODE -ne 0) { throw "[$name] falha ao extrair $file" }

  # Se o pacote veio com uma pasta única dentro, sobe o conteúdo um nível.
  $inner = Get-ChildItem $dest
  if ($inner.Count -eq 1 -and $inner[0].PSIsContainer) {
    Get-ChildItem $inner[0].FullName -Force | Move-Item -Destination $dest
    Remove-Item $inner[0].FullName -Recurse -Force
  }
  if (-not (Test-Path (Join-Path $dest $e.exe))) { throw "[$name] $($e.exe) não encontrado após extrair" }

  # Modo portátil: configurações ficam dentro de emulators/bin/<nome>/
  New-Item -ItemType File -Force (Join-Path $dest "portable.ini") | Out-Null
  Set-Content $stamp $e.version -Encoding ascii
  Write-Host "[$name] pronto em $dest" -ForegroundColor Green
}
