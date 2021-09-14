#!/usr/bin/env bash

#################################################
#### Ensure we are in the right path. ###########
#################################################
if [[ 0 -eq $(echo $0 | grep -c '^/') ]]; then
    # relative path
    EXEC_PATH=$(dirname "`pwd`/$0")
else
    # absolute path
    EXEC_PATH=$(dirname "$0")
fi

EXEC_PATH=$(echo ${EXEC_PATH} | sed 's@/\./@/@g' | sed 's@/\.*$@@')
cd $EXEC_PATH || exit 1
#################################################

SERVER_HOST=http://localhost
RWD_KEY_PATH=/tmp/staking_rwd.key
TD_NODE_KEY="${HOME}/.tendermint/config/priv_validator_key.json"
FRA_TOTAL_AMOUNT=21000000000000000

export LEDGER_DIR=/tmp/xx
export TENDERMINT_PORT=20000
export ABCI_PORT=10000
export SUBMISSION_PORT=$((9000 + $RANDOM % 1000))
export LEDGER_PORT=$((8000 + $RANDOM % 1000))
export TENDERMINT_NODE_KEY_CONFIG_PATH=${TD_NODE_KEY}
export SELF_ADDR=8DB4CBD00D8E6621826BE6A840A98C28D7F27CD9

println() {
    echo -e "\n\x1b[31;01m*===> ${1}\x1b[0m"
}

stop_node() {
    pid=$(ss -ntlp | grep ${ABCI_PORT} | grep -o 'pid=[0-9]\+' | grep -o '[0-9]\+')
    kill $pid 2>/dev/null

    pid=$(ss -ntlp | grep ${TENDERMINT_PORT} | grep -o 'pid=[0-9]\+' | grep -o '[0-9]\+')
    kill $pid 2>/dev/null
}

start_node() {
    abci_validator_node > /tmp/log 2>&1 &

    find ~/.tendermint -name LOCK | xargs rm -f
    nohup tendermint node &
}

init() {
    stop_node
    make -C ../.. stop_debug_env
    pkill -9 tendermint
    pkill -9 abci_validator_node

    make -C ../.. debug_env || exit 1

    printf "bright poem guard trade airport artist soon mountain shoe satisfy fox adapt garden decline uncover when pilot person flat bench connect coach planet hidden" > ${RWD_KEY_PATH}

    fns setup -S ${SERVER_HOST} || exit 1
    fns setup -O ${RWD_KEY_PATH} || exit 1
    fns setup -K ${TD_NODE_KEY} || exit 1

    stt init || exit 1
}

add_new_validator() {
    stop_node

    # waiting cluster to produce some blocks
    # so we can act as a new joined validator node
    sleep 15

    rm -rf ${LEDGER_DIR}
    tendermint unsafe_reset_all || exit 1
    tendermint init || exit 1
    tar -xpf demo_config.tar.gz || exit 1
    mv config.toml genesis.json node_key.json priv_validator_key.json ~/.tendermint/config/ || exit 1
    rm nohup.out 2>/dev/null

    start_node
}

check() {
    curl ${SERVER_HOST}:26657/validators | tail || exit 1
    println "There are 20 initial validators..."

    # at least 88_8888 FRAs
    fns stake -n $((888888 * 1000000)) -R 0.2 -M demo || exit 1
    sleep 30
    curl ${SERVER_HOST}:26657/validators | grep -A 5 ${SELF_ADDR} 2>/dev/null || exit 1
    println "Our validator appears in the validator list after staking..."

    fns stake --append -n $((222222 * 1000000)) || exit 1
    sleep 30
    curl ${SERVER_HOST}:26657/validators | grep -A 5 ${SELF_ADDR} 2>/dev/null || exit 1
    println "Its vote power has been raised after appending a new staking..."

    println "Now we stop it..."
    stop_node
    println "Wait 50s..."
    sleep 50

    println "Now we restart it..."
    start_node
    println "Wait 10s..."
    sleep 10

    grep ${SELF_ADDR} nohup.out
    println "Pay attention to its power change..."

    println "Now we unstake..."
    fns unstake
    println "Wait 30s..."
    sleep 30

    curl ${SERVER_HOST}:26657/validators || exit 1
    println "Our validator has been removed from the validator set..."
    println "The validator set has been restored to its original state..."

    println "Now we stop all other validators..."
    make -C ../.. stop_debug_env
    tail nohup.out
    println "Pay attention to its last state..."
    println "Wait 10s..."
    sleep 10

    println "Now we restart all other validators..."
    make -C ../.. start_debug_env
    println "Wait 10s..."
    sleep 2
    start_node
    sleep 8

    tail nohup.out
    println "Pay attention to its current state..."
    println "YES! All is well..."
    println "Enjoy..."
}

init
add_new_validator
check
