#!/usr/bin/env bash

# env
source tools/devnet/env.sh || exit 1

# show nodes
nodes=`ls -l $DEVNET | grep node  | awk '(NR>0){print $9}' | sort -V`
for node in $nodes
do
    abci=`pgrep -f "findorad node --config $DEVNET/$node/" | tr "\n" " " | xargs echo -n`
    if ! [ -z "$abci" ]
    then
        echo -n "$node: "
        echo -e "${GRN}$abci${NC}"
    fi
done
