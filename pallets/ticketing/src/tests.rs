use crate::{
	mock::*, Error, Event, EventId, EventStatus, Events, NextEventId, Tickets, TicketsHeld,
};
use frame_support::{
	assert_noop, assert_ok, pallet_prelude::ConstU32, traits::fungible::Inspect, BoundedVec,
};
use sp_runtime::{DispatchResult, Percent, TokenError};

// Defaults used by `create_event`: tickets cost 100, at most 3 tickets, at most 2 per buyer,
// resale capped at +20% (120), 10% royalty, sales open at block 1, the event runs 10..20.
const PRICE: Balance = 100;
const CAPACITY: u32 = 3;
const MAX_PER_BUYER: u32 = 2;
const RESALE_CAP: Percent = Percent::from_percent(20);
const ROYALTY: Percent = Percent::from_percent(10);
const SALE_START: u64 = 1;
const STARTS_AT: u64 = 10;
const ENDS_AT: u64 = 20;

fn name(text: &str) -> BoundedVec<u8, ConstU32<32>> {
	text.as_bytes().to_vec().try_into().expect("test names fit the bound")
}

/// Create an event with the default terms except for the given ones.
fn create(
	capacity: u32,
	price: Balance,
	max_per_buyer: u32,
	sale_start: u64,
	starts_at: u64,
	ends_at: u64,
) -> DispatchResult {
	Ticketing::create_event(
		RuntimeOrigin::signed(ORGANIZER),
		name("Rust Meetup"),
		capacity,
		price,
		max_per_buyer,
		RESALE_CAP,
		ROYALTY,
		sale_start,
		starts_at,
		ends_at,
	)
}

fn create_event_with_price(price: Balance) -> EventId {
	let event_id = NextEventId::<Test>::get();
	assert_ok!(create(CAPACITY, price, MAX_PER_BUYER, SALE_START, STARTS_AT, ENDS_AT));
	event_id
}

fn create_event() -> EventId {
	create_event_with_price(PRICE)
}

fn buy(who: AccountId, event_id: EventId) -> DispatchResult {
	Ticketing::buy_ticket(RuntimeOrigin::signed(who), event_id)
}

fn balance(who: AccountId) -> Balance {
	Balances::balance(&who)
}

fn escrow(event_id: EventId) -> Balance {
	balance(Ticketing::escrow_account(event_id))
}

// ---------------------------------------------------------------------------------------------
// create_event
// ---------------------------------------------------------------------------------------------

#[test]
fn create_event_locks_deposit_in_escrow() {
	new_test_ext().execute_with(|| {
		let event_id = create_event();

		assert_eq!(event_id, 0);
		assert_eq!(NextEventId::<Test>::get(), 1);
		let event = Events::<Test>::get(event_id).unwrap();
		assert_eq!(event.organizer, ORGANIZER);
		assert_eq!(event.name, name("Rust Meetup"));
		assert_eq!(event.status, EventStatus::Scheduled);
		assert_eq!((event.sold, event.active, event.deposit), (0, 0, EVENT_DEPOSIT));

		assert_eq!(escrow(event_id), EVENT_DEPOSIT);
		assert_eq!(balance(ORGANIZER), INITIAL_BALANCE - EVENT_DEPOSIT);
		System::assert_last_event(Event::EventCreated { event_id, organizer: ORGANIZER }.into());

		// Each event gets its own escrow account.
		let second = create_event();
		assert_ne!(Ticketing::escrow_account(event_id), Ticketing::escrow_account(second));
	});
}

#[test]
fn create_event_validates_terms() {
	new_test_ext().execute_with(|| {
		assert_noop!(create(0, PRICE, 1, 1, 10, 20), Error::<Test>::InvalidCapacity);
		assert_noop!(create(MAX_CAPACITY + 1, PRICE, 1, 1, 10, 20), Error::<Test>::InvalidCapacity);
		assert_noop!(create(3, PRICE, 0, 1, 10, 20), Error::<Test>::InvalidPurchaseLimit);
		assert_noop!(
			create(3, EXISTENTIAL_DEPOSIT - 1, 1, 1, 10, 20),
			Error::<Test>::PriceBelowMinimum
		);
		// The current block is 1, so the event must start after it.
		assert_noop!(create(3, PRICE, 1, 0, 1, 20), Error::<Test>::InvalidSchedule);
		assert_noop!(create(3, PRICE, 1, 10, 10, 20), Error::<Test>::InvalidSchedule);
		assert_noop!(create(3, PRICE, 1, 1, 10, 10), Error::<Test>::InvalidSchedule);

		// Free events and events priced exactly at the existential deposit are fine.
		assert_ok!(create(3, 0, 1, 1, 10, 20));
		assert_ok!(create(3, EXISTENTIAL_DEPOSIT, 1, 1, 10, 20));
	});
}

// ---------------------------------------------------------------------------------------------
// buy_ticket
// ---------------------------------------------------------------------------------------------

#[test]
fn buy_ticket_moves_payment_into_escrow() {
	new_test_ext().execute_with(|| {
		let event_id = create_event();

		assert_ok!(buy(ALICE, event_id));

		let ticket = Tickets::<Test>::get(event_id, 0).unwrap();
		assert_eq!(ticket.owner, ALICE);
		assert_eq!(ticket.listing, None);
		assert!(!ticket.checked_in);
		let event = Events::<Test>::get(event_id).unwrap();
		assert_eq!((event.sold, event.active), (1, 1));
		assert_eq!(TicketsHeld::<Test>::get(event_id, ALICE), 1);

		assert_eq!(balance(ALICE), INITIAL_BALANCE - PRICE);
		assert_eq!(escrow(event_id), EVENT_DEPOSIT + PRICE);
		System::assert_last_event(
			Event::TicketPurchased { event_id, ticket: 0, buyer: ALICE, price: PRICE }.into(),
		);
	});
}

#[test]
fn buy_ticket_respects_the_sale_window() {
	new_test_ext().execute_with(|| {
		let event_id = NextEventId::<Test>::get();
		assert_ok!(create(CAPACITY, PRICE, MAX_PER_BUYER, 5, STARTS_AT, ENDS_AT));

		assert_noop!(buy(ALICE, event_id), Error::<Test>::SaleNotStarted);
		System::set_block_number(5);
		assert_ok!(buy(ALICE, event_id));
		System::set_block_number(STARTS_AT);
		assert_noop!(buy(BOB, event_id), Error::<Test>::SalesClosed);
	});
}

#[test]
fn buy_ticket_enforces_capacity_and_per_buyer_limit() {
	new_test_ext().execute_with(|| {
		let event_id = create_event();

		assert_ok!(buy(ALICE, event_id));
		assert_ok!(buy(ALICE, event_id));
		assert_noop!(buy(ALICE, event_id), Error::<Test>::PurchaseLimitReached);

		assert_ok!(buy(BOB, event_id));
		assert_noop!(buy(CHARLIE, event_id), Error::<Test>::SoldOut);

		assert_eq!(Tickets::<Test>::get(event_id, 2).unwrap().owner, BOB);
	});
}

#[test]
fn buy_ticket_fails_for_unknown_or_cancelled_events() {
	new_test_ext().execute_with(|| {
		assert_noop!(buy(ALICE, 42), Error::<Test>::EventNotFound);

		let event_id = create_event();
		assert_ok!(Ticketing::cancel_event(RuntimeOrigin::signed(ORGANIZER), event_id));
		assert_noop!(buy(ALICE, event_id), Error::<Test>::EventNotScheduled);
	});
}

#[test]
fn buy_ticket_cannot_reap_the_buyer() {
	new_test_ext().execute_with(|| {
		let event_id = create_event();

		// Paying 100 would leave Dave with 5, below the existential deposit of 10. Purchases use
		// `Preservation::Preserve`, so the transfer fails instead of deleting Dave's account.
		assert_noop!(buy(DAVE, event_id), TokenError::NotExpendable);
	});
}

// ---------------------------------------------------------------------------------------------
// Resale
// ---------------------------------------------------------------------------------------------

#[test]
fn resale_is_capped_and_pays_a_royalty() {
	new_test_ext().execute_with(|| {
		let event_id = create_event();
		assert_ok!(buy(ALICE, event_id));
		let cap = PRICE + RESALE_CAP.mul_floor(PRICE);
		assert_eq!(cap, 120);

		assert_noop!(
			Ticketing::list_ticket(RuntimeOrigin::signed(ALICE), event_id, 0, cap + 1),
			Error::<Test>::PriceAboveResaleCap
		);
		assert_noop!(
			Ticketing::list_ticket(RuntimeOrigin::signed(BOB), event_id, 0, cap),
			Error::<Test>::NotTicketOwner
		);
		assert_ok!(Ticketing::list_ticket(RuntimeOrigin::signed(ALICE), event_id, 0, cap));
		System::assert_last_event(Event::TicketListed { event_id, ticket: 0, price: cap }.into());

		// Bob's `max_price` protects him from paying more than he agreed to.
		assert_noop!(
			Ticketing::buy_listed_ticket(RuntimeOrigin::signed(BOB), event_id, 0, cap - 1),
			Error::<Test>::PriceAboveMax
		);

		let organizer_before = balance(ORGANIZER);
		let alice_before = balance(ALICE);
		assert_ok!(Ticketing::buy_listed_ticket(RuntimeOrigin::signed(BOB), event_id, 0, cap));

		let royalty = 12; // 10% of 120
		assert_eq!(balance(BOB), INITIAL_BALANCE - cap);
		assert_eq!(balance(ALICE), alice_before + cap - royalty);
		assert_eq!(balance(ORGANIZER), organizer_before + royalty);
		// The escrow still holds the face value for a possible refund.
		assert_eq!(escrow(event_id), EVENT_DEPOSIT + PRICE);

		let ticket = Tickets::<Test>::get(event_id, 0).unwrap();
		assert_eq!((ticket.owner, ticket.listing), (BOB, None));
		assert!(!TicketsHeld::<Test>::contains_key(event_id, ALICE));
		assert_eq!(TicketsHeld::<Test>::get(event_id, BOB), 1);
		System::assert_last_event(
			Event::TicketResold {
				event_id,
				ticket: 0,
				seller: ALICE,
				buyer: BOB,
				price: cap,
				royalty,
			}
			.into(),
		);
	});
}

#[test]
fn listings_can_be_withdrawn_and_not_self_bought() {
	new_test_ext().execute_with(|| {
		let event_id = create_event();
		assert_ok!(buy(ALICE, event_id));
		assert_ok!(Ticketing::list_ticket(RuntimeOrigin::signed(ALICE), event_id, 0, PRICE));

		assert_noop!(
			Ticketing::buy_listed_ticket(RuntimeOrigin::signed(ALICE), event_id, 0, PRICE),
			Error::<Test>::CannotBuyOwnTicket
		);
		assert_noop!(
			Ticketing::delist_ticket(RuntimeOrigin::signed(BOB), event_id, 0),
			Error::<Test>::NotTicketOwner
		);

		assert_ok!(Ticketing::delist_ticket(RuntimeOrigin::signed(ALICE), event_id, 0));
		System::assert_last_event(Event::TicketDelisted { event_id, ticket: 0 }.into());
		assert_eq!(Tickets::<Test>::get(event_id, 0).unwrap().listing, None);

		assert_noop!(
			Ticketing::delist_ticket(RuntimeOrigin::signed(ALICE), event_id, 0),
			Error::<Test>::NotListed
		);
		assert_noop!(
			Ticketing::buy_listed_ticket(RuntimeOrigin::signed(BOB), event_id, 0, PRICE),
			Error::<Test>::NotListed
		);
	});
}

#[test]
fn resale_respects_the_per_buyer_limit() {
	new_test_ext().execute_with(|| {
		let event_id = create_event();
		assert_ok!(buy(ALICE, event_id));
		assert_ok!(buy(BOB, event_id));
		assert_ok!(buy(BOB, event_id));
		assert_ok!(Ticketing::list_ticket(RuntimeOrigin::signed(ALICE), event_id, 0, PRICE));

		assert_noop!(
			Ticketing::buy_listed_ticket(RuntimeOrigin::signed(BOB), event_id, 0, PRICE),
			Error::<Test>::PurchaseLimitReached
		);
	});
}

#[test]
fn resale_closes_when_the_event_starts() {
	new_test_ext().execute_with(|| {
		let event_id = create_event();
		assert_ok!(buy(ALICE, event_id));
		assert_ok!(Ticketing::list_ticket(RuntimeOrigin::signed(ALICE), event_id, 0, PRICE));

		System::set_block_number(STARTS_AT);
		assert_noop!(
			Ticketing::list_ticket(RuntimeOrigin::signed(ALICE), event_id, 0, PRICE),
			Error::<Test>::SalesClosed
		);
		assert_noop!(
			Ticketing::buy_listed_ticket(RuntimeOrigin::signed(BOB), event_id, 0, PRICE),
			Error::<Test>::SalesClosed
		);
	});
}

// ---------------------------------------------------------------------------------------------
// check_in
// ---------------------------------------------------------------------------------------------

#[test]
fn check_in_admits_once_and_ends_resale() {
	new_test_ext().execute_with(|| {
		let event_id = create_event();
		assert_ok!(buy(ALICE, event_id));
		assert_ok!(Ticketing::list_ticket(RuntimeOrigin::signed(ALICE), event_id, 0, PRICE));

		assert_noop!(
			Ticketing::check_in(RuntimeOrigin::signed(ALICE), event_id, 0),
			Error::<Test>::NotOrganizer
		);
		assert_noop!(
			Ticketing::check_in(RuntimeOrigin::signed(ORGANIZER), event_id, 7),
			Error::<Test>::TicketNotFound
		);

		System::set_block_number(STARTS_AT);
		assert_ok!(Ticketing::check_in(RuntimeOrigin::signed(ORGANIZER), event_id, 0));
		System::assert_last_event(
			Event::TicketCheckedIn { event_id, ticket: 0, owner: ALICE }.into(),
		);

		let ticket = Tickets::<Test>::get(event_id, 0).unwrap();
		assert!(ticket.checked_in);
		assert_eq!(ticket.listing, None);
		assert_noop!(
			Ticketing::check_in(RuntimeOrigin::signed(ORGANIZER), event_id, 0),
			Error::<Test>::AlreadyCheckedIn
		);
	});
}

#[test]
fn checked_in_tickets_cannot_be_listed() {
	new_test_ext().execute_with(|| {
		let event_id = create_event();
		assert_ok!(buy(ALICE, event_id));
		// Doors can open before the event starts, while resale is still open.
		assert_ok!(Ticketing::check_in(RuntimeOrigin::signed(ORGANIZER), event_id, 0));

		assert_noop!(
			Ticketing::list_ticket(RuntimeOrigin::signed(ALICE), event_id, 0, PRICE),
			Error::<Test>::AlreadyCheckedIn
		);
	});
}

#[test]
fn check_in_closes_when_the_event_ends() {
	new_test_ext().execute_with(|| {
		let event_id = create_event();
		assert_ok!(buy(ALICE, event_id));

		System::set_block_number(ENDS_AT);
		assert_noop!(
			Ticketing::check_in(RuntimeOrigin::signed(ORGANIZER), event_id, 0),
			Error::<Test>::EventEnded
		);
	});
}

// ---------------------------------------------------------------------------------------------
// Cancellation and refunds
// ---------------------------------------------------------------------------------------------

#[test]
fn cancelled_events_refund_every_ticket_then_return_the_deposit() {
	new_test_ext().execute_with(|| {
		let event_id = create_event();
		assert_ok!(buy(ALICE, event_id));
		assert_ok!(buy(BOB, event_id));

		assert_noop!(
			Ticketing::cancel_event(RuntimeOrigin::signed(ALICE), event_id),
			Error::<Test>::NotOrganizer
		);
		assert_noop!(
			Ticketing::refund_ticket(RuntimeOrigin::signed(ALICE), event_id, 0),
			Error::<Test>::EventNotCancelled
		);
		assert_ok!(Ticketing::cancel_event(RuntimeOrigin::signed(ORGANIZER), event_id));
		System::assert_last_event(Event::EventCancelled { event_id }.into());
		assert_eq!(Events::<Test>::get(event_id).unwrap().status, EventStatus::Cancelled);
		assert_noop!(
			Ticketing::cancel_event(RuntimeOrigin::signed(ORGANIZER), event_id),
			Error::<Test>::EventNotScheduled
		);

		// The deposit stays locked while refunds are outstanding.
		assert_noop!(
			Ticketing::settle_event(RuntimeOrigin::signed(ORGANIZER), event_id),
			Error::<Test>::RefundsOutstanding
		);

		// Charlie holds no ticket but can still push Alice's refund to her.
		assert_ok!(Ticketing::refund_ticket(RuntimeOrigin::signed(CHARLIE), event_id, 0));
		assert_eq!(balance(ALICE), INITIAL_BALANCE);
		assert_eq!(balance(CHARLIE), INITIAL_BALANCE);
		System::assert_last_event(
			Event::TicketRefunded { event_id, ticket: 0, owner: ALICE, amount: PRICE }.into(),
		);
		assert_noop!(
			Ticketing::refund_ticket(RuntimeOrigin::signed(ALICE), event_id, 0),
			Error::<Test>::TicketNotFound
		);

		assert_ok!(Ticketing::refund_ticket(RuntimeOrigin::signed(BOB), event_id, 1));
		assert_eq!(balance(BOB), INITIAL_BALANCE);
		assert_eq!(escrow(event_id), EVENT_DEPOSIT);
		assert_eq!(Events::<Test>::get(event_id).unwrap().active, 0);
		assert!(!TicketsHeld::<Test>::contains_key(event_id, ALICE));
		assert!(!TicketsHeld::<Test>::contains_key(event_id, BOB));

		assert_ok!(Ticketing::settle_event(RuntimeOrigin::signed(ORGANIZER), event_id));
		assert_eq!(balance(ORGANIZER), INITIAL_BALANCE);
		assert_eq!(escrow(event_id), 0);
		System::assert_last_event(Event::EventSettled { event_id, payout: EVENT_DEPOSIT }.into());
	});
}

#[test]
fn refunds_go_to_the_current_owner_after_a_resale() {
	new_test_ext().execute_with(|| {
		let event_id = create_event();
		assert_ok!(buy(ALICE, event_id));
		assert_ok!(Ticketing::list_ticket(RuntimeOrigin::signed(ALICE), event_id, 0, 120));
		assert_ok!(Ticketing::buy_listed_ticket(RuntimeOrigin::signed(BOB), event_id, 0, 120));
		assert_ok!(Ticketing::cancel_event(RuntimeOrigin::signed(ORGANIZER), event_id));

		let bob_before = balance(BOB);
		assert_ok!(Ticketing::refund_ticket(RuntimeOrigin::signed(ALICE), event_id, 0));
		// Refunds are at face value, not at the resale price.
		assert_eq!(balance(BOB), bob_before + PRICE);
	});
}

#[test]
fn events_cannot_be_cancelled_once_started() {
	new_test_ext().execute_with(|| {
		let event_id = create_event();
		System::set_block_number(STARTS_AT);
		assert_noop!(
			Ticketing::cancel_event(RuntimeOrigin::signed(ORGANIZER), event_id),
			Error::<Test>::EventAlreadyStarted
		);
	});
}

// ---------------------------------------------------------------------------------------------
// Settlement
// ---------------------------------------------------------------------------------------------

#[test]
fn settle_pays_the_escrow_out_after_the_event() {
	new_test_ext().execute_with(|| {
		let event_id = create_event();
		assert_ok!(buy(ALICE, event_id));
		assert_ok!(buy(BOB, event_id));

		assert_noop!(
			Ticketing::settle_event(RuntimeOrigin::signed(ORGANIZER), event_id),
			Error::<Test>::EventNotEnded
		);
		System::set_block_number(ENDS_AT);
		assert_noop!(
			Ticketing::settle_event(RuntimeOrigin::signed(ALICE), event_id),
			Error::<Test>::NotOrganizer
		);

		assert_ok!(Ticketing::settle_event(RuntimeOrigin::signed(ORGANIZER), event_id));

		let payout = EVENT_DEPOSIT + 2 * PRICE;
		assert_eq!(balance(ORGANIZER), INITIAL_BALANCE - EVENT_DEPOSIT + payout);
		assert_eq!(escrow(event_id), 0);
		// The empty escrow account was removed from state.
		assert!(!System::account_exists(&Ticketing::escrow_account(event_id)));
		assert_eq!(Events::<Test>::get(event_id).unwrap().status, EventStatus::Settled);
		System::assert_last_event(Event::EventSettled { event_id, payout }.into());

		assert_noop!(
			Ticketing::settle_event(RuntimeOrigin::signed(ORGANIZER), event_id),
			Error::<Test>::AlreadySettled
		);
		assert_noop!(buy(CHARLIE, event_id), Error::<Test>::EventNotScheduled);
	});
}

#[test]
fn free_events_work_end_to_end() {
	new_test_ext().execute_with(|| {
		let event_id = create_event_with_price(0);

		assert_ok!(buy(ALICE, event_id));
		assert_eq!(balance(ALICE), INITIAL_BALANCE);
		// The resale cap of a free ticket is zero.
		assert_noop!(
			Ticketing::list_ticket(RuntimeOrigin::signed(ALICE), event_id, 0, 1),
			Error::<Test>::PriceAboveResaleCap
		);

		System::set_block_number(ENDS_AT);
		assert_ok!(Ticketing::settle_event(RuntimeOrigin::signed(ORGANIZER), event_id));
		assert_eq!(balance(ORGANIZER), INITIAL_BALANCE);
	});
}

// ---------------------------------------------------------------------------------------------
// View functions
// ---------------------------------------------------------------------------------------------

#[test]
fn view_functions_report_event_state() {
	new_test_ext().execute_with(|| {
		let event_id = create_event();
		assert_ok!(buy(ALICE, event_id));

		assert_eq!(Ticketing::max_resale_price(event_id), Some(120));
		assert_eq!(Ticketing::max_resale_price(42), None);
		assert_eq!(Ticketing::escrow_balance(event_id), EVENT_DEPOSIT + PRICE);
		assert_eq!(Ticketing::tickets_held_by(event_id, ALICE), 1);
		assert_eq!(Ticketing::tickets_held_by(event_id, BOB), 0);
	});
}
