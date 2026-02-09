#!/bin/bash
set -e # Exit on error

# Define execution command (use cargo run for path robustness in workspace)
# We use --quiet to reduce cargo build output noise
BIN="cargo run -p sovereign-tee-core --quiet --"

echo -e "\n=== Test Scenario 1: Standard Seal Strategy ==="
# Cleanup
rm -f group.json dao.hex tee.hex dao_share.seal tee_share.store

# 1. Init
$BIN genesis-init --threshold 2
$BIN genesis-join --name Alice
$BIN genesis-join --name Bob
$BIN genesis-launch

# 2. Execute
$BIN proposal-execute \
    --recipient "0x1111111111111111111111111111111111111111111111111111111111111111" \
    --amount 100

echo "✅ Strategy A Passed"

echo -e "\n=== Test Scenario 2: NFT Sharding Strategy ==="
# Cleanup
rm -f group.json dao.hex tee.hex dao_share.seal tee_share.store shard_*.hex

# 1. Init
$BIN genesis-init --threshold 2
$BIN genesis-join --name Alice
$BIN genesis-join --name Bob
$BIN genesis-launch --strategy nft-sharding --shards 5

# 2. Execute (with 2 shards)
$BIN proposal-execute \
    --strategy nft-sharding \
    --shards-in shard_1.hex shard_2.hex \
    --recipient "0x2222222222222222222222222222222222222222222222222222222222222222" \
    --amount 200

echo "✅ Strategy B Passed"

# Cleanup
rm -f group.json dao.hex tee.hex dao_share.seal tee_share.store shard_*.hex
echo -e "\n=== All System Tests Passed Successfully ==="
