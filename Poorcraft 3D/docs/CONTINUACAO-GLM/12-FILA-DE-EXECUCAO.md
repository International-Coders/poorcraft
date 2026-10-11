# 12 — Fila de execução forçada

## Regra de prioridade

P0 confiança da primeira sessão → P1 loop jogável → P2 identidade/variedade →
P3 escala social/reinos → P4 distribuição. Um P0 aberto bloqueia volume novo.

## Épico A — confiança espacial e câmera

### N01 — Harness adversarial de confiança

Criar fixtures deliberadamente ruins para provar que os gates detectam:
viewmodel invertido/clipado, spawn dentro de árvore, prop 0.5 m no ar e água
sem suporte. O job fecha apenas quando cada mutação falha pelo motivo correto e
a cena válida passa.

### N02 — Safe spawn multi-seed

Implementar busca validada, sidecar de candidatos e 256 seeds no teste diário.
Rota mostra visão inicial e primeiro recurso.

### N03 — RiverCarve e suporte de água

Unificar leito/superfície/visibilidade e transformar os seeds ruins conhecidos
em fixtures. Seis seeds × dez câmeras, incluindo altura.

### N04 — GroundingResult universal

Árvores, pedras, estruturas e loot usam uma autoridade; remover correções Y no
renderer. Captura collision/normal/footprint.

### N05 — Viewmodel foundation

Rig de braços, `ViewmodelSpec`, pass e matriz FOV. Primeira pá/axe passa mão,
grip, sweep, retículo e parede.

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
