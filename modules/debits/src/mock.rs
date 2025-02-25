//! Mocks for the debit module.

#![cfg(test)]

use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use primitives::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

use super::*;

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

mod debits {}

impl_outer_event! {
	pub enum TestEvent for Runtime {

	}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

pub type AccountId = u64;
type BlockNumber = u64;

pub type Balance = u64;
pub type DebitBalance = u32;
pub type Amount = i64;
pub type CurrencyId = u32;

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
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
}

impl orml_tokens::Trait for Runtime {
	type Event = ();
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
}
pub type Tokens = orml_tokens::Module<Runtime>;

parameter_types! {
	pub const ExistentialDeposit: u64 = 0;
	pub const TransferFee: u64 = 0;
	pub const CreationFee: u64 = 2;
	pub const GetStableCurrencyId: CurrencyId = AUSD;
	pub const GetNativeCurrencyId: CurrencyId = ACA;
}

impl pallet_balances::Trait for Runtime {
	type Balance = Balance;
	type OnFreeBalanceZero = ();
	type OnNewAccount = ();
	type TransferPayment = ();
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ExistentialDeposit;
	type TransferFee = TransferFee;
	type CreationFee = CreationFee;
}
pub type PalletBalances = pallet_balances::Module<Runtime>;

pub type AdaptedBasicCurrency =
	orml_currencies::BasicCurrencyAdapter<Runtime, PalletBalances, Balance, orml_tokens::Error>;

impl orml_currencies::Trait for Runtime {
	type Event = ();
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
}
pub type Currencies = orml_currencies::Module<Runtime>;

impl Trait for Runtime {
	type CurrencyId = CurrencyId;
	type Currency = Currencies;
	type GetStableCurrencyId = GetStableCurrencyId;
	type DebitBalance = DebitBalance;
	type Convert = ConvertHandler;
	type DebitAmount = Amount;
}
pub type DebitsModule = Module<Runtime>;

pub const ALICE: AccountId = 1;
pub const ACA: CurrencyId = 0;
pub const AUSD: CurrencyId = 1;
pub const BTC: CurrencyId = 2;

pub struct ConvertHandler;
impl Convert<(CurrencyId, DebitBalance), Balance> for ConvertHandler {
	fn convert(a: (CurrencyId, DebitBalance)) -> Balance {
		let debit_balance: u32 = (a.1 / DebitBalance::from(2u32)).into();
		let balance = debit_balance as u64;
		balance
	}
}

pub struct ExtBuilder {
	currency_ids: Vec<CurrencyId>,
	endowed_accounts: Vec<AccountId>,
	initial_balance: Balance,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			currency_ids: vec![AUSD, BTC],
			endowed_accounts: vec![ALICE],
			initial_balance: 1000,
		}
	}
}

impl ExtBuilder {
	pub fn build(self) -> runtime_io::TestExternalities {
		let mut t = system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			tokens: self.currency_ids,
			initial_balance: self.initial_balance,
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
