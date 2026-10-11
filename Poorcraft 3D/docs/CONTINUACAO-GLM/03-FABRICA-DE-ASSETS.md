# 03 — Fábrica de assets jogáveis

## Princípio

Um asset não é um arquivo `.glb`. É um pacote de geometria, material,
significado, contatos, colisão, animação, consumidor, desempenho, licença e
provas. A IA pode gerar uma base; a fábrica transforma essa base em algo que o
jogo consegue usar sem braços voando, caixas ruins ou proporções incoerentes.

## AssetSpec, a fonte antes da malha

Todo asset começa em JSON com:

- `id`, família, variante, estágio e proprietário;
- função jogável e verbos suportados;
- escala real em metros e envelope de bounds;
- `up_axis=Y`, forward, origem e unidades;
- materiais e paleta permitidos;
- triângulos/vértices/texturas por LOD;
- colliders e regra de navegação;
- sockets nomeados, transforms e tolerâncias;
- partes móveis, pivôs, rig e clips;
- estados visuais e transições;
- requisitos de world model e viewmodel;
- consumidores em código;
- cenas de prova, câmeras e interações;
- proveniência, licença, hashes e revisão de originalidade.

O gerador lê essa especificação. Nunca infere silenciosamente metros a partir
de pixels ou corrige eixo de cada arquivo com um transform escondido no jogo.

## Pipeline de 12 portas

1. **Brief:** função, silhueta, escala e linguagem de facção aprovadas.
2. **Referência original:** moodboard de materiais/formas, sem copiar asset.
3. **Base:** assetgen determinístico, Blender ou modelo generativo opcional.
4. **Topologia:** manifold, normais, winding, degenerados e ilhas desconectadas.
5. **Métrica:** bounds e pivô dentro do contrato; pés/base em Y=0 quando cabível.
6. **Material:** PBR coerente, UVs, texel density, alpha e compressão.
7. **Semântica:** sockets, hinges, workspots, entradas/saídas, grip e muzzle.
8. **Física:** collider simples, suporte, mass class e navegação.
9. **Variações:** somente após o exemplar base passar as portas 1–8.
10. **Integração:** manifesto, carregador e consumidor real.
11. **Prova:** beauty, wireframe, normals, anchors, collision e rota jogável.
12. **Promoção:** protótipo → beta só após comparação e inspeção humana.

## Gates geométricos automáticos

- glTF 2.0 válido pelo validador Khronos;
- ausência de NaN/infinito;
- índices dentro do buffer;
- winding e normais consistentes;
- nenhuma ilha solta fora da allowlist;
- peças articuladas conectadas ao rig esperado;
- base toca o plano, salvo assets explicitamente suspensos;
- bounds dentro de 5% do AssetSpec;
- pivô dentro do volume útil ou no socket previsto;
- triângulos, draw calls, materiais e texturas dentro do orçamento;
- LODs preservam silhueta e sockets;
- nomes e extensões seguem taxonomia;
- hash e proveniência registrados.

## Gates semânticos por categoria

### Ferramenta

Grip socket, mão dominante, segunda mão opcional, direção de golpe, cabeça da
ferramenta, sweep volume, colisão de mundo, viewmodel separado e animações de
idle/use/equip. Uma pá cuja lâmina aponta para o player falha.

### Baú

Base apoiada, hinge coerente, tampa sem interpenetração, volume interno,
collider fechado/aberto, ponto de interação, áudio, inventário persistente e
oclusão. Uma caixa decorativa sem abrir não pode ser catalogada como baú.

### Humanoide

Hierarquia de ossos, pesos normalizados, pés no chão, mãos nos sockets, limites
de torção e testes de todas as animações. Partes sem peso ou distantes do corpo
falham antes do import.

### Construção

Footprint, pontos de snap, suporte, socket de porta/janela/telhado, collider e
navmesh. Beauty shot sem montagem real não prova módulo.

### Prop natural

Regra de ancoragem ao terreno, inclinação máxima, offset de raiz, footprint,
oclusão de spawn e biomas permitidos. Pedras e árvores não podem flutuar.

## Capturas obrigatórias

Para cada exemplar promovido:

- três vistas ortográficas com escala;
- beauty 3/4;
- wireframe;
- normais/tangentes;
- UV/material IDs;
- anchors e pivôs;
- colliders;
- LOD0/1/2 na distância de troca;
- asset no mundo ao lado do player;
- interação antes/durante/depois;
- viewmodel em três FOVs se equipado.

Cada PNG recebe sidecar com asset id, hash, pose, câmera, FOV, resolução,
quality tier, draw calls, triângulos, material, bounds e veredito.

## Uso de geração neural

Hunyuan3D/TRELLIS ou serviço semelhante pode fornecer concept mesh. A saída
fica em `generated-candidate`, nunca em `compiled` diretamente. Ela deve ser
retopologizada, escalada, orientada, materializada, licenciada, comparada e
passar todas as portas. Se o host não tiver VRAM ou licença adequada, o job usa
assetgen/Blender procedural; qualidade vem do gate, não do modelo escolhido.

## A regra contra volume falso

Não produzir lotes antes de fechar o primeiro exemplar. Sequência correta:

`1 conceito → 1 asset integrado → 1 rota verde → 3 variantes → família`.

Qualquer lote com taxa de rejeição acima de 20% pausa e corrige gerador/spec.
