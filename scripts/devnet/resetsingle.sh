#!/usr/bin/env bash

# paths
SINGLE="$FIN_HOME/single"

# clean node
rm -rf $SINGLE/*

# reset tendermint node
tendermint init --home $SINGLE  > /dev/null
toml set --toml-path $SINGLE/config/config.toml consensus.create_empty_blocks_interval 1s
toml set --toml-path $SINGLE/config/config.toml consensus.timeout_commit 15s

# create abci dir
mkdir -p $SINGLE/abci
