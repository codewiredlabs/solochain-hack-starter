# End-to-end checks

Scripts that drive a running development chain over its WebSocket RPC. They sign real
transactions as the well-known development accounts and check balances, events, storage and view
functions after each step. Unit tests check a pallet in isolation; these check that the pallet
works inside the real runtime, with real fees, real blocks and real metadata.

## Requirements

- [Node.js](https://nodejs.org/) 18 or later.
- A development chain built from this repository, listening on `ws://127.0.0.1:9944`. See
  [Run the Solochain on Linux or macOS](../../docs/RUNNING.md) for how to build and start it.

## Run the ticketing check

In one terminal, start a development chain from the repository root:

```sh
./target/release/solochain-starter-node --dev
```

In a second terminal, install the pinned dependencies once and run the script:

```sh
cd scripts/e2e
npm ci
npm run ticketing
```

To use a node on another port, pass its address: `npm run ticketing -- ws://127.0.0.1:9945`.

The run takes about two minutes, because every step waits for its transaction to be included in a
block and blocks are 6 seconds apart. It works on a fresh chain or one that has been running for a
while: it reads the next event id from the chain and only compares balance changes.

## What it checks

**Scenario 1: sell, resell, check in, settle, purge.** Alice funds Charlie, then creates an event
with a 10 UNIT ticket, a 20% resale cap and a 10% royalty.

- Bob buys a ticket. The view functions report a 12 UNIT resale cap, one ticket held by Bob and
  11 UNIT in escrow.
- Bob's listing at 13 UNIT is rejected and his listing at 12 UNIT is accepted.
- Charlie buys it. Alice receives exactly 1.2 UNIT in royalty and Bob exactly 10.8 UNIT.
- Alice checks Charlie in, after which Charlie can no longer list the ticket.
- Settling before the event ends is rejected. After it ends, settling pays Alice the 10 UNIT of
  sales and leaves the 1 UNIT deposit in escrow.
- Bob purges the event. Alice gets the deposit back, and the event and its tickets are gone from
  storage.

**Scenario 2: cancel, refund, settle, purge.** Alice creates a second event, and Bob and Charlie
buy a ticket each.

- Alice cancels the event. Charlie pushes Bob's refund, and Bob receives exactly 5 UNIT.
- Settling is rejected while Charlie's refund is outstanding.
- Alice refunds Charlie, settles with nothing to pay out, and Bob's purge returns her deposit.

## Reading the result

Each check prints `PASS` or `FAIL`, and each transaction prints its outcome and the ticketing
events it emitted:

```text
      Bob listTicket 13 UNIT: error ticketing.PriceAboveResaleCap
PASS  listing above the 20% cap is rejected
...
ALL CHECKS PASSED
```

The exit code is `0` when every check passes, `1` when a check fails and `2` when the script
itself fails, for example because no node is listening at the address.
