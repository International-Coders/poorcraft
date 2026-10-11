# 13 — Plano executável N01–N05: confiança espacial

Este plano transforma os cinco primeiros cards em trabalho que uma sessão GLM
5.3 consegue executar sem “resolver” o defeito ajustando apenas a screenshot.
Cada card segue: reproduzir → mutar → provar falha → implementar → provar o
caminho real → inspecionar.

## Resultado do épico

Ao terminar N05, o player nasce em lugar seguro e legível, rios pertencem ao
terreno, qualquer objeto conhece seu contato com o solo e uma ferramenta em
primeira pessoa parece segurada e usada por um corpo. O renderer não pode
esconder erros de worldgen.

## N01 — Harness adversarial de confiança

### Objetivo

Criar infraestrutura de “cenas culpadas”. Ela demonstra que gates ficam
vermelhos pelas razões certas. O resultado não é uma imagem bonita: é a certeza
de que quatro classes de regressão não atravessam a bateria.

| Fixture | Mutação deliberada | Gate que falha | Controle válido |
| --- | --- | --- | --- |
| bad_viewmodel_flip | escala X negativa ou grip atrás da mão | hemisfério/grip/winding | ferramenta na mão dominante |
| bad_spawn_tree | cápsula intersecta tronco e copa | clearance + visão frontal | mesma seed, candidato vizinho |
| bad_prop_float | contato 0,50 m acima do solo | grounding/contact delta | prop assentado pela autoridade |
| bad_water_air | seção d’água sem leito amostrado | suporte do leito | seção sobre canal |

Cada fixture contém seed, versão do gerador, pose, parâmetros da mutação,
resultado semântico, ROI e erro esperado. A mensagem nomeia sujeito, medição,
tolerância e seed.

### Implementação

1. Criar em pc3d_world tipos puros TrustFixture, TrustViolation e TrustVerdict.
2. Adicionar builders das quatro fixtures. A mutação vive apenas em teste ou
   no comando explícito de auditoria, nunca no runtime normal.
3. Criar comando --trust-audit que renderiza culpado e controle na mesma câmera.
4. Gravar sidecar com estado, anchors, contato, profundidade e water samples.
5. Adicionar make p3d-trust-audit; falhar se culpado passar ou controle falhar.

### Provas negativas

- Desabilitar cada gate deve produzir mutante sobrevivente e falhar o meta-teste.
- Trocar a câmera não converte fixture culpada em válida.
- Ocultar mesh no renderer não altera veredito semântico.
- Dado não finito é falha própria; não pode ser convertido silenciosamente.

N01 fecha com 4/4 culpados recusados, 4/4 controles aceitos, oito capturas
inspecionadas e sidecars concordando com pixels.

## N02 — Safe spawn multi-seed

A autoridade atual é pc3d_world/src/player.rs::spawn_safe. Evoluir esse caminho;
não criar spawn alternativo para o app.

SpawnCandidate contém posição dos pés, normal, inclinação, clearance da cápsula,
distância de água/queda, oclusão frontal, recurso próximo, caminho até o
recurso, biome e razões de rejeição. Score só ordena candidatos já válidos.

1. Enumerar anéis deterministicamente.
2. Pré-filtrar oceano, inclinação, queda e altura.
3. Consultar superfície final, incluindo edits.
4. Testar cápsula inteira e margem de saída.
5. Raycastear cone frontal; tronco, rocha ou parede próxima reprovam.
6. Confirmar recurso por caminho navegável, não distância euclidiana.
7. Desempatar por coordenada estável.
8. Persistir escolha e versão do algoritmo.

Matriz: PR 32 seeds × 4 direções; nightly 256 seeds; pré-release 2.048 seeds
headless. Rota visual com 16 spawns entre costa, floresta, montanha e fronteira.
Zero cápsulas bloqueadas e zero água/quedas imediatas.

## N03 — RiverCarve e suporte de água

Cadeia única: Watershed → RiverPath → RiverBed → WaterSurface →
render/collision/nav. Leito e superfície carregam o mesmo river_id e versão.

1. Persistir uma seed do bug de água suspensa e fazê-la falhar em N01.
2. Gerar perfil longitudinal monotônico, com lago explicitamente classificado.
3. Escavar leito por discharge e misturar margens numa faixa limitada.
4. Criar superfície somente onde amostras inferiores confirmam leito.
5. Atualizar leito, água, collider e nav após edit/dique.
6. Provar save/reload e dirty rebuild local.

Leis: profundidade dentro do intervalo; segmentos comuns não sobem a jusante;
margens conectadas; nenhum triângulo fora do footprint; rebuild limitado.
Prova: seis seeds × dez câmeras, com color, depth, normals, water id e wireframe.

## N04 — GroundingResult universal

Criar consulta pura GroundQuery → GroundingResult com surface_point, normal,
slope, material, footprint, penetration, clearance, confidence e versão.
Árvore, pedra, loot, prédio, workspot, NPC e decoração consomem essa resposta.
É proibido somar offsets Y particulares no renderer.

1. Inventariar surface_height, ground e offsets de placement.
2. Classificar consumidor: point, footprint, capsule ou foundation.
3. Testar plano, encosta, margem e edit.
4. Migrar um consumidor por vez e apagar autoridade antiga.
5. Capturar footprint, normal, contato e collider.
6. Busca estática bloqueia offsets proibidos conhecidos.

Props rígidos: contato visível ≤ 2 cm. Fundação usa toda a área, não o pivot.
NPC usa cápsula. Tolerância nunca é escolhida olhando uma única screenshot.

## N05 — Viewmodel foundation

O item de mundo descreve volume real, drop e colisão. O viewmodel descreve
braços, mãos, ferramenta, grip e animação em espaço de câmera. Compartilham
identidade/material, nunca transform final ou LOD.

ViewmodelSpec contém mão dominante, grips, idle, envelope de tela, near
clearance, exclusão do retículo, FOV de referência, sweep, hit window,
equip/unequip e reação a parede/teto.

- pass separado, projeção derivada do FOV e depth policy documentada;
- iluminação coerente e motion reduzível;
- wall pushback consulta o mundo, não depth pós-processado;
- primeira pá/machado completa antes de cinco placeholders;
- gameplay e animação compartilham a hit window.

Matriz: FOV 60/75/90 × 16:9/16:10/21:9 × esquerda/direita ×
idle/equip/ataque/parede. Capturas incluem máscara, retículo, hand sockets e
envelope. Nada toca near plane, aparece invertido, cobre retículo indevidamente
ou continua na tela após desequipar.

## Dependências

N01 abre N02, N03 e N04. N05 só fecha quando N01 e N04 estiverem verdes. Cada
card gera checkpoint próprio; não existe reescrita de cinco cards sem revisão
jogável.
