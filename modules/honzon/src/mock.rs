//! Mocks for the debit module.

#![cfg(test)]

use frame_support::{impl_outer_origin, parameter_types};
use primitives::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

use orml_traits::PriceProvider;
use support::{AuctionManager, ExchangeRate, Price, Rate, Ratio};

use super::*;

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
	pub const ExistentialDeposit: u64 = 0;
	pub const TransferFee: u64 = 0;
	pub const CreationFee: u64 = 2;
	pub const CollateralCurrencyIds: Vec<CurrencyId> = vec![BTC, DOT];
	pub const GlobalStabilityFee: Rate = Rate::from_parts(0);
	pub const DefaultLiquidationRatio: Ratio = Ratio::from_rational(3, 2);
	pub const DefaulDebitExchangeRate: ExchangeRate = ExchangeRate::from_natural(1);
	pub const MinimumDebitValue: Balance = 2;
	pub const GetNativeCurrencyId: CurrencyId = ACA;
	pub const GetStableCurrencyId: CurrencyId = AUSD;
}

pub type AccountId = u64;
pub type BlockNumber = u64;
pub type Balance = u64;
pub type DebitBalance = u64;
pub type Amount = i64;
pub type DebitAmount = i64;
pub type CurrencyId = u32;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const ALIEX: AccountId = 3;

pub const ACA: CurrencyId = 0;
pub const AUSD: CurrencyId = 1;
pub const BTC: CurrencyId = 2;
pub const DOT: CurrencyId = 3;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;

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

impl debits::Trait for Runtime {
	type Currency = Currencies;
	type GetStableCurrencyId = GetStableCurrencyId;
	type DebitBalance = DebitBalance;
	type CurrencyId = CurrencyId;
	type DebitAmount = DebitAmount;
	type Convert = cdp_engine::DebitExchangeRateConvertor<Runtime>;
}
pub type DebitCurrency = debits::Module<Runtime>;

impl vaults::Trait for Runtime {
	type Event = ();
	type Convert = cdp_engine::DebitExchangeRateConvertor<Runtime>;
	type Currency = Tokens;
	type DebitCurrency = DebitCurrency;
	type RiskManager = CdpEngineModule;
}
pub type VaultsModule = vaults::Module<Runtime>;

pub struct MockPriceSource;
impl PriceProvider<CurrencyId, Price> for MockPriceSource {
	#[allow(unused_variables)]
	fn get_price(base: CurrencyId, quote: CurrencyId) -> Option<Price> {
		Some(Price::from_natural(1))
	}
}

pub struct MockAuctionManager;
impl AuctionManager<AccountId> for MockAuctionManager {
	type CurrencyId = CurrencyId;
	type Balance = Balance;
	type Amount = Amount;

	#[allow(unused_variables)]
	fn increase_surplus(increment: Self::Balance) {}

	#[allow(unused_variables)]
	fn new_collateral_auction(
		who: AccountId,
		currency_id: Self::CurrencyId,
		amount: Self::Balance,
		target: Self::Balance,
		bad_debt: Self::Balance,
	) {
	}
}

impl cdp_engine::Trait for Runtime {
	type Event = ();
	type AuctionManagerHandler = MockAuctionManager;
	type Currency = Currencies;
	type PriceSource = MockPriceSource;
	type CollateralCurrencyIds = CollateralCurrencyIds;
	type GlobalStabilityFee = GlobalStabilityFee;
	type DefaultLiquidationRatio = DefaultLiquidationRatio;
	type DefaulDebitExchangeRate = DefaulDebitExchangeRate;
	type MinimumDebitValue = MinimumDebitValue;
	type GetStableCurrencyId = GetStableCurrencyId;
}

pub type CdpEngineModule = cdp_engine::Module<Runtime>;

impl Trait for Runtime {
	type Event = ();
}

pub type HonzonModule = Module<Runtime>;

pub struct ExtBuilder {
	currency_ids: Vec<CurrencyId>,
	endowed_accounts: Vec<AccountId>,
	initial_balance: Balance,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			currency_ids: vec![BTC, DOT],
			endowed_accounts: vec![ALICE, BOB],
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
