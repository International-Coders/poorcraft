# 04 — Viewmodel, mãos, ferramentas e câmera

## Por que é sistema próprio

O objeto em primeira pessoa precisa parecer preso ao corpo, responder ao
movimento e nunca bloquear o que o player mira. Reutilizar a transformação do
world model causa pá de lado, braço flutuante, clipping e escala errada.

## Autoridades

- World model: posição física, drop, sombra, colisão e visão de terceiros.
- Viewmodel: câmera própria ou pass dedicado, rig de braços, grip sockets,
  animação e offsets por família.
- Ação: domínio calcula alvo, custo e resultado; animação não decide regra.
- Retículo: deriva do raycast de gameplay, nunca da ponta visual da ferramenta.

## ViewmodelSpec

Cada item equipável registra:

- mão dominante e suporte;
- socket primário/secundário;
- posição/rotação/escala base;
- FOV de viewmodel e near plane;
- poses idle, walk, sprint, jump, use, block, equip e unequip;
- duração e janela de impacto;
- sway máximo, bob, recoil e retorno;
- envelope de tela permitido;
- distância mínima do centro/retículo;
- política de oclusão e sombras;
- câmera/rig de prova.

Offsets são dados versionados, não números espalhados no renderer.

## Calibração visual disciplinada

1. Renderizar grade de frente/lado/topo com eixos do socket.
2. Posicionar mão no grip, não o asset por tentativa.
3. Verificar posição da segunda mão quando existir.
4. Ajustar silhueta no idle em FOV 60, 75 e 90.
5. Executar ciclo completo sem alvo.
6. Executar contra alvo a 0.5, 1, 2 e alcance máximo.
7. Testar parede encostada, teto baixo, sprint, salto e agachamento futuro.
8. Conferir retículo e impacto no mesmo frame sem exigir alinhamento falso da
   ponta da ferramenta.
9. Comparar mão esquerda/direita caso acessibilidade permita espelhamento.
10. Salvar preset e capturas; só depois ajustar acabamento.

## Gates obrigatórios

- Nenhum pixel do viewmodel cruza o near plane.
- AABB projetada ocupa entre o mínimo e máximo da classe do item.
- Retículo permanece desobstruído por margem configurada.
- Mão primária fica dentro da tolerância do grip em todos os keyframes.
- Mão secundária não flutua quando o spec exige two-hand.
- Ferramenta não entra na câmera durante bob/sway/sprint.
- Cabeça/lâmina aponta para o hemisfério correto da ação.
- Sweep visual e janela de impacto diferem menos que a tolerância declarada.
- Equip/unequip não teleporta por um frame.
- Troca de FOV/aspect ratio não inverte, corta ou alonga.
- Som e efeito ocorrem no evento semântico, não em qualquer frame de animação.
- World model continua correto quando o viewmodel está oculto.

## Matriz de prova

Combinações mínimas:

- itens: mãos, axe, pickaxe, shovel, hoe, hammer, arma, comida;
- FOV: 60/75/90;
- aspecto: 16:9, 16:10, ultrawide seguro;
- pose: parado, andando, sprint, salto, uso;
- contexto: aberto, parede, teto, alvo, sem alvo;
- qualidade: low/high;
- handedness: padrão e espelho quando implementado.

Não é necessário multiplicar todas as combinações em cada commit. Um conjunto
pairwise cobre o ciclo diário; a matriz completa roda na promoção/release.

## Critério humano

Além dos números, uma pessoa responde:

- Parece que a mão realmente segura o peso?
- A direção da ação combina com o movimento?
- Consigo ver o alvo e o feedback?
- O tamanho parece plausível comparado ao mundo?
- Dez minutos de uso causam enjoo ou distração?

Se qualquer resposta for negativa, o asset continua protótipo.
