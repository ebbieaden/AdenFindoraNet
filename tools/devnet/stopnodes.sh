#!/usr/bin/env bash

# env
source ./tools/devnet/env.sh || exit 1

# stop one single node if specified
Node=""
if [ ! -z "$1" ]; then
    Node="node$1"
fi

# stop abcis
nodes=`ls -l $DEVNET | grep node  | awk '(NR>0){print $9}' | sort -V`

killed=false
for node in $nodes
do
    abci=`pgrep -f "abcid $DEVNET/$node$" | tr "\n" " " | xargs echo -n`
    if [ ! -z "$abci" ] && ([ -z "$Node" ] || [ "$Node" = "$node" ]); then
        if [ "$killed" = false ]; then
            echo -n "killed abci: "
            killed=true
        fi
        kill -9 $abci
        echo -en "${YEL}$abci ${NC}"
    fi
done

if [ "$killed" = true ]; then
    echo
fi

exit 0
