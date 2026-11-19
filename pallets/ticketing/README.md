# Ticketing Pallet

Sell event tickets on-chain. Payments sit in a per-event escrow until the event is over, resale is
capped so scalpers can't flip tickets at any price, and the organizer earns a royalty on every
resale.

## How it works

```text
 organizer ── create_event ──► escrow account (deposit)
 buyer ────── buy_ticket ────► escrow account (+ face value)
 holder ───── list_ticket (≤ price + resale_cap)
 buyer ────── buy_listed_ticket ─► seller (price − royalty), organizer (royalty)
 organizer ── check_in            (ticket can no longer be resold)
 organizer ── settle_event ──────► ticket sales paid to the organizer, after ends_at
 anyone ───── purge_event ───────► storage deleted, deposit returned to the organizer
```

If the organizer cancels before the event starts, every ticket becomes refundable at face value,
and the event can only be settled once all tickets are refunded.

| Extrinsic | Who | When |
|---|---|---|
| `create_event` | anyone (becomes the organizer) | `starts_at` in the future |
| `buy_ticket` | `BuyerOrigin` | `sale_start ≤ now < starts_at` |
| `list_ticket` / `delist_ticket` | ticket owner | before `starts_at`, not checked in |
| `buy_listed_ticket` | `BuyerOrigin` | before `starts_at` |
| `check_in` | organizer | before `ends_at` |
| `cancel_event` | organizer | before `starts_at` |
| `refund_ticket` | anyone (refund goes to the owner) | event cancelled |
| `settle_event` | organizer | after `ends_at`, or after all refunds of a cancelled event |
| `purge_event` | anyone (deposit goes to the organizer) | event settled; repeat until `EventPurged` |

View functions (callable through the `RuntimeViewFunction` runtime API): `max_resale_price`,
`escrow_balance`, `tickets_held_by`.

## Design decisions

- **One escrow account per event**, derived from `PalletId` and the event id. The organizer's
  deposit goes in first, so the account always stays above the existential deposit while refunds
  are owed, and one event's funds can never pay another event's refunds. `integrity_test` fails if
  the runtime's `AccountId` is too short to keep these accounts distinct.
- **No free transfers.** Tickets only change hands through `buy_listed_ticket`, where the resale
  cap and royalty are enforced. A free transfer would let people sell off-chain and "gift" the
  ticket, bypassing both.
- **Slippage protection.** `buy_listed_ticket` takes a `max_price`, so a seller can't raise the
  price between the buyer signing and the transaction landing.
- **Anyone can push a refund.** Refunds always go to the ticket owner, so the organizer can refund
  holders who never claim and settle the event.
- **The deposit pays for cleanup.** Settling pays out ticket sales only. The deposit comes back
  when `purge_event` deletes the event, so organizers have a reason to free the storage. Anyone may
  purge, so cleanup doesn't depend on the organizer's key.
- **Bounded purges.** One `purge_event` call deletes at most `RemoveItemsLimit` tickets and is
  charged only for the tickets it actually deleted. Events of any size can be cleaned up without
  an oversized transaction.
- **Non-zero prices must be at least the existential deposit**, so a refund can always create the
  recipient's account if it was reaped in the meantime.
- **Tickets live in the pallet's own storage, not `pallet-nfts`.** NFTs transfer freely, which
  would bypass the resale cap.

## Configuration

| Item | Purpose | Solochain value |
|---|---|---|
| `Currency` | Token tickets are paid in | `Balances` |
| `BuyerOrigin` | Who may buy; the seam for a "one person, one ticket" check | `EnsureSigned<AccountId>` |
| `BlockNumberProvider` | Clock for sale windows and event times | `System` (a parachain should use relay chain blocks) |
| `PalletId` | Seed of escrow accounts | `py/tickt` |
| `EventDeposit` | Locked per event, returned when the event is purged | `1 UNIT` |
| `MaxCapacity` | Largest event | `10_000` |
| `MaxNameLength` | Longest name in bytes | `64` |
| `RemoveItemsLimit` | Most tickets one `purge_event` call deletes | `1_000` |

## Try it in Polkadot-JS Apps

[Run the Solochain on Linux or macOS](../../docs/RUNNING.md) shows how to start the chain and
connect Polkadot-JS Apps. Every call below is made under **Developer → Extrinsics**, where amount
fields take plain integers in the smallest unit: 1 UNIT is `1000000000000` (12 zeros).

1. Note the current block number in **Network → Explorer**. Blocks are 6 seconds apart, so 100
   blocks is 10 minutes.
2. Only Alice and Bob start with funds. Under **Accounts**, send Charlie `100` UNIT from Alice.
   (The **Send** dialog does take amounts in UNIT.)
3. As **Alice**, call `ticketing.createEvent` with: a name, capacity `100`, price
   `10000000000000` (10 UNIT), max per buyer `2`, resale cap `20`, royalty `10`, sale start `0`,
   starts at `current + 100`, ends at `current + 200`. The first event gets id `0`.
4. As **Bob**, call `ticketing.buyTicket(0)`.
5. As **Bob**, call `ticketing.listTicket(0, 0, 12000000000000)`. Anything above 12 UNIT is
   rejected.
6. As **Charlie**, call `ticketing.buyListedTicket(0, 0, 12000000000000)`. Alice receives the 10%
   royalty and Bob the rest.
7. As **Alice**, call `ticketing.checkIn(0, 0)`.
8. After the `ends at` block, call `ticketing.settleEvent(0)` as **Alice**. She receives the
   10 UNIT of ticket sales.
9. As any account, call `ticketing.purgeEvent(0)`. The event and its ticket are deleted and Alice
   gets her 1 UNIT deposit back.

Inspect state under **Developer → Chain state → ticketing** (`events`, `tickets`, `ticketsHeld`).

## Tests and benchmarks

```sh
cargo test -p pallet-ticketing                                # unit tests
cargo test -p pallet-ticketing --features runtime-benchmarks  # plus benchmark tests
```

Every unit test ends by checking the pallet's storage invariants (`do_try_state`): ticket counts
match `active` and `TicketsHeld`, and every escrow can pay its outstanding refunds plus the
deposit. The same check runs in the `try_state` hook when the runtime is built with
`try-runtime`.

[`scripts/e2e`](../../scripts/e2e/README.md) runs both the resale and the cancellation flows
against a live development chain.

Regenerate `src/weights.rs` after changing an extrinsic:

```sh
cargo build --release -p solochain-starter-runtime --features runtime-benchmarks
frame-omni-bencher v1 benchmark pallet \
  --runtime target/release/wbuild/solochain-starter-runtime/solochain_starter_runtime.compact.compressed.wasm \
  --pallet pallet_ticketing --extrinsic "*" \
  --template pallets/benchmarking/frame-weight-template.hbs \
  --output pallets/ticketing/src/weights.rs
```

Install the bencher with `cargo install --locked frame-omni-bencher@0.29.0` (the version shipped
with Polkadot SDK stable2609).

## Not in v1

- **Proof of Personhood.** `BuyerOrigin` is ready for it, but how a non-People-Chain runtime
  verifies personhood aliases is not documented yet.
- **Paying in other assets** (for example USDC from Asset Hub via XCM), once this runs as a
  parachain.
- **Per-ticket storage deposits.** The flat `EventDeposit` motivates the purge, but buyers don't
  pay for the storage their tickets use.
