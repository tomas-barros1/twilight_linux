# Twilight Linux

Filtro de luz azul nativo para Linux, inspirado no [Twilight](https://twilight.urbandroid.org/) para Android.

Interface **GTK 4 + libadwaita** (Relm4). O filtro real no display é aplicado pelo **[sunsetr](https://github.com/psi4j/sunsetr)** — esta aplicação é um front-end que controla o daemon via CLI (`status`, `get`, `set`, `preset`, `stop`), sem reimplementar o backend Wayland/Hyprland e sem alterar o `hyprland.conf`.

## Dependências (Arch / CachyOS)

```bash
sudo pacman -S --needed rust gtk4 libadwaita pkgconf sunsetr
```

Opcional (fallback Hyprland via sunsetr):

```bash
sudo pacman -S --needed hyprsunset
```

## Compilar e instalar

```bash
cd twilight_pc
cargo build --release
install -Dm755 target/release/twilight-linux ~/.local/bin/twilight-linux
install -Dm644 data/org.twilightlinux.Twilight.desktop \
  ~/.local/share/applications/org.twilightlinux.Twilight.desktop
```

Ou rode direto:

```bash
cargo run --release
```

## Uso

1. Certifique-se de que o **sunsetr** está disponível (`sunsetr --version`).
2. Abra o Twilight Linux — o painel mostra período, temperatura e gamma atuais.
3. Use o interruptor para ligar/desligar o filtro.
4. Ajuste temperatura (K), intensidade e gamma; as mudanças vão para o sunsetr com debounce.
5. Presets **Dia / Pôr do sol / Noite** usam modo estático; **Agendado** restaura o modo de transição (geo/manual).
6. Em **Ajustes**: horários, localização, backend (`auto` / `hyprland` / `hyprsunset` / `wayland`), autostart via `systemd --user`.

A interface segue o idioma do sistema (`LANG`): **português** ou **inglês** (traduções em `locales/*.json` via [rust-i18n](https://github.com/longbridge/rust-i18n)).

Fechar a janela **não** desliga o filtro (opção “Ocultar ao fechar”). O processo `sunsetr` continua aplicando a temperatura no compositor.

### Configuração isolada

O Twilight **não** usa `~/.config/sunsetr/`. Em vez disso grava em:

`~/.config/twilightlinux/sunsetr/sunsetr.toml`

Ao ligar o filtro, o app para qualquer sunsetr padrão (incluindo `sunsetr.service`) e inicia uma instância com `--config` apontando para essa pasta.

> Vem de uma versão anterior (`twilight_pc`)? A config de `~/.config/twilight/` é **migrada automaticamente** para `~/.config/twilightlinux/` na primeira execução, e a unit `twilight-sunsetr.service` é regravada se apontar para o caminho antigo.

### Troca de backend

Trocar o backend em Ajustes (`auto` → `hyprland` / `hyprsunset` / `wayland`) **reinicia a instância do sunsetr** para recriar o backend — sem isso a mudança não teria efeito no daemon. O app valida antes se a ferramenta necessária existe (ex.: `hyprsunset` instalado, sessão Hyprland para `hyprland`).

## Autostart do filtro (systemd --user)

Pelo app (Ajustes → “Iniciar filtro com a sessão”) ou manualmente:

```bash
systemctl --user enable --now sunsetr.service
```

Isso **não** edita a configuração do Hyprland.

## Conflitos

O app avisa se detectar `hyprsunset`, `gammastep`, `wlsunset` ou `redshift` rodando em paralelo. Prefira um único gerenciador de temperatura de cor.

## Configuração persistente

| Arquivo | Função |
|---------|--------|
| `~/.config/twilightlinux/config.toml` | Preferências da UI |
| `~/.config/twilightlinux/sunsetr/` | Config do sunsetr **isolada** (via `--config`) |
| `~/.config/sunsetr/` | **Não é usado** pelo Twilight |

## Desenvolvimento

```bash
cargo check
cargo test
cargo build --release
```

Backend smoke-test (com sunsetr rodando):

```bash
sunsetr status --json
sunsetr get all --json
```

## Licença

MIT
