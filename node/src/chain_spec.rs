use sc_service::ChainType;
use solochain_starter_runtime::WASM_BINARY;

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec;

/// Token details that wallets and Polkadot-JS Apps read through the `system_properties` RPC to
/// show balances in UNIT.
fn chain_properties() -> sc_service::Properties {
	let mut properties = sc_service::Properties::new();
	properties.insert("tokenSymbol".into(), "UNIT".into());
	// `UNIT` in the runtime is 10^12 of the smallest unit.
	properties.insert("tokenDecimals".into(), 12.into());
	// Matches `SS58Prefix` in the runtime: the generic Substrate address format.
	properties.insert("ss58Format".into(), 42.into());
	properties
}

pub fn development_chain_spec() -> Result<ChainSpec, String> {
	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name("Development")
	.with_properties(chain_properties())
	.with_id("dev")
	.with_chain_type(ChainType::Development)
	.with_genesis_config_preset_name(sp_genesis_builder::DEV_RUNTIME_PRESET)
	.build())
}

pub fn local_chain_spec() -> Result<ChainSpec, String> {
	Ok(ChainSpec::builder(
		WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?,
		None,
	)
	.with_name("Local Testnet")
	.with_properties(chain_properties())
	.with_id("local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_preset_name(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET)
	.build())
}
