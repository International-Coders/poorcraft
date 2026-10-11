# 05 — Mundo, biomas, rios, props e spawn

## Uma única autoridade espacial

Altura visível, colisão, navegação, água, placement e spawn devem consultar o
mesmo terreno final: base + biome modifier + river carve + estruturas + edits.
Nenhum subsistema pode usar uma aproximação antiga depois que outro modifica a
superfície.

## Spawn como busca com prova

Para cada seed, avaliar candidatos em anéis e pontuar:

- cápsula do player completamente livre;
- dois pés apoiados em superfície caminhável;
- inclinação e degrau dentro do controlador;
- clearance vertical e horizontal;
- nenhuma árvore, rocha, asset, parede ou água intersectando;
- câmera e segmento frontal livres por distância mínima;
- margem segura de rio, penhasco e queda;
- recurso inicial alcançável por caminho navegável;
- bancada/abrigo possível nas proximidades;
- região não reservada por capital/ruína/evento;
- luz e clima que permitem ler a cena inicial.

Se nenhum candidato passa, expandir busca determinística. Nunca aceitar um
candidato ruim por timeout silencioso. Sidecar de spawn registra rejeições e o
motivo do vencedor.

## Rios fisicamente críveis

Um rio passa somente quando:

- cada amostra flui para igual ou menor elevação, salvo queda d'água nomeada;
- RiverCarve cria leito sob toda superfície visível;
- largura, profundidade e velocidade vêm do flow record;
- margens conectam ao terreno sem faixa vazia;
- água é clipada/streamed com o mesmo anel e fog do terreno;
- nenhuma superfície azul existe sobre void ou acima da tolerância do leito;
- confluências não criam T-junctions abertas;
- pontes, rodas e pesca consultam o mesmo rio;
- editar barragem invalida apenas regiões limitadas e persiste;
- vistas ao nível do chão e aéreas passam em vários seeds.

Pixels azuis em uma imagem não provam rio. Sidecar inclui amostras de leito,
água, suporte, slope, discharge, mesh section e visibilidade.

## Props ancorados

Árvore, pedra, arbusto, construção e loot solto usam `GroundingResult`:

- ponto da superfície final;
- normal e inclinação;
- profundidade de raiz/base;
- footprint e clearance;
- tolerância por categoria;
- collider resultante;
- razão de rejeição.

O renderer não corrige Y de um prop depois do spawn. Corrigir visualmente sem
corrigir simulação gera sombra/colisão/nav divergentes.

## Biomas que são lugares

Um biome precisa diferir em mais que cor ou flores:

- macro relevo e horizonte;
- solo e materiais;
- vegetação e densidade;
- água/clima/luz/som;
- recursos e perigos;
- arquitetura/facção provável;
- traversal e decisões de construção;
- uma oportunidade exclusiva e uma restrição legível.

Para cada família, medir distribuição em 64+ seeds, área contínua, borda,
repetição, spawn rate, slope, recursos e identidade visual. Biomes raros devem
ser encontráveis por mapa/pistas, não apenas por sorte.

## Bateria espacial

- propriedades em milhares de amostras sem carregar GPU;
- seeds fixos de regressão;
- seeds aleatórios registrados quando falham;
- mapa de relief/slope/water/support;
- percurso de player e nav;
- câmeras chão/alto em 8 direções;
- alterações e reload;
- streaming cruzando fronteiras;
- low/high quality;
- comparação determinística;
- soak de dias e edits.

Todo seed descoberto com rio voando, spawn enterrado ou prop flutuante vira
fixture permanente e minimizada.
