#!/bin/bash
set -ex

solana-keygen new --no-passphrase --outfile /tmp/fee-payer-keypair.json
echo "$BRIDGE_KEYPAIR_B64" | base64 -d > /ledger/program-id-keypair.json
PROGRAM_ID=$(solana-keygen pubkey /ledger/program-id-keypair.json)
echo "Using Program ID: $PROGRAM_ID"

# ждем валидатор до 10 секунд
for i in {1..10}; do
  solana --url http://solana-validator:8899 ping && break
  echo "Waiting for validator..."
  sleep 1
done

solana --url http://solana-validator:8899 airdrop 2 /tmp/fee-payer-keypair.json --keypair /tmp/fee-payer-keypair.json
solana program deploy \
  --url http://solana-validator:8899 \
  --program-id /ledger/program-id-keypair.json \
  --keypair /tmp/fee-payer-keypair.json \
  /project/target/deploy/w3b2_bridge_program.so
