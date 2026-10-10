# 00 — Leia primeiro: o contrato de continuidade

## Resultado esperado

O proprietário deve poder abrir um runtime recente, jogar, apontar o que gosta
ou não gosta e direcionar o próximo passo. A IA assume o trabalho mecânico de
diagnosticar, implementar, testar, capturar, comparar, empacotar e registrar.
Ela não assume o gosto do proprietário e não esconde incertezas atrás de uma
contagem grande de testes.

## O problema que este pack resolve

O projeto já possui muita engenharia: worldgen determinístico, renderer wgpu,
assets GLB gerados, observatory, rotas automatizadas, persistência, UI, áudio e
centenas de testes. Mesmo assim, um build pode apresentar um rio no ar, câmera
dentro de uma árvore, cave proof ilegível, braços separados do corpo ou uma
ferramenta atravessando a câmera. Isso ocorre quando o teste prova apenas um
contador interno ou a existência de pixels, e não a experiência pretendida.

Este manual muda o significado de “feito”:

> Feito = estado correto + imagem correta + interação correta + persistência
> correta + desempenho dentro do orçamento + runtime reproduzível.

## A separação do repositório

- `../../..` é `poorcraft-novo/`, o jogo ativo.
- `../../../..` é a raiz de controle.
- `../../../../poorcraft-antigo/` preserva LOREFORGE/POORCRAFT, seus saves,
  provas e releases. Não copie features automaticamente entre as engines.
- A documentação histórica compartilhada permanece em `../../../../docs/`.

O legado serve como fonte de ideias e implementações já estudadas. O novo jogo
é livre para portar um conceito, mas deve revalidá-lo contra sua arquitetura,
direção visual, controles, formato de save e bateria atuais.

## O que a GLM faz ao iniciar uma sessão

1. Lê `AGENTS.md`, este pack, `STATE.md`, a última entrada do devlog e o diff.
2. Joga ou reproduz a rota relevante antes de editar.
3. Escreve uma frase de player: “quando X, vejo/faço Y; deveria Z”.
4. Identifica a autoridade de dados: terreno, inventário, item, câmera, água,
   animação, UI, som, save ou rede.
5. Cria ou endurece o gate que detecta a falha atual.
6. Executa o gate no estado ruim e preserva a evidência vermelha quando útil.
7. Implementa a menor fatia jogável que fecha o problema.
8. Executa a escada proporcional de testes, não só um teste unitário feliz.
9. Inspeciona visualmente as capturas e mede os pixels/estado semântico.
10. Gera runtime, atualiza registros, commita e envia ao GitHub.

## O que a GLM não faz

- Não aumenta contagem de assets sem consumidores jogáveis.
- Não gera 100 variações de uma geometria ruim.
- Não considera “o JSON parseia” como prova de um objeto 3D.
- Não reposiciona a câmera até esconder um bug que existe em jogo.
- Não grava golden screenshot do resultado defeituoso para tornar o teste verde.
- Não reduz assertions para acomodar flakiness; remove a causa da flakiness.
- Não instala um MCP, modelo ou pacote porque apareceu bem ranqueado.
- Não anuncia Steam Early Access antes do primeiro loop ter valor e estabilidade.
- Não copia expressão protegida dos jogos de referência.

## Definição mínima de job

Todo job nomeia:

- problema visível;
- jogador e momento afetados;
- autoridade de dados;
- arquivos e camadas tocadas;
- antes reproduzível;
- depois mensurável;
- testes que devem falhar antes e passar depois;
- capturas e sidecars;
- orçamento de desempenho;
- impacto de save/rede/plataforma;
- risco e rollback;
- limitações restantes;
- próximo job único.

## Regra de uma hora

É aceitável gastar uma hora ajustando uma pá na mão, um baú, uma silhueta ou
uma margem de HUD, desde que a hora produza conhecimento reutilizável: presets,
âncoras, medidas, capturas comparáveis e gates. É inaceitável gastar a mesma
hora repetindo tentativa visual sem registrar escala, pose, FOV e erro.
