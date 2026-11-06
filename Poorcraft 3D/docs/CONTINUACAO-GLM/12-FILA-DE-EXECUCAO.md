# 12 — Fila de execução forçada

## Regra de prioridade

P0 confiança da primeira sessão → P1 loop jogável → P2 identidade/variedade →
P3 escala social/reinos → P4 distribuição. Um P0 aberto bloqueia volume novo.

## Épico A — confiança espacial e câmera

### N01 — Harness adversarial de confiança — FECHADO (loop 475, 2026-10-10)

Criar fixtures deliberadamente ruins para provar que os gates detectam:
viewmodel invertido/clipado, spawn dentro de árvore, prop 0.5 m no ar e água
sem suporte. O job fecha apenas quando cada mutação falha pelo motivo correto e
a cena válida passa.

Fechado: `pc3d_world::trust` (tipos TrustFixture/TrustViolation/TrustVerdict,
quatro gates puros, builders, meta de gate load-bearing, 14 testes) +
`--trust-audit` / `make p3d-trust-audit` (relatório + sidecar + capturas
janela do fixture de spawn: culpado sob o pinheiro vs controle no terraço
aberto; os outros três culpados são recusados antes da malha e o sidecar
registra indisponibilidade honesta de captura até N03/N05). Fechamento: 4/4
culpados recusados, 4/4 controles aceitos, 4/4 gates load-bearing, pixels de
acordo com o sidecar.

### N02 — Safe spawn multi-seed — FECHADO (loop 477, 2026-11-06)

Implementar busca validada, sidecar de candidatos e 256 seeds no teste diário.
Rota mostra visão inicial e primeiro recurso.

Fechado: `pc3d_world::spawn::decide_spawn` (busca em anéis concêntricos até
384 slots, prefilter de bioma, leis de suporte/margem, trust gate 8 direções,
recurso inicial confirmado por A* com cache, survivors pontuados, fallback
tier-2 honesto com `used_fallback`) + `spawn_matrix_ok` (prova property) +
`--spawn-matrix` / `make p3d-spawn-matrix` (32 seeds headless, sidecars
`decision_seed{N}.json`) + `--spawn-route` / `make p3d-spawn-route` (rota
visual com captura dupla por seed: olho no yaw do gate + ombro olhando o
recurso). Ambos integrados ao `p3d-beta`. Fechamento: 256-seed probe GREEN
(158 tier-1, 158/158 gate-clean no próprio yaw, 158/158 nav-confirmado,
6 biomas variados), suite pc3d_world 313 testes verdes, rota 4-stop
verificada por pixel (duas perspectivas distintas por seed).

### N03 — RiverCarve e suporte de água (FECHADO, loop 478)

Unificar leito/superfície/visibilidade e transformar os seeds ruins conhecidos
em fixtures. Seis seeds × dez câmeras, incluindo altura.

Fechamento (parcial honesto): leito/superfície/água UNIFICADOS na autoridade
voxel (`pc3d_world::river_carve` → `WorldGen::carve`; `final_solid` e
`regenerate_patch` compartilham a mesma função); lei de materialização
(profundidade ≥ 1,2 m; rampa ≤ 3,0 m) com gate `p3dn03_every_channel_
holds_voxel_water` (5 seeds × 40 edges); lei de aresta única (dois extremos
≥ threshold) no carve, nas faixas e nos testes; WATERSHED_HALF=70 única
constante exportada; faixas amostram a superfície esculpida; superfície
luta pula facetas do canal; `--water-route`/`make p3d-water-route`
(6 seeds × 2 capturas, gates voxel-truth + pixel, sidecars verdict PASS)
no p3d-beta; suíte 740/0; probe RED arquivado (shots/n03-red). Deferrals
registrados no STATE: trust bed gate lê a superfície analítica (mover ao
leito esculpido) e 2/10 câmeras por seed (depth/normals/water-id/wireframe).

### N04 — GroundingResult universal — FECHADO (loop 479, 2026-11-06)

Árvores, pedras, estruturas e loot usam uma autoridade; remover correções Y no
renderer. Captura collision/normal/footprint.

Fechado: `pc3d_world::grounding` (GroundingResult: surface_y_mm = topo da
célula sólida mais alta, normal/slope por diferenças centrais 1 m, material,
clearance de cápsula até 16 m, grounded, channel_hit; GroundQuery Point /
Footprint {half_w,half_h} responde a coluna mais baixa / Capsule {r,h};
`foundation_ok` exige coluna sólida, seca, fora de canal em toda a área;
FORBIDDEN_OFFSETS registra os 3 offsets históricos). Consumidores migrados:
NPC (`npc_world_pos` via Capsule — o +1.0 m apagado; âncoras de frame via
Point), flora (`table_pick` recusa planta em coluna afogada; instanciação
GPU e grama via Point), city (`surface_base` = mínimo do footprint; portos
de nav via Point com convenção y-sólida documentada), settlement (water wheel
na superfície d'água esculpida, dock via Footprint; âncoras via Point),
slice (spawn e props de praça via Point), water (`strip_for`, detector de
cruzamento e `water_y` de prova leem `river_channel().surface_m` dentro do
canal — a faixa ficava na margem analítica +0,35 m, metros acima do leito),
`renderer.ground_y_at` fallback via Point (streamed deltas compõem antes, lei
das camadas). Gate estático `tests/n04_forbidden_offsets.rs` varre as fontes
de produção do pc3d_render e falha se um offset Y-particular sobrevive em
componente de grounding — provado por mutação (injetado `+1.0` → gate RED,
restaurado → GREEN). Fechamento: pc3d_world 324/0, pc3d_render 259/0 +
gate 1/1, suite completa p3d-test verde; `--play-npcs` (torso ocupado,
sky-delta 0.49–0.75) e `--water-route 4` (8 capturas, sidecars verdict PASS:
voxel truth bed_solid + water_cells ≥ 2, água visível em overview e margem)
regenerados pós-fix e inspecionados; testes p3dn04_* (8) cobrem plano,
encosta, canal, footprint, fundação e cápsula. Deferral honesto: campo
`confidence`/`version` do card não tem consumidor jogável ainda — entra
quando o primeiro consumidor o exigir (lei do card: nada entra sem
consumidor).

### N05 — Viewmodel foundation — FECHADO (loop 480, 2026-11-06)

Rig de braços, `ViewmodelSpec`, pass e matriz FOV. Primeira pá/axe passa mão,
grip, sweep, retículo e parede.

Fechado: `pc3d_render::viewmodel` (spec completa, projeção derivada do FOV
com depth policy documentada, matriz 60/75/90 × 16:9/16:10/21:9 × mãos ×
idle/equip/ataque/parede; pose RESOLVIDA contra a célula mais apertada após
RED honesto), pass separado `pc3d.pass.viewmodel` com projeção própria e sol
compartilhado em espaço de câmera, gate de recusa no equip (spec ruim é
recusada pelo nome e nada monta), unequip sem resíduo, sweep com hit window
compartilhada, wall probe consultando O MUNDO (nunca depth pós-processado).
Primeira ferramenta completa: a pá em espaço de câmera. Provas: 8 testes
puros + 5 testes GPU por referência-dif + `route_viewmodel` PASS (8611 px
ferramenta, 8477 px sweep, 6216 px parede, matriz limpa). A segunda
ferramenta (machado) entra com seu consumidor de gameplay.

## Épico B — primeira sessão

### N06 — Hotbar de itens possuídos

Eliminar palette grátis e ligar quantidade/durabilidade/seleção.

### N07 — ActionResolver mouse-first

Left/right/E/Tab substituem letras de sistema, com prompts do binding map.

### N08 — Recursos soltos e primeira ferramenta

Spawn encontra branch/stone/food; receita explica propósito; viewmodel e som.

### N09 — Bancada e ghost de construção

Preview e commit compartilham `evaluate_placement`; custo/suporte/colisão.

### N10 — Shovel/hoe e terreno previsível

Lower/raise/level/smooth com preview pós-mesh e remesh local.

### N11 — Abrigo, fogo e primeira noite

Jornada completa em 20 minutos, save/reload e playtest desconhecido.

## Épico C — identidade do mundo

### N12 — Canon original dos seis reinos

Nomes, valores, contradições, materiais, música, profissões e revisão legal.

### N13 — Kit visual base por reino

Um módulo perfeito por kit antes do lote; depois capital completa.

### N14 — NPCs com corpo confiável

Rig, mãos/pés, animações, workspots e três profissões úteis.

### N15 — Biome identity pass

Relevo, traversal, recursos, clima, som e oportunidade exclusiva por família.

### N16 — Consequência testemunhada

Uma ação do player muda NPC, assentamento e journal de forma persistente.

## Épico D — áudio

### N17 — Music event model e MIDI debug

16 vozes determinísticas, dois estados e testes de evento.

### N18 — Scheduler 60 s ahead

Worker, crossfade, fallback e benchmark sem underrun.

### N19 — Paletas de reino

Motivos originais, construção/exploração, noite/caverna/clima.

## Épico E — QA e ferramentas

### N20 — glTF Validator no asset gate

Pin, cache local e relatório por arquivo.

### N21 — Fuzz de formato

Save, manifesto, comandos e protocolo; corpus e minimização.

### N22 — Mutation gate

Spawn, grounding, água, placement, inventário e save com kill-rate report.

### N23 — Observatory v2

Color/depth/normals/ids/anchors/collision + semantic sidecar por rota.

### N24 — Matriz CI três sistemas

Test/build/package por OS, artifacts e smoke instalado.

## Épico F — alfa e Steam

### N25 — Packages reproduzíveis

Windows/Linux/macOS com build id, licença e manifestos.

### N26 — SteamPipe templates

Depots, branches e preview sem segredos/AppID fictício.

### N27 — Steam Playtest readiness

Primeira hora, crash/soak, feedback e rollback. Early Access permanece bloqueado
até o gate comercial do documento 10.

## Como escolher

O próximo job é sempre o primeiro N-item não fechado cujos pré-requisitos estão
verdes. Na data desta doutrina, o próximo é **N01**. Não iniciar N12–N27 para
evitar um defeito em N01–N11.

Cada fechamento atualiza este arquivo, `doctrine.json`, o estado vivo e o
devlog. Se uma descoberta mudar a ordem, registrar a razão e preservar o item.
