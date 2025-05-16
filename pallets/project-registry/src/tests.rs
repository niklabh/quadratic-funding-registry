use crate::{mock::*, Error, Event, CampaignStatus};
use frame_support::{assert_noop, assert_ok, BoundedVec};
use sp_runtime::traits::BadOrigin;

fn make_bounded_string<const N: u32>(s: &str) -> BoundedVec<u8, ConstU32<N>> {
    BoundedVec::try_from(s.as_bytes().to_vec()).unwrap()
}

#[test]
fn create_campaign_works() {
    new_test_ext().execute_with(|| {
        // Arrange
        let owner = 1;
        System::set_block_number(1);
        Timestamp::set_timestamp(100);
        let _ = Balances::deposit_creating(&owner, 1000);

        let name = make_bounded_string::<50>("Test Campaign");
        let desc = make_bounded_string::<1000>("Description");
        let link = Some(make_bounded_string::<200>("https://example.com"));
        
        let metadata = pallet_project_registry::Metadata {
            name,
            description: desc,
            link,
        };

        // Act
        assert_ok!(ProjectRegistry::create_campaign(
            RuntimeOrigin::signed(owner),
            metadata,
            200, // start
            300, // end
            500, // soft_cap
            1000, // hard_cap
        ));

        // Assert
        let campaign = ProjectRegistry::campaigns(0).unwrap();
        assert_eq!(campaign.owner, owner);
        assert_eq!(campaign.status, CampaignStatus::Upcoming);
        assert_eq!(campaign.soft_cap, 500);
        assert_eq!(campaign.hard_cap, 1000);

        System::assert_has_event(RuntimeEvent::ProjectRegistry(Event::CampaignCreated {
            campaign_id: 0,
            owner,
        }));
    });
}

#[test]
fn create_campaign_validates_caps() {
    new_test_ext().execute_with(|| {
        let owner = 1;
        let _ = Balances::deposit_creating(&owner, 1000);
        Timestamp::set_timestamp(100);

        let metadata = pallet_project_registry::Metadata {
            name: make_bounded_string::<50>("Test"),
            description: make_bounded_string::<1000>("Desc"),
            link: None,
        };

        assert_noop!(
            ProjectRegistry::create_campaign(
                RuntimeOrigin::signed(owner),
                metadata.clone(),
                200,
                300,
                1000, // soft_cap > hard_cap
                500,  // hard_cap
            ),
            Error::<Test>::CapsInvalid
        );
    });
}

#[test]
fn contribute_works() {
    new_test_ext().execute_with(|| {
        // Arrange
        let owner = 1;
        let contributor = 2;
        System::set_block_number(1);
        Timestamp::set_timestamp(100);
        let _ = Balances::deposit_creating(&owner, 1000);
        let _ = Balances::deposit_creating(&contributor, 1000);

        let metadata = pallet_project_registry::Metadata {
            name: make_bounded_string::<50>("Test"),
            description: make_bounded_string::<1000>("Desc"),
            link: None,
        };

        assert_ok!(ProjectRegistry::create_campaign(
            RuntimeOrigin::signed(owner),
            metadata,
            50,  // start in past
            300, // end
            500, // soft_cap
            1000, // hard_cap
        ));

        // Act
        assert_ok!(ProjectRegistry::contribute(
            RuntimeOrigin::signed(contributor),
            0, // campaign_id
            200, // amount
        ));

        // Assert
        let campaign = ProjectRegistry::campaigns(0).unwrap();
        assert_eq!(campaign.matched, 200);
        
        System::assert_has_event(RuntimeEvent::ProjectRegistry(Event::ContributionMade {
            campaign_id: 0,
            who: contributor,
            amount: 200,
        }));
    });
}

#[test]
fn cancel_campaign_works() {
    new_test_ext().execute_with(|| {
        // Arrange
        let owner = 1;
        System::set_block_number(1);
        Timestamp::set_timestamp(100);
        let _ = Balances::deposit_creating(&owner, 1000);

        let metadata = pallet_project_registry::Metadata {
            name: make_bounded_string::<50>("Test"),
            description: make_bounded_string::<1000>("Desc"),
            link: None,
        };

        assert_ok!(ProjectRegistry::create_campaign(
            RuntimeOrigin::signed(owner),
            metadata,
            200,
            300,
            500,
            1000,
        ));

        // Act
        assert_ok!(ProjectRegistry::cancel_campaign(
            RuntimeOrigin::signed(owner),
            0, // campaign_id
        ));

        // Assert
        let campaign = ProjectRegistry::campaigns(0).unwrap();
        assert_eq!(campaign.status, CampaignStatus::Cancelled);
        
        System::assert_has_event(RuntimeEvent::ProjectRegistry(Event::CampaignCancelled {
            campaign_id: 0,
        }));
    });
}

#[test]
fn claim_refund_works() {
    new_test_ext().execute_with(|| {
        // Arrange
        let owner = 1;
        let contributor = 2;
        System::set_block_number(1);
        Timestamp::set_timestamp(100);
        let _ = Balances::deposit_creating(&owner, 1000);
        let _ = Balances::deposit_creating(&contributor, 1000);

        let metadata = pallet_project_registry::Metadata {
            name: make_bounded_string::<50>("Test"),
            description: make_bounded_string::<1000>("Desc"),
            link: None,
        };

        assert_ok!(ProjectRegistry::create_campaign(
            RuntimeOrigin::signed(owner),
            metadata,
            50,
            300,
            500,
            1000,
        ));

        assert_ok!(ProjectRegistry::contribute(
            RuntimeOrigin::signed(contributor),
            0,
            200,
        ));

        assert_ok!(ProjectRegistry::cancel_campaign(
            RuntimeOrigin::signed(owner),
            0,
        ));

        // Act
        assert_ok!(ProjectRegistry::claim_refund(
            RuntimeOrigin::signed(contributor),
            0,
        ));

        // Assert
        assert_eq!(
            ProjectRegistry::campaign_contributions(0, contributor),
            0
        );
        
        System::assert_has_event(RuntimeEvent::ProjectRegistry(Event::RefundClaimed {
            campaign_id: 0,
            who: contributor,
            amount: 200,
        }));
    });
}

#[test]
fn update_metadata_works() {
    new_test_ext().execute_with(|| {
        // Arrange
        let owner = 1;
        System::set_block_number(1);
        Timestamp::set_timestamp(100);
        let _ = Balances::deposit_creating(&owner, 1000);

        let metadata = pallet_project_registry::Metadata {
            name: make_bounded_string::<50>("Test"),
            description: make_bounded_string::<1000>("Desc"),
            link: None,
        };

        assert_ok!(ProjectRegistry::create_campaign(
            RuntimeOrigin::signed(owner),
            metadata.clone(),
            200,
            300,
            500,
            1000,
        ));

        let new_metadata = pallet_project_registry::Metadata {
            name: make_bounded_string::<50>("Updated Test"),
            description: make_bounded_string::<1000>("Updated Desc"),
            link: Some(make_bounded_string::<200>("https://test.com")),
        };

        // Act
        assert_ok!(ProjectRegistry::update_metadata(
            RuntimeOrigin::signed(owner),
            0,
            new_metadata.clone(),
        ));

        // Assert
        let campaign = ProjectRegistry::campaigns(0).unwrap();
        assert_eq!(campaign.metadata.name, new_metadata.name);
        assert_eq!(campaign.metadata.description, new_metadata.description);
        assert_eq!(campaign.metadata.link, new_metadata.link);
        
        System::assert_has_event(RuntimeEvent::ProjectRegistry(Event::MetadataUpdated {
            campaign_id: 0,
        }));
    });
}

#[test]
fn lifecycle_transitions_work() {
    new_test_ext().execute_with(|| {
        // Arrange
        let owner = 1;
        let contributor = 2;
        System::set_block_number(1);
        Timestamp::set_timestamp(100);
        let _ = Balances::deposit_creating(&owner, 1000);
        let _ = Balances::deposit_creating(&contributor, 1000);

        let metadata = pallet_project_registry::Metadata {
            name: make_bounded_string::<50>("Test"),
            description: make_bounded_string::<1000>("Desc"),
            link: None,
        };

        assert_ok!(ProjectRegistry::create_campaign(
            RuntimeOrigin::signed(owner),
            metadata,
            50,  // start in past
            150, // end soon
            500, // soft_cap
            1000, // hard_cap
        ));

        assert_ok!(ProjectRegistry::contribute(
            RuntimeOrigin::signed(contributor),
            0,
            600, // Above soft cap
        ));

        // Act - Move time past end
        Timestamp::set_timestamp(200);
        ProjectRegistry::on_initialize(2);

        // Assert
        let campaign = ProjectRegistry::campaigns(0).unwrap();
        assert_eq!(campaign.status, CampaignStatus::Success);
        
        System::assert_has_event(RuntimeEvent::ProjectRegistry(Event::CampaignFinalized {
            campaign_id: 0,
            status: CampaignStatus::Success,
        }));
    });
} 