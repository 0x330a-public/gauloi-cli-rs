# Gauloi CLI

>Disclaimer: don't use this for actual transactions or anything where you are using real money 

BTC<->ETH p2p swaps, with a very epic user interface that involves copying and pasting hex encoded strings
in a command line.

### Architecture

BTC to ETH p2p swaps independent on communication channel, using deterministic :tm: address creation and on-chain balances and txs

parties using this CLI create hot wallets in the folder that the CLI is executed then
load BTC/ETH into account listed in the `accounts` command when you're in the CLI.
BTC side then creates a swap via `create` command, encoding swap information including the number of each asset (derive price from ratio),
as well as hash of the unlock preimage required for spending the P2WSH address.
ETH side parses the hex string via the `parse` command, which displays the offer info for you to accept or discard. Upon accepting, a new "response" is printed out,
which the BTC side imports via `import` and checks against the offers it has created to match up the preimage hashes.

Once both sides have the "full picture" of swap information, they can derive the BTC side P2WSH, which is made up of the hash of both party public keys,
the pre-image hash, and the time lock before the BTC party can reclaim their funds in the timeout branch of the HTLC contract.

The BTC party can then commit the agreed funds to the HTLC and the ETH party can validate the amount ready to unlock.
Once the ETH party verifies an initial deposit they can create a swap on the GauloiSwapFactory contract, specifying the BTC side's eth address as an unlocker
to prevent front-running the claim of ETH funds by others. The preimage hash is also committed to the ETH side, such that only the preimage with a known hash value
can claim the funds.

Once the BTC side submits an ETH transaction to claim the funds the ETH party then knows the preimage to unlock the BTC side funds and the swap is complete.
