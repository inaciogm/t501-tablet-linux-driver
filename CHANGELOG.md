# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2026-10-21

### Added
- **Natural Drag-to-Scroll Mode**: Pressione o botão físico "B" da mesa ou segure o botão lateral da caneta para rolar páginas e documentos suavemente arrastando a ponta.
- **Tap-to-Click**: Toque rápido na mesa no modo de rolagem dispara clique nativo (`BTN_TOUCH` e `BTN_LEFT`) sob o cursor sem risco de arrasto acidental.
- **Instalador Unificado (`install.sh`)**: Instalação automatizada em um único comando no Bazzite/Fedora/Arch.
- **Desinstalador Completo (`uninstall.sh`)**: Remoção limpa de binários, serviços systemd e regras udev.
- **Pacote de Distribuição Pré-compilado**: Disponibilização de tarball release standalone x86_64 em `dist/`.

### Fixed
- **Eliminação de Tremor no Hover (Anti-Flicker)**: Implementação de histerese física com dois limiares (`TOUCH_ENTER_RAW = 1480` e `TOUCH_EXIT_RAW = 1600`) e normalização linear contínua da pressão, eliminando o piscar crônico do cursor quando a caneta flutua a 1–2 mm da mesa.
- **Detecção Confiável de Proximidade**: Uso do byte 7 de presença de hardware e timeout de leitura USB de 60 ms para liberação imediata de `BTN_TOOL_PEN` e `BTN_TOUCH` ao levantar a caneta.
- **Ordem de Emissão evdev**: `BTN_TOUCH` emitido prioritariamente no frame antes dos eixos de coordenadas, em conformidade com as especificações do kernel Linux.
- **Prevenção de Botões Presos**: Liberação forçada de botões stylus ao perder contato magnético com a mesa.

## [1.0.0] - 2026-07-16

### Added
- Suporte inicial ao modo de área total para Tomate MTM-1106 / Bazzite.
- Integração da sequência de inicialização de 4 etapas DIGImend.
- Serviço de usuário no systemd com regras uaccess sem necessidade de permissões root 0666.
- Agrupamento de coordenadas diagonais em batch único evdev.
