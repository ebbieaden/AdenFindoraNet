# Tendermint ABCI

![](https://github.com/FindoraNetwork/FGR/blob/master/src/pics/preflow.png)

## Staking Test

### Run Auto Cases

`make staking_test`

### Manual Operations

A successful `make` will pruduce a test tool called `stt`, and put it to `~/.cargo/bin`.

```shell
stt 0.1.0
FindoraNetwork
A manual test tool for the staking function.

USAGE:
    stt [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    claim
    delegate
    help          Prints this message or the help of the given subcommand(s)
    init
    show
    undelegate
```

```shell
stt-init

USAGE:
    stt init

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
```

```shell
stt-delegate

USAGE:
    stt delegate [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -n, --amount <Amount>          how much FRA to delegate
    -u, --user <User>              user name of delegator
    -v, --validator <Validator>    which validator to delegate to
```

```shell
stt-undelegate

USAGE:
    stt undelegate [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -u, --user <User>    user name of delegator
```

```shell
stt-claim

USAGE:
    stt claim [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -n, --amount <Amount>    how much FRA to delegate
    -u, --user <User>        user name of delegator
```

```shell
stt-show

USAGE:
    stt show [FLAGS] [OPTIONS]

FLAGS:
    -b, --coinbase          show the infomation about coinbase
    -h, --help              Prints help information
    -r, --root-mnemonic     show the pre-defined root mnemonic
    -U, --user-list         show the pre-defined user list
    -v, --validator-list    show the pre-defined validator list
    -V, --version           Prints version information

OPTIONS:
    -u, --user <User>    user name of delegator
```
