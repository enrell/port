# Plano: port — Gerenciador de Portas Linux com TUI

## Goal
Criar um aplicativo CLI em Rust chamado `port` que resolve o problema de processos fantasmas ocupando portas após crashes de terminal. A ferramenta apresenta uma TUI (ratatui) listando portas abertas relevantes, permite busca e kill forçado via confirmação modal.

---

## Current Context / Assumptions
- Sistema alvo: Linux (usa /proc/net/tcp, /proc/[pid]/exe, lsof ou netstat)
- Cargo/rustc já instalados
- Portas de sistema devem ser filtradas (22-ssh, 80, 443, systemd, docker containers, etc.)
- Kill é sempre forçado (SIGKILL)

---

## Proposed Approach
Arquitetura simples em camadas:

```
src/
├── main.rs          # Entry point: inicializa TUI
├── app.rs           # State management (AppState, Mode)
├── ui.rs            # Rendering (ratatui widgets)
├── events.rs        # Input handling (keyboard events)
├── ports.rs         # Coleta de dados de portas (/proc parsing)
├── process.rs       # Operações em processos (nome, path, kill)
└── filter.rs        # Filtragem de portas do sistema
```

### Tech Stack
- **ratatui** → TUI framework
- **crossterm** → Eventos de teclado cross-platform
- **sysinfo** → Info de processos (alternativa ao parsing manual)
- **nom** ou regex → Parsing opcional do /proc/net/tcp

---

## Step-by-Step Plan

### Passo 1: Estrutura Inicial do Projeto
```bash
cargo new port --bin
cd port
```
Adicionar ao Cargo.toml:
- ratatui = "0.28"
- crossterm = "0.28"
- sysinfo = "0.30"

### Passo 2: Módulo ports.rs — Coleta de Portas
- Ler `/proc/net/tcp` e `/proc/net/tcp6`
- Parsear: inode, local_address (hex IP:port)
- Mapear inode → pid via `/proc/[pid]/fd/`
- Extrair: porta numérica, pid, nome do processo, path do executável
- Retornar: `Vec<PortInfo>`

### Passo 3: Módulo filter.rs — Filtragem
Implementar blacklist de:
- Portas: 22, 80, 443, 53, 3306, 5432, 6379, etc.
- Processos: systemd, dockerd, containerd, sshd, tor
- UIDs: 0 (root) para serviços do sistema

Função: `is_user_port(port: u16, name: &str) -> bool`

### Passo 4: Módulo process.rs — Operações em Processos
- `get_process_info(pid: u32) -> ProcessInfo` → nome, path
- `kill_process(pid: u32) -> Result<(), Error>` → SIGKILL via libc::kill

### Passo 5: Módulo app.rs — State Management
Struct App com:
- `ports: Vec<PortInfo>` (dados)
- `filtered: Vec<usize>` (índices após busca)
- `selected: usize` (cursor)
- `search_query: String`
- `mode: Mode` (Normal, ConfirmKill { pid, name })
- `message: Option<String>` (feedback)

Métodos:
- `next()`, `previous()` → navegação
- `update_search(query)` → filtragem dinâmica
- `confirm_kill()` → transição modo confirmação
- `execute_kill()` → chama process::kill

### Passo 6: Módulo ui.rs — Rendering
Layout em 3 áreas:
1. **Header**: título + campo de busca
2. **Table**: colunas PORT | NAME | PATH (highlight na linha selecionada)
3. **Footer**: hints de navegação

Modal de confirmação (popup central):
```
┌─────────────────────────┐
│  Kill process?          │
│  firefox (PID: 12345)     │
│                          │
│  [Yes]  No               │
└─────────────────────────┘
```

### Passo 7: Módulo events.rs — Input Handling
Eventos crossterm:
- `q` ou `Ctrl+C` → quit
- `/` ou `i` → modo busca (digitos atualizam query)
- `j/↓` ou `k/↑` → navegação
- `Enter` → confirm_kill (se houver item selecionado)
- `Esc` → cancela modo busca ou fecha modal
- No modal: `y/Y` → confirm kill, `n/N` ou `Esc` → cancela

### Passo 8: main.rs — Entry Point
```rust
fn main() -> Result<(), Box<dyn Error>> {
    terminal::enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new();
    
    app.refresh_ports(); // carga inicial
    
    loop {
        terminal.draw(|f| ui::render(f, &app))?;
        if events::handle(&mut app)? { break; }
    }
    
    terminal::disable_raw_mode()?;
    Ok(())
}
```

### Passo 9: Testes Unitários
Criar `src/tests/` ou inline com `#[cfg(test)]`:

- `filter_tests.rs`: testar blacklist de portas/processos
- `ports_tests.rs`: mock de /proc/net/tcp, verificar parse correto
- `app_tests.rs`: testar navegação, busca, mudança de modo
- `process_tests.rs`: mock de kill, verificar chamada correta

Cobertura mínima: 70% das funções públicas.

### Passo 10: Build e Release
```bash
cargo build --release
# Binário em: target/release/port
```

---

## Files to Create

| Arquivo | Propósito |
|---------|-----------|
| `Cargo.toml` | Dependências |
| `src/main.rs` | Entry point |
| `src/app.rs` | State machine |
| `src/ui.rs` | Ratatui widgets |
| `src/events.rs` | Keyboard handler |
| `src/ports.rs` | Parser de /proc/net/tcp |
| `src/process.rs` | Info e kill de processos |
| `src/filter.rs` | Blacklist lógica |
| `src/lib.rs` | Re-export modules (para testes) |

---

## Tests / Validation

1. **Teste de integração manual**:
   - Rodar `nc -l 9999` em outro terminal
   - Abrir `port`
   - Verificar se porta 9999 aparece na lista
   - Buscar por "nc", selecionar, Enter, confirmar kill
   - Verificar se nc foi terminado

2. **Testes unitários**:
   ```bash
   cargo test
   ```

3. **Casos edge**:
   - Processo termina entre listagem e kill (erro graceful)
   - Permissão negada ao tentar kill de processo root (mensagem apropriada)
   - Lista vazia (mensagem "Nenhuma porta relevante aberta")

---

## Risks, Tradeoffs, Open Questions

### Risks
- **Parsing /proc/net/tcp** é sensitivo a versões de kernel. Mitigação: usar `lsof -i -P -n` como fallback se /proc parsing falhar.
- **Permissões**: kill de processos de outros usuários requer root. A ferramenta deve mostrar "Permission denied" sem crashar.
- **Race conditions**: processo pode morrer entre a listagem e o kill. Tratar erro ESRCH.

### Tradeoffs
- **sysinfo vs manual**: sysinfo é mais simples mas adiciona dependência. Parsing manual de /proc é mais leve. **Decisão**: usar sysinfo para process_info, manual para portas.
- **TUI vs CLI puro**: TUI adiciona complexidade inicial mas UX superior. User requested TUI → manter ratatui.
- **Portas filtradas**: lista hardcoded vs config file. MVP → hardcoded, extensível via env var futuramente.

### Open Questions
1. Deve incluir portas IPv6? (Sim, /proc/net/tcp6 existe)
2. Ordenação default: por porta (numérico) ou por nome? (Sugerido: por porta)
3. Suporte a UDP? (Não no MVP, focar TCP)

---

## Estimativa
- **Setup + estrutura**: 30 min
- **Lógica de portas/processos**: 1h
- **TUI + eventos**: 1.5h
- **Testes**: 1h
- **Polimento**: 30 min

**Total estimado**: ~4-5 horas de implementação focada.
