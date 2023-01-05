#!/bin/bash

CURRENT_PATH=$(pwd)
cd $CURRENT_PATH/packages/contracts/token_contract && cargo test
cd $CURRENT_PATH/packages/contracts/exchange_contract && cargo test
cd $CURRENT_PATH/packages/contracts/router_contract && cargo test
cd $CURRENT_PATH/packages/contracts/registry_contract && cargo test
cd $CURRENT_PATH/packages/contracts/vault_contract && cargo test
