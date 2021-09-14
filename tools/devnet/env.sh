#!/usr/bin/env bash
RED='\033[31m'
GRN="\033[32m"
YEL='\033[33m'
NC='\033[0m'

TMP_HOME=/tmp/findora
FIN_HOME="${FIN_HOME:=$TMP_HOME}"
DEVNET="$FIN_HOME/devnet"
ABCI_LOG_LEVEL="baseapp=info,account=info,ethereum=info,evm=info,eth_rpc=info"
TENDERMINT_LOG_LEVEL="main:info,state:info,*:error"

#echo "$TMP_HOME"
#echo "$FIN_HOME"
#echo "$DEVNET"
