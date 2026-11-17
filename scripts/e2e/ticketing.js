// End-to-end check of pallet-ticketing against a running development chain.
//
// Usage, from this folder:  npm ci && npm run ticketing -- [ws://127.0.0.1:9944]
// Exit code: 0 when every check passes, 1 when a check fails, 2 when the script itself fails.
const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { u8aConcat, u8aToHex } = require('@polkadot/util');

const UNIT = 1_000_000_000_000n;
const url = process.argv[2] || 'ws://127.0.0.1:9944';
let failures = 0;

function check(cond, msg) {
  console.log(`${cond ? 'PASS' : 'FAIL'}  ${msg}`);
  if (!cond) failures += 1;
}

async function main() {
  const api = await ApiPromise.create({ provider: new WsProvider(url), noInitWarn: true });
  if (!api.tx.ticketing) throw new Error(`the chain at ${url} has no ticketing pallet`);
  console.log(`connected to "${await api.rpc.system.chain()}" at ${url}\n`);

  const keyring = new Keyring({ type: 'sr25519' });
  const [alice, bob, charlie] = ['//Alice', '//Bob', '//Charlie'].map((s) => keyring.addFromUri(s));

  // Balance checks only compare accounts that did not sign the transaction, so fees don't matter.
  const free = async (who) => (await api.query.system.account(who.address)).data.free.toBigInt();

  // Submit a transaction and wait until it is in a block. Resolves with the dispatch error, if
  // any, and the ticketing events it emitted.
  function send(signer, tx, label) {
    return new Promise((resolve, reject) => {
      let unsub = () => {};
      tx.signAndSend(signer, ({ status, events, dispatchError }) => {
        if (status.isInvalid || status.isDropped || status.isUsurped) {
          unsub();
          reject(new Error(`${label}: transaction ${status.type}`));
          return;
        }
        if (!status.isInBlock) return;
        unsub();
        let err = null;
        if (dispatchError) {
          if (dispatchError.isModule) {
            const meta = api.registry.findMetaError(dispatchError.asModule);
            err = `${meta.section}.${meta.name}`;
          } else {
            err = dispatchError.toString();
          }
        }
        const ticketing = events
          .filter(({ event }) => event.section === 'ticketing')
          .map(({ event }) => ({ method: event.method, data: event.data }));
        console.log(`      ${label}: ${err ? `error ${err}` : 'ok'} ${ticketing.map((e) => e.method).join(', ')}`);
        resolve({ err, ticketing });
      }).then((u) => { unsub = u; }).catch(reject);
    });
  }

  function waitForBlock(target) {
    return new Promise((resolve) => {
      let unsub = () => {};
      api.rpc.chain.subscribeNewHeads((h) => {
        if (h.number.toNumber() >= target) { unsub(); resolve(); }
      }).then((u) => { unsub = u; });
    });
  }

  // Call a pallet view function through the `RuntimeViewFunction` runtime API. Its id, argument
  // types and return type come from the runtime's metadata (version 16 or later).
  async function view(pallet, method, ...args) {
    const metadata = api.runtimeMetadata;
    const fn = metadata.asLatest.pallets
      .find((p) => p.name.toString() === pallet)
      ?.viewFunctions?.find((f) => f.name.toString() === method);
    if (!fn) throw new Error(`no view function ${pallet}.${method} in metadata v${metadata.version}`);
    const input = u8aConcat(...fn.inputs.map((param, i) =>
      api.registry.createTypeUnsafe(api.registry.createLookupType(param.type), [args[i]]).toU8a()));
    const id = fn.id.toU8a();
    // Pass the input as hex: a raw Uint8Array would be read as already length-prefixed.
    const result = await api.call.runtimeViewFunction.executeViewFunction(
      { prefix: id.slice(0, 16), suffix: id.slice(16) }, u8aToHex(input));
    if (result.isErr) throw new Error(`${pallet}.${method} failed: ${result.asErr}`);
    return api.registry.createTypeUnsafe(api.registry.createLookupType(fn.output), [result.asOk.toU8a(true)]);
  }

  // ------------------------------------------------------------------------------------------
  console.log('Scenario 1: sell, resell, check in, settle, purge');
  // ------------------------------------------------------------------------------------------

  // The `development` genesis preset only funds Alice, Bob and their stash accounts.
  let r = await send(alice, api.tx.balances.transferKeepAlive(charlie.address, 100n * UNIT), 'Alice funds Charlie');
  check(!r.err, 'Charlie funded with 100 UNIT');

  let head = (await api.rpc.chain.getHeader()).number.toNumber();
  const eventId = (await api.query.ticketing.nextEventId()).toNumber();
  // Seven transactions must land before the event starts, one per block at best.
  const startsAt = head + 14;
  const endsAt = head + 18;
  console.log(`      chain at block ${head}; event ${eventId} starts at ${startsAt}, ends at ${endsAt}`);

  r = await send(alice, api.tx.ticketing.createEvent(
    'Rust Meetup', 100, 10n * UNIT, 2, 20, 10, 0, startsAt, endsAt), 'Alice createEvent');
  check(!r.err && r.ticketing[0]?.method === 'EventCreated', 'Alice created the event');

  r = await send(bob, api.tx.ticketing.buyTicket(eventId), 'Bob buyTicket');
  check(!r.err && r.ticketing[0]?.method === 'TicketPurchased', 'Bob bought ticket 0 at face value');

  const cap = await view('Ticketing', 'max_resale_price', eventId);
  check(cap.isSome && cap.unwrap().toBigInt() === 12n * UNIT, 'view max_resale_price = 12 UNIT (10 + 20%)');
  const held = await view('Ticketing', 'tickets_held_by', eventId, bob.address);
  check(held.toNumber() === 1, 'view tickets_held_by(Bob) = 1');
  const escrow = await view('Ticketing', 'escrow_balance', eventId);
  check(escrow.toBigInt() === 11n * UNIT, 'view escrow_balance = 11 UNIT (deposit + one sale)');

  r = await send(bob, api.tx.ticketing.listTicket(eventId, 0, 13n * UNIT), 'Bob listTicket 13 UNIT');
  check(r.err === 'ticketing.PriceAboveResaleCap', 'listing above the 20% cap is rejected');

  r = await send(bob, api.tx.ticketing.listTicket(eventId, 0, 12n * UNIT), 'Bob listTicket 12 UNIT');
  check(!r.err, 'listing at the cap (12 UNIT) is accepted');

  let aliceBefore = await free(alice);
  const bobBefore = await free(bob);
  r = await send(charlie, api.tx.ticketing.buyListedTicket(eventId, 0, 12n * UNIT), 'Charlie buyListedTicket');
  check(!r.err && r.ticketing[0]?.method === 'TicketResold', 'Charlie bought the listed ticket');
  check((await free(alice)) - aliceBefore === 12n * UNIT / 10n, 'organizer Alice received the 10% royalty (1.2 UNIT)');
  check((await free(bob)) - bobBefore === 12n * UNIT * 9n / 10n, 'seller Bob received 10.8 UNIT');
  const ticket = (await api.query.ticketing.tickets(eventId, 0)).unwrap();
  check(ticket.owner.toString() === charlie.address, 'ticket 0 now belongs to Charlie');

  r = await send(alice, api.tx.ticketing.checkIn(eventId, 0), 'Alice checkIn');
  check(!r.err && r.ticketing[0]?.method === 'TicketCheckedIn', 'Alice checked Charlie in');

  r = await send(charlie, api.tx.ticketing.listTicket(eventId, 0, 10n * UNIT), 'Charlie listTicket');
  check(r.err === 'ticketing.AlreadyCheckedIn', 'a checked-in ticket cannot be resold');

  r = await send(alice, api.tx.ticketing.settleEvent(eventId), 'Alice settleEvent (too early)');
  check(r.err === 'ticketing.EventNotEnded', 'settling before the end is rejected');

  // ------------------------------------------------------------------------------------------
  console.log('\nScenario 2: cancel, refund, settle, purge');
  // ------------------------------------------------------------------------------------------

  head = (await api.rpc.chain.getHeader()).number.toNumber();
  const gigId = (await api.query.ticketing.nextEventId()).toNumber();
  r = await send(alice, api.tx.ticketing.createEvent(
    'Cancelled Gig', 10, 5n * UNIT, 2, 0, 0, 0, head + 100, head + 200), 'Alice createEvent');
  check(!r.err, `Alice created event ${gigId}`);
  r = await send(bob, api.tx.ticketing.buyTicket(gigId), 'Bob buyTicket');
  check(!r.err, 'Bob bought ticket 0');
  r = await send(charlie, api.tx.ticketing.buyTicket(gigId), 'Charlie buyTicket');
  check(!r.err, 'Charlie bought ticket 1');

  r = await send(alice, api.tx.ticketing.cancelEvent(gigId), 'Alice cancelEvent');
  check(!r.err && r.ticketing[0]?.method === 'EventCancelled', 'Alice cancelled the event');

  let before = await free(bob);
  r = await send(charlie, api.tx.ticketing.refundTicket(gigId, 0), "Charlie refunds Bob's ticket");
  check(!r.err && (await free(bob)) - before === 5n * UNIT, 'anyone can push a refund: Bob got 5 UNIT back');

  r = await send(alice, api.tx.ticketing.settleEvent(gigId), 'Alice settleEvent (refund pending)');
  check(r.err === 'ticketing.RefundsOutstanding', 'settling with a refund outstanding is rejected');

  before = await free(charlie);
  r = await send(alice, api.tx.ticketing.refundTicket(gigId, 1), "Alice refunds Charlie's ticket");
  check(!r.err && (await free(charlie)) - before === 5n * UNIT, 'Charlie got 5 UNIT back');

  r = await send(alice, api.tx.ticketing.settleEvent(gigId), 'Alice settleEvent');
  check(!r.err && r.ticketing[0]?.data?.payout?.toBigInt() === 0n, 'nothing to pay out after full refunds');

  aliceBefore = await free(alice);
  r = await send(bob, api.tx.ticketing.purgeEvent(gigId), 'Bob purgeEvent');
  check(!r.err && r.ticketing[0]?.method === 'EventPurged', 'Bob purged the cancelled event');
  check((await free(alice)) - aliceBefore === UNIT, 'Alice got her 1 UNIT deposit back');

  // ------------------------------------------------------------------------------------------
  console.log(`\nScenario 1, continued: settle and purge event ${eventId}`);
  // ------------------------------------------------------------------------------------------

  console.log(`      waiting for block ${endsAt}...`);
  await waitForBlock(endsAt);
  r = await send(alice, api.tx.ticketing.settleEvent(eventId), 'Alice settleEvent');
  // Named event fields are exposed as getters on the event data.
  const payout = r.ticketing[0]?.data?.payout?.toBigInt() ?? -1n;
  check(!r.err && payout === 10n * UNIT, `settling paid out the 10 UNIT of sales (got ${payout})`);
  check((await view('Ticketing', 'escrow_balance', eventId)).toBigInt() === UNIT, 'the 1 UNIT deposit stays in escrow');

  aliceBefore = await free(alice);
  r = await send(bob, api.tx.ticketing.purgeEvent(eventId), 'Bob purgeEvent');
  check(!r.err && r.ticketing[0]?.method === 'EventPurged', 'Bob purged the settled event');
  check((await free(alice)) - aliceBefore === UNIT, 'Alice got her 1 UNIT deposit back');
  check((await api.query.ticketing.events(eventId)).isNone, 'the event is gone from storage');
  check((await api.query.ticketing.tickets(eventId, 0)).isNone, 'its tickets are gone from storage');

  await api.disconnect();
  console.log(failures === 0 ? '\nALL CHECKS PASSED' : `\n${failures} CHECK(S) FAILED`);
  process.exit(failures === 0 ? 0 : 1);
}

main().catch((e) => { console.error(e); process.exit(2); });
