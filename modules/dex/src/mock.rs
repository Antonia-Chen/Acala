//! Mocks for the dex module.

#![cfg(test)]

use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use primitives::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

use super::*;

mod dex {
	pub use super::super::*;
	use frame_support::impl_outer_event;
}

impl_outer_event! {
	pub enum TestEvent for Runtime {
		dex<T>, orml_tokens<T>,
	}
}
impl_outer_origin! {
	pub enum Origin for Runtime {}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
	pub const GetBaseCurrencyId: CurrencyId = AUSD;
	pub const GetExchangeFee: FixedU128 = FixedU128::from_rational(1, 100);
}

pub type AccountId = u64;
pub type BlockNumber = u64;
pub type CurrencyId = u32;
pub type Share = u128;
pub type Balance = u128;
pub type Amount = i128;

impl system::Trait for Runtime {
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = ();
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
}
pub type System = system::Module<Runtime>;

impl orml_tokens::Trait for Runtime {
	type Event = TestEvent;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
}
pub type Tokens = orml_tokens::Module<Runtime>;

impl Trait for Runtime {
	type Event = TestEvent;
	type Currency = Tokens;
	type Share = Share;
	type GetBaseCurrencyId = GetBaseCurrencyId;
	type GetExchangeFee = GetExchangeFee;
}
pub type DexModule = Module<Runtime>;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CAROL: AccountId = 3;

pub const AUSD: CurrencyId = 1;
pub const BTC: CurrencyId = 2;
pub const DOT: CurrencyId = 3;

pub struct ExtBuilder {
	currency_id: Vec<CurrencyId>,
	endowed_accounts: Vec<AccountId>,
	initial_balance: Balance,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			currency_id: vec![AUSD, BTC, DOT],
			endowed_accounts: vec![ALICE, BOB],
			initial_balance: 1_000_000_000_000_000_000u128,
		}
	}
}

impl ExtBuilder {
	pub fn build(self) -> runtime_io::TestExternalities {
		let mut t = system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			tokens: self.currency_id,
			initial_balance: self.initial_balance,
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
