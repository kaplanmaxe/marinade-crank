# Marinade Crank

Stakes to a validator of your choosing. The validator needs to be eligible stake in order for this to run successfully.

**This is an unofficial tool, not written or maintained by Marinade. It is provided as is as an open source tool.**

This was originally made as a script. It is being OSS'd to help other validators.

### Building

`cargo build`

### Running

**This tool can be run with any keypair so long as it is funded with a sufficient amount of SOL. There is no need to run this with your validator identity.**

It is recommended to simulate first. This is because `stakeReserve` can only be called at a specific slot. By simulating, the output will either show:

1. Which slot you can run this program

```
./target/debug/marinade-crank --vote-account H4QVPxS7napq3NEYxqLhxbKi9nJ8s56dD2EQZGsyZ3sb --simulate --keypair <PATH_TO_KEYPAIR_FILE> --cluster https://api.mainnet-beta.solana.com
Attempting to stake H4QVPxS7napq3NEYxqLhxbKi9nJ8s56dD2EQZGsyZ3sb with 2379 SOL
Simulation result: Ok(Response { context: RpcResponseContext { slot: 265745295, api_version: Some(RpcApiVersion(Version { major: 1, minor: 17, patch: 28 })) }, value: RpcSimulateTransactionResult { err: Some(InstructionError(2, Custom(6042))), logs: Some(["Program ComputeBudget111111111111111111111111111111 invoke [1]", "Program ComputeBudget111111111111111111111111111111 success", "Program ComputeBudget111111111111111111111111111111 invoke [1]", "Program ComputeBudget111111111111111111111111111111 success", "Program MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD invoke [1]", "Program log: Instruction: StakeReserve", "Program 11111111111111111111111111111111 invoke [2]", "Program 11111111111111111111111111111111 success", "Program consumption: 77570 units remaining", "Program log: AnchorError thrown in programs/marinade-finance/src/instructions/crank/stake_reserve.rs:171. Error Code: TooEarlyForStakeDelta. Error Number: 6042. Error Message: Too early for stake delta.", "Program log: Left: 265745298", "Program log: Right: 266093999", "Program MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD consumed 31901 of 99700 compute units", "Program MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD failed: custom program error: 0x179a"]), accounts: None, units_consumed: Some(300), return_data: None } })
```

2. That the validator has already reached the stake target

```
./target/debug/marinade-crank --vote-account EdGevanAjM8a6Gg9KxBVrmVdZAUGAZ9xaVd7t9R4H2x --simulate --keypair <PATH_TO_KEYPAIR_FILE> --cluster https://api.mainnet-beta.solana.com
Validator EdGevanAjM8a6Gg9KxBVrmVdZAUGAZ9xaVd7t9R4H2x already reached stake target. Active balance: 21482, stake_target: 20313
```

If a validator is eligble for stake as shown in #1, read the output where it says `Too early for stake`. In that log, you will see a value for `Left` and `Right`. The value for `Left` is the current slot in the epoch. The value of `Right` is the slot when you can call the `stakeReserve` instruction. **If you try to call the instruction before the right slot, it will fail.**

Then simply run the program but omit `--simulate`.

### Priority fees

To run with a priority fee set, you can set `--with-compute-unit-price`.