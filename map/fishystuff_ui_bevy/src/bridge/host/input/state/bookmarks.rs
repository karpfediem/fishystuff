use crate::bridge::contract::FishyMapInputState;
use crate::plugins::bookmarks::BookmarkState;

pub(super) fn apply_bookmarks(input: &FishyMapInputState, bookmarks: &mut BookmarkState) {
    let entries_changed = bookmarks.entries != input.ui.bookmarks;
    let selected_changed = bookmarks.selected_ids != input.ui.bookmark_selected_ids;
    if !entries_changed && !selected_changed {
        return;
    }
    bookmarks.entries = input.ui.bookmarks.clone();
    bookmarks.selected_ids = input.ui.bookmark_selected_ids.clone();
}
