# WMK BIM Studio — manual de primeiro contato

**Personalização:** Walney Moreira Klein — walneyk@hotmail.com.

## 1. Preparar o ambiente
Instale o Rust 1.95+ (toolchain MSVC), Git e Visual Studio Build Tools com C++ no Windows. Em PowerShell:
```powershell
git clone https://github.com/WalneyNF/WMK-BIM-STUDIO.git
cd WMK-BIM-STUDIO
cargo run --release -p bimcraft -- --sample
```
A primeira compilação pode demorar. Não há instalador WMK validado nesta etapa.

## 2. Conhecer a interface
- **Início:** projeto novo, abrir projeto, casa de exemplo, recentes.
- **Arquitetura:** paredes, portas, janelas, pisos, telhado e ambientes.
- **Propriedades:** parâmetros do objeto selecionado.
- **Navegação 3D:** órbita, zoom e pan.
- **Documentação:** plantas, cortes, elevações e pranchas.
- **Salvar:** mantenha backups e teste alterações em cópias.

## 3. Exercício inicial
1. Abra o projeto exemplo (`--sample`).
2. Selecione uma parede; examine suas propriedades.
3. Abra a vista 3D e teste órbita e zoom.
4. Altere um elemento simples e observe as vistas.
5. Salve com outro nome.

## 4. Idioma
O fork passa a iniciar em português quando não há preferência anterior; uma escolha de idioma já salva continua prevalecendo. Frases sem tradução no catálogo poderão aparecer em inglês.

## 5. Planta JPG/PNG/PDF
O reconhecimento automático e a integração ao desktop são metas do roadmap, não funções concluídas. Consulte `roadmap.html` para distinguir entregas das propostas.

## 6. Problemas comuns
- **Cargo não encontrado:** confirme instalação do Rust e reinicie o terminal.
- **Erro do linker:** instale componentes C++/MSVC do Visual Studio.
- **Programa não abre:** verifique o log `bimcraft.log` conforme documentação original.
- **Arquivo de imagem não abre:** o aplicativo ainda não suporta isso como BIM gerado automaticamente.
