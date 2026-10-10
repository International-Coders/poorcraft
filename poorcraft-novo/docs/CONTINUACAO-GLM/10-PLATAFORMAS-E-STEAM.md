# 10 — Windows, Linux, macOS e Steam desde agora

## Política

Código novo considera as três plataformas no design. “Funciona no Mac do
desenvolvedor” é apenas uma célula da matriz. Cada release candidate precisa
ser compilado e lançado a partir do pacote produzido no próprio sistema.

## Matriz mínima

| Alvo | Artefato | GPU/backends | Smoke |
| --- | --- | --- | --- |
| Windows x86_64 | `.exe` + pasta/zip | DX12; Vulkan quando suportado | instalar/copiar, criar mundo, jogar, salvar, reabrir |
| Linux x86_64 | tarball/AppImage futuro | Vulkan | extração limpa, permissões, áudio, save |
| macOS universal futuro | `.app` + `.dmg` | Metal | Gatekeeper local, Retina, áudio, save |

Primeiro é aceitável x86_64 por plataforma. Apple Silicon nativo já existe no
host; universal2 vira gate antes de anúncio comercial. A arquitetura e o
target ficam no nome/manifesto do pacote.

## Regras de portabilidade

- paths por `PathBuf`, sem separador manual;
- case sensitivity testada em Linux;
- nenhum shell/arquivo temporário obrigatório no runtime;
- filesystem gravável vem da pasta de dados do usuário;
- saves separados de assets e executável;
- shaders validados nos backends disponíveis;
- input não depende de scancode de um teclado;
- áudio falha para silêncio gracioso;
- fonte, DPI e safe area testados;
- relógio/determinismo não dependem de timezone;
- build sem SDK Steam continua funcionando por fallback;
- símbolos/logs e build id acompanham cada artefato.

## Steam

Steam Early Access só faz sentido quando há build jogável com valor atual e
plano real de continuidade; não serve como crowdfunding ou pré-venda. Até esse
gate, usar branches internas ou Steam Playtest é mais reversível.

Estrutura prevista:

- um depot comum para assets quando útil;
- depots específicos Windows, Linux e macOS;
- branch `internal`, depois `playtest`, depois `early-access`;
- SteamPipe VDF gerado de template com AppID fornecido pelo proprietário;
- preview build antes de upload;
- pacote inclui o depot correto para cada OS;
- sem credenciais no repositório;
- rollback e notas de build;
- save path, cloud e compatibilidade versionados;
- crash/privacy/telemetry opt-in documentados.

Fontes oficiais pesquisadas:

- SteamPipe: <https://partner.steamgames.com/doc/sdk/uploading>
- Depots por sistema: <https://partner.steamgames.com/doc/store/application/depots>
- Early Access: <https://partner.steamgames.com/doc/store/earlyaccess>

## Gate de alfa distribuível

- primeira hora compreensível e divertida;
- sem spawn, rio ou viewmodel P0 conhecidos;
- save/load e migração confiáveis;
- settings dirigem engine;
- crash-free soak definido;
- performance low/mid dentro da meta;
- controle mouse/teclado completo e controller explicitamente marcado;
- três pacotes reproduzíveis em CI/hosts reais;
- página de limitações honesta;
- feedback in-game/externo com build id;
- conteúdo e licenças auditados;
- processo de update/rollback ensaiado.

## O que não entra no repositório

Steamworks SDK proprietário, chaves, senhas, arquivos de conta e builds não
licenciados. O repo guarda templates, scripts sem segredo, checks e docs.
