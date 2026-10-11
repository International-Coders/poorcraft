# AGENTS.md — POORCRAFT NOVO

Este diretório é o jogo ativo. Antes de alterar código, assets, UI, áudio,
worldgen ou empacotamento, leia nesta ordem:

1. `docs/CONTINUACAO-GLM/00-LEIA-PRIMEIRO.md`.
2. `docs/CONTINUACAO-GLM/01-VISAO-E-IDENTIDADE.md`.
3. `docs/CONTINUACAO-GLM/02-MODO-DE-OPERACAO-GLM.md`.
4. `docs/CONTINUACAO-GLM/12-FILA-DE-EXECUCAO.md` e o plano detalhado da
   faixa atual (13–16).
5. O documento especializado da tarefa atual.
6. `../docs/POORCRAFT-3D/STATE.md` e `../docs/POORCRAFT-3D/CURRENT-PLAN.md`
   para o histórico medido que antecede esta doutrina.

## Leis obrigatórias

- Um job entrega uma mudança jogável, testes, provas e um runtime. Markdown
  sozinho não fecha job.
- Nenhum asset entra no catálogo jogável sem manifesto, proveniência, escala,
  eixos, orçamento, colisão, sockets, consumidor, captura e rota de uso.
- Um asset visto na mão é um **viewmodel**, não a cópia do modelo de mundo.
  Deve passar os gates de mão dominante, FOV, clip, oclusão, animação e alvo.
- Screenshot não prova comportamento. Prova visual exige pixels + estado
  semântico + câmera declarada + comparação + inspeção humana.
- Teste verde que não detecta a falha descrita é um teste inútil. Primeiro
  reproduza o defeito; depois demonstre que o gate fica vermelho; só então
  conserte.
- Rios, árvores, construções, NPCs e props obedecem ao mesmo solo autoritativo
  usado por render, colisão e navegação. Nada pode flutuar ou enterrar o player.
- Spawn é uma busca validada, nunca um ponto aproximado: cápsula livre, chão
  caminhável, visão frontal limpa, água segura e recursos alcançáveis.
- O jogo ensina fazendo. O primeiro jogador deve descobrir coleta, ferramenta,
  bancada, terreno, abrigo, fogo e primeira noite sem ler código ou lista de
  atalhos.
- Referências como Valheim, Skyrim e Heroes III descrevem qualidades. Nomes,
  mapas, arte, música, texto, personagens e progressão devem ser originais.
- Windows, Linux e macOS são alvos desde a mudança inicial. Steam é um canal de
  distribuição planejado, não um remendo ao final.
- Não baixar modelos, pesos, SoundFonts, executáveis ou assets sem registrar
  origem, licença, hash, tamanho, hardware necessário e política de remoção.
- `doctrine.json` é contrato. Rode `make p3d-doctrine-check` antes de fechar
  qualquer job que altere a doutrina ou seus gates.

## Sequência por job

Orientar → reproduzir → escrever contrato → fazer o gate falhar → implementar
→ testes focados → `make p3d-test` → rota visual afetada → inspeção dos PNGs →
runtime → registros → commit → push.

Se uma etapa não puder ser executada, o job não recebe o rótulo “pronto”.
Registre a limitação e deixe a próxima ação precisa.
