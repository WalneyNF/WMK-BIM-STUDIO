# Personalização WMK — diretrizes e estado

Créditos de personalização: **Walney Moreira Klein — walneyk@hotmail.com**.

## Identidade
- Nome público: **WMK BIM Studio**.
- Tema proposto: **Hacker/engenharia**, azul-marinho quase preto, ciano e prata.
- Logo vetorial do README: `assets/wmk/logo-wmk-bim-studio.svg`.
- Tela inicial: título WMK, chamada em português e créditos.
- Barra inferior da tela de início: créditos WMK.
- Preservar atribuição a IPedrax e licenças originais.

## Alterações de código desta etapa
- Título da janela: WMK BIM Studio.
- Linguagem padrão sem configuração salva: português.
- Página de início e URL do código: WMK.
- README em português; original movido por referência para `README.original.en.md`.
- Roadmap WMK separado do roadmap original.

## Ainda pendente de implementação e testes
- Substituir ícones binários originais (ICO, PNG, ICNS) pelo logotipo aprovado, regenerando todos os tamanhos corretamente.
- Completar mensagens vazias em `translations/ui/pt.po` e `translations/project/pt.po`.
- Revisar textos técnicos, instaladores, identificadores Linux/macOS e updater.
- Integrar o importador de planta JPG/PNG/PDF com um modelo BIM 3D.
- Fazer build e rodar testes antes de publicar executáveis.

**Importante:** nomes internos de crates Rust e extensões de projeto permanecem inalterados nesta etapa para não quebrar dependências.
