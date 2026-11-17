//! Benchmarks for `pallet_ticketing`.
//!
//! Each benchmark sets up the most expensive path of its extrinsic: the longest event name, a
//! non-zero price and royalty, and an existing listing where one can be cleared.

use super::*;
use crate::Pallet as Ticketing;
use alloc::vec;
use frame_benchmarking::v2::*;
use frame_support::{
	assert_ok,
	traits::{fungible::Mutate, EnsureOrigin, Get},
};
use frame_system::RawOrigin;
use sp_runtime::{
	traits::{BlockNumberProvider, Saturating},
	Percent,
};

const SEED: u32 = 0;

/// Ticket price used by the benchmarks: ten times the existential deposit, and never zero.
fn ticket_price<T: Config>() -> BalanceOf<T> {
	T::Currency::minimum_balance().max(1u32.into()).saturating_mul(10u32.into())
}

/// Give `who` enough to pay deposits, tickets and resale prices many times over.
fn fund<T: Config>(who: &T::AccountId) {
	let amount = T::EventDeposit::get()
		.saturating_add(ticket_price::<T>())
		.saturating_mul(1_000u32.into());
	T::Currency::set_balance(who, amount);
}

fn longest_name<T: Config>() -> BoundedVec<u8, T::MaxNameLength> {
	vec![b'x'; T::MaxNameLength::get() as usize]
		.try_into()
		.expect("the vector has exactly MaxNameLength bytes")
}

/// Create an event whose sales are open now, start in 10 blocks and end in 20.
fn create_open_event<T: Config>(organizer: &T::AccountId) -> EventId {
	fund::<T>(organizer);
	let now = T::BlockNumberProvider::current_block_number();
	let event_id = NextEventId::<T>::get();
	assert_ok!(Ticketing::<T>::create_event(
		RawOrigin::Signed(organizer.clone()).into(),
		longest_name::<T>(),
		T::MaxCapacity::get(),
		ticket_price::<T>(),
		T::MaxCapacity::get(),
		Percent::from_percent(50),
		Percent::from_percent(10),
		now,
		now.saturating_add(10u32.into()),
		now.saturating_add(20u32.into()),
	));
	event_id
}

/// Buy a ticket for `holder` without going through `BuyerOrigin`.
fn issue_ticket<T: Config>(event_id: EventId, holder: &T::AccountId) -> TicketIndex {
	fund::<T>(holder);
	Ticketing::<T>::do_buy_ticket(holder.clone(), event_id).expect("sales are open")
}

fn list<T: Config>(event_id: EventId, ticket: TicketIndex, holder: &T::AccountId) -> BalanceOf<T> {
	let event = Events::<T>::get(event_id).expect("the event exists");
	let price = Ticketing::<T>::resale_price_cap(&event);
	assert_ok!(Ticketing::<T>::list_ticket(
		RawOrigin::Signed(holder.clone()).into(),
		event_id,
		ticket,
		price
	));
	price
}

/// An origin accepted by `BuyerOrigin`, and the funded account it resolves to.
fn buyer<T: Config>() -> Result<(T::RuntimeOrigin, T::AccountId), BenchmarkError> {
	let origin = T::BuyerOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
	let who =
		T::BuyerOrigin::ensure_origin(origin.clone()).map_err(|_| BenchmarkError::Weightless)?;
	fund::<T>(&who);
	Ok((origin, who))
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn create_event() {
		let organizer: T::AccountId = whitelisted_caller();
		fund::<T>(&organizer);
		let name = longest_name::<T>();
		let capacity = T::MaxCapacity::get();
		let price = ticket_price::<T>();
		let now = T::BlockNumberProvider::current_block_number();
		let starts_at = now.saturating_add(10u32.into());
		let ends_at = now.saturating_add(20u32.into());
		let event_id = NextEventId::<T>::get();

		#[extrinsic_call]
		_(
			RawOrigin::Signed(organizer),
			name,
			capacity,
			price,
			capacity,
			Percent::from_percent(50),
			Percent::from_percent(10),
			now,
			starts_at,
			ends_at,
		);

		assert!(Events::<T>::contains_key(event_id));
	}

	#[benchmark]
	fn buy_ticket() -> Result<(), BenchmarkError> {
		let event_id = create_open_event::<T>(&account("organizer", 0, SEED));
		let (origin, who) = buyer::<T>()?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, event_id);

		assert_eq!(Tickets::<T>::get(event_id, 0).map(|ticket| ticket.owner), Some(who));
		Ok(())
	}

	#[benchmark]
	fn list_ticket() {
		let event_id = create_open_event::<T>(&account("organizer", 0, SEED));
		let holder: T::AccountId = whitelisted_caller();
		let ticket = issue_ticket::<T>(event_id, &holder);
		let event = Events::<T>::get(event_id).expect("the event exists");
		let price = Ticketing::<T>::resale_price_cap(&event);

		#[extrinsic_call]
		_(RawOrigin::Signed(holder), event_id, ticket, price);

		assert_eq!(Tickets::<T>::get(event_id, ticket).and_then(|t| t.listing), Some(price));
	}

	#[benchmark]
	fn delist_ticket() {
		let event_id = create_open_event::<T>(&account("organizer", 0, SEED));
		let holder: T::AccountId = whitelisted_caller();
		let ticket = issue_ticket::<T>(event_id, &holder);
		list::<T>(event_id, ticket, &holder);

		#[extrinsic_call]
		_(RawOrigin::Signed(holder), event_id, ticket);

		assert_eq!(Tickets::<T>::get(event_id, ticket).and_then(|t| t.listing), None);
	}

	#[benchmark]
	fn buy_listed_ticket() -> Result<(), BenchmarkError> {
		let event_id = create_open_event::<T>(&account("organizer", 0, SEED));
		let seller: T::AccountId = account("seller", 0, SEED);
		let ticket = issue_ticket::<T>(event_id, &seller);
		let price = list::<T>(event_id, ticket, &seller);
		let (origin, who) = buyer::<T>()?;

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, event_id, ticket, price);

		assert_eq!(Tickets::<T>::get(event_id, ticket).map(|t| t.owner), Some(who));
		Ok(())
	}

	#[benchmark]
	fn check_in() {
		let organizer: T::AccountId = whitelisted_caller();
		let event_id = create_open_event::<T>(&organizer);
		let holder: T::AccountId = account("holder", 0, SEED);
		let ticket = issue_ticket::<T>(event_id, &holder);
		list::<T>(event_id, ticket, &holder);

		#[extrinsic_call]
		_(RawOrigin::Signed(organizer), event_id, ticket);

		assert_eq!(Tickets::<T>::get(event_id, ticket).map(|t| t.checked_in), Some(true));
	}

	#[benchmark]
	fn cancel_event() {
		let organizer: T::AccountId = whitelisted_caller();
		let event_id = create_open_event::<T>(&organizer);

		#[extrinsic_call]
		_(RawOrigin::Signed(organizer), event_id);

		assert_eq!(Events::<T>::get(event_id).map(|e| e.status), Some(EventStatus::Cancelled));
	}

	#[benchmark]
	fn refund_ticket() {
		let organizer: T::AccountId = account("organizer", 0, SEED);
		let event_id = create_open_event::<T>(&organizer);
		let holder: T::AccountId = account("holder", 0, SEED);
		let ticket = issue_ticket::<T>(event_id, &holder);
		assert_ok!(Ticketing::<T>::cancel_event(RawOrigin::Signed(organizer).into(), event_id));
		let caller: T::AccountId = whitelisted_caller();

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), event_id, ticket);

		assert!(!Tickets::<T>::contains_key(event_id, ticket));
	}

	#[benchmark]
	fn settle_event() {
		let organizer: T::AccountId = whitelisted_caller();
		let event_id = create_open_event::<T>(&organizer);
		issue_ticket::<T>(event_id, &account("holder", 0, SEED));
		let ends_at = Events::<T>::get(event_id).expect("the event exists").ends_at;
		T::BlockNumberProvider::set_block_number(ends_at);

		#[extrinsic_call]
		_(RawOrigin::Signed(organizer), event_id);

		assert_eq!(Events::<T>::get(event_id).map(|e| e.status), Some(EventStatus::Settled));
	}

	impl_benchmark_test_suite!(Ticketing, crate::mock::new_test_ext(), crate::mock::Test);
}
