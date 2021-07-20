#!/usr/bin/env bash

# env
source scripts/devnet/env.sh || exit 1

# start abcis
nodes=`ls -l $DEVNET | grep node  | awk '(NR>0){print $9}' | sort -V`
for node in $nodes
do
    #rm -rf ${DEVNET}/${node}/abci/db
    SelfAddr=$(grep 'address' ${DEVNET}/${node}/config/priv_validator_key.json | grep -oE '[^",]{40}')
    TD_NODE_SELF_ADDR=$SelfAddr \
        LEDGER_DIR=$DEVNET/$node/abci \
        RUST_LOG=$ABCI_LOG_LEVEL \
        abci_validator_node $DEVNET/$node >> $DEVNET/$node/abci_validator.log 2>&1  &
done

# start nodes
for node in $nodes
do
    tendermint node --home $DEVNET/$node --log_level $TENDERMINT_LOG_LEVEL >> $DEVNET/$node/consensus.log 2>&1  &
done

# show abcis and nodes
for node in $nodes
do
    echo -n "$node: "
    abci=`pgrep -f "abci_validator_node $DEVNET/$node$" | tr "\n" " " | xargs echo -n`
    echo -en "abci(${GRN}$abci${NC}) <---> "
    sleep 0.5
    node=`pgrep -f "tendermint node --home $DEVNET/$node" | tr "\n" " " | xargs echo -n`
    echo -e "node(${GRN}$node${NC})"
done
