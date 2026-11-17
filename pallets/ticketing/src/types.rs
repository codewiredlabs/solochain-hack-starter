//! Types stored by the ticketing pallet.
//!
//! The structs are generic over plain types (`AccountId`, `Balance`, ...) rather than over the
//! pallet's `T: Config`, so the standard derives work without `NoBound` helpers. `lib.rs` defines
//! `EventDetailsOf<T>` and `TicketOf<T>` aliases that plug in the concrete runtime types.

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::Percent;

/// Identifier of an event. Allocated sequentially from [`crate::NextEventId`].
pub type EventId = u32;

/// Position of a ticket within its event. The n-th ticket sold for an event gets index `n - 1`.
pub type TicketIndex = u32;

/// Lifecycle of an event.
///
/// ```text
///               cancel_event (before starts_at)
///  Scheduled ─────────────────────────────────► Cancelled
///      │                                            │
///      │ settle_event (after ends_at)               │ settle_event (after every ticket is refunded)
///      ▼                                            ▼
///   Settled ◄───────────────────────────────────────┘
///      │
///      │ purge_event (repeated until no tickets are left)
///      ▼
///   deleted from storage, deposit returned
/// ```
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
pub enum EventStatus {
	/// Tickets can be bought and resold until `starts_at`, and checked in until `ends_at`.
	Scheduled,
	/// The organizer called off the event. Every ticket can be refunded at face value.
	Cancelled,
	/// The ticket sales were paid out to the organizer. The event is closed for good and can be
	/// purged from storage.
	Settled,
}

/// Everything the chain knows about an event.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
pub struct EventDetails<AccountId, Balance, BlockNumber, Name> {
	/// The account that created the event. It checks tickets in and receives the proceeds.
	pub organizer: AccountId,
	/// Human-readable event name, bounded by `Config::MaxNameLength`.
	pub name: Name,
	/// Maximum number of tickets that will ever be sold.
	pub capacity: u32,
	/// Tickets sold so far. Also the [`TicketIndex`] the next ticket will get.
	pub sold: u32,
	/// Tickets in storage: sold, not refunded and not yet purged. Settling a cancelled event
	/// requires this to reach zero.
	pub active: u32,
	/// Face value of one ticket, paid into the event's escrow account.
	pub price: Balance,
	/// Maximum number of tickets one buyer may hold for this event.
	pub max_per_buyer: u32,
	/// Maximum resale markup over face value. `Percent::from_percent(20)` caps resale at 120%.
	pub resale_cap: Percent,
	/// Share of every resale price paid to the organizer.
	pub royalty: Percent,
	/// First block at which primary sales are open.
	pub sale_start: BlockNumber,
	/// Block at which the event starts. Sales, resales and cancellation close here.
	pub starts_at: BlockNumber,
	/// Block at which the event ends. Check-in closes and the organizer can settle.
	pub ends_at: BlockNumber,
	/// Amount the organizer locked in escrow when creating the event (`Config::EventDeposit`).
	/// Returned when the event is purged.
	pub deposit: Balance,
	/// Where the event is in its lifecycle.
	pub status: EventStatus,
}

/// A single ticket.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo, MaxEncodedLen)]
pub struct Ticket<AccountId, Balance> {
	/// Current holder of the ticket.
	pub owner: AccountId,
	/// Asking price when the holder has listed the ticket for resale.
	pub listing: Option<Balance>,
	/// Set by the organizer at the door. A checked-in ticket can no longer be resold.
	pub checked_in: bool,
}
