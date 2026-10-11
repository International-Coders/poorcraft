# 09 — Áudio, tema sonoro e música procedural

## Camadas

1. SFX funcional: passos, ferramenta, impacto, UI, fogo, máquina.
2. Ambiente espacial: vento, água, fauna, clima, cidade, caverna.
3. Música procedural opcional: acompanha exploração/construção sem fingir ser
   a soundtrack autoral final.
4. Soundtrack original: composições aprovadas, separada do sistema gerativo.

## Gerador com um minuto de antecedência

Um `MusicDirector` determinístico mantém uma janela de 60–90 segundos:

- lê seed do mundo, região, reino, hora, clima, abrigo, combate e atividade;
- escolhe estado musical com histerese para evitar mudanças nervosas;
- gera eventos em compassos futuros, nunca áudio no último instante;
- agenda 16 vozes por padrão, até 32 no tier alto;
- renderiza em worker thread para buffer PCM ou envia eventos ao synth interno;
- cruza blocos com crossfade e preserva fase/tempo;
- se atrasar, repete cama segura; nunca trava o frame.

O player pode desligar música procedural, manter ambiente/SFX e futuramente
preferir a soundtrack original.

## Linguagem musical

- modos, escalas e intervalos próprios por reino;
- instrumentação abstrata/original por síntese e samples licenciados;
- densidade baixa na construção;
- tensão cresce com ameaça testemunhada, não por estado secreto distante;
- noite, caverna e ruína alteram espaço/timbre antes de alterar melodia;
- motivos podem retornar após eventos importantes;
- silêncio é um estado planejado.

## Arquitetura segura

O protótipo preferido amplia `pc3d_audio` e seu mixer determinístico, evitando
dependência nativa nova. MIDI pode ser formato intermediário/export de debug.
Se FluidSynth for avaliado, deve ser opcional: é multiplataforma, mas adiciona
biblioteca nativa e requer SoundFont com licença registrada. Nunca baixar uma
SoundFont em runtime.

Um script Python offline pode gerar `.mid` e render de referência, mas o jogo
não deve exigir Python instalado. O runtime final usa Rust/áudio empacotado.

## Contrato de evento

Cada bloco gerado registra:

- seed e versão do compositor;
- estado musical de entrada/saída;
- tempo, métrica, tonalidade/modo;
- tracks/vozes, patches e automação;
- eventos fonte do mundo;
- hash do MIDI/event stream;
- hash/estatísticas do PCM quando renderizado;
- tempo de geração e underruns.

## Gates

- mesma entrada produz evento byte-idêntico;
- vozes simultâneas ≤ orçamento;
- notas dentro de range e duração;
- nenhuma NaN, clipping ou DC excessivo;
- transição sem click e diferença limitada de loudness;
- thread de áudio sem alocação em steady state;
- gerador permanece 60 s à frente sob benchmark;
- fallback funciona se worker falhar;
- settings de volume/mute persistem e dirigem mixer;
- Windows/Linux/macOS abrem dispositivo ou entram em silêncio gracioso;
- licença de todos os samples/SoundFonts é verificável;
- teste humano detecta repetição e fadiga em sessão longa.

## Fases de implementação

1. Event model e export MIDI determinístico.
2. Scheduler de 16 vozes + synth interno simples.
3. Janela ahead, worker e fallback.
4. Dois estados: construção e exploração.
5. Realm palettes e transições.
6. Clima/noite/caverna e tensão.
7. Settings e mix espacial.
8. Soak de duas horas e revisão humana.

Não gerar “música infinita” antes de dois minutos bons e não cansativos.
