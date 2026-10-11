# 15 — Plano executável N12–N19: identidade viva e som

Este épico converte referências em qualidades originais. Valheim inspira
exploração/construção legíveis; Skyrim, descoberta e liberdade; Heroes III,
contraste ideológico. Nenhum nome, melodia, personagem, mapa, brasão ou asset é
copiado.

## N12 — Bíblia dos seis reinos

Cada reino recebe tese, virtude, contradição, relação com vida/morte/ordem/caos,
clima, relevo, materiais, silhueta, construção, trabalho, guerra, funeral,
profissões, criaturas, tecnologia/magia, bens, motivo musical abstrato, cinco
conflitos internos e cinco relações externas.

Nenhuma facção pode ser apenas boa, má, ordem, caos, viva ou morta. Toda tese
tem preço visível e algo que o player pode admirar, rejeitar ou transformar.

## N13 — Kit visual por reino

Fazer módulo exemplar por vez: portão, parede, torre, casa, oficina e marcador.
O primeiro portão passa 16 gates antes do lote. Gramática paramétrica define
proporções, materiais, sockets, dano e LOD, sem autorizar caixas genéricas.
Provar silhueta a 200 m, leitura a 30 m e detalhe a 2 m. A rota inclui capital,
colisão, porta e NPC.

## N14 — NPCs com corpo

Rig canônico, variação controlada e roupas em camadas. Mãos usam sockets, pés
usam IK/grounding, cabeça tem limites e root speed é conhecido. Braço voando,
peso órfão, T-pose, foot-slide e ferramenta atravessando corpo são falhas.

Começar com coletor, construtor e guarda. Cada um trabalha em workspot, reage ao
player e deixa consequência. Crowd só entra depois que um indivíduo passa.

## N15 — Identidade de biome

Cada família difere em relevo, travessia, recursos, risco, clima/som e
oportunidade exclusiva. Cor de grama não basta. Medir distribuição, tamanho de
manchas, fronteira, distância do spawn, tempo de encontro e vizinhos. Gerar
atlas semântico e jornada a pé; seeds ruins viram fixtures.

## N16 — Consequência testemunhada

A cadeia mínima é: ação do player → testemunha → opinião/necessidade → ação do
NPC → mudança no assentamento → journal → save/reload. Exemplo original: o
player restaura ou desvia água de oficina; rotina, produção, fala e reação
externa mudam. O sistema registra fatos, não apenas cutscene.

## N17 — Eventos musicais

Representação independente do synth: tempo, compasso, modo, 16 vozes expansíveis
a 32, note on/off, velocity, pan, expressão, camadas e seed musical. Estados:
exploração, construção, noite, caverna, perigo e reino.

Testar ordem, faixa de notas, note-off completo, determinismo, polifonia e
ausência de loop curto. MIDI é modo de debug/export; runtime pode sintetizar
eventos diretamente.

## N18 — Scheduler 60–90 s à frente

Worker agenda ao menos 60 s em blocos; callback não aloca. Mudança urgente usa
ponte/crossfade. Se worker atrasar, mantém textura segura; se falhar, toca
ambiente; se dispositivo sumir, jogo continua; se fila encher, descarta futuro
distante.

Medir p50/p95/p99 do gerador, zero underruns em soak, memória, responsividade e
saída determinística gravável.

## N19 — Paletas dos reinos

Cada reino recebe instrumentação abstrata original, densidade, registro, ritmo
e espaço. SoundFont/plugin só entra com licença, hash e pacote nos três sistemas.
O sistema respeita silêncio: construir não significa música constante.

Revisar 20 min sem repetição óbvia, exploração→construção, dia→noite, duas
fronteiras, combate→calma, mix sob efeitos/voz e opção desligada. Promoção exige
WAV/MIDI de seeds fixas, eventos e audição humana.
