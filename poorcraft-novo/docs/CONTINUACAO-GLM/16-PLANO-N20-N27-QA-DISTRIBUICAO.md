# 16 — Plano executável N20–N27: QA industrial e distribuição

O objetivo é detectar defeitos antes do proprietário jogar. Ferramentas externas
só entram após licença, segurança, reprodutibilidade e utilidade medida.

## N20 — glTF Validator

Fixar versão/checksum; manter fixture válida e inválidas para buffer ausente,
accessor fora do limite, NaN, normal ruim, textura ausente e extensão
incompatível. Emitir JSON por asset. Erro bloqueia; warning só entra em allowlist
com razão e expiração. Rodar offline/cacheado e incremental por hash.

Validação de especificação complementa, não substitui, gates semânticos.

## N21 — Fuzz

Alvos: saves, manifests JSON/TOML, comandos de observatory e rede. Nunca
panic/OOM/hang; inválido retorna erro limitado; válido preserva roundtrip.
Corpus contém arquivos reais, bordas e regressões minimizadas. PR roda smoke,
nightly aumenta orçamento e semanal registra cobertura/crashes. Nightly
toolchain fica isolada do produto.

## N22 — Mutation gate

Escopo: spawn, grounding, água, placement/custo, inventário e save. Relatório
separa killed, survived, timeout e unviable. Meta inicial 90% geral e 100% nas
condições críticas. Sobrevivente crítico bloqueia ou vira fixture com prazo.

## N23 — Observatory v2

Cada rota inclui color, depth linear, normals, object/semantic ids, anchors,
collision, nav, grounding, câmera/FOV/aspect/seed/build/adapter/tier, input,
eventos, state hash, p50/p95/p99, ROIs, verdicts e hashes. Comparator distingue
mudança esperada, ruído e regressão. Imagem bonita com sidecar contraditório
falha.

## N24 — CI Windows/Linux/macOS

Por sistema: parse/lint → testes puros → workspace → release → pacote nativo →
launch do pacote → criar/salvar/recarregar → upload com build id. GPU tests
rodam em executor apropriado. Falta de hardware é indisponibilidade explícita,
nunca skip verde de release.

## N25 — Pacotes reproduzíveis

Alvos: poorcraft3d-windows-x86_64.zip, poorcraft3d-linux-x86_64.tar.gz e
poorcraft3d-macos.dmg. Incluir cliente, servidor quando aplicável, licença,
créditos, config e manifest. Build id aparece no menu/log/diagnóstico.

Windows testa runtime/DLL; Linux testa baseline ABI; macOS testa arquitetura,
quarantine e, quando houver credenciais, assinatura/notarização. Diferenças de
timestamp/assinatura ficam isoladas.

## N26 — SteamPipe

Templates com placeholders para app/depot/build e branches internal, playtest,
alpha e default. AppID real, credenciais e SteamGuard nunca entram no Git.
Primeiro preview/dry-run; depots por sistema e servidor. Excluir saves, shots,
symbols privados e ferramentas. Manter rollback do build anterior.

## N27 — Steam Playtest

Exigir primeira hora aprovada, crash-free soak, migration, install/update/
uninstall/rollback, input/resolução/acessibilidade, feedback, privacidade,
screenshots verdadeiras, suporte e known issues. Playtest valida operação sem
promessa comercial. Early Access só quando o build atual for divertido,
confiável e sustentável.

## Cadência

| Faixa | Conteúdo | Frequência |
| --- | --- | --- |
| commit | fases 1–16 + gates focados | sempre |
| PR | 1–19 + GPU + rotas afetadas | sempre |
| nightly | 1–28 + fuzz/mutation/soak | diário |
| candidato | 1–30 + 3 pacotes + humano | promoção |
| Steam | candidato + preview/install/rollback | externa |

Falta de tokens ou duração longa não transforma parcial em verde. Registrar
checkpoint, artifacts, o que não rodou e a próxima ação exata.
