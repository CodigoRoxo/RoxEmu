<#
.SYNOPSIS
  Compila os cores libretro do ROXemu a partir do código-fonte oficial.

.DESCRIPTION
  Lê cores/cores.lock.toml, clona cada core habilitado no commit fixado, compila e copia a .dll
  para cores/bin/. Na primeira vez grava o SHA-256; nas próximas, confere se o hash bate.

  Por enquanto só cores com build = "cmake" (SwanStation). Os que usam make + MinGW entram depois.

.EXAMPLE
  ./cores/build-cores.ps1
  ./cores/build-cores.ps1 -Only swanstation
#>
param(
  [string]$Only,
  [ValidateSet("Release", "RelWithDebInfo", "Debug")]
  [string]$Config = "Release"
)

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
$lockPath = Join-Path $root "cores.lock.toml"
$srcDir = Join-Path $root "src"
$buildDir = Join-Path $root "build"
$binDir = Join-Path $root "bin"
New-Item -ItemType Directory -Force $srcDir, $buildDir, $binDir | Out-Null

# --- leitor TOML mínimo (só tabelas [nome] e chaves texto/booleano) ---
function Read-CoreLock([string]$path) {
  $cores = [ordered]@{}
  $current = $null
  foreach ($raw in Get-Content $path -Encoding UTF8) {
    $line = ($raw -replace '\s+#.*$', '').Trim()
    if (-not $line -or $line.StartsWith('#')) { continue }
    if ($line -match '^\[(.+)\]$') { $current = $Matches[1]; $cores[$current] = [ordered]@{ enabled = $true }; continue }
    if ($current -and $line -match '^([A-Za-z0-9_]+)\s*=\s*(.+)$') {
      $key = $Matches[1]
      $value = $Matches[2].Trim()
      if ($value -match '^"(.*)"$') { $value = $Matches[1] }
      elseif ($value -eq 'true') { $value = $true }
      elseif ($value -eq 'false') { $value = $false }
      $cores[$current][$key] = $value
    }
  }
  return $cores
}

function Set-LockHash([string]$name, [string]$hash) {
  $lines = Get-Content $lockPath -Encoding UTF8
  $inside = $false
  for ($i = 0; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match '^\[(.+)\]$') { $inside = ($Matches[1] -eq $name) }
    elseif ($inside -and $lines[$i] -match '^sha256\s*=') { $lines[$i] = "sha256 = `"$hash`"" }
  }
  # UTF-8 sem BOM (TOML não aceita BOM)
  [System.IO.File]::WriteAllLines($lockPath, [string[]]$lines, (New-Object System.Text.UTF8Encoding $false))
}

function Find-CMake {
  $cmd = Get-Command cmake -ErrorAction SilentlyContinue
  if ($cmd) { return $cmd.Source }
  $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
  if (Test-Path $vswhere) {
    $vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    $candidate = Join-Path $vs "Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe"
    if (Test-Path $candidate) { return $candidate }
  }
  throw "CMake não encontrado. Instale o componente 'C++ CMake tools for Windows' do Visual Studio."
}

$lock = Read-CoreLock $lockPath
foreach ($name in $lock.Keys) {
  $core = $lock[$name]
  if ($Only -and $name -ne $Only) { continue }
  if (-not $core.enabled) { Write-Host "[$name] desabilitado, pulando." -ForegroundColor DarkGray; continue }
  if ($core.build -ne "cmake") { Write-Warning "[$name] build '$($core.build)' ainda não suportado."; continue }

  Write-Host "[$name] commit $($core.commit)" -ForegroundColor Magenta
  $src = Join-Path $srcDir $name
  if (-not (Test-Path (Join-Path $src ".git"))) {
    git init --quiet $src
    git -C $src remote add origin $core.repo
  }
  git -C $src fetch --quiet --depth 1 origin $core.commit
  git -C $src checkout --quiet --force FETCH_HEAD
  git -C $src submodule update --quiet --init --recursive --depth 1

  $cmake = Find-CMake
  $out = Join-Path $buildDir $name
  & $cmake -S $src -B $out -G "Visual Studio 17 2022" -A x64 -DCMAKE_BUILD_TYPE=$Config
  if ($LASTEXITCODE -ne 0) { throw "[$name] falha no configure do CMake" }
  & $cmake --build $out --config $Config --parallel
  if ($LASTEXITCODE -ne 0) { throw "[$name] falha na compilação" }

  $dll = Get-ChildItem $out -Recurse -Filter $core.output | Select-Object -First 1
  if (-not $dll) { throw "[$name] $($core.output) não foi gerada" }
  Copy-Item $dll.FullName (Join-Path $binDir $core.output) -Force

  $hash = (Get-FileHash $dll.FullName -Algorithm SHA256).Hash.ToLower()
  if (-not $core.sha256) {
    Set-LockHash $name $hash
    Write-Host "[$name] hash registrado: $hash" -ForegroundColor Green
  } elseif ($core.sha256 -ne $hash) {
    throw "[$name] HASH DIFERENTE do registrado! esperado $($core.sha256), obtido $hash"
  } else {
    Write-Host "[$name] hash confere: $hash" -ForegroundColor Green
  }
}
