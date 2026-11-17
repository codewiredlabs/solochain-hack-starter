//! # Ticketing Pallet
//!
//! Sell event tickets on-chain, with the money held in escrow until the event is over and
//! resale rules that stop scalpers from flipping tickets at any price.
//!
//! ## Overview
//!
//! - An **organizer** creates an event with a capacity, a face-value price, a per-buyer limit, a
//!   resale cap, a royalty and three block numbers: when sales open (`sale_start`), when the event
//!   starts (`starts_at`) and when it ends (`ends_at`).
//! - Every event gets its own **escrow account**, derived from the pallet's `PalletId` and the
//!   event id. The organizer's `EventDeposit` goes in first, so the escrow account always exists
//!   and never drops below the existential deposit while tickets are outstanding.
//! - **Buyers** pay face value into the escrow account. Nobody, including the organizer, can touch
//!   that money before the event ends.
//! - A holder can **list** a ticket for resale at no more than `price + resale_cap`. Another buyer
//!   pays the listed price: the organizer receives `royalty` of it and the seller the rest. There
//!   is deliberately no free transfer extrinsic, because a "gift" after an off-chain sale would
//!   bypass the resale cap.
//! - The organizer **checks tickets in** at the door. A checked-in ticket can no longer be resold.
//! - After `ends_at`, the organizer **settles** the event and receives the whole escrow: ticket
//!   sales plus the deposit.
//! - If the organizer **cancels** before `starts_at`, every ticket becomes refundable at face
//!   value. Anyone may trigger a refund (the money always goes to the ticket's owner), and the
//!   organizer only gets the deposit back once every ticket has been refunded.
//!
//! ## Moving to a parachain
//!
//! Two `Config` items exist so this pallet can move to a parachain without code changes:
//!
//! - `BuyerOrigin` decides who may buy. The solochain uses plain signed accounts. It is the place
//!   to plug in a "one person, one purchase" check, such as Polkadot's Proof of Personhood, once
//!   that is consumable from a parachain.
//! - `BlockNumberProvider` decides which clock event times use. A parachain should use relay chain
//!   block numbers, because parachain block times change with async backing and elastic scaling.

#![cfg_attr(not(feature = "std"), no_std)]
// `#[pallet::call]` generates a constructor taking every argument of `create_event`, and an
// attribute on the function itself cannot reach that generated code. The Polkadot SDK
// workspace allows this lint globally for the same reason.
#![allow(clippy::too_many_arguments)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod types;
pub mod weights;

pub use types::*;
pub use weights::WeightInfo;

use frame_support::{traits::fungible::Inspect, BoundedVec};
use sp_runtime::traits::BlockNumberProvider;

/// Balance type of the currency tickets are paid in.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

/// Block number type of `Config::BlockNumberProvider`. All event times are expressed in it.
pub type ProvidedBlockNumberFor<T> =
	<<T as Config>::BlockNumberProvider as BlockNumberProvider>::BlockNumber;

/// [`EventDetails`] with this runtime's concrete types.
pub type EventDetailsOf<T> = EventDetails<
	<T as frame_system::Config>::AccountId,
	BalanceOf<T>,
	ProvidedBlockNumberFor<T>,
	BoundedVec<u8, <T as Config>::MaxNameLength>,
>;

/// [`Ticket`] with this runtime's concrete types.
pub type TicketOf<T> = Ticket<<T as frame_system::Config>::AccountId, BalanceOf<T>>;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungible::Mutate,
			tokens::{Fortitude, Preservation},
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{
		traits::{AccountIdConversion, Saturating, Zero},
		Percent,
	};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configuration trait for the pallet.
	///
	/// There is no `type RuntimeEvent`: since Polkadot SDK stable2506 the `#[pallet::config]` macro
	/// adds the `RuntimeEvent` bound automatically.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The currency tickets are paid in.
		type Currency: Mutate<Self::AccountId>;

		/// Origin allowed to buy tickets. Its success value is the account that pays and holds
		/// the ticket.
		///
		/// Use `frame_system::EnsureSigned` for "any signed account". Swap in a stricter origin to
		/// require proof that the buyer is a unique person.
		type BuyerOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::AccountId>;

		/// The clock that sale windows and event times are measured against.
		///
		/// A solochain uses `frame_system::Pallet<Runtime>`. A parachain should use
		/// `cumulus_pallet_parachain_system::RelaychainDataProvider<Runtime>`.
		type BlockNumberProvider: BlockNumberProvider;

		/// Identifier used to derive each event's escrow account.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Amount the organizer locks in the event's escrow when creating an event. It is returned
		/// on settlement. Must be at least the existential deposit (checked in `integrity_test`).
		#[pallet::constant]
		type EventDeposit: Get<BalanceOf<Self>>;

		/// Largest capacity an event may have.
		#[pallet::constant]
		type MaxCapacity: Get<u32>;

		/// Longest event name, in bytes.
		#[pallet::constant]
		type MaxNameLength: Get<u32>;

		/// Weights of this pallet's extrinsics, generated by the benchmarks.
		type WeightInfo: WeightInfo;
	}

	/// Id the next created event will get.
	#[pallet::storage]
	pub type NextEventId<T> = StorageValue<_, EventId, ValueQuery>;

	/// All events, by id.
	///
	/// `Twox64Concat` is safe here because event ids are allocated by the pallet, so users cannot
	/// pick keys that unbalance the storage trie.
	#[pallet::storage]
	pub type Events<T: Config> = StorageMap<_, Twox64Concat, EventId, EventDetailsOf<T>>;

	/// All tickets, by event and ticket index.
	#[pallet::storage]
	pub type Tickets<T: Config> =
		StorageDoubleMap<_, Twox64Concat, EventId, Twox64Concat, TicketIndex, TicketOf<T>>;

	/// Number of tickets each account holds for an event, used to enforce `max_per_buyer`.
	///
	/// Accounts are user-chosen, so they use the cryptographic `Blake2_128Concat` hasher.
	#[pallet::storage]
	pub type TicketsHeld<T: Config> =
		StorageDoubleMap<_, Twox64Concat, EventId, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// An event was created and its deposit locked in escrow.
		EventCreated { event_id: EventId, organizer: T::AccountId },
		/// A ticket was bought at face value.
		TicketPurchased {
			event_id: EventId,
			ticket: TicketIndex,
			buyer: T::AccountId,
			price: BalanceOf<T>,
		},
		/// A ticket was listed for resale.
		TicketListed { event_id: EventId, ticket: TicketIndex, price: BalanceOf<T> },
		/// A resale listing was withdrawn.
		TicketDelisted { event_id: EventId, ticket: TicketIndex },
		/// A listed ticket changed hands. `royalty` of `price` went to the organizer.
		TicketResold {
			event_id: EventId,
			ticket: TicketIndex,
			seller: T::AccountId,
			buyer: T::AccountId,
			price: BalanceOf<T>,
			royalty: BalanceOf<T>,
		},
		/// The organizer admitted a ticket holder.
		TicketCheckedIn { event_id: EventId, ticket: TicketIndex, owner: T::AccountId },
		/// The organizer cancelled an event. Its tickets are now refundable.
		EventCancelled { event_id: EventId },
		/// A ticket of a cancelled event was refunded at face value.
		TicketRefunded {
			event_id: EventId,
			ticket: TicketIndex,
			owner: T::AccountId,
			amount: BalanceOf<T>,
		},
		/// The escrow was paid out to the organizer.
		EventSettled { event_id: EventId, payout: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// No event with this id.
		EventNotFound,
		/// No ticket with this index for the event.
		TicketNotFound,
		/// Capacity must be between 1 and `MaxCapacity`.
		InvalidCapacity,
		/// `max_per_buyer` must be at least 1.
		InvalidPurchaseLimit,
		/// Times must satisfy `now < starts_at`, `sale_start < starts_at` and `starts_at <
		/// ends_at`.
		InvalidSchedule,
		/// A non-zero price must be at least the existential deposit, so refunds can always be
		/// paid out.
		PriceBelowMinimum,
		/// Event ids are exhausted.
		TooManyEvents,
		/// The event is cancelled or settled.
		EventNotScheduled,
		/// Ticket sales have not opened yet.
		SaleNotStarted,
		/// The event has started, so sales and resales are closed.
		SalesClosed,
		/// Every ticket has been sold.
		SoldOut,
		/// The buyer already holds `max_per_buyer` tickets for this event.
		PurchaseLimitReached,
		/// Only the ticket's owner can do this.
		NotTicketOwner,
		/// Only the event's organizer can do this.
		NotOrganizer,
		/// The ticket has already been checked in.
		AlreadyCheckedIn,
		/// The asking price is above `price + resale_cap`.
		PriceAboveResaleCap,
		/// The ticket is not listed for resale.
		NotListed,
		/// A holder cannot buy their own listing.
		CannotBuyOwnTicket,
		/// The listing costs more than the buyer's `max_price`.
		PriceAboveMax,
		/// The event has ended, so tickets can no longer be checked in.
		EventEnded,
		/// The event has started, so it can no longer be cancelled.
		EventAlreadyStarted,
		/// Refunds are only available for cancelled events.
		EventNotCancelled,
		/// The event has not ended yet, so the escrow cannot be paid out.
		EventNotEnded,
		/// Some tickets of the cancelled event have not been refunded yet.
		RefundsOutstanding,
		/// The event was already settled.
		AlreadySettled,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn integrity_test() {
			assert!(
				T::EventDeposit::get() >= T::Currency::minimum_balance(),
				"EventDeposit must be at least the existential deposit, or escrow accounts could be reaped",
			);
			assert!(T::MaxCapacity::get() > 0, "MaxCapacity must be at least 1");
			// Escrow accounts are `"modl" ++ PalletId ++ event_id` (16 bytes), truncated to the
			// size of `AccountId`. With a shorter `AccountId` every event would share one escrow.
			assert!(
				Self::escrow_account(0) != Self::escrow_account(1),
				"AccountId is too short to derive a separate escrow account per event",
			);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create an event and lock `EventDeposit` from the caller in its escrow account.
		///
		/// - `price`: face value of one ticket. Zero makes a free event; otherwise it must be at
		///   least the existential deposit.
		/// - `resale_cap`: maximum resale markup over `price`.
		/// - `royalty`: share of each resale price paid to the organizer.
		/// - `sale_start`, `starts_at`, `ends_at`: block numbers of the `BlockNumberProvider`.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_event())]
		pub fn create_event(
			origin: OriginFor<T>,
			name: BoundedVec<u8, T::MaxNameLength>,
			capacity: u32,
			#[pallet::compact] price: BalanceOf<T>,
			max_per_buyer: u32,
			resale_cap: Percent,
			royalty: Percent,
			sale_start: ProvidedBlockNumberFor<T>,
			starts_at: ProvidedBlockNumberFor<T>,
			ends_at: ProvidedBlockNumberFor<T>,
		) -> DispatchResult {
			let organizer = ensure_signed(origin)?;

			// Check everything before touching balances or storage.
			ensure!(capacity > 0 && capacity <= T::MaxCapacity::get(), Error::<T>::InvalidCapacity);
			ensure!(max_per_buyer > 0, Error::<T>::InvalidPurchaseLimit);
			ensure!(
				price.is_zero() || price >= T::Currency::minimum_balance(),
				Error::<T>::PriceBelowMinimum
			);
			let now = T::BlockNumberProvider::current_block_number();
			ensure!(
				now < starts_at && sale_start < starts_at && starts_at < ends_at,
				Error::<T>::InvalidSchedule
			);

			let event_id = NextEventId::<T>::get();
			let next_event_id = event_id.checked_add(1).ok_or(Error::<T>::TooManyEvents)?;

			let deposit = T::EventDeposit::get();
			T::Currency::transfer(
				&organizer,
				&Self::escrow_account(event_id),
				deposit,
				Preservation::Preserve,
			)?;

			Events::<T>::insert(
				event_id,
				EventDetails {
					organizer: organizer.clone(),
					name,
					capacity,
					sold: 0,
					active: 0,
					price,
					max_per_buyer,
					resale_cap,
					royalty,
					sale_start,
					starts_at,
					ends_at,
					deposit,
					status: EventStatus::Scheduled,
				},
			);
			NextEventId::<T>::put(next_event_id);

			Self::deposit_event(Event::EventCreated { event_id, organizer });
			Ok(())
		}

		/// Buy the next ticket of an event at face value. The payment goes into escrow.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::buy_ticket())]
		pub fn buy_ticket(origin: OriginFor<T>, event_id: EventId) -> DispatchResult {
			let buyer = T::BuyerOrigin::ensure_origin(origin)?;
			Self::do_buy_ticket(buyer, event_id).map(|_| ())
		}

		/// List one of your tickets for resale at `price`, which may not exceed the event's
		/// resale cap. Listing again replaces the previous price.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::list_ticket())]
		pub fn list_ticket(
			origin: OriginFor<T>,
			event_id: EventId,
			ticket: TicketIndex,
			#[pallet::compact] price: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let event = Self::tradable_event(event_id)?;
			ensure!(price <= Self::resale_price_cap(&event), Error::<T>::PriceAboveResaleCap);

			Tickets::<T>::try_mutate(event_id, ticket, |maybe_ticket| -> DispatchResult {
				let ticket = maybe_ticket.as_mut().ok_or(Error::<T>::TicketNotFound)?;
				ensure!(ticket.owner == who, Error::<T>::NotTicketOwner);
				ensure!(!ticket.checked_in, Error::<T>::AlreadyCheckedIn);
				ticket.listing = Some(price);
				Ok(())
			})?;

			Self::deposit_event(Event::TicketListed { event_id, ticket, price });
			Ok(())
		}

		/// Withdraw one of your resale listings.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::delist_ticket())]
		pub fn delist_ticket(
			origin: OriginFor<T>,
			event_id: EventId,
			ticket: TicketIndex,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Tickets::<T>::try_mutate(event_id, ticket, |maybe_ticket| -> DispatchResult {
				let ticket = maybe_ticket.as_mut().ok_or(Error::<T>::TicketNotFound)?;
				ensure!(ticket.owner == who, Error::<T>::NotTicketOwner);
				ensure!(ticket.listing.take().is_some(), Error::<T>::NotListed);
				Ok(())
			})?;

			Self::deposit_event(Event::TicketDelisted { event_id, ticket });
			Ok(())
		}

		/// Buy a listed ticket. Fails if the listing costs more than `max_price`, which protects
		/// the buyer if the seller raises the price after the buyer submitted the transaction.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::buy_listed_ticket())]
		pub fn buy_listed_ticket(
			origin: OriginFor<T>,
			event_id: EventId,
			ticket: TicketIndex,
			#[pallet::compact] max_price: BalanceOf<T>,
		) -> DispatchResult {
			let buyer = T::BuyerOrigin::ensure_origin(origin)?;
			let event = Self::tradable_event(event_id)?;

			let mut details =
				Tickets::<T>::get(event_id, ticket).ok_or(Error::<T>::TicketNotFound)?;
			let price = details.listing.ok_or(Error::<T>::NotListed)?;
			ensure!(details.owner != buyer, Error::<T>::CannotBuyOwnTicket);
			ensure!(price <= max_price, Error::<T>::PriceAboveMax);
			let buyer_held = TicketsHeld::<T>::get(event_id, &buyer);
			ensure!(buyer_held < event.max_per_buyer, Error::<T>::PurchaseLimitReached);

			let royalty = event.royalty.mul_floor(price);
			let seller_amount = price.saturating_sub(royalty);
			T::Currency::transfer(&buyer, &details.owner, seller_amount, Preservation::Preserve)?;
			T::Currency::transfer(&buyer, &event.organizer, royalty, Preservation::Preserve)?;

			let seller = core::mem::replace(&mut details.owner, buyer.clone());
			details.listing = None;
			Tickets::<T>::insert(event_id, ticket, details);
			Self::decrement_held(event_id, &seller);
			TicketsHeld::<T>::insert(event_id, &buyer, buyer_held.saturating_add(1));

			Self::deposit_event(Event::TicketResold {
				event_id,
				ticket,
				seller,
				buyer,
				price,
				royalty,
			});
			Ok(())
		}

		/// Admit a ticket holder. Only the organizer can call this, until the event ends.
		/// Checking in removes any resale listing.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::check_in())]
		pub fn check_in(
			origin: OriginFor<T>,
			event_id: EventId,
			ticket: TicketIndex,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let event = Events::<T>::get(event_id).ok_or(Error::<T>::EventNotFound)?;
			ensure!(event.organizer == who, Error::<T>::NotOrganizer);
			ensure!(event.status == EventStatus::Scheduled, Error::<T>::EventNotScheduled);
			ensure!(
				T::BlockNumberProvider::current_block_number() < event.ends_at,
				Error::<T>::EventEnded
			);

			let owner = Tickets::<T>::try_mutate(
				event_id,
				ticket,
				|maybe_ticket| -> Result<T::AccountId, DispatchError> {
					let ticket = maybe_ticket.as_mut().ok_or(Error::<T>::TicketNotFound)?;
					ensure!(!ticket.checked_in, Error::<T>::AlreadyCheckedIn);
					ticket.checked_in = true;
					ticket.listing = None;
					Ok(ticket.owner.clone())
				},
			)?;

			Self::deposit_event(Event::TicketCheckedIn { event_id, ticket, owner });
			Ok(())
		}

		/// Cancel an event before it starts. Its tickets become refundable at face value.
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::cancel_event())]
		pub fn cancel_event(origin: OriginFor<T>, event_id: EventId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let now = T::BlockNumberProvider::current_block_number();

			Events::<T>::try_mutate(event_id, |maybe_event| -> DispatchResult {
				let event = maybe_event.as_mut().ok_or(Error::<T>::EventNotFound)?;
				ensure!(event.organizer == who, Error::<T>::NotOrganizer);
				ensure!(event.status == EventStatus::Scheduled, Error::<T>::EventNotScheduled);
				ensure!(now < event.starts_at, Error::<T>::EventAlreadyStarted);
				event.status = EventStatus::Cancelled;
				Ok(())
			})?;

			Self::deposit_event(Event::EventCancelled { event_id });
			Ok(())
		}

		/// Refund a ticket of a cancelled event at face value.
		///
		/// Any signed account may call this, and the refund always goes to the ticket's owner.
		/// That lets the organizer (or anyone) push refunds to holders who never claim them, so a
		/// lost key cannot block the organizer's deposit forever.
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::refund_ticket())]
		pub fn refund_ticket(
			origin: OriginFor<T>,
			event_id: EventId,
			ticket: TicketIndex,
		) -> DispatchResult {
			ensure_signed(origin)?;
			let mut event = Events::<T>::get(event_id).ok_or(Error::<T>::EventNotFound)?;
			ensure!(event.status == EventStatus::Cancelled, Error::<T>::EventNotCancelled);
			let details = Tickets::<T>::take(event_id, ticket).ok_or(Error::<T>::TicketNotFound)?;

			// `Preserve` keeps the escrow account alive: it still holds the deposit, which is at
			// least the existential deposit.
			T::Currency::transfer(
				&Self::escrow_account(event_id),
				&details.owner,
				event.price,
				Preservation::Preserve,
			)?;

			event.active.saturating_dec();
			let amount = event.price;
			Events::<T>::insert(event_id, event);
			Self::decrement_held(event_id, &details.owner);

			Self::deposit_event(Event::TicketRefunded {
				event_id,
				ticket,
				owner: details.owner,
				amount,
			});
			Ok(())
		}

		/// Pay the whole escrow (ticket sales plus deposit) out to the organizer and close the
		/// event.
		///
		/// Allowed once a scheduled event has ended, or once every ticket of a cancelled event has
		/// been refunded.
		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::settle_event())]
		pub fn settle_event(origin: OriginFor<T>, event_id: EventId) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let mut event = Events::<T>::get(event_id).ok_or(Error::<T>::EventNotFound)?;
			ensure!(event.organizer == who, Error::<T>::NotOrganizer);
			match event.status {
				EventStatus::Scheduled => ensure!(
					T::BlockNumberProvider::current_block_number() >= event.ends_at,
					Error::<T>::EventNotEnded
				),
				EventStatus::Cancelled => {
					ensure!(event.active == 0, Error::<T>::RefundsOutstanding)
				},
				EventStatus::Settled => return Err(Error::<T>::AlreadySettled.into()),
			}

			// `Expendable` empties the escrow account completely, so the chain reaps it.
			let escrow = Self::escrow_account(event_id);
			let payout = T::Currency::reducible_balance(
				&escrow,
				Preservation::Expendable,
				Fortitude::Polite,
			);
			T::Currency::transfer(&escrow, &event.organizer, payout, Preservation::Expendable)?;

			event.status = EventStatus::Settled;
			Events::<T>::insert(event_id, event);

			Self::deposit_event(Event::EventSettled { event_id, payout });
			Ok(())
		}
	}

	/// Read-only queries that wallets and front ends can call through the
	/// `RuntimeViewFunction` runtime API, without decoding storage themselves.
	#[pallet::view_functions]
	impl<T: Config> Pallet<T> {
		/// Highest price a ticket of `event_id` may be listed at.
		pub fn max_resale_price(event_id: EventId) -> Option<BalanceOf<T>> {
			Events::<T>::get(event_id).map(|event| Self::resale_price_cap(&event))
		}

		/// Balance currently held in the escrow account of `event_id`.
		pub fn escrow_balance(event_id: EventId) -> BalanceOf<T> {
			T::Currency::balance(&Self::escrow_account(event_id))
		}

		/// Number of tickets `who` holds for `event_id`.
		pub fn tickets_held_by(event_id: EventId, who: T::AccountId) -> u32 {
			TicketsHeld::<T>::get(event_id, who)
		}
	}

	impl<T: Config> Pallet<T> {
		/// The escrow account of an event: a sub-account of the pallet account, keyed by event id.
		pub fn escrow_account(event_id: EventId) -> T::AccountId {
			T::PalletId::get().into_sub_account_truncating(event_id)
		}

		/// Face value plus the event's maximum resale markup.
		pub fn resale_price_cap(event: &EventDetailsOf<T>) -> BalanceOf<T> {
			event.price.saturating_add(event.resale_cap.mul_floor(event.price))
		}

		/// Sell the next ticket of `event_id` to `buyer`. Shared by the `buy_ticket` extrinsic
		/// and the benchmarks.
		pub(crate) fn do_buy_ticket(
			buyer: T::AccountId,
			event_id: EventId,
		) -> Result<TicketIndex, DispatchError> {
			let mut event = Events::<T>::get(event_id).ok_or(Error::<T>::EventNotFound)?;
			ensure!(event.status == EventStatus::Scheduled, Error::<T>::EventNotScheduled);
			let now = T::BlockNumberProvider::current_block_number();
			ensure!(now >= event.sale_start, Error::<T>::SaleNotStarted);
			ensure!(now < event.starts_at, Error::<T>::SalesClosed);
			ensure!(event.sold < event.capacity, Error::<T>::SoldOut);
			let held = TicketsHeld::<T>::get(event_id, &buyer);
			ensure!(held < event.max_per_buyer, Error::<T>::PurchaseLimitReached);

			T::Currency::transfer(
				&buyer,
				&Self::escrow_account(event_id),
				event.price,
				Preservation::Preserve,
			)?;

			let ticket = event.sold;
			event.sold.saturating_inc();
			event.active.saturating_inc();
			let price = event.price;
			Events::<T>::insert(event_id, event);
			Tickets::<T>::insert(
				event_id,
				ticket,
				Ticket { owner: buyer.clone(), listing: None, checked_in: false },
			);
			TicketsHeld::<T>::insert(event_id, &buyer, held.saturating_add(1));

			Self::deposit_event(Event::TicketPurchased { event_id, ticket, buyer, price });
			Ok(ticket)
		}

		/// An event whose tickets can still be bought and resold.
		fn tradable_event(event_id: EventId) -> Result<EventDetailsOf<T>, DispatchError> {
			let event = Events::<T>::get(event_id).ok_or(Error::<T>::EventNotFound)?;
			ensure!(event.status == EventStatus::Scheduled, Error::<T>::EventNotScheduled);
			ensure!(
				T::BlockNumberProvider::current_block_number() < event.starts_at,
				Error::<T>::SalesClosed
			);
			Ok(event)
		}

		/// Lower `who`'s ticket count for an event, removing the entry when it reaches zero so
		/// no empty values are left in storage.
		fn decrement_held(event_id: EventId, who: &T::AccountId) {
			TicketsHeld::<T>::mutate_exists(event_id, who, |held| {
				*held = held.and_then(|count| count.checked_sub(1)).filter(|count| *count > 0);
			});
		}
	}
}
