use crate::graph::error::Error;
use sea_orm::{ActiveValue::Set, ConnectionTrait, EntityTrait};
use sea_query::OnConflict;
use std::collections::{HashMap, HashSet};
use tracing::instrument;
use trustify_common::{db::chunk::EntityChunkedIter, purl::Purl};
use trustify_entity::{
    package, package_version,
    qualified_package::{self, Qualifiers},
};

/// Creator of PURLs.
#[derive(Default)]
pub struct PurlCreator {
    purls: HashSet<Purl>,
}

impl PurlCreator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, purl: Purl) {
        self.purls.insert(purl);
    }

    #[instrument(skip_all, fields(num_purls = self.purls.len()), err)]
    pub async fn create<'g, C>(self, db: &C) -> Result<(), Error>
    where
        C: ConnectionTrait,
    {
        if self.purls.is_empty() {
            return Ok(());
        }

        // insert all packages

        let mut packages = HashMap::new();
        let mut versions = HashMap::new();
        let mut qualifieds = HashMap::new();

        for purl in self.purls {
            let (package, version, qualified) = purl.uuids();
            packages
                .entry(package)
                .or_insert_with(|| package::ActiveModel {
                    id: Set(package),
                    r#type: Set(purl.ty),
                    namespace: Set(purl.namespace),
                    name: Set(purl.name),
                });

            versions
                .entry(version)
                .or_insert_with(|| package_version::ActiveModel {
                    id: Set(version),
                    package_id: Set(package),
                    version: Set(purl.version.unwrap_or_default()),
                });

            qualifieds
                .entry(qualified)
                .or_insert_with(|| qualified_package::ActiveModel {
                    id: Set(qualified),
                    package_version_id: Set(version),
                    qualifiers: Set(Qualifiers(purl.qualifiers)),
                });
        }

        // insert packages

        for batch in &packages.into_values().chunked() {
            package::Entity::insert_many(batch)
                .on_conflict(
                    OnConflict::columns([package::Column::Id])
                        .do_nothing()
                        .to_owned(),
                )
                .do_nothing()
                .exec(db)
                .await?;
        }

        // insert all package versions

        for batch in &versions.into_values().chunked() {
            package_version::Entity::insert_many(batch)
                .on_conflict(
                    OnConflict::columns([package_version::Column::Id])
                        .do_nothing()
                        .to_owned(),
                )
                .do_nothing()
                .exec(db)
                .await?;
        }

        // insert all qualified packages

        for batch in &qualifieds.into_values().chunked() {
            qualified_package::Entity::insert_many(batch)
                .on_conflict(
                    OnConflict::columns([qualified_package::Column::Id])
                        .do_nothing()
                        .to_owned(),
                )
                .do_nothing()
                .exec(db)
                .await?;
        }

        // return

        Ok(())
    }
}
