#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get, Parameter};
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use orml_utilities::FixedU128;
use rstd::{convert::TryInto, result};
use sp_runtime::{
	traits::{
		AccountIdConversion, Bounded, CheckedAdd, CheckedSub, MaybeSerializeDeserialize, Member, SimpleArithmetic,
	},
	ModuleId,
};
use support::DexManager;
use system::{self as system, ensure_signed};

mod mock;
mod tests;

const MODULE_ID: ModuleId = ModuleId(*b"aca/dexm");

type BalanceOf<T> = <<T as Trait>::Currency as MultiCurrency<<T as system::Trait>::AccountId>>::Balance;
type CurrencyIdOf<T> = <<T as Trait>::Currency as MultiCurrency<<T as system::Trait>::AccountId>>::CurrencyId;

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type Currency: MultiCurrencyExtended<Self::AccountId>;
	type Share: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;
	type GetBaseCurrencyId: Get<CurrencyIdOf<Self>>;
	type GetExchangeFee: Get<FixedU128>;
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		<T as Trait>::Share,
		Balance = BalanceOf<T>,
		CurrencyId = CurrencyIdOf<T>,
	{
		AddLiquidity(AccountId, CurrencyId, Balance, Balance, Share),
		WithdrawLiquidity(AccountId, CurrencyId, Balance, Balance, Share),
		Swap(AccountId, CurrencyId, Balance, CurrencyId, Balance),
	}
);

decl_error! {
	/// Error for cdp dex module.
	pub enum Error {
		BaseCurrencyIdNotAllowed,
		TokenNotEnough,
		ShareNotEnough,
		InvalidBalance,
		CanNotSwapItself,
		InacceptablePrice,
		InvalidLiquidityIncrement,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Dex {
		LiquidityPool get(fn liquidity_pool): map CurrencyIdOf<T> => (BalanceOf<T>, BalanceOf<T>);
		TotalShares get(fn total_shares): map CurrencyIdOf<T> => T::Share;
		Shares get(fn shares): double_map CurrencyIdOf<T>, blake2_256(T::AccountId) => T::Share;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		fn swap_currency(origin, supply: (CurrencyIdOf<T>, BalanceOf<T>), target: (CurrencyIdOf<T>, BalanceOf<T>)) {
			let who = ensure_signed(origin)?;
			let base_currency_id = T::GetBaseCurrencyId::get();
			ensure!(
				target.0 != supply.0,
				Error::CanNotSwapItself.into(),
			);

			if target.0 == base_currency_id {
				Self::swap_other_to_base(who, supply.0, supply.1, target.1)?;
			} else if supply.0 == base_currency_id {
				Self::swap_base_to_other(who, target.0, supply.1, target.1)?;
			} else {
				Self::swap_other_to_other(who, supply.0, supply.1, target.0, target.1)?;
			}
		}

		fn add_liquidity(origin, other_currency_id: CurrencyIdOf<T>, max_other_currency_amount: BalanceOf<T>, max_base_currency_amount: BalanceOf<T>) {
			let who = ensure_signed(origin)?;
			let base_currency_id = T::GetBaseCurrencyId::get();
			ensure!(
				other_currency_id != base_currency_id,
				Error::BaseCurrencyIdNotAllowed.into(),
			);
			ensure!(
				max_other_currency_amount != 0.into() && max_base_currency_amount != 0.into(),
				Error::InvalidBalance.into(),
			);

			let total_shares = Self::total_shares(other_currency_id);
			let (other_currency_increment, base_currency_increment, share_increment): (BalanceOf<T>, BalanceOf<T>, T::Share) =
			if total_shares == 0.into() {
				// initialize this liquidity pool, the initial share is equal to the max value between base currency amount and other currency amount
				let initial_share = TryInto::<T::Share>::try_into(
					TryInto::<u128>::try_into(
						rstd::cmp::max(max_other_currency_amount, max_base_currency_amount)
					).unwrap_or(u128::max_value())
				).unwrap_or(T::Share::max_value());

				(max_other_currency_amount, max_base_currency_amount, initial_share)
			} else {
				let (other_currency_pool, base_currency_pool): (BalanceOf<T>, BalanceOf<T>) = Self::liquidity_pool(other_currency_id);

				let other_base_price = FixedU128::from_rational(
					TryInto::<u128>::try_into(base_currency_pool).unwrap_or(u128::max_value()),
					TryInto::<u128>::try_into(other_currency_pool).unwrap_or(u128::max_value()),
				);

				let input_other_base_price = FixedU128::from_rational(
					TryInto::<u128>::try_into(max_base_currency_amount).unwrap_or(u128::max_value()),
					TryInto::<u128>::try_into(max_other_currency_amount).unwrap_or(u128::max_value()),
				);

				if input_other_base_price <= other_base_price {
					// max_other_currency_amount may be too much, calculate the actual other currency amount
					let base_other_price = FixedU128::from_rational(
						TryInto::<u128>::try_into(other_currency_pool).unwrap_or(u128::max_value()),
						TryInto::<u128>::try_into(base_currency_pool).unwrap_or(u128::max_value()),
					);
					let other_currency_amount = base_other_price.checked_mul_int(&max_base_currency_amount).unwrap_or(BalanceOf::<T>::max_value());
					let share = FixedU128::from_rational(
						TryInto::<u128>::try_into(other_currency_amount).unwrap_or(u128::max_value()),
						TryInto::<u128>::try_into(other_currency_pool).unwrap_or(u128::max_value()),
					).checked_mul_int(&total_shares).unwrap_or(0.into());
					(other_currency_amount, max_base_currency_amount, share)
				} else {
					// max_base_currency_amount is too much, calculate the actual base currency amount
					let base_currency_amount = other_base_price.checked_mul_int(&max_other_currency_amount).unwrap_or(BalanceOf::<T>::max_value());
					let share = FixedU128::from_rational(
						TryInto::<u128>::try_into(base_currency_amount).unwrap_or(u128::max_value()),
						TryInto::<u128>::try_into(base_currency_pool).unwrap_or(u128::max_value()),
					).checked_mul_int(&total_shares).unwrap_or(0.into());
					(max_other_currency_amount, base_currency_amount, share)
				}
			};

			ensure!(
				share_increment > 0.into() && other_currency_increment > 0.into() && base_currency_increment > 0.into(),
				Error::InvalidLiquidityIncrement.into(),
			);
			ensure!(
				T::Currency::ensure_can_withdraw(base_currency_id, &who, base_currency_increment).is_ok()
				&&
				T::Currency::ensure_can_withdraw(other_currency_id, &who, other_currency_increment).is_ok(),
				Error::TokenNotEnough.into(),
			);
			T::Currency::transfer(other_currency_id, &who, &Self::account_id(), other_currency_increment)
			.expect("never failed because after checks");
			T::Currency::transfer(base_currency_id, &who, &Self::account_id(), base_currency_increment)
			.expect("never failed because after checks");
			<TotalShares<T>>::mutate(other_currency_id, |share| *share += share_increment);
			<Shares<T>>::mutate(other_currency_id, &who, |share| *share += share_increment);
			<LiquidityPool<T>>::mutate(other_currency_id, |pool| {
				let newpool = (pool.0 + other_currency_increment, pool.1 + base_currency_increment);
				*pool = newpool;
			});
			Self::deposit_event(RawEvent::AddLiquidity(
				who,
				other_currency_id,
				other_currency_increment,
				base_currency_increment,
				share_increment,
			));
		}

		fn withdraw_liquidity(origin, currency_id: CurrencyIdOf<T>, share_amount: T::Share) {
			let who = ensure_signed(origin)?;
			let base_currency_id = T::GetBaseCurrencyId::get();
			ensure!(
				currency_id != base_currency_id,
				Error::BaseCurrencyIdNotAllowed.into(),
			);
			ensure!(
				Self::shares(currency_id, &who) >= share_amount && share_amount > 0.into(),
				Error::ShareNotEnough.into(),
			);

			let (other_currency_pool, base_currency_pool): (BalanceOf<T>, BalanceOf<T>) = Self::liquidity_pool(currency_id);
			let proportion = FixedU128::from_rational(
				TryInto::<u128>::try_into(share_amount).unwrap_or(u128::max_value()),
				TryInto::<u128>::try_into(Self::total_shares(currency_id)).unwrap_or(u128::max_value()),
			);
			let withdraw_other_currency_amount = proportion.checked_mul_int(&other_currency_pool).unwrap_or(BalanceOf::<T>::max_value());
			let withdraw_base_currency_amount = proportion.checked_mul_int(&base_currency_pool).unwrap_or(BalanceOf::<T>::max_value());
			if withdraw_other_currency_amount > 0.into() {
				T::Currency::transfer(currency_id, &Self::account_id(), &who, withdraw_other_currency_amount)
				.expect("never failed because after checks");
			}
			if withdraw_base_currency_amount > 0.into() {
				T::Currency::transfer(base_currency_id, &Self::account_id(), &who, withdraw_base_currency_amount)
				.expect("never failed because after checks");
			}
			<TotalShares<T>>::mutate(currency_id, |share| *share -= share_amount);
			<Shares<T>>::mutate(currency_id, &who, |share| *share -= share_amount);
			<LiquidityPool<T>>::mutate(currency_id, |pool| {
				let newpool = (pool.0 - withdraw_other_currency_amount, pool.1 - withdraw_base_currency_amount);
				*pool = newpool;
			});

			Self::deposit_event(RawEvent::WithdrawLiquidity(
				who,
				currency_id,
				withdraw_base_currency_amount,
				withdraw_base_currency_amount,
				share_amount,
			));
		}
	}
}

impl<T: Trait> Module<T> {
	pub fn account_id() -> T::AccountId {
		MODULE_ID.into_account()
	}

	pub fn calculate_swap_target_amount(
		supply_pool: BalanceOf<T>,
		target_pool: BalanceOf<T>,
		supply_amount: BalanceOf<T>,
	) -> BalanceOf<T> {
		// new_target_pool = supply_pool * target_pool / (supply_amount + supply_pool)
		let new_target_pool = supply_pool
			.checked_add(&supply_amount)
			.and_then(|n| {
				Some(FixedU128::from_rational(
					TryInto::<u128>::try_into(supply_pool).unwrap_or(u128::max_value()),
					TryInto::<u128>::try_into(n).unwrap_or(u128::max_value()),
				))
			})
			.and_then(|n| n.checked_mul_int(&target_pool))
			.unwrap_or(0.into());

		// new_target_pool should be more then 0
		if new_target_pool != 0.into() {
			// actual can get = (target_pool - new_target_pool) * (1 - GetExchangeFee)
			target_pool
				.checked_sub(&new_target_pool)
				.and_then(|n| {
					n.checked_sub(
						&T::GetExchangeFee::get()
							.checked_mul_int(&n)
							.unwrap_or(BalanceOf::<T>::max_value()),
					)
				})
				.unwrap_or(0.into())
		} else {
			0.into()
		}
	}

	pub fn calculate_swap_supply_amount(
		supply_pool: BalanceOf<T>,
		target_pool: BalanceOf<T>,
		target_amount: BalanceOf<T>,
	) -> BalanceOf<T> {
		// new_target_pool = target_pool - target_amount / (1 - GetExchangeFee)
		// supply_amount = target_pool * supply_pool / new_target_pool - supply_pool
		FixedU128::from_natural(1)
			.checked_sub(&T::GetExchangeFee::get())
			.and_then(|n| FixedU128::from_natural(1).checked_div(&n))
			.and_then(|n| n.checked_mul_int(&target_amount))
			.and_then(|n| target_pool.checked_sub(&n))
			.and_then(|n| {
				Some(FixedU128::from_rational(
					TryInto::<u128>::try_into(supply_pool).unwrap_or(u128::max_value()),
					TryInto::<u128>::try_into(n).unwrap_or(u128::max_value()),
				))
			})
			.and_then(|n| n.checked_mul_int(&target_pool))
			.and_then(|n| n.checked_sub(&supply_pool))
			.unwrap_or(0.into())
	}

	// use other currency to swap base currency
	pub fn swap_other_to_base(
		who: T::AccountId,
		other_currency_id: CurrencyIdOf<T>,
		other_currency_amount: BalanceOf<T>,
		min_base_currency_amount: BalanceOf<T>,
	) -> result::Result<(), Error> {
		ensure!(
			other_currency_amount > 0.into()
				&& T::Currency::ensure_can_withdraw(other_currency_id, &who, other_currency_amount).is_ok(),
			Error::TokenNotEnough,
		);
		let base_currency_id = T::GetBaseCurrencyId::get();
		let (other_currency_pool, base_currency_pool) = Self::liquidity_pool(other_currency_id);
		let base_currency_amount =
			Self::calculate_swap_target_amount(other_currency_pool, base_currency_pool, other_currency_amount);
		ensure!(
			base_currency_amount >= min_base_currency_amount,
			Error::InacceptablePrice,
		);

		T::Currency::transfer(other_currency_id, &who, &Self::account_id(), other_currency_amount)
			.expect("never failed because after checks");
		T::Currency::transfer(base_currency_id, &Self::account_id(), &who, base_currency_amount)
			.expect("never failed because after checks");
		<LiquidityPool<T>>::mutate(other_currency_id, |pool| {
			let newpool = (pool.0 + other_currency_amount, pool.1 - base_currency_amount);
			*pool = newpool;
		});
		Self::deposit_event(RawEvent::Swap(
			who,
			other_currency_id,
			other_currency_amount,
			base_currency_id,
			base_currency_amount,
		));
		Ok(())
	}

	// use base currency to swap other currency
	pub fn swap_base_to_other(
		who: T::AccountId,
		other_currency_id: CurrencyIdOf<T>,
		base_currency_amount: BalanceOf<T>,
		min_other_currency_amount: BalanceOf<T>,
	) -> result::Result<(), Error> {
		let base_currency_id = T::GetBaseCurrencyId::get();
		ensure!(
			base_currency_amount > 0.into()
				&& T::Currency::ensure_can_withdraw(base_currency_id, &who, base_currency_amount).is_ok(),
			Error::TokenNotEnough,
		);
		let (other_currency_pool, base_currency_pool) = Self::liquidity_pool(other_currency_id);
		let other_currency_amount =
			Self::calculate_swap_target_amount(base_currency_pool, other_currency_pool, base_currency_amount);
		ensure!(
			other_currency_amount >= min_other_currency_amount,
			Error::InacceptablePrice,
		);

		T::Currency::transfer(base_currency_id, &who, &Self::account_id(), base_currency_amount)
			.expect("never failed because after checks");
		T::Currency::transfer(other_currency_id, &Self::account_id(), &who, other_currency_amount)
			.expect("never failed because after checks");
		<LiquidityPool<T>>::mutate(other_currency_id, |pool| {
			let newpool = (pool.0 - other_currency_amount, pool.1 + base_currency_amount);
			*pool = newpool;
		});
		Self::deposit_event(RawEvent::Swap(
			who,
			base_currency_id,
			base_currency_amount,
			other_currency_id,
			other_currency_amount,
		));
		Ok(())
	}

	// use other currency to swap another other currency
	pub fn swap_other_to_other(
		who: T::AccountId,
		supply_other_currency_id: CurrencyIdOf<T>,
		supply_other_currency_amount: BalanceOf<T>,
		target_other_currency_id: CurrencyIdOf<T>,
		min_target_other_currency_amount: BalanceOf<T>,
	) -> result::Result<(), Error> {
		ensure!(
			supply_other_currency_amount > 0.into()
				&& T::Currency::ensure_can_withdraw(supply_other_currency_id, &who, supply_other_currency_amount)
					.is_ok(),
			Error::TokenNotEnough,
		);
		let (supply_other_currency_pool, supply_base_currency_pool) = Self::liquidity_pool(supply_other_currency_id);
		let intermediate_base_currency_amount = Self::calculate_swap_target_amount(
			supply_other_currency_pool,
			supply_base_currency_pool,
			supply_other_currency_amount,
		);
		let (target_other_currency_pool, target_base_currency_pool) = Self::liquidity_pool(target_other_currency_id);
		let target_other_currency_amount = Self::calculate_swap_target_amount(
			target_base_currency_pool,
			target_other_currency_pool,
			intermediate_base_currency_amount,
		);
		ensure!(
			target_other_currency_amount >= min_target_other_currency_amount,
			Error::InacceptablePrice,
		);

		T::Currency::transfer(
			supply_other_currency_id,
			&who,
			&Self::account_id(),
			supply_other_currency_amount,
		)
		.expect("never failed because after checks");
		T::Currency::transfer(
			target_other_currency_id,
			&Self::account_id(),
			&who,
			target_other_currency_amount,
		)
		.expect("never failed because after checks");
		<LiquidityPool<T>>::mutate(supply_other_currency_id, |pool| {
			let newpool = (
				pool.0 + supply_other_currency_amount,
				pool.1 - intermediate_base_currency_amount,
			);
			*pool = newpool;
		});
		<LiquidityPool<T>>::mutate(target_other_currency_id, |pool| {
			let newpool = (
				pool.0 - target_other_currency_amount,
				pool.1 + intermediate_base_currency_amount,
			);
			*pool = newpool;
		});
		Self::deposit_event(RawEvent::Swap(
			who,
			supply_other_currency_id,
			supply_other_currency_amount,
			target_other_currency_id,
			target_other_currency_amount,
		));
		Ok(())
	}
}

impl<T: Trait> DexManager<T::AccountId, CurrencyIdOf<T>, BalanceOf<T>> for Module<T> {
	type Error = Error;

	fn get_supply_amount(
		supply_currency_id: CurrencyIdOf<T>,
		target_currency_id: CurrencyIdOf<T>,
		target_currency_amount: BalanceOf<T>,
	) -> BalanceOf<T> {
		let base_currency_id = T::GetBaseCurrencyId::get();
		if supply_currency_id == target_currency_id {
			0.into()
		} else if target_currency_id == base_currency_id {
			let (other_currency_pool, base_currency_pool) = Self::liquidity_pool(supply_currency_id);
			Self::calculate_swap_supply_amount(other_currency_pool, base_currency_pool, target_currency_amount)
		} else if supply_currency_id == base_currency_id {
			let (other_currency_pool, base_currency_pool) = Self::liquidity_pool(target_currency_id);
			Self::calculate_swap_supply_amount(base_currency_pool, other_currency_pool, target_currency_amount)
		} else {
			let (target_other_currency_pool, target_base_currency_pool) = Self::liquidity_pool(target_currency_id);
			let intermediate_base_currency_amount = Self::calculate_swap_supply_amount(
				target_base_currency_pool,
				target_other_currency_pool,
				target_currency_amount,
			);
			let (supply_other_currency_pool, supply_base_currency_pool) = Self::liquidity_pool(supply_currency_id);
			Self::calculate_swap_supply_amount(
				supply_other_currency_pool,
				supply_base_currency_pool,
				intermediate_base_currency_amount,
			)
		}
	}

	fn exchange_currency(
		who: T::AccountId,
		supply: (CurrencyIdOf<T>, BalanceOf<T>),
		target: (CurrencyIdOf<T>, BalanceOf<T>),
	) -> Result<(), Self::Error> {
		let base_currency_id = T::GetBaseCurrencyId::get();
		ensure!(target.0 != supply.0, Error::CanNotSwapItself.into());
		if target.0 == base_currency_id {
			Self::swap_other_to_base(who, supply.0, supply.1, target.1)
		} else if supply.0 == base_currency_id {
			Self::swap_base_to_other(who, target.0, supply.1, target.1)
		} else {
			Self::swap_other_to_other(who, supply.0, supply.1, target.0, target.1)
		}
	}
}
