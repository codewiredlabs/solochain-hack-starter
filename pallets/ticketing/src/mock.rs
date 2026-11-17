use crate as pallet_ticketing;
use frame_support::{
	derive_impl, parameter_types,
	traits::{ConstU32, ConstU64},
	PalletId,
};
use frame_system::EnsureSigned;
use sp_runtime::{traits::IdentityLookup, BuildStorage};

type Block = frame_system::mocking::MockBlock<Test>;
/// 16 bytes, so each event's escrow sub-account (`"modl" ++ PalletId ++ event_id`) stays unique.
/// The usual test `u64` would truncate them all to the same account.
pub type AccountId = u128;
pub type Balance = u64;

#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask,
		RuntimeViewFunction
	)]
	pub struct Test;

	#[runtime::pallet_index(0)]
	pub type System = frame_system::Pallet<Test>;

	#[runtime::pallet_index(1)]
	pub type Balances = pallet_balances::Pallet<Test>;

	#[runtime::pallet_index(2)]
	pub type Ticketing = pallet_ticketing::Pallet<Test>;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<AccountId>;
	type AccountData = pallet_balances::AccountData<Balance>;
}

/// Larger than the usual test value of 1, so the tests can exercise the "price below the
/// existential deposit" check.
pub const EXISTENTIAL_DEPOSIT: Balance = 10;
pub const EVENT_DEPOSIT: Balance = 100;
pub const MAX_CAPACITY: u32 = 100;
/// Small, so the tests can purge an event over several calls.
pub const REMOVE_ITEMS_LIMIT: u32 = 2;

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
	type ExistentialDeposit = ConstU64<EXISTENTIAL_DEPOSIT>;
}

parameter_types! {
	pub const TicketingPalletId: PalletId = PalletId(*b"py/tickt");
}

impl pallet_ticketing::Config for Test {
	type Currency = Balances;
	type BuyerOrigin = EnsureSigned<AccountId>;
	type BlockNumberProvider = System;
	type PalletId = TicketingPalletId;
	type EventDeposit = ConstU64<EVENT_DEPOSIT>;
	type MaxCapacity = ConstU32<MAX_CAPACITY>;
	type MaxNameLength = ConstU32<32>;
	type RemoveItemsLimit = ConstU32<REMOVE_ITEMS_LIMIT>;
	type WeightInfo = ();
}

pub const ORGANIZER: AccountId = 1;
pub const ALICE: AccountId = 2;
pub const BOB: AccountId = 3;
pub const CHARLIE: AccountId = 4;
/// Can afford one 100-unit ticket only by dropping below the existential deposit.
pub const DAVE: AccountId = 5;
pub const INITIAL_BALANCE: Balance = 10_000;
pub const DAVE_BALANCE: Balance = 105;

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(ORGANIZER, INITIAL_BALANCE),
			(ALICE, INITIAL_BALANCE),
			(BOB, INITIAL_BALANCE),
			(CHARLIE, INITIAL_BALANCE),
			(DAVE, DAVE_BALANCE),
		],
		..Default::default()
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	let mut ext: sp_io::TestExternalities = storage.into();
	// Events are only recorded from block 1 onwards.
	ext.execute_with(|| System::set_block_number(1));
	ext
}

/// Run `test` on a fresh chain, then check that the pallet's storage invariants still hold.
pub fn build_and_execute(test: impl FnOnce()) {
	new_test_ext().execute_with(|| {
		test();
		Ticketing::do_try_state().expect("storage invariants hold after the test");
	});
}
