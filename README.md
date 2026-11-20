# ROXemu

Seus jogos, num lugar só. Uma estante bonita com capas, controle na mão e tudo compilado a partir
do código-fonte, sem executável suspeito baixado por aí.

> Status: **Fase 0 (fundação)**. A interface já abre com jogos de exemplo; o runner que roda os
> jogos chega na Fase 1. Veja o plano completo em [docs/PLANO.md](docs/PLANO.md).

## Estrutura

```
apps/launcher/        Interface (Tauri 2 + React + TypeScript)
crates/rox-common     Tipos compartilhados: consoles, pastas de dados, protocolo launcher <-> runner
crates/rox-core-host  Frontend libretro (carrega os cores .dll)
crates/rox-runner     Processo que roda o jogo
crates/rox-library    Formatos, .cue, serial do PS1
crates/rox-metadata   Capas (libretro-thumbnails) e metadados
cores/                Versões fixadas dos cores e script de build a partir do fonte
prototypes/           Protótipos visuais em HTML
```

## Pré-requisitos (Windows)

- Visual Studio 2022 com "Desenvolvimento para desktop com C++" (inclui o CMake)
- Rust estável (`rustup`)
- Node.js 22+ e pnpm 10+

## Comandos

```powershell
pnpm install          # dependências da interface
pnpm dev              # abre o ROXemu em modo desenvolvimento
cargo test --workspace
./cores/build-cores.ps1   # compila o SwanStation a partir do fonte
cargo build -p rox-runner --release
```

### Rodar um jogo de PS1 (Fase 1)

1. Coloque a BIOS do seu console (ex.: `scph5501.bin`) em `%APPDATA%\ROXemu\bios\`
   (o aviso "BIOS do PS1 faltando" no topo do app abre essa pasta).
2. Compile o runner uma vez: `cargo build -p rox-runner --release`.
3. No app (`pnpm dev`), clique em **+ Jogo**, escolha o `.cue`/`.chd`/`.m3u` e aperte **Jogar**.

Ou direto pela linha de comando:

```powershell
.\target\release\rox-runner.exe --core .\cores\bin\swanstation_libretro.dll --game "D:\Jogos\PS1\Seu Jogo (USA).cue"
```

| Teclado | Controle | PS1 |
|---|---|---|
| Setas / WASD | D-pad / analógico | Direcional |
| K / L / J / I | A / B / X / Y | ✕ / ○ / □ / △ |
| Q / E · 1 / 3 | LB / RB · LT / RT | L1 / R1 · L2 / R2 |
| Enter / Backspace | Start / Back | Start / Select |
| Espaço (segurar) | — | Acelerar |
| F2 / F4 | — | Salvar / carregar estado |
| Esc | Guide | Voltar para o ROXemu |

## Atalhos na interface

| Teclado | Controle | Ação |
|---|---|---|
| ← → | Direcional / analógico | Navegar entre jogos |
| Enter | A | Jogar |
| Q / E | LB / RB | Trocar categoria |
| — | Y | Detalhes |
| — | B | Favoritar |
| — | Start | Alternar Console / Biblioteca |
| Esc | B | Fechar a tela de "iniciando" |

## Legal

O ROXemu não vem com BIOS nem jogos. Use dumps dos seus próprios consoles e discos.
Licença: GPL-3.0-or-later.
