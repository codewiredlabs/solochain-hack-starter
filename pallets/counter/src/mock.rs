use crate as pallet_counter;
use frame_support::{derive_impl, parameter_types};
use sp_runtime::BuildStorage;

type Block = frame_system::mocking::MockBlock<Test>;

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
	pub type Counter = pallet_counter::Pallet<Test>;
}

// System pallet configuration
#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
}

// Custom pallet configuration
parameter_types! {
	pub const CounterMaxValue: u32 = 10;
}

impl pallet_counter::Config for Test {
	type CounterMaxValue = CounterMaxValue;
	type WeightInfo = pallet_counter::weights::SubstrateWeight<Test>;
}

// Test externalities initialization
pub fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
