#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub const BLOCKS_IN_YEAR: u64 = 60 * 60 * 24 * 365 / 6;
pub const MAX_PERCENTAGE: u64 = 10_000;
pub const REWARD_PER_BLOCK: u64 = 300_000_000_000_000;
#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
    pub struct StakingTotal<M: ManagedTypeApi> {
        pub stake_amount: BigUint<M>,
        pub last_action_block: u64,
        pub last_action_block_timestamp: u64
    }

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
    pub struct StakingPosition<M: ManagedTypeApi> {
        pub stake_amount: BigUint<M>,
        pub last_action_block: u64,
        pub last_action_block_timestamp: u64
    }

/// An empty contract. To be used as a template when starting a new contract from scratch.
#[multiversx_sc::contract]
pub trait StakingContract {
    #[init]
    fn init(&self) {
        self.staking_total().set(StakingTotal {
            stake_amount: BigUint::zero(),
            last_action_block: 0,
            last_action_block_timestamp: 0
        });
    }

    fn stake(&self) {
        let payment_amount = self.call_value().egld_value().clone_value();
        require!(payment_amount > 0, "Must pay more than 0");

        let caller = self.blockchain().get_caller();
        let stake_mapper = self.staking_position(&caller);

        let new_user = self.staked_addresses().insert(caller.clone());
        /* the user has a already a staking position */
        let mut staking_pos = if !new_user {
            stake_mapper.get()
        } else {
        /* new user */
            let current_block = self.blockchain().get_block_epoch();
            let current_block_timestamp: u64 =  self.blockchain().get_block_timestamp();
            StakingPosition {
                stake_amount: BigUint::zero(),
                last_action_block: current_block,
                last_action_block_timestamp: current_block_timestamp
            }
        };

        self.claim_rewards_for_user(&caller, &mut staking_pos);
        staking_pos.stake_amount += payment_amount.clone();

        stake_mapper.set(&staking_pos);

        let mut staking_total = self.staking_total().get();
        staking_total.stake_amount += payment_amount.clone();
        staking_total.last_action_block = self.blockchain().get_block_epoch();
        staking_total.last_action_block_timestamp = self.blockchain().get_block_timestamp();
        
        self.staking_total().set(&staking_total);

    }


    #[endpoint]
    fn unstake(&self, opt_unstake_amount: OptionalValue<BigUint>) {
        let caller = self.blockchain().get_caller();
        self.require_user_staked(&caller);
        
        let stake_mapper = self.staking_position(&caller);
        let mut staking_pos = stake_mapper.get();

        let unstake_amount = match opt_unstake_amount {
            OptionalValue::Some(amt) => amt,
            OptionalValue::None => staking_pos.stake_amount.clone(),
        };
        require!(
            unstake_amount > 0 && unstake_amount <= staking_pos.stake_amount,
            "Invalid unstake amount"
        );

        self.claim_rewards_for_user(&caller, &mut staking_pos);
        staking_pos.stake_amount -= &unstake_amount.clone();

        if staking_pos.stake_amount > 0 {
            stake_mapper.set(&staking_pos);
        } else {
            stake_mapper.clear();
            self.staked_addresses().swap_remove(&caller);
        }

        self.send().direct_egld(&caller, &unstake_amount);
        
        let mut staking_total = self.staking_total().get();
        staking_total.stake_amount -= &unstake_amount.clone();

        self.staking_total().set(&staking_total);
    }

    #[endpoint(claimRewards)]
    fn claim_rewards(&self) {
        let caller = self.blockchain().get_caller();
        self.require_user_staked(&caller);

        let stake_mapper = self.staking_position(&caller);
        let mut staking_pos = stake_mapper.get();
        self.claim_rewards_for_user(&caller, &mut staking_pos);

        stake_mapper.set(&staking_pos);
    }

    fn require_user_staked(&self, user: &ManagedAddress) {
        require!(self.staked_addresses().contains(user), "Must stake first");
    }

    fn claim_rewards_for_user(
        &self,
        user: &ManagedAddress,
        staking_pos: &mut StakingPosition<Self::Api>,
    ) {
        let reward_amount = self.calculate_rewards(staking_pos);
        let current_block = self.blockchain().get_block_nonce();
        staking_pos.last_action_block = current_block;

        if reward_amount > 0 {
            self.send().direct_egld(user, &reward_amount);
        }
    }

    fn calculate_rewards(&self, staking_position: &StakingPosition<Self::Api>) -> BigUint {
        let current_block = self.blockchain().get_block_nonce();
        if current_block <= staking_position.last_action_block {
            return BigUint::zero();
        }
        
        let block_diff = current_block - staking_position.last_action_block;
        &staking_position.stake_amount * REWARD_PER_BLOCK * block_diff
    }

    #[view(calculateRewardsForUser)]
    fn calculate_rewards_for_user(&self, addr: ManagedAddress) -> BigUint {
        let staking_pos = self.staking_position(&addr).get();
        self.calculate_rewards(&staking_pos)
    }

    #[view(getStakedAddresses)]
    #[storage_mapper("stakedAddresses")]
    fn staked_addresses(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[view(getStakingPosition)]
    #[storage_mapper("stakingPosition")]
    fn staking_position(
        &self,
        addr: &ManagedAddress,
    ) -> SingleValueMapper<StakingPosition<Self::Api>>;

    #[view(getApy)]
    #[storage_mapper("apy")]
    fn apy(&self) -> SingleValueMapper<u64>;

    #[view(getStakingTotal)]
    #[storage_mapper("stakingTotal")]
    fn staking_total(&self) -> SingleValueMapper<StakingTotal<Self::Api>>;


}