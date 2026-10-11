# Continuação GLM 5.3 — manual operacional do POORCRAFT 3D

Este pack converte a direção do proprietário em regras de execução. Ele não é
uma lista de sonhos e não substitui o jogo. Sua função é impedir que uma sessão
de IA faça centenas de alterações aparentemente produtivas sem provar que o
player consegue jogar, enxergar e confiar no resultado.

## Ordem de leitura

1. [00 — Leia primeiro](00-LEIA-PRIMEIRO.md)
2. [01 — Visão e identidade](01-VISAO-E-IDENTIDADE.md)
3. [02 — Modo de operação GLM](02-MODO-DE-OPERACAO-GLM.md)
4. [03 — Fábrica de assets](03-FABRICA-DE-ASSETS.md)
5. [04 — Viewmodel e câmera](04-VIEWMODEL-E-CAMERA.md)
6. [05 — Mundo, rios e spawn](05-MUNDO-RIOS-SPAWN.md)
7. [06 — Bateria 30×](06-BATERIA-DE-TESTES-30X.md)
8. [07 — Primeira sessão, HUD e construção](07-PRIMEIRA-SESSAO-HUD-CONSTRUCAO.md)
9. [08 — Lore, facções e tom](08-LORE-FACCOES-E-TOM.md)
10. [09 — Áudio e música procedural](09-AUDIO-E-MUSICA-PROCEDURAL.md)
11. [10 — Plataformas e Steam](10-PLATAFORMAS-E-STEAM.md)
12. [11 — Ferramentas pesquisadas](11-FERRAMENTAS-PESQUISADAS.md)
13. [12 — Fila de execução](12-FILA-DE-EXECUCAO.md)
14. [13 — Plano N01–N05: confiança espacial](13-PLANO-N01-N05-CONFIANCA-ESPACIAL.md)
15. [14 — Plano N06–N11: primeira hora](14-PLANO-N06-N11-PRIMEIRA-HORA.md)
16. [15 — Plano N12–N19: identidade e som](15-PLANO-N12-N19-IDENTIDADE-E-SOM.md)
17. [16 — Plano N20–N27: QA e distribuição](16-PLANO-N20-N27-QA-DISTRIBUICAO.md)

Arquivos de máquina:

- `doctrine.json`: 30 fases, gates mínimos, plataformas e ferramentas.
- O teste `pc3d_assets::doctrine` valida o contrato e a presença deste pack.
- `make p3d-doctrine-check` é a porta rápida antes de qualquer implementação.

## Relação com documentação anterior

`../../../../docs/POORCRAFT-3D/` contém a constituição, decisões, planos e
auditorias anteriores. Eles continuam valiosos. Quando um documento antigo
declara uma feature “feita”, a fonte, a rota jogada e a prova atual prevalecem.
O pack atual muda a prioridade: confiabilidade da experiência, sem apagar a
ambição de mundo, facções, indústria, magia e império.

## Regra de interpretação

Palavras como “deve”, “falha”, “bloqueia”, “prova” e “não pode” são normativas.
“Pode”, “candidato” e “experimento” indicam liberdade controlada. Quando houver
conflito, a experiência do player e a prova honesta vencem o volume de código.
