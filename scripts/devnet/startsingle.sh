#!/usr/bin/env bash

# env
source scripts/devnet/env.sh || exit 1

# paths
SINGLE="$FIN_HOME/single"

# start abci
echo -n "starting node: "
abci_validator_node $SINGLE >> $SINGLE/abci_validator.log 2>&1  &
sleep 2

# start node
tendermint node --home $SINGLE >> $SINGLE/consensus.log 2>&1  &

# show pids
abci=`pgrep -f abci_validator_node | tr "\n" " " | xargs echo -n`
node=`pgrep -f "tendermint node --home.*" | tr "\n" " " | xargs echo -n`
echo -e "abci(${GRN}$abci${NC}) <---> node(${GRN}$node${NC})"
