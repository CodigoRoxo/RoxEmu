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
./cores/build-cores.ps1          # compila o SwanStation (PS1) a partir do fonte
./emulators/fetch-emulators.ps1  # baixa o PCSX2 (PS2) do release oficial e confere o SHA-256
cargo build -p rox-runner --release
```

### PS3

O PS3 roda no **RPCS3** embutido (release oficial, SHA-256 conferido pelo `fetch-emulators.ps1`).
Ele precisa do **firmware oficial** (`PS3UPDAT.PUP`), que a Sony oferece de graça no site dela: em
Configurações → PlayStation 3, use "Baixar no site da Sony" e depois "Instalar firmware". Jogos:
adicione a **pasta** do jogo (Biblioteca → + Pasta de PS3) ou a **ISO** (decifrada ou com o `.dkey`
ao lado). O ROXemu lê nome e código do `PARAM.SFO`, abre direto em tela cheia, com o sistema em
português e o controle (DualSense, DualShock 4 ou Xbox) já mapeado.

### PS2

O PS2 roda no **PCSX2** embutido: o ROXemu configura e abre ele direto em tela cheia, já com a
BIOS, pastas de save e controle. Coloque a BIOS do seu PS2 (arquivo de 4 MB, ex.: `scph39001.bin`)
na mesma pasta de BIOS do ROXemu. ISO e CHD funcionam; com CHD, deixe os jogos numa pasta `ps2`
para o ROXemu saber o console. Para sair de qualquer jogo: segure SELECT + START (1 s) ou aperte
o botão PS / Guide, e o ROXemu fecha o jogo e volta para a estante.

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

Padrão do PS5: ✕ confirma e ○ volta. Os símbolos na tela seguem o controle conectado
(PlayStation ou Xbox) e podem ser fixados em Configurações.

| Teclado | PlayStation | Xbox | Ação |
|---|---|---|---|
| Setas | Direcional / analógico (segurar repete) | D-pad / analógico | Navegar (fileira e grade da Biblioteca) |
| Enter | ✕ | A | Jogar / confirmar / abrir pasta |
| Esc / Backspace | ○ | B | Voltar (fecha telas, volta para "Todos", sai da Biblioteca) |
| I | △ | Y | Detalhes do jogo |
| F | □ | X | Favoritar |
| Q / E | L1 / R1 | LB / RB | Trocar estante (Console) ou categoria (Biblioteca) |
| O | OPTIONS | Menu | Opções do jogo (favoritar, artes, remover, **desinstalar**) |
| Tab | — | — | Alternar Console / Biblioteca |
| — | Create | View | Adicionar jogo |
| — | SELECT + START (segurar) / PS | View + Menu / Guide | Fechar o jogo e voltar para o console |

**Desinstalar** apaga os arquivos do jogo do disco para sempre (sem lixeira), depois de uma
confirmação que começa em "Cancelar". Só apaga o que é do jogo (o .cue e seus .bin, os discos do
.m3u, a pasta do jogo de PS3); saves e memory cards ficam guardados.

## Legal

O ROXemu não vem com BIOS nem jogos. Use dumps dos seus próprios consoles e discos.
Licença: GPL-3.0-or-later.
