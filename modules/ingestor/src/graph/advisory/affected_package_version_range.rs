use crate::graph::advisory::advisory_vulnerability::AdvisoryVulnerabilityContext;
use std::fmt::{Debug, Formatter};
use trustify_entity::affected_package_version_range;

#[derive(Clone)]
pub struct AffectedPackageVersionRangeContext<'g> {
    pub advisory_vulnerability: AdvisoryVulnerabilityContext<'g>,
    pub affected_package_version_range: affected_package_version_range::Model,
}

impl Debug for AffectedPackageVersionRangeContext<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.affected_package_version_range.fmt(f)
    }
}

impl<'g> AffectedPackageVersionRangeContext<'g> {
    pub fn new(
        advisory_vulnerability: &AdvisoryVulnerabilityContext<'g>,
        affected_package_version_range: affected_package_version_range::Model,
    ) -> Self {
        Self {
            advisory_vulnerability: advisory_vulnerability.clone(),
            affected_package_version_range,
        }
    }
}
