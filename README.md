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
./cores/build-cores.ps1   # compila o SwanStation a partir do fonte (Fase 1)
```

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
