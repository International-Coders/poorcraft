# 11 — Ferramentas pesquisadas e política de adoção

Pesquisa executada em 2026-10-09. Uma entrada aqui não significa instalação ou
aprovação. Versões mudam; antes de adotar, rever fonte oficial, licença e risco.

## Recomendadas para prova imediata

### Khronos glTF Validator

- Fonte: <https://github.com/KhronosGroup/glTF-Validator>
- Uso: validar `.gltf/.glb` contra a especificação antes do loader.
- Valor: detecta estrutura, referências e problemas de formato; não julga se
  a pá está bonita ou na mão correta.
- Adoção: alta prioridade, pin de versão e execução offline na fábrica.

### cargo-mutants

- Fonte: <https://github.com/sourcefrog/cargo-mutants>
- Uso: injetar defeitos e medir se os testes críticos realmente falham.
- Valor: melhora a efetividade, especialmente spawn, suporte, custos e saves.
- Adoção: noturna/incremental; primeiro tornar suítes determinísticas.

### cargo-fuzz / libFuzzer

- Fonte: <https://github.com/rust-fuzz/cargo-fuzz>
- Uso: fuzz de parsers de save, manifestos, protocolo e comandos.
- Limite: toolchain nightly/LLVM e melhor suporte Unix; CI separada.
- Adoção: corpus versionado, tempo limitado diário, casos minimizados.

### cargo-nextest

- Fonte: <https://github.com/nextest-rs/nextest>
- Uso: execução rápida, retries explicitamente visíveis, JUnit e perf de testes.
- Valor: reduz tempo sem alterar assertions.
- Adoção: auxiliar; `cargo test` continua gate de compatibilidade.

## Candidatas para arte 3D assistida

### Blender MCP (glonorce/Blender_mcp)

- Fonte: <https://github.com/glonorce/Blender_mcp>
- Relato do projeto: ferramentas para cena, BVH/assembly e testes.
- Valor: permitir à IA inspecionar bounds, peças desconectadas, sockets,
  wireframe e renders em Blender.
- Risco: execução `bpy` arbitrária e dependência de Blender aberto.
- Gate: sandbox local, allowlist, workspace temporário e comparação com CLI.

### Hunyuan3D-2/2.1

- Fonte: <https://github.com/Tencent-Hunyuan/Hunyuan3D-2>
- Valor: image-to-3D, malha e textura/PBR; API e add-on Blender disponíveis.
- Risco: pesos grandes, VRAM, licença dos pesos/saídas e topologia não pronta.
- Gate: opcional, offline, candidato somente; passa as 12 portas de assets.

### TRELLIS

- Fonte: <https://github.com/microsoft/TRELLIS> ou repositório oficial vigente
  confirmado na data de adoção.
- Valor: variações e image-to-3D.
- Gate: mesma política de candidato; não baixar automaticamente neste job.

### Studio Foundation / bforge

- Fonte: <https://github.com/lxsolutions/studio-foundation>
- Valor: ideias de Blender headless determinístico e assets com prova.
- Risco: licença source-available e stack maior; não copiar sem revisão.
- Adoção: estudar contratos/ideias, não incorporar código por padrão.

Ferramentas Godot MCP encontradas oferecem editor/playtest muito bons, mas este
jogo usa engine própria em Rust/wgpu. Migrar para Godot não é objetivo. O valor
é conceitual: input automatizado, screenshot, state query e undo são padrões a
replicar no observatory atual.

## GPU

- NVIDIA Nsight Graphics: <https://docs.nvidia.com/nsight-graphics/>
- AMD Radeon Developer Tool Suite: <https://gpuopen.com/rgp/>
- RenderDoc: <https://renderdoc.org/>

Usar quando disponível no hardware certo. Nenhuma captura de um vendor prova o
outro. Marcadores wgpu e cenas reproduzíveis vêm primeiro.

## MIDI e síntese

### FluidSynth

- Fonte: <https://github.com/FluidSynth/fluidsynth>
- Multiplataforma e baseado em SoundFont 2.
- Bom para protótipo/offline; integração nativa e SoundFont licenciado aumentam
  o pacote. O synth Rust já existente é a primeira opção no runtime.

## “Huflo”

Não foi encontrada uma ferramenta pública inequívoca com esse nome nas buscas
realizadas. Pode ser ferramenta local, apelido, extensão ou grafia diferente.
Não remover nem alterar o que já estiver instalado; antes de integrar, registrar
URL/repositório, versão e função exata. Esta nota impede a IA de inventar uma
dependência para preencher a lacuna.

## Decisão

Adotar primeiro validadores pequenos e determinísticos. Modelos generativos e
MCPs entram somente quando fecham uma lacuna mensurável que o assetgen e o
observatory não fecham.
