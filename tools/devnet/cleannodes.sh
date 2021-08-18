#!/usr/bin/env bash

# env
source tools/devnet/env.sh || exit

# stop nodes
script_stop=$(dirname "$0")/stopnodes.sh
bash "$script_stop"

# clean single node if specified
Node=""
if [ ! -z "$1" ]; then
    Node="node$1"
fi

# clean nodes
nodes=`ls -l $DEVNET | grep node | awk '(NR>0){print $9}' | sort -V`
echo -n "cleaned: "
for node in $nodes
do
if [ ! -z "$Node" ] && [ "$Node" = "$node" ]; then
    echo -en "$node "
    # abcis
    rm -rf $DEVNET/$node/abci/*.db
    rm -rf $DEVNET/$node/abci/utxo_map
    rm -rf $DEVNET/$node/abci/txn_merkle
    rm -rf $DEVNET/$node/abci/txn_log
    rm -rf $DEVNET/$node/abci/block_merkle

    # tendermint
    rm -rf $DEVNET/$node/data/*.db
    rm -rf $DEVNET/$node/data/cs.wal
    rm -rf $DEVNET/$node/config/addrbook.json
    rm -rf $DEVNET/$node/findorad.log
    rm -rf $DEVNET/$node/consensus.log
    cat > $DEVNET/$node/data/priv_validator_state.json <<EOF
{
  "height": "0",
  "round": "0",
  "step": 0
}
EOF

fi
done
echo
