# ROXemu — Plano do Projeto

> Um lugar só para todos os seus jogos de PS1. Uma estante bonita com capas, controle na mão
> e tudo compilado a partir do código-fonte: nada de executável suspeito baixado por aí.

---

## 1. Visão

| Pilar | O que significa na prática |
|---|---|
| **Tudo num lugar só** | Uma tela inicial com todos os jogos e um clique (ou botão do controle) para jogar. |
| **Bonito** | Uma home no estilo console moderno: fileira de capas rolável, card em destaque que cresce, fundo com a arte do jogo e transições suaves. Identidade própria em roxo, sem copiar logos ou marcas de ninguém. |
| **Confiável ("sem vírus")** | O código é aberto e os cores de emulação são compilados por nós a partir do fonte oficial, com commit fixado. Os builds são publicados com hash SHA-256 e assinatura, e não vem nada empacotado junto. |
| **Expansível** | O PS1 vem primeiro, mas a arquitetura nasce pronta para outros sistemas (SNES, GBA, N64, PS2…). |
| **Formato é problema nosso, não do usuário** | O usuário joga o arquivo na pasta e pronto. Se é BIN/CUE, ISO, CHD, PBP, ZIP ou multi-disco, quem resolve é o módulo de cada console. O usuário nunca precisa saber. |
| **É o SEU console** | O PC liga, o ROXemu abre em tela cheia e tudo funciona só com o controle. É uma experiência de console de verdade, não um "programa de PC". |

### O conceito: ROXemu como console próprio

```
Liga o PC ──► Boot animado ROXemu ──► Escolha de perfil ──► Home (todas as plataformas)
                                                             │
           ┌─────────────────┬────────────────────┬─────────┴──────────┐
        Jogar        Continuar de onde parou   Estante por console   Menu do sistema
                     (último save state)       (PS1, SNES, GBA…)     (config, dormir, desligar)
```

---

## 2. A decisão mais importante: escrever o emulador do zero ou não?

**Recomendação: não escrever o núcleo de PS1 do zero agora.** Usamos cores de código aberto
pela **API libretro** e concentramos nossa energia no que diferencia o ROXemu: a experiência, a
biblioteca e a confiança.

Emular o PS1 com boa compatibilidade envolve a CPU MIPS R3000A, o GTE (coprocessador 3D), a GPU,
o SPU (áudio), o CD-ROM, o MDEC (vídeos) e timings precisos. Projetos maduros levaram **anos** para
chegar onde estão. Se escrevermos o nosso, ficamos meses sem ter nada jogável.

### Cores candidatos (todos de código aberto)

| Core | Licença | Pontos fortes | Uso no ROXemu |
|---|---|---|---|
| **SwanStation** | GPL-3 | Fork libretro do DuckStation (código GPL antigo). Rápido, com upscale, PGXP e renderer por software, OpenGL e Vulkan. | **Core padrão** |
| **Beetle PSX HW** (Mednafen) | GPL-2 | Precisão altíssima e compatibilidade de referência. | Core alternativo ("modo precisão") |
| **PCSX-ReARMed** | GPL-2 | Muito leve. | PCs fracos |

> ⚠️ O DuckStation original mudou de licença em 2024 (CC BY-NC-ND) e não deve ser redistribuído.
> Por isso usamos o **SwanStation**, que é GPL.

**Core próprio (opcional, Fase 7):** pode virar um projeto de estudo depois que o produto estiver
de pé, seguindo a documentação *psx-spx*. Como ele vai implementar a mesma interface libretro,
entra no ROXemu "de graça" ao lado dos outros.

---

## 3. Arquitetura

```
┌───────────────────────────────────────────────┐
│ ROXemu Launcher (Tauri 2)                     │
│ ┌───────────────────────┐  ┌────────────────┐ │
│ │ UI: React + TS        │◄►│ Backend Rust   │ │
│ │ • Home / estante      │  │ • rox-library  │ │
│ │ • Detalhes do jogo    │  │ • rox-metadata │ │
│ │ • Configurações       │  │ • SQLite       │ │
│ │ • Navegação c/ ctrl   │  │ • dispara runner│ │
│ └───────────────────────┘  └───────┬────────┘ │
└────────────────────────────────────┼──────────┘
                                     │ processo filho + IPC (JSON lines)
                                     ▼
┌───────────────────────────────────────────────┐
│ rox-runner (Rust nativo)                      │
│ • janela, áudio e controle via SDL3           │
│ • rox-core-host: carrega o core (.dll) libretro│
│ • renderização OpenGL/Vulkan + shaders        │
│ • menu in-game, save states, fast-forward     │
└───────────────────────┬───────────────────────┘
                        ▼
            swanstation_libretro.dll  (compilado por nós)
```

### Por que dois processos?
- **Se o core travar, o launcher continua de pé.** O runner fecha e o usuário volta para a estante.
- **O jogo roda em uma janela nativa** (tela cheia, VSync, baixa latência), sem passar pela webview.
- **A UI fica livre para ser bonita.** Animações em web (CSS/Framer Motion) são muito mais rápidas
  de iterar do que UI nativa.

### Stack recomendada

| Camada | Escolha | Motivo |
|---|---|---|
| Launcher | **Tauri 2** | É leve (usa o WebView2 que já vem no Windows) e tem um updater com assinatura nativa. |
| UI | **React + TypeScript + Vite** | Ecossistema enorme. A Gamepad API funciona no WebView2. |
| Animações | **Framer Motion** + CSS | Escala, blur e paralaxe suaves nos cards. |
| Backend / runner | **Rust** | Segurança de memória perto de código C (cores), ótimo para FFI. |
| Janela / áudio / input do jogo | **SDL3** (crate `sdl3`) | Suporte de primeira para controles Xbox e DualShock/DualSense. |
| Banco | **SQLite** (`rusqlite`) | É um arquivo local, sem servidor. |
| Monorepo | Cargo workspace + **pnpm** | Um repositório para tudo. |

---

## 4. Estrutura do repositório

```
ROXemu/
├─ apps/
│  └─ launcher/              # Tauri 2
│     ├─ src-tauri/          # comandos Rust expostos à UI
│     └─ ui/                 # React + TS (home, estante, settings)
├─ crates/
│  ├─ rox-common/            # tipos compartilhados, config, protocolo IPC
│  ├─ rox-core-host/         # implementação do frontend libretro (FFI)
│  ├─ rox-runner/            # binário que roda o jogo
│  ├─ rox-library/           # scanner, leitura CUE/BIN/ISO/CHD/M3U, serial
│  └─ rox-metadata/          # base de jogos (DAT), capas, cache
├─ cores/
│  ├─ cores.lock.toml        # core → repositório, commit fixado, hash esperado
│  └─ build-cores.ps1        # compila os cores a partir do fonte
├─ assets/                   # logo, sons de navegação, fontes
├─ docs/
│  └─ PLANO.md
└─ .github/workflows/        # CI: build, testes, cores, release assinada
```

### Dados do usuário (`%APPDATA%\ROXemu\`, ou ao lado do .exe no **modo portátil**)

```
bios/        # BIOS do PS1 que o usuário extraiu do próprio console
games.db     # biblioteca (SQLite)
covers/      # capas, heroes e logos em cache
saves/       # memory cards por jogo
states/      # save states + miniatura
config.toml  # configurações globais e por jogo
```

---

## 5. Como a biblioteca identifica os jogos

1. **Escanear as pastas** que o usuário escolher, procurando `.cue`, `.bin`, `.iso`, `.chd`,
   `.pbp` e `.m3u`.
2. **Agrupar as faixas** (um `.cue` mais vários `.bin`) e os jogos de vários discos (gerar
   `.m3u` automaticamente: FF7, MGS…).
3. **Ler o `SYSTEM.CNF`** dentro do sistema de arquivos ISO9660 do disco
   → `BOOT = cdrom:\SLUS_005.94;1` → **serial `SLUS-00594`**.
4. **Casar o serial** com a base do *Redump* / *libretro-database* para obter o título oficial e a
   região (NTSC-U, PAL, NTSC-J).
5. **Fallback:** se não houver serial, usar o hash CRC32 da faixa de dados. Se nem isso der, usar
   o nome do arquivo.
6. **Baixar a arte e guardar em cache local** (só uma vez):
   - **Capas (box art):** *libretro-thumbnails* (GitHub, gratuito, sem chave de API).
   - **Hero e logo** (fundo grande da home): *SteamGridDB*, que é opcional e usa uma chave do usuário.
   - Opcionalmente *ScreenScraper* / *IGDB* para descrição, ano, gênero e desenvolvedora.
7. **Deixar o usuário trocar a capa** manualmente: arrastar uma imagem para o card.

> 💡 **Formato importa:** um jogo de PS1 "de verdade" é **BIN/CUE** (setores de 2352 bytes, com
> faixas de áudio). Uma `.iso` de 2048 bytes perde as músicas em CD-Audio de vários jogos. Vamos
> aceitar `.iso`, mas a recomendação na UI será BIN/CUE ou **CHD** (comprimido, sem perda).

### Esquema inicial do banco

```sql
games (
  id INTEGER PRIMARY KEY, system TEXT, title TEXT, serial TEXT, region TEXT,
  launch_path TEXT, crc32 TEXT, cover_path TEXT, hero_path TEXT, logo_path TEXT,
  description TEXT, year INTEGER, developer TEXT,
  favorite INTEGER DEFAULT 0, added_at INTEGER, last_played_at INTEGER,
  playtime_sec INTEGER DEFAULT 0, core_override TEXT
)
discs (game_id INTEGER, disc_number INTEGER, path TEXT)
```

---

## 6. A interface (o "portfólio estante")

### Home
- Uma **fileira horizontal de cards** com as capas. O card selecionado **cresce**, ganha brilho e
  mostra o título.
- **Fundo dinâmico:** a arte hero do jogo selecionado, com blur e gradiente roxo, trocando com
  crossfade.
- Abaixo do card: o botão **▶ Jogar**, o tempo jogado, a última sessão e o último save state (com
  miniatura).
- Filas extras: *Continuar jogando*, *Favoritos*, *Adicionados recentemente*.

### Estante (biblioteca completa)
- Uma grade de capas com filtro por região e gênero, busca e ordenação (A-Z, mais jogado,
  recente).

### Navegação
- **Controle primeiro:** direcional e analógico movem, A/✕ joga, B/○ volta, Y/△ favorita, Start
  abre as opções. O teclado e o mouse também funcionam.
- Sons curtos de navegação, que podem ser desligados.

### Modo Console (tela cheia, "o meu console")
- Uma opção para **iniciar junto com o Windows** já em tela cheia, como um console ligando.
- Uma **animação de boot** do ROXemu, com o som da marca (curto, que dá para pular).
- **Perfis** (avatar e nome), cada um com saves, tempo de jogo e favoritos próprios.
- Uma **barra superior** com o relógio, os controles conectados e a bateria do controle sem fio.
- **Menu do sistema** pelo botão Guide: configurações, trocar perfil, sair para o Windows,
  suspender e desligar o PC.
- **Troféus** (via RetroAchievements, Fase 6) e uma tela "Minhas estatísticas" (horas por jogo
  e por console).
- **Temas:** o roxo é o padrão, mas outras cores e papéis de parede podem vir depois.

### Configurações
- Pastas de jogos, BIOS (com indicador ✅/❌ e verificação por hash), controles, vídeo (resolução
  interna, upscale, shader CRT), áudio e core padrão.

### Menu in-game (no runner, abre com Guide/Home ou F1)
- Voltar, salvar estado, carregar estado (com miniaturas), trocar disco, configurações rápidas e
  sair para a estante.

---

## 7. "Sem vírus": como garantir de verdade

1. **Repositório público** (licença **GPL-3**, compatível com os cores).
2. **Cores compilados no nosso CI** a partir do repositório oficial, com o commit fixado em
   `cores.lock.toml`. Nunca baixamos uma `.dll` pronta de terceiros.
3. **Hash SHA-256** de cada arquivo publicado na release.
4. **Assinatura de código no Windows** (Azure Trusted Signing ou certificado OV), o que evita o
   alerta do SmartScreen.
5. **Auto-update assinado** pelo updater do Tauri. O app recusa atualização sem assinatura válida.
6. **Zero telemetria por padrão**, nenhum instalador de terceiros e nenhuma toolbar.
7. **Verificação dos cores ao abrir:** o runner confere o hash da `.dll` antes de carregar.

### Questões legais (importante)
- **A BIOS do PS1 não pode ser distribuída.** O usuário extrai a do próprio console e o ROXemu
  apenas detecta e valida o hash (SCPH-1001, SCPH-5501, SCPH-7502…).
- **Os jogos também não podem ser distribuídos.** Eles são dumps dos discos do próprio usuário.
- **Nada de logos ou marcas registradas** na UI. A identidade é 100% ROXemu.

---

## 8. Roadmap por fases

### Fase 0 — Fundação *(≈ 1 semana)*
- [ ] Instalar o toolchain: Rust (rustup + MSVC Build Tools), pnpm e CMake.
- [ ] Criar o monorepo: Cargo workspace, app Tauri e a UI React.
- [ ] Configurar o CI no GitHub Actions: lint (clippy, eslint), testes e build do Windows.
- [ ] Criar o `cores.lock.toml` e o script que compila o SwanStation do fonte.

**Pronto quando:** o `pnpm tauri dev` abre uma janela vazia do ROXemu e o CI está verde.

### Fase 1 — Runner mínimo: o primeiro jogo bootando *(≈ 2–3 semanas)*
- [ ] `rox-core-host`: carregar a `.dll`, implementar os callbacks libretro (vídeo, áudio, input,
      environment e log).
- [ ] `rox-runner`: janela SDL3, upload de framebuffer por software, áudio com resampling e
      controle e teclado.
- [ ] Rodar pela linha de comando:
      `rox-runner --core swanstation --game "C:\Jogos\X.cue" --bios "...\bios"`.

**Pronto quando:** um jogo de PS1 vai da tela da BIOS até a gameplay, com som e controle.

### Fase 2 — Biblioteca e metadados *(≈ 2 semanas)*
- [ ] Scanner de pastas, parser CUE/ISO9660, extração do serial e suporte a CHD.
- [ ] Importar a base Redump/libretro e casar serial → título e região.
- [ ] Download e cache das capas (libretro-thumbnails), hero e logo opcionais.
- [ ] SQLite, mais comandos Tauri para listar, buscar e atualizar.

**Pronto quando:** apontar uma pasta → todos os jogos aparecem com nome certo e capa.

### Fase 3 — Launcher bonito *(≈ 3 semanas)*
- [ ] Home com fileira rolável, card em destaque e fundo dinâmico.
- [ ] Estante em grade, busca e filtros.
- [ ] Navegação completa por controle (Gamepad API) e sons.
- [ ] Botão Jogar → dispara o runner → recebe eventos (iniciou, fechou, tempo jogado).
- [ ] Tela de configurações e assistente de primeira execução (pastas e BIOS).

**Pronto quando:** dá para usar o ROXemu do sofá, só com o controle, do boot até o jogo e de volta.

### Fase 4 — Experiência in-game *(≈ 3 semanas)*
- [ ] Renderização por hardware (OpenGL/Vulkan): resolução interna 2x–8x e PGXP.
- [ ] Menu in-game sobreposto.
- [ ] Save states com miniatura, memory card separado por jogo e troca de disco.
- [ ] Fast-forward, rewind e shaders (CRT, scanlines).
- [ ] Configuração por jogo (core, resolução, controle).

### Fase 5 — Distribuição segura *(≈ 1–2 semanas)*
- [ ] Instalador (NSIS/MSI via Tauri) e versão portátil (.zip).
- [ ] Assinatura de código, hashes publicados e auto-update assinado.
- [ ] Página de release explicando como verificar os hashes.

### Fase 6 — Além do PS1
- [ ] Abstração `System`: cada console é um módulo que declara:
  - os formatos aceitos e como abrir cada um (BIN/CUE, ISO, CHD, ZIP, M3U…);
  - os cores disponíveis e o padrão;
  - as BIOS exigidas e os hashes válidos;
  - a regra de identificação (serial no disco, cabeçalho da ROM, CRC);
  - a cor e o ícone da plataforma na UI.

  Para o usuário, um jogo é sempre só "um jogo".
- [ ] SNES, Mega Drive, GBA e N64 via cores libretro.
- [ ] RetroAchievements (biblioteca `rcheevos`).
- [ ] Sincronização de saves (pasta na nuvem do usuário).

### Fase 7 (opcional) — Core próprio "ROXcore PSX"
- [ ] Um projeto de estudo: CPU R3000A → GPU → CD-ROM → SPU → GTE → MDEC.
- [ ] Expõe a mesma API libretro, então entra no ROXemu ao lado dos outros cores.

---

## 9. Riscos e como lidar

| Risco | Mitigação |
|---|---|
| Renderização por hardware via libretro é a parte mais chata (contexto GL compartilhado). | Começar com o renderer por **software** (Fase 1) e só depois migrar. |
| Capa errada ou jogo não identificado. | Serial → CRC → nome, e o usuário sempre pode corrigir manualmente. |
| Latência de input. | O runner é nativo, sem passar pela webview, com VSync configurável e *run-ahead* depois. |
| Licenças. | O projeto inteiro é GPL-3 e nenhum core tem licença restritiva. |
| SmartScreen reclamando do .exe. | Assinatura de código desde a primeira release pública. |

---

## 10. Próximos passos imediatos

1. Confirmar a stack (Tauri + React + Rust + SDL3 + SwanStation).
2. Instalar o Rust e o MSVC Build Tools (Node e Git já estão na máquina).
3. Fazer um mockup visual da home para fechar a identidade (cores, tipografia, animação dos cards).
4. Começar a **Fase 0**.
