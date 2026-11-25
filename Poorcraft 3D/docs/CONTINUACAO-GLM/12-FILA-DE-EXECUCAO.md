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

**Wired (loop 481, 2026-11-23):** a segunda ferramenta entrou com consumidor
real — a **picareta** (`ToolKind::Pick`, spec própria + matrix 18 células
`p3dn05_pick_matrix`, spike no mesh factory) e o **consumo do hit window**:
o verbo de cavar (G) com ferramenta equipada arma `dig_hit_pending` e
resolve exatamente uma vez, no frame em que o sweep entra na janela da
spec equipada (mão vazia resolve imediato; janela perdida solta o flag —
sem hit preso). Hotbar de itens possuídos equipa/desequipa o viewmodel
pela seleção (slots 6/7/8 com picareta possuída). Prova fechada:
`route_dig_window` (4 capturas, 9 condições: recusa bare-hand na rocha,
equip pela via real do baixo, resolve dentro da janela t=0.593, crédito
STONE no pack, queda do chão 31.68→30.68, tool-in-frame + sweep-pixels);
mutação `PC3D_MUTATE_HIT_WINDOW_EARLY=1` (janela 0.0) → route FAIL
(resolved false, dropped false), restaurada → PASS. Suíte 802/0
(render 274, world 430).

## Épico B — primeira sessão

### N06 — Hotbar de itens possuídos — FECHADO (loop 482)

Eliminar palette grátis e ligar quantidade/durabilidade/seleção.

FECHADO: o pack já era a verdade dos slots (D2: builds com contagem,
slots 6–9 com o que se carrega); o resto da lei caiu neste loop. A
DURABILIDADE HONESTA: `ToolWear` (pc3d_world::items) é a autoridade de
desgaste — usos restantes por código de ferramenta (wood 12, stone 30,
iron 90), `wear_use` desgasta e QUEBRA na última usação (true → o verbo
remove do pack, limpa a seleção, desequipa o viewmodel, toast honesto
"YOUR WOOD_PICK BROKE"); os dois sítios de uso do picado (resolve do dig
em rocha, golpe do minério) passam pela MESMA lei; display: o count do
slot é USOS RESTANTES. A SELEÇÃO POSSUÍDA: dígito recusa slot vazio,
roda PULA vazios, belt todo vazio não move; clique idem. O SAVE: trailer
`tool_wear` no SessionExtras (depois do byte career; save pré-N06 →
ferramentas novas, compatível nos dois sentidos). PROVA: route_hotbar_wear
(4 capturas inspecionadas: WOOD_PICK 9 → 2 → BROKE + slot vazio →
NEED A PICK FOR STONE com chão intacto; escada 11..0, stone creditado,
dígito 6 recusado pós-quebra); MUTAÇÃO PROVADA: PC3D_MUTATE_WEAR_SKIP=1 →
escada trava em 12, sem quebra, seleção fica 5, G pós-quebra escava
(18.68) → FAIL de 4 ângulos; default → PASS. Suíte 817/0 (render 270,
world 431, save 38, assets 38, core 32, audio 8); doctrine 2/2.

### N07 — ActionResolver mouse-first — FECHADO (loop 483)

Left/right/E/Tab substituem letras de sistema, com prompts do binding map.

FECHADO: o RESOLVER PURO `ui::on_mouse_verb` (ui.rs) decide por
prioridade modal → menu → painel de dados → DIÁLOGO focado → pointer
law → verbos (o diálogo NÃO é painel de dados: sai do muro do tier 2 e
cai no tier 3, onde Left o FECHA sem escavar atrás). A LEI DO POINTER:
ponteiro livre = clique é o LOOK request (CaptureMouse); verbos só com
grab. OS BINDINGS: LMB = DigAtCrosshair (o mesmo verbo do G: sweep do
viewmodel + janela compartilhada + wear), RMB = BuildSelected (re-fire
do F veterano: foundation gate, custo do pack, quest events — UMA lei
de construção, zero duplicação), Tab abre o pack (o reducer PERGUNTE
com TogglePack; o app aplica com as linhas do inventário vivo), E
inalterado. O PROMPT: MOUSE_FIRST_PROMPT ("LMB DIG · RMB BUILD · E USE
· TAB PACK · ESC PAUSE") é o prompt de spawn e a cauda contextual (E
TALK nome · V DELIVER BARS); ESC pausa. PROVA: route_mouse_first
(4 capturas inspecionadas por pixel: viewmodel 5111 px, mundo 2854 px,
pack panel 87843 px; 12 células: prompt LMB/RMB no spawn, pausa
libera o grab, clique livre pede capture, resume agarra, LMB escava
rocha ×3 com o chão caindo 37.67→28.68, Digit(4) seleciona ROCK, RMB
constrói pagando 1 stone (slot 3→2), Tab abre/fecha o pack, diálogo
aberto por E é fechado pelo LMB sem escavar); MUTAÇÃO PROVADA:
PC3D_MUTATE_MOUSE_BUILD_OFF=1 corta o braço Right do resolver →
FAIL exatamente na perna RMB (rmb_built false, slot 3 travado) com as
outras 11 pernas verdes; controle → PASS. Suíte: render lib 273/0.

### N08 — Recursos soltos e primeira ferramenta

Spawn encontra branch/stone/food; receita explica propósito; viewmodel e som.
FECHADO (2026-11-24): seed 123 é a PRIMEIRA seed-showcase (sonda
determinística em [3,200): rio esculpido ATRAVESSA o centro da cidade —
canal em (128,127), margem a 0 m do mid do edge; bolso de caverna no
anel 3). route_first_tool prova A LEI PRIMEIRA HORA em 17 células:
cavar 4 colunas de grama DISTINTAS (a 2ª cavada cai no solo — a lei
estratificada dá wood só na camada de grama) → bancada abre → receita 6
(wood_pick: 4 wood) fabrica → Digit(6) EQUIPA o viewmodel (FIX: o gate
lia "PICK" maiúsculo contra label "wood_pick" minúsculo —
to_uppercase) → margem do rio em alcance de cast (janela densa 24×24 m
no mid do edge, não anéis) → E fisga (caught ×1, onboarding avança) →
X come (food 0.50→1.00). PNGs inspecionados (grass_dug: skyline +
stock; pick_crafted: painel da bancada; fish_caught: água + CAUGHT
FISH; fish_eaten: ATE FISH + fome cheia). MUTAÇÃO PROVADA:
PC3D_MUTATE_EQUIP_OFF=1 corta o braço de equip do SelectHotbar →
FAIL exatamente na célula equipped; controle → PASS. Suíte: render
lib 273/0 + world 329/0 + save 104/0 + doctrine check.

### N09 — Bancada e ghost de construção

Preview e commit compartilham `evaluate_placement`; custo/suporte/colisão.
FECHADO (2026-11-25): a autoridade PURA `evaluate_placement` (app.rs)
decide overlap do corpo (cápsula 0.35 m × 1.8 m vs célula), ocupado
(mapa de construção do host — a MESMA verdade que a malha pinta),
fundação (check_foundation: declive + cantos ≤ 1.5 m) e custo (1 item do
pack por bloco); commit (KeyF/BuildSelected) RE-AVALIA na mesma função
e só paga com veredito verde. Ghost = prisma de 12 barras (0.07 m) pelo
pipeline de assets: VERDE aceito, VERMELHO recusado com razão nomeada;
`build_ghost_rejected()` expõe o veredito vivo, `construction_blocks()`
expõe a verdade de construção sincronizada. route_build_ghost prova 3
golpes em uma lei: escada de 3 grama (soil ×3) → A) mira nos pés →
RED "HOLDS YOU", soil 3→3 (não paga) → B) mira à frente (pitch −0.3
cruza a camada gy+1 a ~2.4 m; raio nivelado voa sobre o mundo — o ar
do alvo é (bx, gy+2, bz−2)) → GREEN, soil 3→2, 1 bloco construído →
C) mesma mira → RED "A BLOCK IS ALREADY THERE", blocos 1→1 (sem pagamento
duplo). Pixel law: ghost_accepted vs ghost_placed na MESMA pose — o
delta 0.0146 é a caixa verdict a repintar. PNGs inspecionados (4/4:
RED+toast HOLDS YOU; GREEN na faixa plana; bloco de soil físico no
mundo; RED+toast ALREADY THERE). MUTAÇÃO PROVADA:
PC3D_MUTATE_PLACEMENT_OVERLAP_OFF=1 corta o gate de overlap → beat A
ACEITA e PAGA (soil 3→2, 2 blocos, um dentro do corpo) → FAIL exato
nas células a_red/a_toast/a_unpaid; controle → PASS. Suíte: render lib
276/0 (3 testes p3dn09: overlap nomeia razão; ocupado+custo antes do
clique; commit≡preview).

### N10 — Shovel/hoe e terreno previsível

FECHADO (2026-11-25): a autoridade PURA `evaluate_sculpt` (app.rs) decide
lower/raise/level/smooth com `SculptVerdict` → a MESMA resposta pinta o
ghost de escultura (CIANO aceito, LARANJA recusado, via
`set_sculpt_ghost`) e comanda o commit (`resolve_sculpt` →
`surface_edit`, o MESMO caminho do dig). Ferramenta: `wood_shovel`
(item 13, Tier 1) + receita 11 UNGATED (wood×2 + soil×1) — a bancada
real a crafta; a frigideira de 5 slots carregados (pick×3, shovel,
bread) abriu o CINTO para 10 slots (HUD slots[10], Digit 0 → slot 10,
layout 10×SLOT + 9×GAP). Teclas: Z (lower −1.0 m), U (level → altura
dos pés do corpo, teto ±1.0 m por golpe). route_sculpt prova 5 leis em
um arco: escada de 3 grama → A) Z DESARMADO na célula → RED
"NEED A SHOVEL TO SHAPE THE GROUND", chão 10.696→10.696 (NÃO move) →
B) bancada: linha 11 AFFORDABLE, Enter crafta, o shovel cai no slot 9
carregado, Digit 9 equipa o viewmodel → C) Z ARMADO → "THE GROUND
SINKS", chão 10.696→9.696 (EXATO um dig depth), ghost CIANO dentro do
buraco real (paredes de soil escuras inspecionadas nos PNGs) → D) U ao
lado do buraco → "THE GROUND LEVELS", 9.696→10.696 (converge aos pés,
nunca teleporta) → E) célula TESTEMUNHA longe: 10.441→10.441 (esculpir
é LOCAL). Pixel law: sculpt_before vs sculpt_lowered na MESMA pose — o
delta 0.6159 é o buraco REAL enchendo o quadro de câmera vertical
(inspeção humana: pit walls + ghost cyan), limites 0.02–0.8 provam
mudança real sem troca de lixo; as leis semânticas fixam o metro exato.
PNGs inspecionados (5/5). MUTAÇÃO PROVADA:
PC3D_MUTATE_SCULPT_TOOL_OFF=1 corta o gate da ferramenta → o Z DESARMADO
UNDE o chão (9.696 vs 10.696, a_unmoved FALSE) e o beat C cava um
SEGUNDO metro (8.696) → FAIL exato nas células a_red/a_toast/a_unmoved/
lowered_exact/leveled_back; controle → PASS. Suíte: render lib 4 testes
p3dn10 (gate nomeia a ferramenta; lower/raise exatos + level converge e
TERMINA; smooth encolhe a variância e recusa chão já liso; o próprio
lever do gate). OBSERVE PASS (5 capturas, 207 frames, p50 2.66 s).

### N11 — Abrigo, fogo e primeira noite

Jornada completa em 20 minutos, save/reload e playtest desconhecido.

## Marco BETA — bateria completa VERDE (loop 487, 2026-11-25)

A bateria `make p3d-beta` (38 estágios) fecha ALL STAGES PASS com 442 PNGs
de prova em disco e `BETA-REPORT.txt` verde. As leis que fecharam a
bateria (cada uma com causa-raiz, não remendo de sintoma):

- **N09 overlap gate mutação provada**: `PC3D_MUTATE_PLACEMENT_OVERLAP_OFF=1`
  → beat A perde o RED ("HOLDS YOU") e o toast → FAIL exato em
  a_red/a_toast; controle PASS. O gate tem dentes.
- **A lei do solo do próprio NPC (app.rs spawn)**: o goblin nascia na
  ALTURA DO PLAYER a 16 células de distância — o solo da própria coluna
  do bicho pode ficar metros abaixo, e um corpo enterrado fica fora de
  alcance de melee para sempre (dy > MELEE_RANGE). O spawn agora responde
  `ground_y_at` da coluna de nascimento (o MESMO solo autoritativo que o
  player pisa). route_social PASS: slain=1, witnessed=3.
- **A ferida presa no quadro do impacto (`last_fall_wound`)**: a barra de
  vida sangra de volta (regen 0.02/s enquanto alimentado) enquanto a lista
  de capturas drena em tempo real — a prova lia a barra, não a lei. O
  latch grava (frame, dano) NO quadro do pouso; o veredicto lê a lei.
- **O poll de pouso em duas fases (pit_wall/plaza_recovery)**: poll com
  pré-condição AIRBORNE (1 m abaixo da borda) + streak de 2 polls no chão
  → o latch pega o quadro DEPOIS do impacto (o poll do quadro do pouso
  roda antes da física). FELL toast + ferida + captura latched no quadro
  do evento (CaptureAtNextFrame) — à prova de ritmo (o toast vive 3 s
  REAIS; um frame fixo o mata a ritmo contendido).
- **A caminhada do rim à prova de ritmo (route_dig)**: 18 frames de W era
  contagem de frames (7.2 m a 96 ms/frame) — agora o poll latcha o
  PRIMEIRO frame em pé fora da coluna cavada na resposta de solo viva;
  end_frame 280 assumiu a saída.
- **A altura do fio que fere (vine_hangoff)**: fio de pinheiro (1,5 m)
  + hop 1,04 m = pico 1,9 m = 6,1 m/s = SEGURO POR LEI — não havia ferida
  a nomear. A busca agora exige fio ≥ 2,5 m (dossel 2,9 m → pico 3,9 m →
  8,8 m/s → a ferida que a lei calcula, 100%→91%, lei-exata). A banda de
  hang aceita o vão legal inteiro [tip, top] (a janela "crossed" da lei
  clampar no tip — a rota registra onde a lei pôs o corpo).
- **Câmera do low-tier restaura a pose VISTA (wilderness)**: o retorno
  usava a pose UNDERGROWTH (mira no chão) — as sondas de céu batiam em
  grama. Restaurar a pose que PROVA céu.
- **A banda da ferida do playtest afrouxada para 0,55**: 0,45 apertada —
  o latch do quadro varia o impacto por um substep; a recuperação da
  praça (0,5 EXATO + toast YOU próprio) já está excluída pelo gate
  fall_toast (exige FELL).
- **deck-bench**: falha de contenção do HOST (cargo test rodando junto) —
  o guard de contenção funcionou; re-run em host quieto PASSA.

Suíte: workspace P3D 820/0; doctrine check 2/0; DMG
`dist3d/poorcraft3d-macos.dmg` reconstruída na árvore committed.


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
