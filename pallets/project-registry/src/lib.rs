//! # Project Registry Pallet
//! 
//! A pallet that implements an on-chain registry of funding campaigns.
//! 
//! ## Overview
//! 
//! This pallet allows users to:
//! - Create funding campaigns with metadata, time bounds, and funding caps
//! - Update campaign metadata and caps before campaign starts
//! - Contribute funds to active campaigns
//! - Cancel campaigns (by owner or root)
//! - Automatically finalize campaigns and handle refunds
//! 
//! ## Terminology
//! 
//! * **Campaign**: A funding initiative with metadata, time bounds, and funding targets.
//! * **Soft Cap**: Minimum funding goal that must be met for the campaign to succeed.
//! * **Hard Cap**: Maximum funding that a campaign can accept.
//! * **Deposit**: Required stake from campaign creators to prevent spam.
//! * **Metadata**: Campaign information including name, description, and optional link.
//! 
//! ## Campaign Lifecycle
//! 
//! 1. **Creation**: Owner creates campaign with metadata and funding goals
//! 2. **Upcoming**: Campaign is created but not yet started
//! 3. **Active**: Campaign is accepting contributions
//! 4. **Finalization**: Campaign ends and is marked as Success/Failed
//! 5. **Refund**: Contributors can claim refunds if campaign failed
//! 
//! ## Interface
//! 
//! ### Dispatchable Functions
//! 
//! * `create_campaign` - Create a new funding campaign
//! * `update_metadata` - Update campaign metadata (only before start)
//! * `set_caps` - Modify funding caps (only before start)
//! * `cancel_campaign` - Cancel a campaign (owner or root only)
//! * `contribute` - Contribute funds to an active campaign
//! * `claim_refund` - Claim refund from failed/cancelled campaigns
//! 
//! ## Security
//! 
//! The pallet implements several security measures:
//! 
//! 1. Required deposits for campaign creation
//! 2. Time-bound operations (updates only before start)
//! 3. Owner-only campaign management
//! 4. Fund reservation for contributions
//! 5. Automatic campaign finalization
//! 6. Safe math operations using `saturating_*` methods

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    pallet_prelude::*,
    traits::{Currency, ReservableCurrency, Get},
    BoundedVec,
};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::{Zero, AtLeast32BitUnsigned};
use sp_std::prelude::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    pub type CampaignId = u32;
    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
    pub type MomentOf<T> = <<T as Config>::Timestamp as frame_support::traits::Time>::Moment;

    #[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Metadata<T: Config> {
        pub name: BoundedVec<u8, T::MaxNameLen>,
        pub description: BoundedVec<u8, T::MaxDescLen>,
        pub link: Option<BoundedVec<u8, T::MaxLinkLen>>,
    }

    #[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum CampaignStatus {
        Upcoming,
        Active,
        Success,
        Failed,
        Cancelled,
    }

    #[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Campaign<T: Config> {
        pub owner: T::AccountId,
        pub metadata: Metadata<T>,
        pub start: MomentOf<T>,
        pub end: MomentOf<T>,
        pub soft_cap: BalanceOf<T>,
        pub hard_cap: BalanceOf<T>,
        pub matched: BalanceOf<T>,
        pub status: CampaignStatus,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        
        /// The currency type for handling funds
        type Currency: ReservableCurrency<Self::AccountId>;
        
        /// Timestamp used for campaign timing
        type Timestamp: frame_support::traits::Time;
        
        /// Maximum length for campaign names
        #[pallet::constant]
        type MaxNameLen: Get<u32>;
        
        /// Maximum length for campaign descriptions
        #[pallet::constant]
        type MaxDescLen: Get<u32>;
        
        /// Maximum length for campaign links
        #[pallet::constant]
        type MaxLinkLen: Get<u32>;
        
        /// Maximum number of active campaigns
        #[pallet::constant]
        type MaxActive: Get<u32>;

        /// Minimum deposit required to create a campaign
        #[pallet::constant]
        type MinimumDeposit: Get<BalanceOf<Self>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type NextCampaignId<T> = StorageValue<_, CampaignId, ValueQuery>;

    #[pallet::storage]
    pub type Campaigns<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        CampaignId,
        Campaign<T>,
    >;

    #[pallet::storage]
    pub type ActiveCampaigns<T: Config> = StorageValue<
        _,
        BoundedVec<CampaignId, T::MaxActive>,
        ValueQuery,
    >;

    #[pallet::storage]
    pub type CampaignContributions<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        CampaignId,
        Blake2_128Concat,
        T::AccountId,
        BalanceOf<T>,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Campaign created. [campaign_id, owner]
        CampaignCreated { campaign_id: CampaignId, owner: T::AccountId },
        /// Campaign metadata updated. [campaign_id]
        MetadataUpdated { campaign_id: CampaignId },
        /// Campaign caps updated. [campaign_id, soft_cap, hard_cap]
        CapsUpdated { campaign_id: CampaignId, soft_cap: BalanceOf<T>, hard_cap: BalanceOf<T> },
        /// Campaign cancelled. [campaign_id]
        CampaignCancelled { campaign_id: CampaignId },
        /// Contribution made to campaign. [campaign_id, who, amount]
        ContributionMade { campaign_id: CampaignId, who: T::AccountId, amount: BalanceOf<T> },
        /// Campaign finalized. [campaign_id, status]
        CampaignFinalized { campaign_id: CampaignId, status: CampaignStatus },
        /// Refund claimed. [campaign_id, who, amount]
        RefundClaimed { campaign_id: CampaignId, who: T::AccountId, amount: BalanceOf<T> },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Campaign not found
        CampaignNotFound,
        /// Not the campaign owner
        NotOwner,
        /// Invalid time range specified
        InvalidTimeRange,
        /// Invalid cap values
        CapsInvalid,
        /// Campaign is not active
        NotActive,
        /// Campaign hard cap would be exceeded
        HardCapExceeded,
        /// Campaign already finalized
        AlreadyFinalized,
        /// No contribution found to refund
        NoContributionFound,
        /// Maximum number of active campaigns reached
        TooManyActiveCampaigns,
        /// Campaign has not failed or been cancelled
        NotRefundable,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            let mut weight = Weight::zero();
            let now = T::Timestamp::now();
            
            let active = ActiveCampaigns::<T>::get();
            let mut updated = active.clone();
            
            for campaign_id in active.iter() {
                weight = weight.saturating_add(T::DbWeight::get().reads(1));
                
                if let Some(mut campaign) = Campaigns::<T>::get(campaign_id) {
                    if campaign.status == CampaignStatus::Active && now >= campaign.end {
                        // Finalize campaign
                        campaign.status = if campaign.matched >= campaign.soft_cap {
                            CampaignStatus::Success
                        } else {
                            CampaignStatus::Failed
                        };
                        
                        Campaigns::<T>::insert(campaign_id, campaign.clone());
                        updated.retain(|id| id != campaign_id);
                        
                        Self::deposit_event(Event::CampaignFinalized {
                            campaign_id: *campaign_id,
                            status: campaign.status,
                        });
                        
                        weight = weight.saturating_add(T::DbWeight::get().writes(1));
                    }
                }
            }
            
            if updated != active {
                ActiveCampaigns::<T>::put(updated);
                weight = weight.saturating_add(T::DbWeight::get().writes(1));
            }
            
            weight
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1, 2))]
        pub fn create_campaign(
            origin: OriginFor<T>,
            metadata: Metadata<T>,
            start: MomentOf<T>,
            end: MomentOf<T>,
            soft_cap: BalanceOf<T>,
            hard_cap: BalanceOf<T>,
        ) -> DispatchResult {
            let owner = ensure_signed(origin)?;
            
            ensure!(start < end, Error::<T>::InvalidTimeRange);
            ensure!(soft_cap <= hard_cap, Error::<T>::CapsInvalid);
            ensure!(
                !soft_cap.is_zero() && !hard_cap.is_zero(),
                Error::<T>::CapsInvalid
            );
            
            let now = T::Timestamp::now();
            let status = if now < start {
                CampaignStatus::Upcoming
            } else if now <= end {
                CampaignStatus::Active
            } else {
                return Err(Error::<T>::InvalidTimeRange.into());
            };
            
            // Reserve the deposit
            T::Currency::reserve(&owner, T::MinimumDeposit::get())?;
            
            let campaign_id = NextCampaignId::<T>::get();
            let campaign = Campaign {
                owner: owner.clone(),
                metadata,
                start,
                end,
                soft_cap,
                hard_cap,
                matched: Zero::zero(),
                status,
            };
            
            Campaigns::<T>::insert(campaign_id, campaign);
            NextCampaignId::<T>::put(campaign_id.saturating_add(1));
            
            if status == CampaignStatus::Active {
                ActiveCampaigns::<T>::try_mutate(|campaigns| {
                    campaigns.try_push(campaign_id)
                }).map_err(|_| Error::<T>::TooManyActiveCampaigns)?;
            }
            
            Self::deposit_event(Event::CampaignCreated {
                campaign_id,
                owner,
            });
            
            Ok(())
        }

        #[pallet::weight(5_000 + T::DbWeight::get().reads_writes(1, 1))]
        pub fn update_metadata(
            origin: OriginFor<T>,
            campaign_id: CampaignId,
            metadata: Metadata<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            
            Campaigns::<T>::try_mutate(campaign_id, |maybe_campaign| -> DispatchResult {
                let campaign = maybe_campaign.as_mut().ok_or(Error::<T>::CampaignNotFound)?;
                ensure!(campaign.owner == who, Error::<T>::NotOwner);
                ensure!(campaign.status == CampaignStatus::Upcoming, Error::<T>::NotActive);
                
                campaign.metadata = metadata;
                Self::deposit_event(Event::MetadataUpdated { campaign_id });
                Ok(())
            })
        }

        #[pallet::weight(5_000 + T::DbWeight::get().reads_writes(1, 1))]
        pub fn set_caps(
            origin: OriginFor<T>,
            campaign_id: CampaignId,
            soft_cap: BalanceOf<T>,
            hard_cap: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            
            ensure!(soft_cap <= hard_cap, Error::<T>::CapsInvalid);
            ensure!(
                !soft_cap.is_zero() && !hard_cap.is_zero(),
                Error::<T>::CapsInvalid
            );
            
            Campaigns::<T>::try_mutate(campaign_id, |maybe_campaign| -> DispatchResult {
                let campaign = maybe_campaign.as_mut().ok_or(Error::<T>::CampaignNotFound)?;
                ensure!(campaign.owner == who, Error::<T>::NotOwner);
                ensure!(campaign.status == CampaignStatus::Upcoming, Error::<T>::NotActive);
                
                campaign.soft_cap = soft_cap;
                campaign.hard_cap = hard_cap;
                
                Self::deposit_event(Event::CapsUpdated {
                    campaign_id,
                    soft_cap,
                    hard_cap,
                });
                Ok(())
            })
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(2, 2))]
        pub fn cancel_campaign(
            origin: OriginFor<T>,
            campaign_id: CampaignId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            
            Campaigns::<T>::try_mutate(campaign_id, |maybe_campaign| -> DispatchResult {
                let campaign = maybe_campaign.as_mut().ok_or(Error::<T>::CampaignNotFound)?;
                ensure!(
                    campaign.owner == who || frame_system::Pallet::<T>::is_root(origin.clone()),
                    Error::<T>::NotOwner
                );
                ensure!(
                    campaign.status == CampaignStatus::Upcoming || campaign.status == CampaignStatus::Active,
                    Error::<T>::AlreadyFinalized
                );
                
                campaign.status = CampaignStatus::Cancelled;
                
                // Remove from active campaigns if needed
                if campaign.status == CampaignStatus::Active {
                    ActiveCampaigns::<T>::try_mutate(|campaigns| {
                        campaigns.retain(|id| *id != campaign_id);
                        Ok(())
                    })?;
                }
                
                // Unreserve the deposit for the owner
                T::Currency::unreserve(&campaign.owner, T::MinimumDeposit::get());
                
                Self::deposit_event(Event::CampaignCancelled { campaign_id });
                Ok(())
            })
        }

        #[pallet::weight(15_000 + T::DbWeight::get().reads_writes(2, 2))]
        pub fn contribute(
            origin: OriginFor<T>,
            campaign_id: CampaignId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            
            Campaigns::<T>::try_mutate(campaign_id, |maybe_campaign| -> DispatchResult {
                let campaign = maybe_campaign.as_mut().ok_or(Error::<T>::CampaignNotFound)?;
                ensure!(campaign.status == CampaignStatus::Active, Error::<T>::NotActive);
                
                let new_total = campaign.matched.saturating_add(amount);
                ensure!(new_total <= campaign.hard_cap, Error::<T>::HardCapExceeded);
                
                // Reserve the contribution
                T::Currency::reserve(&who, amount)?;
                
                // Update contribution tracking
                CampaignContributions::<T>::try_mutate(
                    campaign_id,
                    who.clone(),
                    |contribution| -> DispatchResult {
                        *contribution = contribution.saturating_add(amount);
                        Ok(())
                    }
                )?;
                
                campaign.matched = new_total;
                
                Self::deposit_event(Event::ContributionMade {
                    campaign_id,
                    who,
                    amount,
                });
                Ok(())
            })
        }

        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(2, 2))]
        pub fn claim_refund(
            origin: OriginFor<T>,
            campaign_id: CampaignId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            
            let campaign = Campaigns::<T>::get(campaign_id).ok_or(Error::<T>::CampaignNotFound)?;
            ensure!(
                campaign.status == CampaignStatus::Failed || campaign.status == CampaignStatus::Cancelled,
                Error::<T>::NotRefundable
            );
            
            let contribution = CampaignContributions::<T>::take(campaign_id, who.clone());
            ensure!(!contribution.is_zero(), Error::<T>::NoContributionFound);
            
            // Unreserve and transfer the contribution back
            T::Currency::unreserve(&who, contribution);
            
            Self::deposit_event(Event::RefundClaimed {
                campaign_id,
                who,
                amount: contribution,
            });
            
            Ok(())
        }
    }
} 