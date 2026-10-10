# 06 — Bateria de testes 30× mais efetiva

## O que “30×” significa

Não significa trinta vezes mais asserts frágeis. Significa cobrir trinta
perspectivas de falha, transformar bugs visuais em invariantes e medir a
efetividade dos próprios testes. `doctrine.json` contém exatamente 30 fases e
o código recusa reduzir essa escada.

## Escada de 30 fases

1. Formatação e arquivos obrigatórios.
2. Parse de TOML/JSON/schemas.
3. Lints e warnings proibidos.
4. Unitários de domínio.
5. Propriedades matemáticas.
6. Determinismo e ordem de geração.
7. Testes de catálogo e referências.
8. Contratos de asset.
9. glTF/mesh/material offline.
10. Sockets, rig, colliders e navegação.
11. Viewmodel sem GPU.
12. Spawn e grounding multi-seed.
13. Hidrologia e RiverCarve.
14. Biome census e encontrabilidade.
15. Save round-trip e migração.
16. Replay de comandos.
17. Fuzz de parsers, saves e rede.
18. Mutation testing em regras críticas.
19. Simulação curta headless.
20. Soak longo e orçamento de eventos.
21. GPU smoke por backend disponível.
22. Capturas semânticas por cena.
23. Comparação de pixels por ROI e máscara.
24. Overlay de profundidade/normais/wireframe/anchors.
25. Rotas de input e UI reais.
26. Jornada de novo player.
27. Dois clientes e autoridade quando aplicável.
28. Performance, memória e stutter.
29. Pacote limpo Windows/Linux/macOS e launch smoke.
30. Playtest humano + auditoria de evidência.

Os detalhes de máquina, tipos e evidências mínimas ficam no JSON para impedir
que uma edição de texto diminua silenciosamente a barra.

## Como cada prova visual funciona

Uma cena visual declara:

- intenção e falhas óbvias que devem ser detectadas;
- seed, pose, FOV, resolução, tier, horário e clima;
- subjects e bounding boxes esperadas;
- ROIs positivas e negativas;
- estado semântico antes/depois;
- tolerância de pixel, profundidade, movimento e layout;
- artefatos: color, depth, normals, ids, wireframe, collision, state JSON;
- veredito por assertion;
- confirmação humana registrada.

O gate falha se o subject estiver fora da câmera, mesmo que a cena tenha
pixels. Também falha se um objeto cobrir o centro excessivamente, se a câmera
estiver dentro de geometria ou se água aparecer sem suporte.

## Testes que testam os testes

- **Mutation testing:** `cargo-mutants` deve matar mutações em regras de spawn,
  custo, suporte, saves e autoridade. Mutante sobrevivente vira backlog.
- **Fault injection:** deslocar artificialmente água/prop/viewmodel em fixture
  deve acionar o gate correto.
- **Golden adversarial:** manter cenas negativas que nunca podem ser aceitas.
- **Flake replay:** qualquer falha não reprodutível guarda seed, trace e host;
  executar N vezes antes de relaxar tolerância.
- **Coverage de requisitos:** cada lei liga a teste/rota; linha sem prova falha.

## Pirâmide diária, noturna e release

### Por commit

Fases 1–16 aplicáveis, teste focado, captura afetada, smoke curto.

### Noturna

Fuzz limitado, mutation incremental, matriz de seeds, observatory, determinismo,
soak e performance. Falhas criam pacote reproduzível; não alteram golden.

### Release/Steam branch

Todas as 30 fases, três plataformas, pacote instalado a partir do artefato,
save de migração, jornada humana e assinatura do relatório.

## Métricas úteis

- defeitos reais detectados por gate;
- mutant kill rate em regras críticas;
- flake rate por rota;
- seeds cobertos e minimizados;
- requisitos com prova ligada;
- tempo até reprodução;
- regressões escapadas para playtest;
- cobertura de plataformas/backends;
- custo total da bateria e p95 por fase.

Contagem bruta de testes aparece no relatório, mas não é objetivo de produto.

## Política de flakiness

Uma rota flaky não recebe retry escondido até ficar verde. Primeiro classificar:
relógio, frame trigger, ordem de hash, streaming, GPU, input, filesystem ou
concorrência. Fixar origem, tornar trigger baseado em estado/posição e rodar
repetidamente. Retry pode existir para coleta, nunca para mascarar veredito.
