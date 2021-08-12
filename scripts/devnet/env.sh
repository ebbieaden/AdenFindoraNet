#!/usr/bin/env bash
RED='\033[31m'
GRN="\033[32m"
YEL='\033[33m'
NC='\033[0m'

# paths
TMP_HOME=/tmp/findora
FIN_HOME="${FIN_HOME:=$TMP_HOME}"
DEVNET="$FIN_HOME/devnet"

# binary config
BIN_CFG_DEFAULT=release
BIN_CFG="${BIN_CFG:=$BIN_CFG_DEFAULT}"

# logs
ABCI_LOG_LEVEL="baseapp=info,account=info,ethereum=info,evm=info,eth_rpc=info"

# show envs
if [ "$1" == "s" ]; then
    echo "FIN_HOME = $FIN_HOME"
    echo "DEVNET   = $DEVNET"
    echo "BIN_CFG  = $BIN_CFG"
fi
