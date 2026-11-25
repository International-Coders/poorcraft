# 14 — Plano executável N06–N11: a primeira hora

O próprio loop de sobreviver, construir e descobrir deve ensinar por ação,
feedback e consequência. Quem nunca leu os documentos precisa chegar à
primeira noite.

| Minuto | Descoberta | Ação | Estado persistido |
| --- | --- | --- | --- |
| 0–2 | olhar, mover, objetivo | encontra recurso | spawn/onboarding |
| 2–5 | coletar | galho, pedra, alimento | hotbar |
| 5–8 | fabricar | primeira ferramenta | receita/durabilidade |
| 8–12 | terreno | corta/cava/nivela | edit local |
| 12–18 | construir | bancada e abrigo | custo/estrutura |
| 18–25 | sobreviver | fogo/comida/cobertura | calor/fome |
| 25–35 | noite | fica ou explora | risco/recompensa |
| 35–45 | continuar | save/reload | jornada retomada |

Tempos são metas de observação, não cronômetros que retiram liberdade.

## N06 — Hotbar de itens possuídos

- Remover palette gratuita do survival.
- InventorySlotId estável alimenta hotbar, mão, crafting e save.
- Quantidade zero limpa seleção; quebra de ferramenta deixa mão vazia.
- Stack, durabilidade e seleção têm uma autoridade.
- Preview informa falta de material antes do clique.
- Testar scroll, teclado, full inventory, save/reload e rede.

Capturas: normal, item baixo, quebrado, mão vazia, bloqueado e tela pequena sem
colidir com vida/comida/objetivo.

## N07 — ActionResolver mouse-first

Resolvedor recebe contexto + bindings + alvo e retorna ação prioritária e
prompt. UI não escreve tecla fixa. Prioridade: modal → menu → interação focal →
ataque/coleta → construção → navegação. Esquerdo é primário, direito secundário,
E interage e Tab abre inventário.

Provar que remap muda ação e texto; um clique não dispara dois sistemas; prompt
some sem alvo; replay independe do frame rate.

## N08 — Recursos e primeira ferramenta — FECHADO (2026-11-24)

Spawn oferece galho, pedra e alimento por regras de habitat e reachability, sem
formação idêntica em toda seed. A ferramenta pronta precisa de ícone, nome,
custo, asset de mundo, viewmodel, som, durabilidade, ação e drop. Consumidor
ausente falha catálogo.

FECHAMENTO: seed 123 = primeira seed-showcase (rio esculpido no centro +
bolso de caverna no anel 3; sonda determinística [3,200)). route_first_tool
17 células VERDES: 4 colunas de grama distintas (lei estratificada) →
bancada → receita 6 wood_pick → equipa viewmodel (fix case-blind no gate
"PICK") → margem do rio (janela densa 24×24 m no edge mid) → E fisga →
X come (food 1.00). PNGs inspecionados. MUTAÇÃO PROVADA:
PC3D_MUTATE_EQUIP_OFF=1 → FAIL na célula equipped; controle PASS.
Suíte: render 273/0 + world 329/0 + save 104/0 + doctrine green.

## N09 — Bancada e ghost

evaluate_placement é a autoridade de preview, commit, servidor e replay. Retorna
posição ajustada, rotação, suporte, colisão, custo e rejeição localizada. Ghost
usa cor + forma + texto. Commit reavalia e paga só após sucesso.

Matriz: plano/encosta, snap, sobreposição, sem suporte/material, dentro do
player, alcance, rotação, multiplayer, save/reload e reembolso.

FECHADO (2026-11-25): evaluate_placement (app.rs, pura) cobre overlap do
corpo, ocupado, fundação e custo; commit (F/RMB) re-avalia na MESMA função
e paga só no verde. Ghost de 12 barras pelo pipeline de assets: verde
aceito / vermelho recusado com razão nomeada. route_build_ghost: 3 golpes
(A overlap → RED HOLDS YOU sem pagar; B à frente → GREEN paga 1 soil e
constrói; C mesma mira → RED ALREADY THERE sem pagamento duplo) + pixel
law (delta 0.0146 na MESMA pose = a caixa repinta). PNGs 4/4 inspecionados.
MUTAÇÃO PROVADA: PC3D_MUTATE_PLACEMENT_OVERLAP_OFF=1 → beat A aceita e
paga (bloco dentro do corpo) → FAIL exato em a_red/a_toast/a_unpaid;
controle PASS. Suíte: render lib 276/0 (3 p3dn09).

## N10 — Terreno previsível

FECHADO (2026-11-25). Lower, raise, level e smooth mostram preview do
mesmo cálculo do commit (`evaluate_sculpt` → `SculptVerdict` → ghost
ciano/laranja + `resolve_sculpt` → `surface_edit`). Propriedades provadas
na rota `route_sculpt`: fora do gate não muda bytes (bare hand → RED
"NEED A SHOVEL", chão parado); lower é EXATO (−1.0 m = DIG_DEPTH_M);
level converge aos pés do corpo e TERMINA (segundo U no chão arrived →
"THE GROUND IS ALREADY SHAPED"); smooth reduz variância (testes de
unidade p3dn10); a célula testemunha longe não muda bytes. Ferramenta
real: wood_shovel (item 13) craftado na bancada (receita 11 ungated
wood×2 + soil×1) e equipado (viewmodel Shovel). O remesh local é o
`surface_edit` do dig (mesma dirty region); água/nav/save herdam o
caminho NWR-005 já provado.

## N11 — Abrigo, fogo e noite

Abrigo deriva de cobertura, paredes, abertura e distância segura do fogo, não
de boolean marcado ao colocar peça. Fogo tem combustível, calor, luz, áudio,
fumaça, risco e estado salvo. A noite cria decisão sem matar iniciante sem
chance legível.

Rota capstone: save novo → N06–N10 por prompts → abrigo/fogo → primeira noite →
fechar em estado não trivial → recarregar inventário/edits/fogo/onboarding →
jogar mais cinco minutos.

## Playtest desconhecido

Três pessoas/sessões sem documentos. Registrar ações, hesitações, texto lido,
mortes, atalhos tentados e pedidos de ajuda, sem orientar. Meta inicial: 80%
faz primeira ferramenta e 70% chega a abrigo/fogo. Bloqueio comum é bug, não
“erro do usuário”. Telemetria permanece local/opt-in.
