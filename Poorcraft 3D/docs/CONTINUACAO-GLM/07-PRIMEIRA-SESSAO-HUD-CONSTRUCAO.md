# 07 — Primeira sessão, HUD e construção

## Player alvo

Pessoa que conhece jogos de sobrevivência, mas nunca viu POORCRAFT. Ela não
conhece letras de debug, nomes de crates ou lore. O jogo deve mostrar uma ação
possível, permitir tentativa segura e responder com causa e efeito.

## Primeiros 20 minutos

### Minuto 0–2: orientação

- spawn limpo e horizonte com dois pontos de interesse;
- WASD/mouse/Space ensinados apenas quando necessários;
- mãos vazias e um prompt contextual para recurso próximo;
- HUD mostra vida, stamina, fome e hotbar real, sem material grátis.

### Minuto 2–5: primeira ferramenta

- coletar ramo/pedra/comida com E ou ação primária contextual;
- Tab abre inventário + crafting;
- uma receita possível é destacada e explica utilidade;
- craft equipa ou torna equipar óbvio;
- viewmodel passa os gates do documento 04.

### Minuto 5–12: local e bancada

- ferramenta melhora coleta;
- martelo abre peças por categoria, não teclado enciclopédico;
- ghost mostra custo, suporte, colisão, snap e rotação;
- bancada estende as receitas dentro da mesma tela.

### Minuto 12–20: abrigo e noite

- hoe/shovel permitem preparar terreno com preview exato;
- player fecha abrigo simples e coloca fogo;
- UI explica apenas o bloqueio atual;
- som, luz e animação confirmam conclusão;
- journal aponta ao mundo mais amplo, não prende em tutorial.

## HUD mínimo

- vitais agrupados, legíveis e sem cobrir centro;
- hotbar 1–8 com item real, quantidade, durabilidade e seleção;
- retículo contextual que muda somente quando a ação muda;
- prompt curto vindo do binding map;
- objetivo atual opcional e dispensável;
- feedback de coleta/custo/negação;
- ícones originais testados em escala e daltonismo.

O HUD não mostra barras reservadas vazias. Mana aparece quando mistérios a
desbloqueia; energia/temperatura aparecem no contexto que lhes dá sentido.

## Construção confiável

O preview e o commit chamam a mesma função pura `evaluate_placement` e recebem:

- transform final e sockets escolhidos;
- suporte e estabilidade;
- colisão com player/NPC/mundo;
- slope/grounding;
- propriedade e permissão;
- custo e inventário;
- impacto em navegação/água;
- reason code quando inválido.

Verde significa que a ação será aceita exatamente naquela forma. Âmbar indica
mudança/custo nomeado. Vermelho bloqueia e explica em linguagem de player.

## Construir tudo que quiser

Liberdade não significa ausência de regra. O sistema deve permitir composição
modular, peças estruturais, terreno moldável, rotação/snap e peças decorativas,
mantendo física e custo legíveis. Suporte estrutural pode ser simplificado,
desde que consistente, previsível e visualizado.

## Teste com pessoa desconhecida

Sem instrução verbal, observar:

- tempo até mover/olhar;
- tempo até coletar;
- primeira abertura de Tab;
- primeira ferramenta;
- erros de input;
- leitura de custo/preview;
- primeiro abrigo válido;
- motivo de desistência;
- pergunta feita em voz alta.

O pesquisador não ensina. Depois entrevista: o que achava que aconteceria, o
que aconteceu e o que queria fazer em seguida. A documentação se ajusta ao
comportamento real, não ao esperado.
