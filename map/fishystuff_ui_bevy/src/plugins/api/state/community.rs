use std::collections::HashMap;

use fishystuff_api::models::fish::CommunityFishZoneSupportResponse;

use crate::prelude::*;

#[derive(Resource)]
pub struct CommunityFishZoneSupportIndex {
    pub status: String,
    pub revision: Option<String>,
    by_item_id: HashMap<i32, Vec<u32>>,
}

impl Default for CommunityFishZoneSupportIndex {
    fn default() -> Self {
        Self {
            status: "fish community support: pending".to_string(),
            revision: None,
            by_item_id: HashMap::new(),
        }
    }
}

impl CommunityFishZoneSupportIndex {
    pub fn replace_from_response(&mut self, response: CommunityFishZoneSupportResponse) {
        self.revision = Some(response.revision.clone());
        self.by_item_id.clear();
        self.by_item_id.reserve(response.fish.len());
        for mut entry in response.fish {
            entry.zone_rgbs.sort_unstable();
            entry.zone_rgbs.dedup();
            if entry.zone_rgbs.is_empty() {
                continue;
            }
            self.by_item_id.insert(entry.item_id, entry.zone_rgbs);
        }
        self.status = format!("fish community support: {}", self.by_item_id.len());
    }

    pub fn zone_rgbs_for_item(&self, item_id: i32) -> &[u32] {
        self.by_item_id
            .get(&item_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use fishystuff_api::models::fish::{
        CommunityFishZoneSupportEntry, CommunityFishZoneSupportResponse,
    };

    use super::CommunityFishZoneSupportIndex;

    #[test]
    fn support_index_deduplicates_zone_rgbs() {
        let mut index = CommunityFishZoneSupportIndex::default();
        index.replace_from_response(CommunityFishZoneSupportResponse {
            revision: "community-rev".to_string(),
            fish: vec![CommunityFishZoneSupportEntry {
                item_id: 820240,
                zone_rgbs: vec![3, 1, 3, 2],
            }],
            ..CommunityFishZoneSupportResponse::default()
        });

        assert_eq!(index.revision.as_deref(), Some("community-rev"));
        assert_eq!(index.zone_rgbs_for_item(820240), &[1, 2, 3]);
    }
}
