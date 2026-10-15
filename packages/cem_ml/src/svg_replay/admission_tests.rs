use super::*;
use crate::import::svg_presentation::import_svg_presentation_attribute;

fn plan() -> SvgReplayAuthorPlan {
    let tree = crate::import::import_data_bytes(
        b"<svg xmlns='http://www.w3.org/2000/svg'><path fill='url(paint.svg#p)'/></svg>",
        "application/xml",
        "cem",
        "urn:admission-test",
    )
    .unwrap();
    let attr = (0..tree.ast().nodes.len())
        .find_map(|id| {
            let n = tree.node(id as u32)?;
            (n.name.as_ref()?.local_name == "fill").then_some(id as u32)
        })
        .unwrap();
    let owner = tree.node(attr).unwrap().parent.unwrap();
    let source = import_svg_presentation_attribute(tree, owner, attr, 4096).unwrap();
    SvgReplayAuthorPlan::new(
        super::super::SvgReplayPlan::new(
            &[source],
            "https://page.test/",
            "https://page.test/index.html",
            "0123456789abcdef0123456789abcdef",
            Default::default(),
        )
        .unwrap(),
    )
    .unwrap()
}
struct Fake {
    stamp: ProviderStamp,
    other_winner: bool,
    foreign: Option<SvgReplayAuthorPlan>,
    preparation: bool,
    race: bool,
    disconnect: bool,
    bad_value: bool,
    stale_preparation: bool,
    cancel: Option<crate::scheduler::AbortSignal>,
}
impl Fake {
    fn new(owner: &AdmissionOwner) -> Self {
        Self {
            stamp: ProviderStamp::fixture(owner),
            other_winner: false,
            foreign: None,
            preparation: true,
            race: false,
            disconnect: false,
            bad_value: false,
            stale_preparation: false,
            cancel: None,
        }
    }
}
impl OriginProvider for Fake {
    fn snapshot(&mut self) -> Result<ProviderStamp, String> {
        if self.disconnect {
            return Err("disconnected".into());
        }
        Ok(self.stamp.clone())
    }
    fn preparation_current(&mut self) -> Result<(), String> {
        if self.preparation {
            Ok(())
        } else {
            Err("base/policy changed".into())
        }
    }
    fn winner(&mut self, route: &super::super::SvgReplayRoute) -> Result<Winner, String> {
        if self.race {
            self.stamp.revision += 1;
        }
        if self.stale_preparation {
            self.preparation = false;
        }
        if let Some(cancel) = &self.cancel {
            cancel.abort();
        }
        Ok(Winner {
            source: self.foreign.as_ref().map_or_else(
                || route.consumer.clone(),
                |p| p.plan.routes()[0].consumer.clone(),
            ),
            presentation: !self.other_winner,
            computed: if self.bad_value {
                "url(https://page.test/other.svg#p)".into()
            } else {
                format!("url(\"{}\")", route.source_url)
            },
        })
    }
}
fn candidate() -> (AdmissionOwner, AdmissionLease) {
    let p = plan();
    let owner = AdmissionOwner::new(&p, 1).unwrap();
    let lease = owner.candidate(p).unwrap();
    (owner, lease)
}
#[test]
fn admission_retains_owner_and_has_no_delivery_authority() {
    let (owner, mut lease) = candidate();
    assert_eq!(lease.state(), AdmissionState::Candidate);
    let mut provider = Fake::new(&owner);
    lease.admit(&mut provider).unwrap();
    assert_eq!(lease.state(), AdmissionState::Admitted);
    lease.validate(&mut provider).unwrap();
    assert!(lease
        .blockers()
        .contains(&"svg-replay-alias-installation-required"));
    assert!(lease
        .blockers()
        .contains(&"svg-resource-consumption-required"));
    assert!(lease.admit(&mut provider).is_err());
    assert_eq!(lease.state(), AdmissionState::Revoked);
    assert!(lease.plan.is_none());
}
#[test]
fn same_value_wrong_winner_and_foreign_source_reject() {
    for foreign in [false, true] {
        let (owner, mut lease) = candidate();
        let mut provider = Fake::new(&owner);
        if foreign {
            provider.foreign = Some(plan());
        } else {
            provider.other_winner = true;
        }
        assert!(lease.admit(&mut provider).is_err());
        assert_eq!(lease.state(), AdmissionState::Revoked);
    }
}
#[test]
fn unsupported_context_and_preparation_fail_closed() {
    for mode in 0..7 {
        let (owner, mut lease) = candidate();
        let mut p = Fake::new(&owner);
        match mode {
            0 => p.stamp.engine = "other".into(),
            1 => p.stamp.version = "next".into(),
            2 => p.stamp.protocol = 2,
            3 => p.stamp.static_screen = false,
            4 => p.stamp.forced_colors = true,
            5 => p.stamp.generation += 1,
            _ => p.preparation = false,
        }
        assert!(lease.admit(&mut p).is_err());
        assert_eq!(lease.state(), AdmissionState::Revoked);
    }
}
#[test]
fn provider_identity_revision_disconnect_and_base_changes_revoke() {
    for mode in 0..5 {
        let (owner, mut lease) = candidate();
        let mut p = Fake::new(&owner);
        lease.admit(&mut p).unwrap();
        match mode {
            0 => p.stamp.identity = Arc::new(()),
            1 => p.stamp.revision += 2, // A -> B -> A; URLs unchanged
            2 => p.disconnect = true,
            3 => p.preparation = false,
            _ => {
                let other = AdmissionOwner::new(lease.plan.as_ref().unwrap(), 1).unwrap();
                p.stamp.owner = Arc::downgrade(&other.inner);
            }
        }
        assert!(lease.validate(&mut p).is_err());
        assert_eq!(lease.state(), AdmissionState::Revoked);
        assert!(lease.plan.is_none());
        assert!(lease.admit(&mut Fake::new(&owner)).is_err());
    }
}
#[test]
fn races_during_capture_and_read_revoke() {
    for during_read in [false, true] {
        let (owner, mut lease) = candidate();
        let mut p = Fake::new(&owner);
        if during_read {
            lease.admit(&mut p).unwrap();
        }
        p.race = true;
        let result = if during_read {
            lease.validate(&mut p)
        } else {
            lease.admit(&mut p)
        };
        assert!(result.is_err());
        assert_eq!(lease.state(), AdmissionState::Revoked);
    }
}
#[test]
fn owner_generation_document_and_disposal_are_not_transferable() {
    let p = plan();
    let owner = AdmissionOwner::new(&p, 1).unwrap();
    assert!(owner.candidate(plan()).is_err());
    let mut lease = owner.candidate(p).unwrap();
    let mut provider = Fake::new(&owner);
    lease.admit(&mut provider).unwrap();
    owner.invalidate();
    assert_eq!(lease.state(), AdmissionState::Revoked);
    assert!(lease.validate(&mut provider).is_err());
    let (owner, mut lease) = candidate();
    let mut provider = Fake::new(&owner);
    lease.admit(&mut provider).unwrap();
    drop(owner);
    assert_eq!(lease.state(), AdmissionState::Revoked);
    assert!(lease.validate(&mut provider).is_err());
}
#[test]
fn cancellation_before_and_during_observation_revokes() {
    for during in [false, true] {
        let (owner, mut lease) = candidate();
        let mut p = Fake::new(&owner);
        let cancel = lease.plan.as_ref().unwrap().plan.cancel.clone();
        if during {
            p.cancel = Some(cancel);
        } else {
            cancel.abort();
        }
        assert!(lease.admit(&mut p).is_err());
        assert!(lease.plan.is_none());
        drop(owner);
    }
}
#[test]
fn candidate_cannot_be_read_and_closed_lease_cannot_recover() {
    let (owner, mut lease) = candidate();
    assert!(lease.validate(&mut Fake::new(&owner)).is_err());
    assert!(lease.admit(&mut Fake::new(&owner)).is_err());
    let (owner, mut lease) = candidate();
    lease.revoke();
    lease.revoke();
    assert!(lease.admit(&mut Fake::new(&owner)).is_err());
}

#[test]
fn computed_and_post_observation_preparation_failures_revoke() {
    for during_read in [false, true] {
        for post in [false, true] {
            let (owner, mut lease) = candidate();
            let mut p = Fake::new(&owner);
            if during_read {
                lease.admit(&mut p).unwrap();
            }
            p.bad_value = !post;
            p.stale_preparation = post;
            let result = if during_read {
                lease.validate(&mut p)
            } else {
                lease.admit(&mut p)
            };
            assert!(result.is_err());
            assert!(lease.plan.is_none());
        }
    }
}
#[test]
fn recapture_requires_new_owner_and_provider_binding() {
    let p = plan();
    assert!(AdmissionOwner::new(&p, 0).is_err());
    let owner = AdmissionOwner::new(&p, 1).unwrap();
    owner.invalidate();
    assert!(owner.candidate(p).is_err());
    let (owner, mut lease) = candidate();
    let mut provider = Fake::new(&owner);
    lease.admit(&mut provider).unwrap();
    lease.revoke();
    let (next_owner, mut next) = candidate();
    let mut next_provider = Fake::new(&next_owner);
    next.admit(&mut next_provider).unwrap();
    next.validate(&mut next_provider).unwrap();
    assert!(lease.validate(&mut next_provider).is_err());
    assert!(next.validate(&mut provider).is_err());
}

#[test]
fn another_author_plan_in_the_same_document_cannot_reuse_owner() {
    let p = plan();
    let owner = AdmissionOwner::new(&p, 1).unwrap();
    let source = p.plan.routes()[0].consumer.clone();
    let other = SvgReplayAuthorPlan::new(
        super::super::SvgReplayPlan::new(
            &[source],
            "https://page.test/",
            "https://page.test/index.html",
            "1123456789abcdef0123456789abcdef",
            Default::default(),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(owner.candidate(other).is_err());
    assert!(owner.candidate(p).is_ok());
}
