# POORCRAFT — dois projetos, uma raiz de controle

O repositório foi separado fisicamente em duas gerações:

- [`Poorcraft/`](Poorcraft/) preserva LOREFORGE/POORCRAFT, seus
  crates, mods, saves, screenshots e runtimes.
- [`Poorcraft 3D/`](Poorcraft%203D/) contém o POORCRAFT 3D ativo.

O Makefile, o histórico geral e a documentação compartilhada permanecem na
raiz. Rode `make help` daqui. Para continuar o jogo atual, comece em
[`Poorcraft 3D/docs/CONTINUACAO-GLM/README.md`](Poorcraft%203D/docs/CONTINUACAO-GLM/README.md)
e valide a doutrina com `make p3d-doctrine-check`.

Comandos principais:

```sh
make test                 # legado
make p3d-test             # jogo atual
make p3d-doctrine-check   # contrato GLM/QA
make p3d-beta             # bateria visual longa do jogo atual
make runtimes             # runtimes do legado
make p3d-dmg              # app/DMG do jogo atual
```

Não mova saves de volta para a raiz e não misture crates das duas engines. Uma
ideia pode ser portada do legado; código só entra no novo jogo após validação
contra a arquitetura e os gates atuais.
