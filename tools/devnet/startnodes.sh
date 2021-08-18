#!/usr/bin/env bash

# env
source ./tools/devnet/env.sh || exit 1

# start nodes
nodes=`ls -l $DEVNET | grep node  | awk '(NR>0){print $9}' | sort -V`
for node in $nodes
do
        RUST_LOG=$ABCI_LOG_LEVEL \
        target/$BIN_CFG/findorad node \
        --config $DEVNET/$node/config/config.toml \
        --ledger-dir $DEVNET/$node/abci \
        --tendermint-node-key-config-path ${DEVNET}/${node}/config/priv_validator_key.json \
        >> $DEVNET/$node/findorad.log 2>&1  &
done

# show nodes
for node in $nodes
do
    echo -n "$node: "
    abci=`pgrep -f "findorad node --config $DEVNET/$node/" | tr "\n" " " | xargs echo -n`
    echo -e "${GRN}$abci${NC}"
done
