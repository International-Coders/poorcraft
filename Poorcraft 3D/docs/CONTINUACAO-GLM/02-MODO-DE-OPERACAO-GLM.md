# 02 — Modo de operação para GLM 5.3

## Objetivo

Fazer a GLM agir como equipe de engenharia, arte técnica e QA orientada por
evidência. A inteligência vem de contexto pequeno e preciso, ferramentas
estruturadas e loops fechados; não de um prompt enorme repetido sem inspeção.

## Contexto obrigatório por tarefa

A sessão carrega apenas o necessário, mas nunca omite:

- constituição e visão;
- estado medido e próximo job;
- contrato da feature;
- módulo vizinho usado como padrão;
- esquema/manifesto pertinente;
- captura ruim e sidecar da reprodução;
- orçamento e matriz de plataformas;
- falhas conhecidas e arquivos sujos não relacionados.

## Protocolo R-F-I-P-V

### 1. Reproduzir

Execute a rota exata. Capture seed, pose, FOV, resolução, quality tier, estado
do inventário, item equipado, alvo, frame, versão e hash. Uma frase “parece
errado” vira uma reprodução determinística.

### 2. Fazer falhar

Adicione um gate que detecta a falha real. Exemplos:

- água com pixels sem terreno/suporte abaixo;
- cápsula do spawn intersecta prop, árvore ou estrutura;
- viewmodel ocupa mais que a fração permitida ou cruza near plane;
- mão e cabo não compartilham socket durante animação;
- caixa tem tampa visual sem hinge/interação;
- botão visível não tem action id;
- receita “SHORT” não nomeia o próximo ingrediente alcançável.

Rode o gate e registre o vermelho. Se ele já passa, não prova o problema.

### 3. Implementar

Trabalhe na autoridade correta e mantenha camadas:

- `pc3d_core`: identidade, tempo, comandos e formato;
- `pc3d_world`: terreno, simulação, catálogo e regras;
- `pc3d_assets`: manifestos, geometria, materiais, validação;
- `pc3d_render`: GPU, câmera, input, UI e viewmodel;
- `pc3d_save`: persistência e migração;
- `pc3d_audio`: mixer, síntese, eventos e música;
- app: composição e rotas; não duplicar regra de domínio.

### 4. Provar

Suba pela escada de 30 fases do documento 06. Não é obrigatório rodar todas em
todo commit intermediário; é obrigatório executar todas as fases aplicáveis e
a bateria completa nos checkpoints de release. As provas incluem dados e
imagem, e rotas visuais são inspecionadas com olhos humanos.

### 5. Verificar produto

Jogue a sequência como alguém que não conhece o código. Pergunte: o próximo
passo é visível? O input é previsível? O efeito corresponde ao preview? O som
confirma? O save preserva? Existe razão para continuar?

## Uso eficiente de tokens

- Ler índices e manifestos antes de arquivos gigantes.
- Pedir exports estruturados em vez de descrever cenas da memória.
- Produzir diffs pequenos por card, ainda que a visão seja grande.
- Guardar decisões em JSON/Markdown testado para a próxima sessão.
- Não repetir diagnóstico já provado; linkar evidência.
- Quando o orçamento apertar, fechar um checkpoint verde e registrar o próximo
  comando exato. Nunca abandonar uma meia-migração.

## Anti-alucinação de engine

Antes de usar uma API, pesquise o código e a versão real. Não presumir Unity,
Godot ou Bevy: a engine é Rust + wgpu própria. Uma ferramenta externa entra
como processo de asset/inspeção, não muda silenciosamente a engine.

## Política de ferramentas externas

Para cada candidato:

1. conferir repositório oficial, manutenção e licença;
2. documentar permissões e superfície de execução;
3. fixar versão/commit e hash;
4. testar em diretório descartável com um asset não proprietário;
5. medir instalação, RAM/VRAM, tempo e tamanho;
6. comparar saída com o pipeline determinístico atual;
7. passar segurança, formato, originalidade e qualidade;
8. só então criar integração opcional e removível.

MCP com execução arbitrária em Blender ou shell é poderoso e deve permanecer
local, limitado ao workspace e desligado em builds de player.

## Relatório de encerramento

O relatório diz, sem linguagem promocional:

- comportamento que mudou;
- defeito que o novo teste detecta;
- comandos executados e contagens;
- capturas inspecionadas;
- frame/memória antes/depois quando relevante;
- runtimes reais no disco;
- plataformas não construídas e motivo;
- limitações/deferimentos;
- commit e resultado do push.
