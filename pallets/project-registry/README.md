# Project Registry Pallet

A Substrate FRAME pallet that implements an on-chain registry for funding campaigns. This pallet enables users to create, manage, and participate in funding campaigns with features like soft/hard caps, time-bound campaigns, and automatic finalization.

## Overview

The Project Registry pallet provides functionality for:

- Creating funding campaigns with metadata and funding goals
- Managing campaign lifecycle (Upcoming → Active → Success/Failed)
- Handling contributions with fund reservation
- Processing refunds for failed/cancelled campaigns
- Automatic campaign finalization based on time and funding goals

## Terminology

- **Campaign**: A funding initiative with metadata, time bounds, and funding targets
- **Soft Cap**: Minimum funding goal that must be met for the campaign to succeed
- **Hard Cap**: Maximum funding that a campaign can accept
- **Deposit**: Required stake from campaign creators to prevent spam
- **Metadata**: Campaign information including name, description, and optional link

## Interface

### Dispatchable Functions

#### Campaign Management
- `create_campaign(metadata, start, end, soft_cap, hard_cap)`: Create a new funding campaign
- `update_metadata(campaign_id, metadata)`: Update campaign metadata (only before start)
- `set_caps(campaign_id, soft_cap, hard_cap)`: Modify funding caps (only before start)
- `cancel_campaign(campaign_id)`: Cancel a campaign (owner or root only)

#### Contribution Handling
- `contribute(campaign_id, amount)`: Contribute funds to an active campaign
- `claim_refund(campaign_id)`: Claim refund from failed/cancelled campaigns

### Storage Items

- `NextCampaignId`: Counter for campaign IDs
- `Campaigns`: Main storage for campaign data
- `ActiveCampaigns`: List of currently active campaign IDs
- `CampaignContributions`: Double map tracking user contributions

### Events

- `CampaignCreated { campaign_id, owner }`
- `MetadataUpdated { campaign_id }`
- `CapsUpdated { campaign_id, soft_cap, hard_cap }`
- `CampaignCancelled { campaign_id }`
- `ContributionMade { campaign_id, who, amount }`
- `CampaignFinalized { campaign_id, status }`
- `RefundClaimed { campaign_id, who, amount }`

### Errors

- `CampaignNotFound`: Campaign ID doesn't exist
- `NotOwner`: Not authorized to perform operation
- `InvalidTimeRange`: Invalid campaign duration
- `CapsInvalid`: Invalid soft/hard cap configuration
- `NotActive`: Campaign not in active state
- `HardCapExceeded`: Contribution would exceed hard cap
- `AlreadyFinalized`: Campaign already ended
- `NoContributionFound`: No contribution to refund
- `TooManyActiveCampaigns`: Active campaign limit reached
- `NotRefundable`: Campaign not in refundable state

## Configuration

The pallet has several configurable parameters:

```rust
pub trait Config: frame_system::Config {
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    type Currency: ReservableCurrency<Self::AccountId>;
    type Timestamp: Time;
    type MaxNameLen: Get<u32>;
    type MaxDescLen: Get<u32>;
    type MaxLinkLen: Get<u32>;
    type MaxActive: Get<u32>;
    type MinimumDeposit: Get<BalanceOf<Self>>;
}
```

### Parameters

- `MaxNameLen`: Maximum length for campaign names (default: 50)
- `MaxDescLen`: Maximum length for campaign descriptions (default: 1000)
- `MaxLinkLen`: Maximum length for campaign links (default: 200)
- `MaxActive`: Maximum number of active campaigns (default: 100)
- `MinimumDeposit`: Required deposit for campaign creation (default: 10 * EXISTENTIAL_DEPOSIT)

## Usage

### Campaign Creation

```rust
// Create campaign metadata
let metadata = Metadata {
    name: b"My Campaign".to_vec().try_into().unwrap(),
    description: b"Campaign description".to_vec().try_into().unwrap(),
    link: Some(b"https://example.com".to_vec().try_into().unwrap()),
};

// Create campaign
ProjectRegistry::create_campaign(
    RuntimeOrigin::signed(account_id),
    metadata,
    start_time,
    end_time,
    soft_cap,
    hard_cap,
)?;
```

### Contributing to a Campaign

```rust
// Contribute to campaign
ProjectRegistry::contribute(
    RuntimeOrigin::signed(account_id),
    campaign_id,
    contribution_amount,
)?;
```

### Claiming Refunds

```rust
// Claim refund from failed/cancelled campaign
ProjectRegistry::claim_refund(
    RuntimeOrigin::signed(account_id),
    campaign_id,
)?;
```

## Security

The pallet implements several security measures:

1. Required deposits for campaign creation
2. Time-bound operations (updates only before start)
3. Owner-only campaign management
4. Fund reservation for contributions
5. Automatic campaign finalization
6. Safe math operations using `saturating_*` methods

## Dependencies

- `frame-support`
- `frame-system`
- `sp-runtime`
- `sp-std`
- `pallet-timestamp`
- `pallet-balances` (or any implementation of `ReservableCurrency`)

## License

MIT-0 