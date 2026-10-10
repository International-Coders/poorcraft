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

## N08 — Recursos e primeira ferramenta

Spawn oferece galho, pedra e alimento por regras de habitat e reachability, sem
formação idêntica em toda seed. A ferramenta pronta precisa de ícone, nome,
custo, asset de mundo, viewmodel, som, durabilidade, ação e drop. Consumidor
ausente falha catálogo.

Rota: spawn → olhar → coletar → hotbar → receita → craft → equipar → usar →
consequência. Capturas e state hash acompanham a rota.

## N09 — Bancada e ghost

evaluate_placement é a autoridade de preview, commit, servidor e replay. Retorna
posição ajustada, rotação, suporte, colisão, custo e rejeição localizada. Ghost
usa cor + forma + texto. Commit reavalia e paga só após sucesso.

Matriz: plano/encosta, snap, sobreposição, sem suporte/material, dentro do
player, alcance, rotação, multiplayer, save/reload e reembolso.

## N10 — Terreno previsível

Lower, raise, level e smooth mostram preview do mesmo cálculo do commit.
Propriedades: fora do raio não muda bytes; smooth reduz variação; level converge;
custo corresponde ao volume; remesh, água, nav, grounding e save recebem a
mesma dirty region; replay restaura hash.

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
